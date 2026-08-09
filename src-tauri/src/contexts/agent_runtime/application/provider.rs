use super::AgentRuntimeApplicationError;
use crate::contexts::agent_runtime::domain::{
    AgentProviderId, ProviderCapabilities, ProviderMetadata, ProviderReadinessPrerequisites,
    ProviderSessionRef,
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

pub(crate) trait AgentProvider: Send + Sync {
    fn metadata(&self) -> &ProviderMetadata;
    fn capabilities(&self) -> &ProviderCapabilities;
    fn readiness_prerequisites(&self) -> &ProviderReadinessPrerequisites;
    fn output_format(&self) -> ProviderOutputFormat;
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
}
