use std::time::Duration;

use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    redirect::Policy,
    Client, Response,
};
use serde_json::Value;
use url::Host;

use super::builder::ApiRequestBuilder;
use super::tls::TlsConfig;
use super::types::{
    ApiClient, ApiResponse, AuthMethod, RequestBuilderDecorator, TransportPolicy,
    DEFAULT_CONNECT_TIMEOUT_SECS, DEFAULT_PROVIDER_TIMEOUT_SECS,
};

impl ApiClient {
    pub fn new_with_tls(
        host: String,
        auth: AuthMethod,
        tls_config: Option<TlsConfig>,
    ) -> Result<Self> {
        Self::with_timeout_and_tls(
            host,
            auth,
            Duration::from_secs(DEFAULT_PROVIDER_TIMEOUT_SECS),
            tls_config,
        )
    }

    pub fn with_timeout_and_tls(
        host: String,
        auth: AuthMethod,
        timeout: Duration,
        tls_config: Option<TlsConfig>,
    ) -> Result<Self> {
        let mut client_builder = Self::client_builder(timeout);

        if let Some(ref config) = tls_config {
            client_builder = Self::configure_tls(client_builder, config)?;
        }

        let client = client_builder.build()?;

        Ok(Self {
            client,
            host,
            auth,
            default_headers: HeaderMap::new(),
            default_query: Vec::new(),
            timeout,
            tls_config,
            request_builder: None,
            transport_policy: TransportPolicy::Default,
        })
    }

    pub fn host(&self) -> &str {
        &self.host
    }

    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    pub fn default_header(&self, name: &str) -> Option<&str> {
        self.default_headers
            .get(name)
            .and_then(|value| value.to_str().ok())
    }

    pub(crate) fn client_builder(timeout: Duration) -> reqwest::ClientBuilder {
        Client::builder()
            .connect_timeout(Duration::from_secs(DEFAULT_CONNECT_TIMEOUT_SECS))
            .read_timeout(timeout)
    }

    fn rebuild_client(&mut self) -> Result<()> {
        let mut client_builder =
            Self::client_builder(self.timeout).default_headers(self.default_headers.clone());
        client_builder = Self::configure_transport(client_builder, &self.transport_policy);

        if let Some(ref tls_config) = self.tls_config {
            client_builder = Self::configure_tls(client_builder, tls_config)?;
        }

        self.client = client_builder.build()?;
        Ok(())
    }

    fn configure_transport(
        client_builder: reqwest::ClientBuilder,
        transport_policy: &TransportPolicy,
    ) -> reqwest::ClientBuilder {
        match transport_policy {
            TransportPolicy::Default => client_builder,
            TransportPolicy::HttpsOnly => client_builder.https_only(true),
            TransportPolicy::LoopbackHttp => {
                client_builder
                    .no_proxy()
                    .redirect(Policy::custom(|attempt| {
                        if attempt.previous().len() > 10 {
                            return attempt.error("too many redirects");
                        }
                        let url = attempt.url();
                        let is_loopback_http = url.scheme() == "http"
                            && match url.host() {
                                Some(Host::Domain(host)) => host.eq_ignore_ascii_case("localhost"),
                                Some(Host::Ipv4(address)) => address.is_loopback(),
                                Some(Host::Ipv6(address)) => address.is_loopback(),
                                None => false,
                            };
                        if url.scheme() == "https" || is_loopback_http {
                            attempt.follow()
                        } else {
                            attempt.error("redirect violates the loopback transport policy")
                        }
                    }))
            }
            TransportPolicy::SameOrigin(origin) => {
                let origin = origin.clone();
                client_builder.redirect(Policy::custom(move |attempt| {
                    if attempt.previous().len() >= 10 {
                        return attempt.error("too many redirects");
                    }
                    if attempt.url().origin() == origin {
                        attempt.follow()
                    } else {
                        attempt.error("redirect crosses the authenticated request origin")
                    }
                }))
            }
        }
    }

