use super::api_process_adapter::{summarize_turns, wire_format_for};
use crate::contexts::agent_runtime::application::{
    StructuredModelEvaluationError, StructuredModelPurpose, StructuredModelTransport,
    StructuredModelTransportRequest,
};
use crate::platform::network::blocking_http_client;
use serde_json::json;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

pub(crate) struct HttpStructuredModelTransport;

impl StructuredModelTransport for HttpStructuredModelTransport {
    fn evaluate(
        &self,
        request: StructuredModelTransportRequest,
    ) -> Result<String, StructuredModelEvaluationError> {
        let wire_format = wire_format_for(&request.config)
            .map_err(|_| StructuredModelEvaluationError::ProviderUnavailable)?;
        let client = blocking_http_client(Duration::from_millis(request.timeout_ms))
            .map_err(|_| StructuredModelEvaluationError::ProviderFailure)?;
        let turns = vec![json!({
            "role": "user",
            "content": request.sanitized_json,
        })];
        let cancelled = AtomicBool::new(false);
        let response_guard = match request.purpose {
            StructuredModelPurpose::Assessment => {
                "Return exactly one JSON object matching the assessment schema. Do not call tools."
            }
            StructuredModelPurpose::SkillEvolutionGeneration => {
                "Return exactly one JSON object matching the Skill-generation schema. Do not call tools or emit hidden reasoning."
            }
        };
        summarize_turns(
            &wire_format,
            &client,
            &request.api_key,
            &request.config.model_id,
            Some(&request.system_instruction),
            &turns,
            response_guard,
            &cancelled,
            Some(request.max_output_tokens),
        )
        .map_err(|error| classify_transport_error(&error))?
        .ok_or(StructuredModelEvaluationError::ProviderFailure)
    }
}

fn classify_transport_error(error: &str) -> StructuredModelEvaluationError {
    let normalized = error.to_ascii_lowercase();
    if normalized.contains("timed out") || normalized.contains("timeout") {
        StructuredModelEvaluationError::Timeout
    } else if normalized.contains("http 429") || normalized.contains("rate limit") {
        StructuredModelEvaluationError::RateLimited
    } else {
        StructuredModelEvaluationError::ProviderFailure
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_errors_are_sanitized_into_stable_categories() {
        assert_eq!(
            classify_transport_error("request timed out with secret=abc"),
            StructuredModelEvaluationError::Timeout
        );
        assert_eq!(
            classify_transport_error("received HTTP 429 key=abc"),
            StructuredModelEvaluationError::RateLimited
        );
        assert_eq!(
            classify_transport_error("provider body contains private text"),
            StructuredModelEvaluationError::ProviderFailure
        );
    }
}
