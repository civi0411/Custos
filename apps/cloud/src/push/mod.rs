use custos_protocol::relay::{
    PushDeliveryCounters, ZeroPiiPushAdapter, ZeroPiiPushCommand, ZeroPiiPushGateway,
};
use std::sync::Arc;

pub struct NoopPushAdapter;

#[async_trait::async_trait]
impl ZeroPiiPushAdapter for NoopPushAdapter {
    async fn send(&self, _cmd: &ZeroPiiPushCommand) -> Result<(), String> {
        Ok(())
    }
}

pub struct PushService {
    gateway: ZeroPiiPushGateway,
}

impl PushService {
    pub fn new(adapter: Arc<dyn ZeroPiiPushAdapter>) -> Self {
        Self {
            gateway: ZeroPiiPushGateway::new(adapter),
        }
    }

    pub async fn notify(&self, cmd: ZeroPiiPushCommand) -> Result<(), String> {
        self.gateway.dispatch(&cmd).await
    }

    pub fn counters(&self) -> &PushDeliveryCounters {
        &self.gateway.counters
    }
}

