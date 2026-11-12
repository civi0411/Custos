use async_trait::async_trait;
use custos_core::contracts::harness::{
    AgentRuntimePort, CostVisibility, HarnessProfile, ToolMediationLevel, WorktreeOwnership,
};
use custos_domain::{
    Action, ActionIntent, ActionLifecycleState, ContextPack, DomainError, WorkerRun,
};
use custos_provider::ProviderRequest;

use super::tool::{classify_tool_risk, extract_target, InvocationContext, ToolResult};
use super::writer::{truncate_to_char_boundary, LimitedWriter};
use super::GovernedAgentRuntime;

#[async_trait]
impl AgentRuntimePort for GovernedAgentRuntime {
    fn harness_id(&self) -> &str {
        "governed-agent-runtime"
    }

    fn profile(&self) -> HarnessProfile {
        let tool_mediation = if self.available_tools.is_empty() {
            ToolMediationLevel::ObserveOnly
        } else {
            ToolMediationLevel::CustosMediated
        };
        HarnessProfile {
            harness_id: "governed-agent-runtime".into(),
            tool_mediation,
            worktree_ownership: WorktreeOwnership::SharedLive,
            supports_cancel: true,
            supports_steer: true,
            cost_visibility: CostVisibility::ExactTokens,
        }
    }

    async fn cancel_run(&self, run_id: &str) -> Result<(), DomainError> {
        let mut cancelled = self.cancelled_runs.write().await;
        cancelled.insert(run_id.to_string());
        Ok(())
    }

    async fn steer_run(&self, run_id: &str, guidance: &str) -> Result<(), DomainError> {
        let mut steer = self.steer_messages.write().await;
        steer
            .entry(run_id.to_string())
            .or_default()
            .push(guidance.to_string());
        Ok(())
    }

