use super::AgentRuntimeApplicationError;
use crate::contexts::agent_runtime::domain::{
    AgentProviderId, ProviderCancellationPolicy, ProviderCapabilities, ProviderCapability,
    ProviderHealth, ProviderMetadata, ProviderParserPolicy, ProviderReadinessPrerequisites,
    ProviderSessionRef, ProviderVersionProbe,
};
use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderPromptDelivery {
    Stdin,
    Argument,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderInvocationSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) prompt_delivery: ProviderPromptDelivery,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderInteractiveInvocationSpec {
    pub(crate) executable: String,
    pub(crate) args: Vec<String>,
    pub(crate) assigned_runtime_session_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderOutputFormat {
    ClaudeStreamJson,
    StructuredJsonLines,
    AntigravityStreamJson,
}

pub(crate) struct ProviderGenerationInvocationRequest<'a> {
    pub(crate) executable: String,
    pub(crate) prompt: &'a str,
    pub(crate) provider_session: Option<&'a ProviderSessionRef>,
    pub(crate) managed_args: &'a [String],
    pub(crate) role_briefing: Option<&'a str>,
}

pub(crate) struct ProviderInteractiveInvocationRequest<'a> {
    pub(crate) executable: String,
    pub(crate) provider_session: Option<&'a ProviderSessionRef>,
    pub(crate) managed_args: &'a [String],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderPermissionMode {
    Readonly,
    Standard,
    Unrestricted,
}

pub(crate) struct ProviderOptionRequest<'a> {
    pub(crate) permission: Option<ProviderPermissionMode>,
    pub(crate) model: Option<&'a str>,
    pub(crate) reasoning: Option<&'a str>,
}

pub(crate) trait AgentProvider: Send + Sync {
    fn metadata(&self) -> &ProviderMetadata;
    fn capabilities(&self) -> &ProviderCapabilities;
    fn readiness_prerequisites(&self) -> &ProviderReadinessPrerequisites;
    fn output_format(&self) -> ProviderOutputFormat;
    fn parser_policy(&self) -> ProviderParserPolicy;
    fn version_probe(&self) -> &ProviderVersionProbe;
    fn cancellation_policy(&self) -> ProviderCancellationPolicy;
    fn classify_health(&self, executable_available: bool, version_valid: bool) -> ProviderHealth;
    fn map_options(
        &self,
        request: ProviderOptionRequest<'_>,
    ) -> Result<Vec<String>, AgentProviderError>;
    fn prepare_generation(
        &self,
        request: ProviderGenerationInvocationRequest<'_>,
    ) -> Result<ProviderInvocationSpec, AgentProviderError>;
    fn prepare_interactive(
        &self,
        request: ProviderInteractiveInvocationRequest<'_>,
    ) -> Result<ProviderInteractiveInvocationSpec, AgentProviderError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum AgentProviderError {
    InvalidContract {
        provider_id: String,
        reason: String,
    },
    InvalidManifest(String),
    ExternalProviderUnsupported(String),
    DuplicateProvider(String),
    UnsupportedProvider(String),
    UnsupportedCapability {
        provider_id: String,
        capability: String,
    },
    Preparation {
        provider_id: String,
        message: String,
    },
}

impl fmt::Display for AgentProviderError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidContract {
                provider_id,
                reason,
            } => write!(
                formatter,
                "invalid Agent provider contract '{provider_id}': {reason}"
            ),
            Self::InvalidManifest(reason) => {
                write!(formatter, "invalid Agent provider manifest: {reason}")
            }
            Self::ExternalProviderUnsupported(id) => {
                write!(formatter, "external Agent provider is unsupported: {id}")
            }
            Self::DuplicateProvider(id) => write!(formatter, "duplicate Agent provider: {id}"),
            Self::UnsupportedProvider(id) => write!(formatter, "unsupported Agent provider: {id}"),
            Self::UnsupportedCapability {
                provider_id,
                capability,
            } => write!(
                formatter,
                "Agent provider '{provider_id}' does not support capability '{capability}'"
            ),
            Self::Preparation {
                provider_id,
                message,
            } => write!(
                formatter,
                "Agent provider '{provider_id}' could not prepare runtime work: {message}"
            ),
        }
    }
}

