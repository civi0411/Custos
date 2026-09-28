use async_stream::stream;
use custos_provider_types::conversation::message::Message;
use futures::Stream;
use futures::StreamExt;
use rmcp::model::{CallToolResult, ServerNotification};
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::sync::mpsc;

pub const DECLINED_RESPONSE: &str = "The user has declined to run this tool. \
    DO NOT attempt to call this tool again. \
    If there are no alternative methods to proceed, clearly explain the situation and STOP.";

pub const CHAT_MODE_TOOL_SKIPPED_RESPONSE: &str =
    "Let the user know the tool call was skipped in chat mode. \
    DO NOT apologize for skipping the tool call. \
    Provide an explanation of what the tool call would do, structured as a plan for the user.";

#[derive(Clone)]
pub struct ToolCallNotificationEmitter {
    sender: mpsc::Sender<ServerNotification>,
}

impl ToolCallNotificationEmitter {
    pub fn new(sender: mpsc::Sender<ServerNotification>) -> Self {
        Self { sender }
    }

    pub fn emit_best_effort(&self, notification: ServerNotification) {
        let _ = self.sender.try_send(notification);
    }
}

/// Context passed through the tool execution pipeline.
#[derive(Clone, Debug)]
pub struct ToolCallContext {
    pub session_id: String,
    pub working_dir: Option<PathBuf>,
    pub tool_call_request_id: Option<String>,
}

impl ToolCallContext {
    pub fn new(
        session_id: impl Into<String>,
        working_dir: Option<PathBuf>,
        tool_call_request_id: Option<String>,
    ) -> Self {
        Self {
            session_id: session_id.into(),
            working_dir,
            tool_call_request_id,
        }
    }

    pub fn working_dir_str(&self) -> Option<&str> {
        self.working_dir.as_ref().and_then(|p| p.to_str())
    }
}

pub enum ToolStreamItem<T> {
    ActionRequired(Message),
    Message(ServerNotification),
    Result(T),
}

pub type ToolStream =
    Pin<Box<dyn Stream<Item = ToolStreamItem<Result<CallToolResult, String>>> + Send>>;

pub fn create_tool_stream<S, A, F>(rx: S, action_required_rx: A, done: F) -> ToolStream
where
    S: Stream<Item = ServerNotification> + Send + Unpin + 'static,
    A: Stream<Item = Message> + Send + Unpin + 'static,
    F: Future<Output = Result<CallToolResult, String>> + Send + 'static,
{
    Box::pin(stream! {
        tokio::pin!(done);
        let mut rx = rx;
        let mut action_required_rx = action_required_rx;

        loop {
            tokio::select! {
                Some(msg) = action_required_rx.next() => {
                    yield ToolStreamItem::ActionRequired(msg);
                }
                Some(msg) = rx.next() => {
                    yield ToolStreamItem::Message(msg);
                }
                r = &mut done => {
                    yield ToolStreamItem::Result(r);
                    break;
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tool_stream_yields_result() {
        let (rx_tx, rx) = mpsc::channel(1);
        let (_act_tx, act_rx) = mpsc::channel(1);
        drop(rx_tx); // close rx

        let rx_stream = tokio_stream::wrappers::ReceiverStream::new(rx);
        let act_stream = tokio_stream::wrappers::ReceiverStream::new(act_rx);

        let done = async {
            Ok(CallToolResult::success(vec![
                rmcp::model::ContentBlock::text("done"),
            ]))
        };

        let mut stream = create_tool_stream(rx_stream, act_stream, done);
        let item = stream.next().await.unwrap();
        match item {
            ToolStreamItem::Result(Ok(res)) => {
                let text = res.content[0].as_text().unwrap();
                assert_eq!(text.text, "done");
            }
            _ => panic!("Expected result item"),
        }
    }
}
