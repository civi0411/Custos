use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudRole {
    All,
    Relay,
    Push,
    Webhook,
}

impl CloudRole {
    pub fn parse(raw: &str) -> Result<Self, String> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "all" => Ok(Self::All),
            "relay" => Ok(Self::Relay),
            "push" => Ok(Self::Push),
            "webhook" => Ok(Self::Webhook),
            other => Err(format!("Unsupported custos-cloud role '{other}' (expected all|relay|push|webhook)")),
        }
    }

    pub fn enables_relay(self) -> bool {
        matches!(self, Self::All | Self::Relay)
    }

    pub fn enables_push(self) -> bool {
        matches!(self, Self::All | Self::Push)
    }

    pub fn enables_webhook(self) -> bool {
        matches!(self, Self::All | Self::Webhook)
    }
}

#[derive(Debug, Clone)]
pub struct CloudConfig {
    pub role: CloudRole,
    pub bind_addr: String,
}

impl Default for CloudConfig {
    fn default() -> Self {
        Self {
            role: CloudRole::All,
            bind_addr: "127.0.0.1:7878".into(),
        }
    }
}