    async fn execute_turn(
        &self,
        worker_run: &WorkerRun,
        context_pack: &ContextPack,
    ) -> Result<Vec<ActionIntent>, DomainError> {
        let effective_run_id = worker_run
            .run_id
            .clone()
            .unwrap_or_else(|| worker_run.id.clone());

        if self.cancelled_runs.read().await.contains(&effective_run_id) {
            return Err(DomainError::Validation(format!(
                "Run {} was canceled before execution",
                effective_run_id
            )));
        }

        // 1. Build initial prompt from context pack items and system instructions
        let mut prompt_content = String::new();
        prompt_content.push_str(&format!("System: {}\n\n", self.system_prompt));
        prompt_content.push_str(&format!(
            "Task ID: {}\nWorker Role: {}\n\nContext Items:\n",
            worker_run.task_id, worker_run.worker_id
        ));

        // Inject steering guidance if any
        {
            let mut steer_map = self.steer_messages.write().await;
            if let Some(steer_notes) = steer_map.remove(&effective_run_id) {
                prompt_content.push_str("\n--- Steering Guidance ---\n");
                for note in steer_notes {
                    prompt_content.push_str(&format!("- {}\n", note));
                }
                prompt_content.push('\n');
            }
        }

        for item in &context_pack.items {
            prompt_content.push_str(&format!(
                "--- Source: {} ---\n{}\n\n",
                item.source, item.content
            ));
        }

        // Fast path: if no tool executor is configured, preserve single-turn intent extraction
        let executor = match &self.tool_executor {
            Some(exec) if !self.available_tools.is_empty() => exec.clone(),
            _ => {
                let request = ProviderRequest {
                    request_id: custos_domain::new_id("req"),
                    task_id: worker_run.task_id.clone(),
                    span_num: worker_run.attempt_id,
                    prompt: prompt_content,
                    model: self.provider.provider_id().to_string(),
                    provider_account_id: None,
                    reasoning_effort: None,
                    temperature: Some(0.2),
                    max_tokens: Some(4096),
                    tools: if self.available_tools.is_empty() {
                        None
                    } else {
                        Some(self.available_tools.clone())
                    },
                };
                let response = self.provider.generate(&request).await?;
                let mut action_intents = Vec::new();
                for call in response.tool_calls {
                    let risk = classify_tool_risk(&call.name, &call.arguments);
                    let target = extract_target(&call.arguments);

                    let mut intent =
                        Action::new(call.call_id, call.name, target, call.arguments, risk);
                    intent.task_id = Some(worker_run.task_id.clone());
                    intent.lifecycle_state = ActionLifecycleState::Intent;
                    intent.assurance = custos_domain::Assurance::CustosMediated;
                    action_intents.push(intent);
                }
                return Ok(action_intents);
            }
        };

        // Multi-step governed agent loop:
        // Model propose tool -> authority check -> execute -> next turn continuation
        let mut conversation_history = String::new();
        let mut accumulated_intents = Vec::new();
        let mut completed_cleanly = false;
        let mut was_cancelled = false;

        'loop_turns: for iteration in 0..self.max_iterations {
            if self.cancelled_runs.read().await.contains(&effective_run_id) {
                tracing::info!(
                    run_id = %effective_run_id,
                    iteration,
                    "Agent run canceled during multi-turn loop"
                );
                was_cancelled = true;
                break 'loop_turns;
            }

            let mut current_prompt = prompt_content.clone();
            if !conversation_history.is_empty() {
                current_prompt.push_str("\n--- Conversation History & Tool Observations ---\n");
                current_prompt.push_str(&conversation_history);
            }

            let request = ProviderRequest {
                request_id: custos_domain::new_id("req"),
                task_id: worker_run.task_id.clone(),
                span_num: worker_run.attempt_id.saturating_add(iteration as u32),
                prompt: current_prompt,
                model: self.provider.provider_id().to_string(),
                provider_account_id: None,
                reasoning_effort: None,
                temperature: Some(0.2),
                max_tokens: Some(4096),
                tools: Some(self.available_tools.clone()),
            };

            let response = self.provider.generate(&request).await?;
            if !response.content.trim().is_empty() {
                conversation_history.push_str(&format!("\nModel: {}\n", response.content));
            }

            if response.tool_calls.is_empty() {
                tracing::debug!(
                    run_id = %effective_run_id,
                    iteration,
                    "Model completed without additional tool calls"
                );
                completed_cleanly = true;
                break 'loop_turns;
            }

            for call in response.tool_calls {
                if self.cancelled_runs.read().await.contains(&effective_run_id) {
                    tracing::info!(
                        run_id = %effective_run_id,
                        "Agent run canceled before tool execution"
                    );
                    was_cancelled = true;
                    break 'loop_turns;
                }

                let risk = classify_tool_risk(&call.name, &call.arguments);
                let target = extract_target(&call.arguments);
                let mut intent = Action::new(
                    call.call_id.clone(),
                    call.name.clone(),
                    target,
                    call.arguments.clone(),
                    risk,
                );
                intent.task_id = Some(worker_run.task_id.clone());
                intent.lifecycle_state = ActionLifecycleState::Intent;
                intent.assurance = custos_domain::Assurance::CustosMediated;

                conversation_history.push_str(&format!(
                    "\nTool Request [{}] {}: {}\n",
                    call.call_id, call.name, call.arguments
                ));

                let invocation_ctx = InvocationContext {
                    task_id: worker_run.task_id.clone(),
                    run_id: effective_run_id.clone(),
                    actor: worker_run.worker_id.clone(),
                    scope: None,
                };

                // 1. Authority / Gateway check
                match executor
                    .authorize(&invocation_ctx, &call.name, &call.arguments)
                    .await
                {
                    Ok(()) => {
                        if self.cancelled_runs.read().await.contains(&effective_run_id) {
                            intent.lifecycle_state = ActionLifecycleState::Failed;
                            accumulated_intents.push(intent);
                            was_cancelled = true;
                            break 'loop_turns;
                        }

                        // 2. Execute tool
                        match executor
                            .execute(&invocation_ctx, &call.name, &call.arguments)
                            .await
                        {
                            Ok(ToolResult::Success(val)) => {
                                intent.lifecycle_state = ActionLifecycleState::Receipt;
                                let mut writer = LimitedWriter::new(
                                    Vec::with_capacity(self.max_tool_output_bytes.min(8192)),
                                    self.max_tool_output_bytes,
                                );
                                let output_str = if serde_json::to_writer(&mut writer, &val).is_ok()
                                {
                                    let (raw_buf, capped) = writer.into_inner();
                                    let mut s = String::from_utf8_lossy(&raw_buf).into_owned();
                                    if capped {
                                        truncate_to_char_boundary(
                                            &mut s,
                                            self.max_tool_output_bytes,
                                        );
                                        s.push_str("\n... [TRUNCATED: output exceeded max tool output limit]");
                                    }
                                    s
                                } else {
                                    let mut s = val.to_string();
                                    if s.len() > self.max_tool_output_bytes {
                                        truncate_to_char_boundary(
                                            &mut s,
                                            self.max_tool_output_bytes,
                                        );
                                        s.push_str("\n... [TRUNCATED: output exceeded max tool output limit]");
                                    }
                                    s
                                };
                                conversation_history.push_str(&format!(
                                    "Tool Result [{}]: {}\n",
                                    call.call_id, output_str
                                ));
                            }
                            Ok(ToolResult::PendingApproval { attempt_id }) => {
                                intent.lifecycle_state = ActionLifecycleState::Permitted;
                                conversation_history.push_str(&format!(
                                    "Tool Pending Approval [{}]: Attempt ID '{}' requires human authorization before execution.\n",
                                    call.call_id, attempt_id
                                ));
                            }
                            Ok(ToolResult::Error(err_msg)) => {
                                intent.lifecycle_state = ActionLifecycleState::Failed;
                                conversation_history.push_str(&format!(
                                    "Tool Error [{}]: {}\n",
                                    call.call_id, err_msg
                                ));
                            }
                            Err(err) => {
                                intent.lifecycle_state = ActionLifecycleState::Failed;
                                conversation_history
                                    .push_str(&format!("Tool Error [{}]: {}\n", call.call_id, err));
                            }
                        }
                    }
                    Err(rejection_err) => {
                        // Authority / Gateway rejection: clean error without hanging
                        intent.lifecycle_state = ActionLifecycleState::Failed;
                        conversation_history.push_str(&format!(
                            "Tool Rejected [{}] by authority/gateway: {}\n",
                            call.call_id, rejection_err
                        ));
                    }
                }

                accumulated_intents.push(intent);
            }
        }

        // Cleanup steering and cancellation handles for this run
        {
            let mut cancelled = self.cancelled_runs.write().await;
            cancelled.remove(&effective_run_id);
            let mut steer = self.steer_messages.write().await;
            steer.remove(&effective_run_id);
        }

        if was_cancelled {
            return Err(DomainError::Validation(format!(
                "Run {} was canceled during agent loop execution",
                effective_run_id
            )));
        }

        // E2: Iteration exhaustion without clean completion must return BudgetExceeded (limited), never success
        if !completed_cleanly {
            return Err(DomainError::BudgetExceeded(format!(
                "Agent loop iteration limit of {} exceeded without completion (status: limited)",
                self.max_iterations
            )));
        }

        Ok(accumulated_intents)
    }
}
