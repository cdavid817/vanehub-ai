use super::{blocking_no_redirect_http_client, NetworkError};
use reqwest::blocking::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::{Duration, Instant};
use url::Url;

const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderCredentialProbeProtocol {
    AnthropicMessages,
    OpenAiChatCompletions,
    OpenAiResponses,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderCredentialProbeAuthentication {
    AnthropicApiKey,
    Bearer,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ProviderCredentialValidationStatus {
    Valid,
    InvalidCredential,
    ConfigurationRejected,
    RateLimited,
    ProviderUnavailable,
    Unsupported,
    Inconclusive,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderCredentialValidationResult {
    pub(crate) status: ProviderCredentialValidationStatus,
    pub(crate) latency_ms: u64,
    pub(crate) http_status: Option<u16>,
}

pub(crate) struct ProviderCredentialProbeRequest<'a> {
    pub(crate) base_url: &'a str,
    pub(crate) model: &'a str,
    pub(crate) protocol: ProviderCredentialProbeProtocol,
    pub(crate) authentication: ProviderCredentialProbeAuthentication,
    pub(crate) credential: &'a str,
}

pub(crate) fn unsupported_provider_credential_validation() -> ProviderCredentialValidationResult {
    ProviderCredentialValidationResult {
        status: ProviderCredentialValidationStatus::Unsupported,
        latency_ms: 0,
        http_status: None,
    }
}

pub(crate) fn probe_provider_credential(
    request: ProviderCredentialProbeRequest<'_>,
) -> Result<ProviderCredentialValidationResult, NetworkError> {
    validate_probe_value("model", request.model)?;
    validate_probe_value("credential", request.credential)?;
    let endpoint = probe_endpoint(request.base_url, request.protocol)?;
    let client = blocking_no_redirect_http_client(PROBE_TIMEOUT)?;
    let started = Instant::now();
    let response = authenticated_request(
        client.post(endpoint),
        request.authentication,
        request.credential,
    )
    .json(&probe_body(request.protocol, request.model))
    .send();
    let latency_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

    match response {
        Ok(response) => {
            let http_status = response.status().as_u16();
            Ok(ProviderCredentialValidationResult {
                status: classify_http_status(http_status),
                latency_ms,
                http_status: Some(http_status),
            })
        }
        Err(_) => Ok(ProviderCredentialValidationResult {
            status: ProviderCredentialValidationStatus::ProviderUnavailable,
            latency_ms,
            http_status: None,
        }),
    }
}

fn validate_probe_value(label: &str, value: &str) -> Result<(), NetworkError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(NetworkError::Validation(format!(
            "provider credential probe {label} is invalid"
        )));
    }
    Ok(())
}

fn probe_endpoint(
    base_url: &str,
    protocol: ProviderCredentialProbeProtocol,
) -> Result<Url, NetworkError> {
    let mut url = Url::parse(base_url.trim()).map_err(|_| {
        NetworkError::Validation("provider credential probe URL is invalid".to_string())
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.query().is_some()
    {
        return Err(NetworkError::Validation(
            "provider credential probe URL must be an absolute HTTP(S) base URL".to_string(),
        ));
    }

    let suffix = match protocol {
        ProviderCredentialProbeProtocol::AnthropicMessages => "v1/messages",
        ProviderCredentialProbeProtocol::OpenAiChatCompletions => "chat/completions",
        ProviderCredentialProbeProtocol::OpenAiResponses => "responses",
    };
    let current_path = url.path().trim_end_matches('/');
    if !current_path.ends_with(suffix) {
        let next_path = if current_path.is_empty() {
            format!("/{suffix}")
        } else {
            format!("{current_path}/{suffix}")
        };
        url.set_path(&next_path);
    }
    Ok(url)
}

fn authenticated_request(
    request: RequestBuilder,
    authentication: ProviderCredentialProbeAuthentication,
    credential: &str,
) -> RequestBuilder {
    match authentication {
        ProviderCredentialProbeAuthentication::AnthropicApiKey => request
            .header("x-api-key", credential)
            .header("anthropic-version", "2023-06-01"),
        ProviderCredentialProbeAuthentication::Bearer => request.bearer_auth(credential),
    }
}

fn probe_body(protocol: ProviderCredentialProbeProtocol, model: &str) -> Value {
    match protocol {
        ProviderCredentialProbeProtocol::AnthropicMessages => json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "Reply OK." }],
            "stream": false
        }),
        ProviderCredentialProbeProtocol::OpenAiChatCompletions => json!({
            "model": model,
            "max_tokens": 1,
            "messages": [{ "role": "user", "content": "Reply OK." }],
            "stream": false
        }),
        ProviderCredentialProbeProtocol::OpenAiResponses => json!({
            "model": model,
            "input": "Reply OK.",
            "max_output_tokens": 1,
            "stream": false
        }),
    }
}

fn classify_http_status(status: u16) -> ProviderCredentialValidationStatus {
    match status {
        200..=299 => ProviderCredentialValidationStatus::Valid,
        401 | 403 => ProviderCredentialValidationStatus::InvalidCredential,
        400 | 404 | 405 | 409 | 415 | 422 => {
            ProviderCredentialValidationStatus::ConfigurationRejected
        }
        429 => ProviderCredentialValidationStatus::RateLimited,
        500..=599 => ProviderCredentialValidationStatus::ProviderUnavailable,
        _ => ProviderCredentialValidationStatus::Inconclusive,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_protocol_specific_endpoints_without_duplicating_suffixes() {
        assert_eq!(
            probe_endpoint(
                "https://api.openai.com/v1",
                ProviderCredentialProbeProtocol::OpenAiChatCompletions,
            )
            .expect("chat endpoint")
            .as_str(),
            "https://api.openai.com/v1/chat/completions"
        );
        assert_eq!(
            probe_endpoint(
                "https://api.anthropic.com/v1/messages",
                ProviderCredentialProbeProtocol::AnthropicMessages,
            )
            .expect("anthropic endpoint")
            .as_str(),
            "https://api.anthropic.com/v1/messages"
        );
    }

    #[test]
    fn rejects_credential_targets_with_query_or_user_info() {
        assert!(probe_endpoint(
            "https://user@example.com/v1",
            ProviderCredentialProbeProtocol::OpenAiResponses,
        )
        .is_err());
        assert!(probe_endpoint(
            "https://example.com/v1?redirect=https://attacker.invalid",
            ProviderCredentialProbeProtocol::OpenAiResponses,
        )
        .is_err());
    }

    #[test]
    fn classifies_provider_responses_without_parsing_error_bodies() {
        assert_eq!(
            classify_http_status(204),
            ProviderCredentialValidationStatus::Valid
        );
        assert_eq!(
            classify_http_status(401),
            ProviderCredentialValidationStatus::InvalidCredential
        );
        assert_eq!(
            classify_http_status(422),
            ProviderCredentialValidationStatus::ConfigurationRejected
        );
        assert_eq!(
            classify_http_status(429),
            ProviderCredentialValidationStatus::RateLimited
        );
        assert_eq!(
            classify_http_status(503),
            ProviderCredentialValidationStatus::ProviderUnavailable
        );
    }

    #[test]
    fn probe_bodies_are_bounded_to_one_output_token() {
        assert_eq!(
            probe_body(
                ProviderCredentialProbeProtocol::AnthropicMessages,
                "claude-test"
            )["max_tokens"],
            1
        );
        assert_eq!(
            probe_body(ProviderCredentialProbeProtocol::OpenAiResponses, "gpt-test")
                ["max_output_tokens"],
            1
        );
    }
}
