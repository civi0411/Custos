use super::commands;
use super::{ApiRequest, ApiResponse, LocalApiDispatcher};

impl LocalApiDispatcher {
    pub async fn handle_request(&self, req: ApiRequest) -> ApiResponse {
        let method_kind = commands::classify_method(&req.method);
        let command_id = req
            .params
            .get("command_id")
            .and_then(|v| v.as_str())
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string());

        let (admitted_cmd_id, payload_digest) = if method_kind == commands::MethodKind::Mutation {
            let cmd_id = command_id
                .clone()
                .unwrap_or_else(|| custos_domain::new_id("cmd"));
            let digest = commands::compute_command_fingerprint(&req.params);
            (Some(cmd_id), Some(digest))
        } else {
            (command_id.clone(), None)
        };

        if let (Some(cmd_id), Some(digest), Some(store)) = (
            admitted_cmd_id.as_deref(),
            payload_digest.as_deref(),
            self.kernel.sqlite_store.as_ref(),
        ) {
            let now_ms = chrono::Utc::now().timestamp_millis();
            let ttl_ms = 7 * 24 * 60 * 60 * 1000; // 7 days
            let scope = req.method.split('.').nth(1);

            match store
                .command_ledger()
                .admit(cmd_id, &req.method, digest, scope, now_ms, ttl_ms)
            {
                Ok(custos_core::contracts::storage::CommandAdmission::New) => {}
                Ok(custos_core::contracts::storage::CommandAdmission::Replay(result_json)) => {
                    let val: serde_json::Value = match serde_json::from_str(&result_json) {
                        Ok(value) => value,
                        Err(error) => {
                            return ApiResponse::error(
                                req.id,
                                format!("Stored command result is corrupt: {error}"),
                            )
                        }
                    };
                    if val.get("custos_command_response") == Some(&serde_json::json!(1)) {
                        return match serde_json::from_value::<ApiResponse>(val["response"].clone())
                        {
                            Ok(mut response) => {
                                response.id = req.id;
                                response
                            }
                            Err(error) => ApiResponse::error(
                                req.id,
                                format!("Stored command response is corrupt: {error}"),
                            ),
                        };
                    }
                    return ApiResponse::success(req.id, val);
                }
                Ok(custos_core::contracts::storage::CommandAdmission::InFlight) => {
                    return ApiResponse::error(
                        req.id,
                        format!("Command ID '{cmd_id}' is currently in-flight"),
                    );
                }
                Ok(custos_core::contracts::storage::CommandAdmission::Conflict(msg)) => {
                    return ApiResponse::error(req.id, msg);
                }
                Err(e) => {
                    tracing::error!(command_id = %cmd_id, error = %e, "Command admission failed; mutation denied");
                    return ApiResponse::error(req.id, format!("Command admission failed: {e}"));
                }
            }
        }

        let resp = self.dispatch_application(req).await;

        if let (Some(cmd_id), Some(store)) =
            (admitted_cmd_id.as_deref(), self.kernel.sqlite_store.as_ref())
        {
            // A handler error may follow an external effect; never make it retryable by deleting admission.
            let result_json =
                serde_json::json!({"custos_command_response": 1, "response": &resp}).to_string();
            let mut settlement_res = store.command_ledger().complete(cmd_id, &result_json);
            if settlement_res.is_err() {
                for delay_ms in [5, 25, 125] {
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    settlement_res = store.command_ledger().complete(cmd_id, &result_json);
                    if settlement_res.is_ok() {
                        break;
                    }
                }
            }
            if let Err(e) = settlement_res {
                tracing::error!(command_id = %cmd_id, error = %e, "Command settlement failed after retries; admission remains fenced");
                return ApiResponse::error(resp.id, format!("Command '{cmd_id}' may have taken effect, but its result could not be persisted: {e}. Do not retry with a new command ID; reconcile its state first."));
            }
        }

        resp
    }
}