    #[cfg(any(feature = "rustls-tls", feature = "native-tls"))]
    fn configure_tls(
        mut client_builder: reqwest::ClientBuilder,
        tls_config: &TlsConfig,
    ) -> Result<reqwest::ClientBuilder> {
        if tls_config.is_configured() {
            if let Some(identity) = tls_config.load_identity()? {
                client_builder = client_builder.identity(identity);
            }

            let ca_certs = tls_config.load_ca_certificates()?;
            for ca_cert in ca_certs {
                client_builder = client_builder.add_root_certificate(ca_cert);
            }
        }
        Ok(client_builder)
    }

    #[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
    fn configure_tls(
        client_builder: reqwest::ClientBuilder,
        tls_config: &TlsConfig,
    ) -> Result<reqwest::ClientBuilder> {
        if tls_config.is_configured() {
            return Err(anyhow::anyhow!(
                "Custom TLS configuration requires the `rustls-tls` or `native-tls` feature"
            ));
        }
        Ok(client_builder)
    }

    pub fn with_headers(mut self, headers: HeaderMap) -> Result<Self> {
        self.default_headers = headers;
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn with_query(mut self, params: Vec<(String, String)>) -> Self {
        self.default_query = params;
        self
    }

    pub fn with_header(mut self, key: &str, value: &str) -> Result<Self> {
        let header_name = HeaderName::from_bytes(key.as_bytes())?;
        let header_value = HeaderValue::from_str(value)?;
        self.default_headers.insert(header_name, header_value);
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn with_request_builder(mut self, request_builder: RequestBuilderDecorator) -> Self {
        self.request_builder = Some(request_builder);
        self
    }

    pub fn with_auth(mut self, auth: AuthMethod) -> Self {
        self.auth = auth;
        self
    }

    pub fn with_https_only(mut self) -> Result<Self> {
        self.transport_policy = TransportPolicy::HttpsOnly;
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn with_loopback_http_only(mut self) -> Result<Self> {
        self.transport_policy = TransportPolicy::LoopbackHttp;
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn with_same_origin_redirects(mut self) -> Result<Self> {
        let origin = url::Url::parse(&self.host)
            .map_err(|error| anyhow::anyhow!("Invalid base URL: {}", error))?
            .origin();
        self.transport_policy = TransportPolicy::SameOrigin(origin);
        self.rebuild_client()?;
        Ok(self)
    }

    pub fn request<'a>(&'a self, path: &'a str) -> ApiRequestBuilder<'a> {
        ApiRequestBuilder {
            client: self,
            path,
            headers: HeaderMap::new(),
            streaming: false,
        }
    }

    pub async fn refresh_credentials(&self) -> Result<()> {
        match &self.auth {
            AuthMethod::Custom(provider) => provider.refresh_credentials().await,
            _ => anyhow::bail!("credential refresh not supported"),
        }
    }

    pub async fn api_post(&self, path: &str, payload: &Value) -> Result<ApiResponse> {
        self.request(path).api_post(payload).await
    }

    pub async fn response_post(&self, path: &str, payload: &Value) -> Result<Response> {
        self.request(path).response_post(payload).await
    }

    pub async fn api_get(&self, path: &str) -> Result<ApiResponse> {
        self.request(path).api_get().await
    }

    pub async fn response_get(&self, path: &str) -> Result<Response> {
        self.request(path).response_get().await
    }

    pub(crate) fn build_url(&self, path: &str) -> Result<url::Url> {
        use url::Url;
        let mut base_url =
            Url::parse(&self.host).map_err(|e| anyhow::anyhow!("Invalid base URL: {}", e))?;

        let base_path = base_url.path();
        if !base_path.is_empty() && base_path != "/" && !base_path.ends_with('/') {
            base_url.set_path(&format!("{}/", base_path));
        }

        let mut url = base_url
            .join(path)
            .map_err(|e| anyhow::anyhow!("Failed to construct URL: {}", e))?;

        for (key, value) in &self.default_query {
            url.query_pairs_mut().append_pair(key, value);
        }

        Ok(url)
    }
}

