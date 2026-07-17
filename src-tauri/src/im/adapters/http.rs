use crate::im::runtime::ConnectorRuntimeError;
use async_trait::async_trait;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpMethod {
    Get,
    Post,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpRequest {
    pub method: HttpMethod,
    pub url: String,
    pub headers: BTreeMap<String, String>,
    pub body: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HttpResponse {
    pub status: u16,
    pub body: Value,
}

#[async_trait]
pub trait HttpTransport: Send + Sync {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectorRuntimeError>;
}

pub struct ReqwestHttpTransport {
    client: reqwest::Client,
}

impl ReqwestHttpTransport {
    pub fn new(timeout: std::time::Duration) -> Result<Self, ConnectorRuntimeError> {
        Ok(Self {
            client: crate::network_proxy::http_client(timeout)
                .map_err(|_| ConnectorRuntimeError::new("http-client-init-failed"))?,
        })
    }
}

#[async_trait]
impl HttpTransport for ReqwestHttpTransport {
    async fn execute(&self, request: HttpRequest) -> Result<HttpResponse, ConnectorRuntimeError> {
        let mut builder = match request.method {
            HttpMethod::Get => self.client.get(&request.url),
            HttpMethod::Post => self.client.post(&request.url),
        };
        for (name, value) in request.headers {
            builder = builder.header(name, value);
        }
        if let Some(body) = request.body {
            builder = builder.json(&body);
        }
        let response = builder
            .send()
            .await
            .map_err(|_| ConnectorRuntimeError::new("http-request-failed"))?;
        let status = response.status().as_u16();
        let body = response
            .json::<Value>()
            .await
            .map_err(|_| ConnectorRuntimeError::new("http-response-invalid"))?;
        Ok(HttpResponse { status, body })
    }
}

pub fn require_success(response: &HttpResponse) -> Result<(), ConnectorRuntimeError> {
    if (200..300).contains(&response.status) {
        Ok(())
    } else {
        Err(ConnectorRuntimeError::new(format!(
            "http-status-{}",
            response.status
        )))
    }
}
