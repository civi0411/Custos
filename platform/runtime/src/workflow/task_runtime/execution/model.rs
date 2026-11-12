use super::super::*;
use custos_core::contracts::harness::RunObserver;
use custos_domain::{new_id, SessionId, StartRunCommand};
use custos_provider::request::ProviderRequest;

impl TaskRuntime {
    pub(super) async fn execute_model_turn(
        &self,
        cmd: &StartRunCommand,
        run: &mut Run,
        wrun: &mut WorkerRun,
        requested_mode: &str,
        observer: &Arc<dyn RunObserver>,
    ) -> Result<Option<String>, DomainError> {
        let mut turn_output: Option<String> = None;
        if let Some(ref model) = self.model {
            let mut launch = custos_domain::LaunchAttempt::new(&wrun.id, requested_mode)?;
            let mut prompt_text = cmd.prompt.clone().unwrap_or_else(|| {
                format!("Initial run turn for task {}", cmd.task_id)
            });
            let target_model = cmd
                .model
                .as_deref()
                .filter(|m| !m.trim().is_empty())
                .unwrap_or_else(|| model.provider_id());

            if let Some(ref store) = self.run_store {
                store.save_launch_attempt(&launch).await?;
            }
            run.metadata["launch_attempt_id"] = serde_json::json!(launch.id);
            wrun.continuation_packet_ref = Some(format!("launch:{}", launch.id));

            if let (Some(session_id), Some(session_store)) =
                (cmd.session_id.as_ref(), self.session_store.as_ref())
            {
                let session_id = SessionId::new(session_id.clone());
                if session_store.get_session(&session_id).await?.is_none() {
                    return Err(DomainError::Validation(format!(
                        "Unknown session '{}' for model run",
                        session_id
                    )));
                }
                let journal = session_store.get_journal(&session_id).await?;
                let compiled = crate::context::compile_session_history(
                    &journal,
                    &prompt_text,
                    12_000,
                );
                run.metadata["session_context"] = serde_json::json!({
                    "compiler_version": "journal-recent-v1",
                    "session_id": session_id.0,
                    "included_entry_ids": compiled.included_entry_ids,
                    "omitted_entry_ids": compiled.omitted_entry_ids,
                    "estimated_tokens": compiled.estimated_tokens,
                    "destination_model": target_model,
                });
                prompt_text = compiled.prompt;
            }

            let mut req = ProviderRequest::new(
                new_id("req"),
                &cmd.task_id,
                1,
                prompt_text,
                target_model,
            );
            req.reasoning_effort = run
                .metadata
                .get("reasoning_effort")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            req.provider_account_id = run
                .metadata
                .get("provider_account_id")
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let mut receiver = match model.stream(&req).await {
                Ok(rx) => {
                    launch.mark_launched("model", Some(target_model.to_string()))?;
                    if let Some(ref store) = self.run_store {
                        store.save_launch_attempt(&launch).await?;
                    }
                    rx
                }
                Err(err) => {
                    launch.mark_failed(err.to_string())?;
                    if let Some(ref store) = self.run_store {
                        let _ = store.save_launch_attempt(&launch).await;
                    }
                    return Err(err);
                }
            };
            let mut content = String::new();
            let mut usage = custos_provider::TokenUsage::default();
            let mut completed = false;
            let mut actual_model = target_model.to_string();
            let mut tokens_used = None;
            while let Some(event) = receiver.recv().await {
                use custos_provider::ProviderEventType;
                match event.event_type {
                    ProviderEventType::Chunk => {
                        if let Some(text) = event.text {
                            if content.len().saturating_add(text.len()) > 2 * 1024 * 1024 {
                                return Err(DomainError::Validation(
                                    "Model output exceeded live limit".into(),
                                ));
                            }
                            observer.emit("status", serde_json::json!({"status":"running"}));
                            content.push_str(&text);
                            observer.emit("text_delta", serde_json::json!({"text":text}));
                        }
                    }
                    ProviderEventType::Thinking => {
                        observer.emit("status", serde_json::json!({"status":"thinking"}));
                    }
                    ProviderEventType::ReasoningSummary => {
                        observer.emit("status", serde_json::json!({"status":"thinking"}));
                        observer.emit("reasoning_summary", serde_json::json!({"text":event.text}));
                    }
                    ProviderEventType::ToolCall => {
                        observer.emit("status", serde_json::json!({"status":"tool-running"}));
                        observer.emit("tool_input", serde_json::json!({"calls":event.tool_calls}));
                        if let Some(ref calls) = event.tool_calls {
                            let now_ms = chrono::Utc::now().timestamp_millis();
                            for call in calls {
                                self.supervision.record_span(custos_domain::supervision::RunStepSpan {
                                    id: new_id("step"),
                                    run_id: run.id.clone(),
                                    worker_run_id: Some(wrun.id.clone()),
                                    parent_id: None,
                                    kind: custos_domain::supervision::StepSpanKind::Tool,
                                    name: call.name.clone(),
                                    status: custos_domain::supervision::StepSpanStatus::Success,
                                    started_at_ms: now_ms,
                                    ended_at_ms: Some(now_ms),
                                    duration_ms: Some(0),
                                    metadata: serde_json::json!({ "call_id": call.call_id, "arguments": call.arguments }),
                                });
                            }
                        }
                    }
                    ProviderEventType::ToolOutput => {
                        observer.emit("tool_output", serde_json::json!({"text":event.text}));
                    }
                    ProviderEventType::Usage => {}
                    ProviderEventType::Completed => {
                        if let Some(text) = event.text {
                            content = text;
                        }
                        if let Some(model) = event.model_id {
                            actual_model = model;
                        }
                        tokens_used = event.tokens_used;
                        completed = true;
                    }
                    ProviderEventType::Failed => {
                        let msg = event.error.unwrap_or_else(|| "Provider stream failed".into());
                        let now_ms = chrono::Utc::now().timestamp_millis();
                        let (_, sig) = custos_domain::supervision::compute_error_signature(
                            "ProviderStreamError",
                            &msg,
                        );
                        self.supervision.record_error(custos_domain::supervision::StepError {
                            id: new_id("serr"),
                            run_id: run.id.clone(),
                            worker_run_id: Some(wrun.id.clone()),
                            span_id: None,
                            error_type: "ProviderStreamError".into(),
                            message: msg.clone(),
                            stack: None,
                            agent: target_model.to_string(),
                            signature_hash: sig,
                            timestamp_ms: now_ms,
                        });
                        return Err(DomainError::Validation(msg));
                    }
                }
                if let Some(measured) = event.usage {
                    if measured.input_tokens.is_some() {
                        usage.input_tokens = measured.input_tokens;
                    }
                    if measured.output_tokens.is_some() {
                        usage.output_tokens = measured.output_tokens;
                    }
                    observer.emit(
                        "usage",
                        serde_json::json!({
                            "input_tokens": usage.input_tokens,
                            "output_tokens": usage.output_tokens,
                            "total_tokens": usage.input_tokens.zip(usage.output_tokens).map(|(a, b)| a.saturating_add(b)),
                        }),
                    );
                }
                if completed {
                    break;
                }
            }
            if !completed {
                return Err(DomainError::Validation(
                    "Provider stream ended without completion".into(),
                ));
            }
            let known_total = usage
                .input_tokens
                .zip(usage.output_tokens)
                .map(|(input, output)| input.saturating_add(output));
            let resp = custos_provider::ModelResponse::text(
                content,
                actual_model,
                tokens_used
                    .or(known_total)
                    .or(usage.output_tokens)
                    .unwrap_or(0),
            )
            .with_usage(usage.input_tokens, usage.output_tokens, None);

            observer.emit(
                "usage",
                serde_json::json!({
                    "input_tokens": resp.input_tokens,
                    "output_tokens": resp.output_tokens,
                    "total_tokens": resp.total_tokens,
                }),
            );
            run.metadata["usage"] = serde_json::json!({
                "input_tokens": resp.input_tokens,
                "output_tokens": resp.output_tokens,
                "total_tokens": resp.total_tokens,
            });
            run.metadata["output"] = serde_json::json!(resp.content);
            run.metadata["tokens_used"] = serde_json::json!(resp.tokens_used);
            run.metadata["model_id"] = serde_json::json!(resp.model_id);
            turn_output = Some(resp.content);
            if let Some(ref store) = self.run_store {
                store.save_run(run).await?;
                store.save_worker_run(wrun).await?;
            }
        }
        Ok(turn_output)
    }
}
