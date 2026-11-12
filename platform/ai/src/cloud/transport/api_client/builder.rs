use anyhow::Result;
use reqwest::{
    header::{HeaderMap, HeaderName, HeaderValue},
    Client, Response,
};
use serde_json::Value;

use super::types::{ApiClient, ApiResponse, AuthMethod};

pub struct ApiRequestBuilder<'a> {
    pub(crate) client: &'a ApiClient,
    pub(crate) path: &'a str,
    pub(crate) headers: HeaderMap,
    pub(crate) streaming: bool,
}

impl<'a> ApiRequestBuilder<'a> {
    pub fn header(mut self, key: &str, value: &str) -> Result<Self> {
        let header_name = HeaderName::from_bytes(key.as_bytes())?;
        let header_value = HeaderValue::from_str(value)?;
        self.headers.insert(header_name, header_value);
        Ok(self)
    }

    #[allow(dead_code)]
    pub fn headers(mut self, headers: HeaderMap) -> Self {
        self.headers.extend(headers);
        self
    }

    /// Apply per-request headers from a model config, overriding any static
    /// client headers on key collision.
    pub fn model_headers(self, model_config: &crate::model::ModelConfig) -> Result<Self> {
        match &model_config.request_headers {
            Some(headers) => headers
                .iter()
                .try_fold(self, |builder, (key, value)| builder.header(key, value)),
            None => Ok(self),
        }
    }

    pub fn streaming(mut self, streaming: bool) -> Self {
        self.streaming = streaming;
        self
    }

    pub async fn api_post(self, payload: &Value) -> Result<ApiResponse> {
        let response = self.response_post(payload).await?;
        ApiResponse::from_response(response).await
    }

    async fn send_bounded(&self, request: reqwest::RequestBuilder) -> Result<Response> {
        if self.streaming {
            Ok(crate::cloud::transport::http_status::send_bounded(request, self.client.timeout).await?)
        } else {
            Ok(request.send().await?)
        }
    }

    pub async fn response_post(self, payload: &Value) -> Result<Response> {
        let request = self.send_request(|url, client| client.post(url)).await?;
        self.send_bounded(request.json(payload)).await
    }

    pub async fn multipart_post(self, form: reqwest::multipart::Form) -> Result<Response> {
        let request = self.send_request(|url, client| client.post(url)).await?;
        Ok(request.multipart(form).send().await?)
    }

    pub async fn api_get(self) -> Result<ApiResponse> {
        let response = self.response_get().await?;
        ApiResponse::from_response(response).await
    }

    pub async fn response_get(self) -> Result<Response> {
        let request = self.send_request(|url, client| client.get(url)).await?;
        self.send_bounded(request).await
    }

    pub(crate) async fn send_request<F>(&self, request_builder: F) -> Result<reqwest::RequestBuilder>
    where
        F: FnOnce(url::Url, &Client) -> reqwest::RequestBuilder,
    {
        let url = self.client.build_url(self.path)?;
        let headers = self.headers.clone();
        let mut request = request_builder(url, &self.client.client);
        request = request.headers(headers);

        if !self.streaming {
            request = request.timeout(self.client.timeout);
        }

        if let Some(decorator) = &self.client.request_builder {
            request = decorator(request)?;
        }

        request = match &self.client.auth {
            AuthMethod::NoAuth => request,
            AuthMethod::BearerToken(token) => {
                request.header("Authorization", format!("Bearer {}", token))
            }
            AuthMethod::ApiKey { header_name, key } => request.header(header_name.as_str(), key),
            AuthMethod::Custom(provider) => {
                let (header_name, header_value) = provider.get_auth_header().await?;
                request.header(header_name, header_value)
            }
        };

        Ok(request)
    }
}

