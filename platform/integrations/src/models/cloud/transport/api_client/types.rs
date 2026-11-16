use std::fmt;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    header::HeaderMap,
    Client, Response, StatusCode,
};
use serde_json::Value;

use super::tls::TlsConfig;

pub const DEFAULT_PROVIDER_TIMEOUT_SECS: u64 = 600;
pub const DEFAULT_CONNECT_TIMEOUT_SECS: u64 = 30;

pub type RequestBuilderDecorator =
    Arc<dyn Fn(reqwest::RequestBuilder) -> Result<reqwest::RequestBuilder> + Send + Sync>;

#[derive(Clone)]
pub(crate) enum TransportPolicy {
    Default,
    HttpsOnly,
    LoopbackHttp,
    SameOrigin(url::Origin),
}

pub enum AuthMethod {
    NoAuth,
    BearerToken(String),
    ApiKey { header_name: String, key: String },
    Custom(Box<dyn AuthProvider>),
}

impl fmt::Debug for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::NoAuth => f.debug_tuple("NoAuth").finish(),
            AuthMethod::BearerToken(_) => f.debug_tuple("BearerToken").field(&"[hidden]").finish(),
            AuthMethod::ApiKey { header_name, .. } => f
                .debug_struct("ApiKey")
                .field("header_name", header_name)
                .field("key", &"[hidden]")
                .finish(),
            AuthMethod::Custom(_) => f.debug_tuple("Custom").field(&"[provider]").finish(),
        }
    }
}

#[async_trait]
pub trait AuthProvider: Send + Sync {
    async fn get_auth_header(&self) -> Result<(String, String)>;

    async fn refresh_credentials(&self) -> Result<()> {
        anyhow::bail!("credential refresh not supported")
    }
}

pub struct ApiResponse {
    pub status: StatusCode,
    pub payload: Option<Value>,
}

impl ApiResponse {
    pub async fn from_response(response: Response) -> Result<Self> {
        let status = response.status();
        let payload = response.json().await.ok();
        Ok(Self { status, payload })
    }
}

pub struct ApiClient {
    pub(crate) client: Client,
    pub(crate) host: String,
    pub(crate) auth: AuthMethod,
    pub(crate) default_headers: HeaderMap,
    pub(crate) default_query: Vec<(String, String)>,
    pub(crate) timeout: Duration,
    pub(crate) tls_config: Option<TlsConfig>,
    pub(crate) request_builder: Option<RequestBuilderDecorator>,
    pub(crate) transport_policy: TransportPolicy,
}

impl fmt::Debug for ApiClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ApiClient")
            .field("host", &self.host)
            .field("auth", &"[auth method]")
            .field("timeout", &self.timeout)
            .field("default_headers", &self.default_headers)
            .finish_non_exhaustive()
    }
}

