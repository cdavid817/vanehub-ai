use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, OnePieceDiscoveredModel, OnePieceModelDiscoveryPort,
    OnePieceModelDiscoveryRequest, ProviderCredentialProbeAuthentication,
    ProviderCredentialProbeProtocol, ProviderCredentialProbeRequest,
    ProviderCredentialValidationResult, ProviderCredentialValidationStatus,
};
use crate::platform::network::{
    blocking_no_redirect_http_client, probe_provider_credential,
    ProviderCredentialProbeAuthentication as NetworkProbeAuthentication,
    ProviderCredentialProbeProtocol as NetworkProbeProtocol,
    ProviderCredentialProbeRequest as NetworkProbeRequest,
    ProviderCredentialValidationStatus as NetworkValidationStatus,
};
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION};
use serde::Deserialize;
use std::io::Read;
use std::time::Duration;

const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;

pub(crate) struct HttpOnePieceModelDiscoveryAdapter;

#[derive(Deserialize)]
struct ModelListEnvelope {
    data: Vec<ModelEntry>,
}

#[derive(Deserialize)]
struct ModelEntry {
    id: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
}

impl OnePieceModelDiscoveryPort for HttpOnePieceModelDiscoveryAdapter {
    fn list_models(
        &self,
        request: OnePieceModelDiscoveryRequest,
    ) -> Result<Vec<OnePieceDiscoveredModel>, AgentRuntimeApplicationError> {
        if !request.url.starts_with("https://") {
            return Err(discovery_error("model endpoint must use HTTPS"));
        }
        let client = blocking_no_redirect_http_client(Duration::from_secs(15))
            .map_err(|error| discovery_error(&error.to_string()))?;
        let headers = discovery_headers(&request.strategy, &request.api_key)?;
        let builder = client.get(&request.url).headers(headers);
        let response = builder
            .send()
            .map_err(|error| discovery_error(&error.to_string()))?;
        if !response.status().is_success() {
            return Err(discovery_error(&format!(
                "provider returned HTTP {}",
                response.status().as_u16()
            )));
        }
        if response
            .content_length()
            .is_some_and(|size| size > MAX_RESPONSE_BYTES)
        {
            return Err(discovery_error("provider response is too large"));
        }
        let mut body = Vec::new();
        response
            .take(MAX_RESPONSE_BYTES + 1)
            .read_to_end(&mut body)
            .map_err(|error| discovery_error(&error.to_string()))?;
        if body.len() as u64 > MAX_RESPONSE_BYTES {
            return Err(discovery_error("provider response is too large"));
        }
        parse_models(&request.strategy, &body)
    }

    fn validate_credential(
        &self,
        request: ProviderCredentialProbeRequest,
    ) -> Result<ProviderCredentialValidationResult, AgentRuntimeApplicationError> {
        let result = probe_provider_credential(NetworkProbeRequest {
            base_url: &request.base_url,
            model: &request.model,
            protocol: match request.protocol {
                ProviderCredentialProbeProtocol::AnthropicMessages => {
                    NetworkProbeProtocol::AnthropicMessages
                }
                ProviderCredentialProbeProtocol::OpenAiChatCompletions => {
                    NetworkProbeProtocol::OpenAiChatCompletions
                }
                ProviderCredentialProbeProtocol::OpenAiResponses => {
                    NetworkProbeProtocol::OpenAiResponses
                }
            },
            authentication: match request.authentication {
                ProviderCredentialProbeAuthentication::AnthropicApiKey => {
                    NetworkProbeAuthentication::AnthropicApiKey
                }
                ProviderCredentialProbeAuthentication::Bearer => NetworkProbeAuthentication::Bearer,
            },
            credential: &request.credential,
        })
        .map_err(|error| discovery_error(&error.to_string()))?;
        Ok(ProviderCredentialValidationResult {
            status: match result.status {
                NetworkValidationStatus::Valid => ProviderCredentialValidationStatus::Valid,
                NetworkValidationStatus::InvalidCredential => {
                    ProviderCredentialValidationStatus::InvalidCredential
                }
                NetworkValidationStatus::ConfigurationRejected => {
                    ProviderCredentialValidationStatus::ConfigurationRejected
                }
                NetworkValidationStatus::RateLimited => {
                    ProviderCredentialValidationStatus::RateLimited
                }
                NetworkValidationStatus::ProviderUnavailable => {
                    ProviderCredentialValidationStatus::ProviderUnavailable
                }
                NetworkValidationStatus::Unsupported => {
                    ProviderCredentialValidationStatus::Unsupported
                }
                NetworkValidationStatus::Inconclusive => {
                    ProviderCredentialValidationStatus::Inconclusive
                }
            },
            latency_ms: result.latency_ms,
            http_status: result.http_status,
        })
    }
}

fn discovery_headers(
    strategy: &str,
    api_key: &str,
) -> Result<HeaderMap, AgentRuntimeApplicationError> {
    let key = HeaderValue::from_str(api_key)
        .map_err(|_| discovery_error("API key contains invalid header characters"))?;
    let mut headers = HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("application/json"));
    if strategy == "anthropic" {
        headers.insert("x-api-key", key);
        headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    } else {
        let bearer = HeaderValue::from_str(&format!("Bearer {api_key}"))
            .map_err(|_| discovery_error("API key contains invalid header characters"))?;
        headers.insert(AUTHORIZATION, bearer);
    }
    Ok(headers)
}

fn parse_models(
    strategy: &str,
    body: &[u8],
) -> Result<Vec<OnePieceDiscoveredModel>, AgentRuntimeApplicationError> {
    let entries = if strategy == "openai-array" {
        serde_json::from_slice::<Vec<ModelEntry>>(body)
            .map_err(|error| discovery_error(&error.to_string()))?
    } else {
        serde_json::from_slice::<ModelListEnvelope>(body)
            .map_err(|error| discovery_error(&error.to_string()))?
            .data
    };
    Ok(entries
        .into_iter()
        .filter_map(|entry| {
            let id = entry.id.trim().to_string();
            (!id.is_empty()).then(|| OnePieceDiscoveredModel {
                display_name: entry
                    .display_name
                    .or(entry.name)
                    .filter(|value| !value.trim().is_empty())
                    .unwrap_or_else(|| id.clone()),
                id,
            })
        })
        .collect())
}

fn discovery_error(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(format!("OnePiece model discovery failed: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_openai_and_array_shapes() {
        let openai =
            parse_models("openai", br#"{"data":[{"id":"model-a"}]}"#).expect("OpenAI model list");
        let array = parse_models(
            "openai-array",
            br#"[{"id":"model-b","display_name":"Model B"}]"#,
        )
        .expect("array model list");
        assert_eq!(openai[0].id, "model-a");
        assert_eq!(array[0].display_name, "Model B");
    }

    #[test]
    fn applies_strategy_specific_auth_headers() {
        let anthropic = discovery_headers("anthropic", "secret").expect("Anthropic headers");
        let openai = discovery_headers("openai", "secret").expect("OpenAI headers");
        assert_eq!(
            anthropic
                .get("x-api-key")
                .and_then(|value| value.to_str().ok()),
            Some("secret")
        );
        assert_eq!(
            anthropic
                .get("anthropic-version")
                .and_then(|value| value.to_str().ok()),
            Some("2023-06-01")
        );
        assert_eq!(
            openai
                .get(AUTHORIZATION)
                .and_then(|value| value.to_str().ok()),
            Some("Bearer secret")
        );
    }
}
