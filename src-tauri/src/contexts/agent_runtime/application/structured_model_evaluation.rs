use super::{ApiAgentGateway, ApiCredentialPort, ApiProviderConfig, StoredOnePieceProviderProfile};
use std::sync::Arc;

const ONEPIECE_PROFILE_CREDENTIAL_PREFIX: &str = "onepiece-profile:";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructuredModelEvaluationRequest {
    pub(crate) purpose: StructuredModelPurpose,
    pub(crate) required_profile_id: Option<String>,
    pub(crate) required_model_id: Option<String>,
    pub(crate) system_instruction: String,
    pub(crate) sanitized_json: String,
    pub(crate) max_input_characters: usize,
    pub(crate) max_output_tokens: u32,
    pub(crate) timeout_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructuredModelEvaluationResult {
    pub(crate) purpose: StructuredModelPurpose,
    pub(crate) profile_id: String,
    pub(crate) provider_protocol: String,
    pub(crate) model_id: String,
    pub(crate) response_json: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructuredModelPurpose {
    Assessment,
    SkillEvolutionGeneration,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StructuredModelEvaluationError {
    ProviderUnavailable,
    Timeout,
    RateLimited,
    InvalidRequest,
    ProviderFailure,
}

pub(crate) struct StructuredModelTransportRequest {
    pub(crate) purpose: StructuredModelPurpose,
    pub(crate) config: ApiProviderConfig,
    pub(crate) api_key: String,
    pub(crate) system_instruction: String,
    pub(crate) sanitized_json: String,
    pub(crate) max_output_tokens: u32,
    pub(crate) timeout_ms: u64,
}

pub(crate) trait StructuredModelTransport: Send + Sync {
    fn evaluate(
        &self,
        request: StructuredModelTransportRequest,
    ) -> Result<String, StructuredModelEvaluationError>;
}

pub(crate) trait StructuredModelProfilePort: Send + Sync {
    fn active_profile(
        &self,
    ) -> Result<Option<StoredOnePieceProviderProfile>, StructuredModelEvaluationError>;
}

impl<T: ApiAgentGateway + ?Sized> StructuredModelProfilePort for T {
    fn active_profile(
        &self,
    ) -> Result<Option<StoredOnePieceProviderProfile>, StructuredModelEvaluationError> {
        self.list_onepiece_provider_profiles()
            .map(|profiles| profiles.into_iter().find(|profile| profile.active))
            .map_err(|_| StructuredModelEvaluationError::ProviderUnavailable)
    }
}

#[derive(Clone)]
pub(crate) struct StructuredModelEvaluationService {
    profiles: Arc<dyn StructuredModelProfilePort>,
    credentials: Arc<dyn ApiCredentialPort>,
    transport: Arc<dyn StructuredModelTransport>,
}

impl StructuredModelEvaluationService {
    pub(crate) fn new(
        profiles: Arc<dyn StructuredModelProfilePort>,
        credentials: Arc<dyn ApiCredentialPort>,
        transport: Arc<dyn StructuredModelTransport>,
    ) -> Self {
        Self {
            profiles,
            credentials,
            transport,
        }
    }

    pub(crate) fn evaluate(
        &self,
        request: StructuredModelEvaluationRequest,
    ) -> Result<StructuredModelEvaluationResult, StructuredModelEvaluationError> {
        if request.sanitized_json.is_empty()
            || request.sanitized_json.len() > request.max_input_characters
            || request.max_output_tokens == 0
            || request.timeout_ms == 0
        {
            return Err(StructuredModelEvaluationError::InvalidRequest);
        }
        if request.purpose == StructuredModelPurpose::SkillEvolutionGeneration
            && (request
                .required_profile_id
                .as_deref()
                .is_none_or(str::is_empty)
                || request
                    .required_model_id
                    .as_deref()
                    .is_none_or(str::is_empty))
        {
            return Err(StructuredModelEvaluationError::InvalidRequest);
        }
        let profile = self
            .profiles
            .active_profile()?
            .ok_or(StructuredModelEvaluationError::ProviderUnavailable)?;
        if !matches!(
            profile.interface_format.as_str(),
            "anthropic" | "openai-compatible"
        ) {
            return Err(StructuredModelEvaluationError::ProviderUnavailable);
        }
        if request
            .required_profile_id
            .as_deref()
            .is_some_and(|id| id != profile.id)
            || request
                .required_model_id
                .as_deref()
                .is_some_and(|id| id != profile.model_id)
        {
            return Err(StructuredModelEvaluationError::ProviderUnavailable);
        }
        let credential_key = format!("{ONEPIECE_PROFILE_CREDENTIAL_PREFIX}{}", profile.id);
        let api_key = self
            .credentials
            .fetch(&credential_key)
            .map_err(|_| StructuredModelEvaluationError::ProviderUnavailable)?
            .unwrap_or_default();
        let config = ApiProviderConfig {
            source_provider_id: profile.source_provider_id.clone(),
            model_id: profile.model_id.clone(),
            interface_format: profile.interface_format.clone(),
            base_url: profile.base_url.clone(),
            auto_approve_tools: false,
        };
        let response_json = self.transport.evaluate(StructuredModelTransportRequest {
            purpose: request.purpose,
            config,
            api_key,
            system_instruction: request.system_instruction,
            sanitized_json: request.sanitized_json,
            max_output_tokens: request.max_output_tokens,
            timeout_ms: request.timeout_ms,
        })?;
        Ok(StructuredModelEvaluationResult {
            purpose: request.purpose,
            profile_id: profile.id,
            provider_protocol: profile.interface_format,
            model_id: profile.model_id,
            response_json,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::AgentRuntimeApplicationError;
    use std::sync::Mutex;

    struct FakeProfiles(Option<StoredOnePieceProviderProfile>);

    impl StructuredModelProfilePort for FakeProfiles {
        fn active_profile(
            &self,
        ) -> Result<Option<StoredOnePieceProviderProfile>, StructuredModelEvaluationError> {
            Ok(self.0.clone())
        }
    }

    struct FakeCredentials;

    impl ApiCredentialPort for FakeCredentials {
        fn store(
            &self,
            _agent_id: &str,
            _api_key: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }

        fn fetch(&self, agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
            assert_eq!(agent_id, "onepiece-profile:active-profile");
            Ok(Some("credential".to_string()))
        }

        fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeTransport(Mutex<Option<StructuredModelTransportRequest>>);

    impl StructuredModelTransport for FakeTransport {
        fn evaluate(
            &self,
            request: StructuredModelTransportRequest,
        ) -> Result<String, StructuredModelEvaluationError> {
            *self.0.lock().unwrap_or_else(|error| error.into_inner()) = Some(request);
            Ok("{\"ok\":true}".to_string())
        }
    }

    #[test]
    fn active_profile_is_resolved_without_cli_launch_or_tool_authority() {
        let transport = Arc::new(FakeTransport::default());
        let service = StructuredModelEvaluationService::new(
            Arc::new(FakeProfiles(Some(profile()))),
            Arc::new(FakeCredentials),
            transport.clone(),
        );
        let result = service
            .evaluate(request())
            .unwrap_or_else(|error| panic!("evaluate: {error:?}"));
        let captured = transport
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        let captured = captured
            .as_ref()
            .unwrap_or_else(|| panic!("transport request"));

        assert_eq!(result.profile_id, "active-profile");
        assert_eq!(result.purpose, StructuredModelPurpose::Assessment);
        assert_eq!(captured.purpose, StructuredModelPurpose::Assessment);
        assert_eq!(captured.api_key, "credential");
        assert!(!captured.config.auto_approve_tools);
        assert_eq!(captured.timeout_ms, 15_000);
    }

    #[test]
    fn missing_profile_and_oversized_input_fail_before_transport() {
        let transport = Arc::new(FakeTransport::default());
        let unavailable = StructuredModelEvaluationService::new(
            Arc::new(FakeProfiles(None)),
            Arc::new(FakeCredentials),
            transport.clone(),
        );
        assert_eq!(
            unavailable.evaluate(request()),
            Err(StructuredModelEvaluationError::ProviderUnavailable)
        );

        let bounded = StructuredModelEvaluationService::new(
            Arc::new(FakeProfiles(Some(profile()))),
            Arc::new(FakeCredentials),
            transport.clone(),
        );
        let mut oversized = request();
        oversized.max_input_characters = 1;
        assert_eq!(
            bounded.evaluate(oversized),
            Err(StructuredModelEvaluationError::InvalidRequest)
        );
        assert!(transport
            .0
            .lock()
            .unwrap_or_else(|error| error.into_inner())
            .is_none());
    }

    #[test]
    fn generation_purpose_requires_the_frozen_profile_and_model() {
        let transport = Arc::new(FakeTransport::default());
        let service = StructuredModelEvaluationService::new(
            Arc::new(FakeProfiles(Some(profile()))),
            Arc::new(FakeCredentials),
            transport.clone(),
        );
        let mut generation = request();
        generation.purpose = StructuredModelPurpose::SkillEvolutionGeneration;
        generation.required_profile_id = Some("active-profile".into());
        generation.required_model_id = Some("model".into());
        let result = service.evaluate(generation.clone()).expect("generation");
        assert_eq!(
            result.purpose,
            StructuredModelPurpose::SkillEvolutionGeneration
        );
        generation.required_model_id = Some("other-model".into());
        assert_eq!(
            service.evaluate(generation),
            Err(StructuredModelEvaluationError::ProviderUnavailable)
        );
    }

    fn request() -> StructuredModelEvaluationRequest {
        StructuredModelEvaluationRequest {
            purpose: StructuredModelPurpose::Assessment,
            required_profile_id: None,
            required_model_id: None,
            system_instruction: "bounded".to_string(),
            sanitized_json: "{}".to_string(),
            max_input_characters: 100,
            max_output_tokens: 128,
            timeout_ms: 15_000,
        }
    }

    fn profile() -> StoredOnePieceProviderProfile {
        StoredOnePieceProviderProfile {
            id: "active-profile".to_string(),
            name: "Active".to_string(),
            source_preset_id: None,
            source_provider_id: Some("provider".to_string()),
            source_endpoint_type: None,
            source_preset_version: None,
            provider: "Provider".to_string(),
            model_id: "model".to_string(),
            interface_format: "openai-compatible".to_string(),
            base_url: Some("https://example.test/v1".to_string()),
            active: true,
        }
    }
}