impl std::error::Error for AgentProviderError {}

#[derive(Clone)]
pub(crate) struct ProviderRegistry {
    providers: Arc<BTreeMap<AgentProviderId, Arc<dyn AgentProvider>>>,
}

impl ProviderRegistry {
    pub(crate) fn new(providers: Vec<Arc<dyn AgentProvider>>) -> Result<Self, AgentProviderError> {
        let mut entries = BTreeMap::new();
        for provider in providers {
            let id = provider.metadata().id().clone();
            if entries.insert(id.clone(), provider).is_some() {
                return Err(AgentProviderError::DuplicateProvider(
                    id.as_str().to_string(),
                ));
            }
        }
        Ok(Self {
            providers: Arc::new(entries),
        })
    }

    pub(crate) fn get(&self, id: &str) -> Result<Arc<dyn AgentProvider>, AgentProviderError> {
        if let Some(external_id) = id.strip_prefix("external:") {
            return Err(AgentProviderError::ExternalProviderUnsupported(
                external_id.to_string(),
            ));
        }
        let provider_id = AgentProviderId::parse(id)
            .map_err(|_| AgentProviderError::UnsupportedProvider(id.to_string()))?;
        self.providers
            .get(&provider_id)
            .cloned()
            .ok_or_else(|| AgentProviderError::UnsupportedProvider(id.to_string()))
    }

    pub(crate) fn list(&self) -> Vec<Arc<dyn AgentProvider>> {
        self.providers.values().cloned().collect()
    }

    pub(crate) fn require(
        &self,
        id: &str,
        capability: ProviderCapability,
    ) -> Result<Arc<dyn AgentProvider>, AgentProviderError> {
        let provider = self.get(id)?;
        if provider.capabilities().supports(capability) {
            Ok(provider)
        } else {
            Err(AgentProviderError::UnsupportedCapability {
                provider_id: id.to_string(),
                capability: capability.as_str().to_string(),
            })
        }
    }

    pub(crate) fn resolve_session(
        &self,
        provider_id: &str,
        external_id: Option<&str>,
    ) -> Result<Option<ProviderSessionRef>, AgentRuntimeApplicationError> {
        let provider = self.get(provider_id)?;
        external_id
            .map(|external_id| {
                ProviderSessionRef::new(provider.metadata().id().clone(), external_id)
                    .map_err(AgentRuntimeApplicationError::from)
            })
            .transpose()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        InteractionMode, ProviderCapabilityInput, ProviderFamily, ProviderUsageCapability,
    };

    struct FakeProvider {
        metadata: ProviderMetadata,
        capabilities: ProviderCapabilities,
        readiness: ProviderReadinessPrerequisites,
    }

    impl FakeProvider {
        fn new(id: &str) -> Self {
            Self {
                metadata: ProviderMetadata::new(id, id, ProviderFamily::CodingCli)
                    .expect("metadata"),
                capabilities: ProviderCapabilities::new(ProviderCapabilityInput {
                    interaction_modes: vec![InteractionMode::Cli],
                    session_resume: false,
                    structured_output: true,
                    terminal: true,
                    usage: ProviderUsageCapability::HeadlessReported,
                    permissions: false,
                    model_selection: false,
                    reasoning: false,
                    sandbox: false,
                })
                .expect("capabilities"),
                readiness: ProviderReadinessPrerequisites::new(vec![id.to_string()], None)
                    .expect("readiness"),
            }
        }
    }

    impl AgentProvider for FakeProvider {
        fn metadata(&self) -> &ProviderMetadata {
            &self.metadata
        }

        fn capabilities(&self) -> &ProviderCapabilities {
            &self.capabilities
        }

        fn readiness_prerequisites(&self) -> &ProviderReadinessPrerequisites {
            &self.readiness
        }

        fn output_format(&self) -> ProviderOutputFormat {
            ProviderOutputFormat::StructuredJsonLines
        }

        fn parser_policy(&self) -> ProviderParserPolicy {
            ProviderParserPolicy::new(4096, true).expect("parser policy")
        }

