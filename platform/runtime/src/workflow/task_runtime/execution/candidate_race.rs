use super::super::*;

impl TaskRuntime {
    /// Executes an Orca multi-candidate race across isolated model candidates and optional Test Oracle.
    pub async fn execute_candidate_race(
        &self,
        spec: custos_domain::CandidateRaceSpec,
    ) -> Result<custos_domain::CandidateRaceResult, DomainError> {
        let race_id = spec.id.clone();
        let task_id = spec.task_id.clone();
        let candidates_count = spec.candidates.len();

        if candidates_count == 0 {
            return Err(DomainError::Validation(
                "Candidate race requires at least one candidate".into(),
            ));
        }
        if candidates_count > 4 {
            return Err(DomainError::Validation(format!(
                "Candidate race concurrency capped at 4 candidates, got {candidates_count}"
            )));
        }

        let mut futures = Vec::new();

        for candidate in spec.candidates {
            let runtime = self.clone();
            let task_id = task_id.clone();
            let base_workspace_id = spec.base_workspace_id.clone();
            let test_command = spec.test_command.clone();

            futures.push(async move {
                let start_time = std::time::Instant::now();
                let node_id = format!("candidate:{}", candidate.candidate_id);

                // 1. Resolve candidate workspace path if workspace_coordinator is available
                let target_ws_id = candidate
                    .workspace_id
                    .as_deref()
                    .unwrap_or(&base_workspace_id);
                let (worktree_path, workspace_id) = if let Some(ref ws_coord) = runtime.workspace_coordinator {
                    let ws_id = custos_domain::WorkspaceId::new(target_ws_id);
                    if let Ok(Some(ws)) = ws_coord.get_workspace(&ws_id).await {
                        (ws.path.clone(), ws.id.to_string())
                    } else {
                        (".".to_string(), target_ws_id.to_string())
                    }
                } else {
                    (".".to_string(), target_ws_id.to_string())
                };

                // 2. Dispatch the candidate run with scoped node_id
                let cmd = custos_domain::StartRunCommand::new(&task_id, &candidate.candidate_id)
                    .with_node_id(&node_id)
                    .with_model(&candidate.model_id)
                    .with_harness_id(&candidate.harness_id)
                    .with_workspace_id(&workspace_id);

                let run_res = runtime.dispatch_run(cmd, false).await;

                // 3. Optional Test Oracle verification
                let mut test_passed = run_res.is_ok();
                let mut tests_succeeded = if test_passed { 1 } else { 0 };
                let mut tests_failed = if test_passed { 0 } else { 1 };
                let mut test_output = String::new();

                if let Some(ref test_cmd) = test_command {
                    let test_timeout = std::time::Duration::from_secs(spec.timeout_seconds.max(5));
                    let timeout_ms = test_timeout.as_millis() as u64;
                    let exec_fut = async {
                        if let Some(ref runner) = runtime.command_runner {
                            let res = runner
                                .run_shell(
                                    test_cmd,
                                    std::path::Path::new(&worktree_path),
                                    timeout_ms,
                                )
                                .await?;
                            Ok((res.exit_code == 0, res.stdout, res.stderr))
                        } else if let Ok(harness) =
                            runtime.resolve_harness(Some(&candidate.harness_id))
                        {
                            let res = harness
                                .run_native(test_cmd, std::path::Path::new(&worktree_path))
                                .await?;
                            Ok((res.success, res.stdout, res.stderr))
                        } else {
                            Err(DomainError::Validation(format!(
                                "No CommandRunnerPort or harness '{}' configured for test command execution",
                                candidate.harness_id
                            )))
                        }
                    };

                    match tokio::time::timeout(test_timeout, exec_fut).await {
                        Ok(Ok((success, stdout, stderr))) => {
                            test_passed = success;
                            test_output = stdout + "\n" + &stderr;
                            let (p, f) = parse_test_counts(&test_output);
                            if p > 0 || f > 0 {
                                tests_succeeded = p;
                                tests_failed = f;
                            } else if test_passed {
                                tests_succeeded = 1;
                                tests_failed = 0;
                            } else {
                                tests_succeeded = 0;
                                tests_failed = 1;
                            }
                        }
                        Ok(Err(e)) => {
                            test_passed = false;
                            tests_failed = 1;
                            test_output = format!("Failed to run test oracle: {e}");
                        }
                        Err(_) => {
                            test_passed = false;
                            tests_failed = 1;
                            test_output = format!("Test oracle timed out after {}s", spec.timeout_seconds);
                        }
                    }
                }

                // 4. Collect modified files from workspace if workspace_coordinator is available
                let mut modified_files = Vec::new();
                if let Some(ref ws_coord) = runtime.workspace_coordinator {
                    let ws_id = custos_domain::WorkspaceId::new(&workspace_id);
                    if let Ok(dirty) = ws_coord.inspect_dirty(&ws_id).await {
                        modified_files = dirty.modified_files;
                    }
                }

                custos_domain::CandidateExecutionReport {
                    candidate_id: candidate.candidate_id,
                    model_id: candidate.model_id,
                    workspace_id,
                    worktree_path,
                    test_passed,
                    tests_succeeded,
                    tests_failed,
                    test_output: test_output.trim().to_string(),
                    modified_files,
                    total_tokens: 0,
                    execution_duration_ms: start_time.elapsed().as_millis() as u64,
                }
            });
        }

        let candidate_reports = futures::future::join_all(futures).await;

        // 5. Select winner: candidate with test_passed == true, highest test count, and lowest execution duration
        let winner_candidate_id = candidate_reports
            .iter()
            .filter(|r| r.test_passed)
            .max_by(|a, b| {
                a.tests_succeeded
                    .cmp(&b.tests_succeeded)
                    .then_with(|| b.execution_duration_ms.cmp(&a.execution_duration_ms))
            })
            .map(|r| r.candidate_id.clone());

        Ok(custos_domain::CandidateRaceResult {
            race_id,
            task_id,
            winner_candidate_id,
            candidate_reports,
            completed_at: chrono::Utc::now(),
        })
    }
}

pub(super) fn parse_test_counts(output: &str) -> (u32, u32) {
    let mut passed = 0u32;
    let mut failed = 0u32;
    for line in output.lines() {
        let lower = line.to_ascii_lowercase();
        if lower.contains("passed") || lower.contains("failed") {
            let tokens: Vec<&str> = lower.split_whitespace().collect();
            for i in 0..tokens.len() {
                if tokens[i].starts_with("passed") && i > 0 {
                    if let Ok(n) = tokens[i - 1]
                        .trim_matches(|c: char| !c.is_ascii_digit())
                        .parse::<u32>()
                    {
                        passed = passed.max(n);
                    }
                }
                if tokens[i].starts_with("failed") && i > 0 {
                    if let Ok(n) = tokens[i - 1]
                        .trim_matches(|c: char| !c.is_ascii_digit())
                        .parse::<u32>()
                    {
                        failed = failed.max(n);
                    }
                }
            }
        }
    }
    (passed, failed)
}

