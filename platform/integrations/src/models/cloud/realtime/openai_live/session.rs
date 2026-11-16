use std::collections::VecDeque;
use anyhow::{bail, Result};
use tokio::{
    sync::{broadcast, broadcast::error::RecvError},
    time::timeout,
};

use crate::cloud::realtime::live::{LiveSessionEvent, LIVE_EVENT_CHANNEL_CAPACITY};
use super::types::{
    OpenAiLiveCommand, OpenAiLiveConnectionKind, OpenAiLiveEvent, OpenAiLiveEventKind,
    OpenAiLiveSession, OpenAiLiveSessionEvent, OpenAiLiveSessionId, SESSION_START_TIMEOUT,
};

pub struct StartingOpenAiLiveSession {
    pub(crate) session: OpenAiLiveSession,
    pub(crate) events: broadcast::Receiver<OpenAiLiveSessionEvent>,
    pub(crate) pending_events: VecDeque<OpenAiLiveSessionEvent>,
    pub(crate) connection_kind: OpenAiLiveConnectionKind,
}

impl StartingOpenAiLiveSession {
    pub(crate) async fn ready(
        mut self,
        expected_session_id: Option<OpenAiLiveSessionId>,
    ) -> Result<ConnectedOpenAiLiveSession> {
        timeout(SESSION_START_TIMEOUT, async {
            loop {
                match self.events.recv().await {
                    Ok(LiveSessionEvent::Message(OpenAiLiveEvent {
                        kind: OpenAiLiveEventKind::SessionStarted { session_id, .. },
                        ..
                    })) => {
                        if expected_session_id
                            .as_ref()
                            .is_some_and(|expected| expected != &session_id)
                        {
                            bail!("OpenAI Live startup identity does not match creation identity");
                        }
                        return Ok(self.connected(session_id));
                    }
                    Ok(LiveSessionEvent::Message(event))
                        if matches!(event.kind, OpenAiLiveEventKind::Error { .. }) =>
                    {
                        if let OpenAiLiveEventKind::Error { message, .. } = event.kind {
                            bail!("OpenAI Live startup failed: {message}");
                        }
                    }
                    Ok(LiveSessionEvent::Ended { error, .. }) => {
                        if let Some(error) = error {
                            return Err(anyhow::anyhow!(error.to_string()));
                        }
                        bail!("OpenAI Live session ended before startup");
                    }
                    Ok(event @ LiveSessionEvent::Message(_)) => {
                        if self.pending_events.len() >= LIVE_EVENT_CHANNEL_CAPACITY {
                            bail!("too many events received before OpenAI Live session startup");
                        }
                        self.pending_events.push_back(event);
                    }
                    Err(RecvError::Lagged(count)) => {
                        bail!("OpenAI Live authoritative event receiver lagged by {count} events")
                    }
                    Err(RecvError::Closed) => {
                        bail!("OpenAI Live authoritative event stream closed")
                    }
                }
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("OpenAI Live session startup timed out"))?
    }

    pub(crate) fn connected(self, session_id: OpenAiLiveSessionId) -> ConnectedOpenAiLiveSession {
        ConnectedOpenAiLiveSession {
            session: self.session,
            events: self.events,
            pending_events: self.pending_events,
            connection_kind: self.connection_kind,
            session_id,
        }
    }
}

pub struct ConnectedOpenAiLiveSession {
    session: OpenAiLiveSession,
    events: broadcast::Receiver<OpenAiLiveSessionEvent>,
    pending_events: VecDeque<OpenAiLiveSessionEvent>,
    connection_kind: OpenAiLiveConnectionKind,
    session_id: OpenAiLiveSessionId,
}

impl ConnectedOpenAiLiveSession {
    pub async fn recv(&mut self) -> std::result::Result<OpenAiLiveSessionEvent, RecvError> {
        if let Some(event) = self.pending_events.pop_front() {
            return Ok(event);
        }
        self.events.recv().await
    }

    pub fn session_id(&self) -> &OpenAiLiveSessionId {
        &self.session_id
    }

    pub async fn send(&self, command: OpenAiLiveCommand) -> Result<()> {
        if self.connection_kind != OpenAiLiveConnectionKind::PrimaryWebSocket
            && matches!(&command, OpenAiLiveCommand::AppendAudio(_))
        {
            bail!("audio input is only permitted on an OpenAI Live primary WebSocket");
        }
        self.session.send(command).await
    }

    pub async fn close(&self) -> Result<()> {
        self.session.close().await
    }
}