        fn version_probe(&self) -> &ProviderVersionProbe {
            static PROBE: std::sync::OnceLock<ProviderVersionProbe> = std::sync::OnceLock::new();
            PROBE.get_or_init(|| {
                ProviderVersionProbe::new(vec!["--version".to_string()], 1000)
                    .expect("version probe")
            })
        }

        fn cancellation_policy(&self) -> ProviderCancellationPolicy {
            ProviderCancellationPolicy::process_tree(1000).expect("cancellation policy")
        }

        fn classify_health(
            &self,
            executable_available: bool,
            version_valid: bool,
        ) -> ProviderHealth {
            match (executable_available, version_valid) {
                (true, true) => ProviderHealth::Ready,
                (true, false) => ProviderHealth::Degraded,
                (false, _) => ProviderHealth::Unavailable,
            }
        }

        fn map_options(
            &self,
            _request: ProviderOptionRequest<'_>,
        ) -> Result<Vec<String>, AgentProviderError> {
            Ok(Vec::new())
        }

        fn prepare_generation(
            &self,
            _request: ProviderGenerationInvocationRequest<'_>,
        ) -> Result<ProviderInvocationSpec, AgentProviderError> {
            Err(AgentProviderError::UnsupportedCapability {
                provider_id: self.metadata.id().as_str().to_string(),
                capability: "generation".to_string(),
            })
        }

        fn prepare_interactive(
            &self,
            _request: ProviderInteractiveInvocationRequest<'_>,
        ) -> Result<ProviderInteractiveInvocationSpec, AgentProviderError> {
            Err(AgentProviderError::UnsupportedCapability {
                provider_id: self.metadata.id().as_str().to_string(),
                capability: "interactive".to_string(),
            })
        }
    }

    #[test]
    fn registry_lists_deterministically_and_resolves_exactly() {
        let registry = ProviderRegistry::new(vec![
            Arc::new(FakeProvider::new("z-provider")),
            Arc::new(FakeProvider::new("a-provider")),
        ])
        .expect("registry");
        assert_eq!(
            registry
                .list()
                .iter()
                .map(|provider| provider.metadata().id().as_str())
                .collect::<Vec<_>>(),
            vec!["a-provider", "z-provider"]
        );
        assert_eq!(
            registry
                .get("a-provider")
                .expect("provider")
                .metadata()
                .id()
                .as_str(),
            "a-provider"
        );
        assert!(matches!(
            registry.get("missing"),
            Err(AgentProviderError::UnsupportedProvider(id)) if id == "missing"
        ));
        assert!(matches!(
            registry.get("external:fixture-cli"),
            Err(AgentProviderError::ExternalProviderUnsupported(id)) if id == "fixture-cli"
        ));
    }

    #[test]
    fn registry_rejects_duplicates_and_wraps_session_ids() {
        assert!(matches!(
            ProviderRegistry::new(vec![
                Arc::new(FakeProvider::new("same")),
                Arc::new(FakeProvider::new("same")),
            ]),
            Err(AgentProviderError::DuplicateProvider(id)) if id == "same"
        ));
        let registry =
            ProviderRegistry::new(vec![Arc::new(FakeProvider::new("provider"))]).expect("registry");
        let session = registry
            .resolve_session("provider", Some("external-1"))
            .expect("resolve")
            .expect("session");
        assert_eq!(session.provider_id().as_str(), "provider");
        assert_eq!(session.external_id(), "external-1");
        assert_eq!(
            registry.resolve_session("provider", None).expect("fresh"),
            None
        );
    }

    #[test]
    fn registry_negotiates_capabilities_without_identity_inference() {
        let registry =
            ProviderRegistry::new(vec![Arc::new(FakeProvider::new("provider"))]).expect("registry");
        assert!(registry
            .require("provider", ProviderCapability::Terminal)
            .is_ok());
        assert!(matches!(
            registry.require("provider", ProviderCapability::Reasoning),
            Err(AgentProviderError::UnsupportedCapability { provider_id, capability })
                if provider_id == "provider" && capability == "reasoning"
        ));
    }
}
