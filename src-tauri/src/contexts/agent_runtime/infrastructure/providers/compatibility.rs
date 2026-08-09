use super::{build_interactive_invocation, build_invocation_with_role};
use crate::contexts::agent_runtime::application::{
    AgentProvider, AgentProviderError, ProviderGenerationInvocationRequest,
    ProviderInteractiveInvocationRequest, ProviderInteractiveInvocationSpec,
    ProviderInvocationSpec, ProviderOutputFormat, ProviderRegistry,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, ProviderCapabilities, ProviderCapabilityInput, ProviderFamily,
    ProviderMetadata, ProviderReadinessPrerequisites, ProviderUsageCapability,
};
use std::sync::Arc;

struct CompatibilityCliProvider {
    metadata: ProviderMetadata,
    capabilities: ProviderCapabilities,
    readiness: ProviderReadinessPrerequisites,
    output_format: ProviderOutputFormat,
}

struct CompatibilityProviderDefinition {
    id: &'static str,
    display_name: &'static str,
    executable: &'static str,
    managed_sdk_dependency_id: Option<&'static str>,
    output_format: ProviderOutputFormat,
    usage: ProviderUsageCapability,
    reasoning: bool,
    sandbox: bool,
}

const DEFINITIONS: [CompatibilityProviderDefinition; 5] = [
    CompatibilityProviderDefinition {
        id: "claude-code",
        display_name: "Claude Code",
        executable: "claude",
        managed_sdk_dependency_id: Some("claude-sdk"),
        output_format: ProviderOutputFormat::ClaudeStreamJson,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: true,
        sandbox: false,
    },
    CompatibilityProviderDefinition {
        id: "codex-cli",
        display_name: "Codex CLI",
        executable: "codex",
        managed_sdk_dependency_id: Some("codex-sdk"),
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: true,
        sandbox: true,
    },
    CompatibilityProviderDefinition {
        id: "gemini-cli",
        display_name: "Gemini CLI",
        executable: "gemini",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: false,
        sandbox: true,
    },
    CompatibilityProviderDefinition {
        id: "opencode",
        display_name: "OpenCode",
        executable: "opencode",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessAndTerminalReported,
        reasoning: false,
        sandbox: false,
    },
    CompatibilityProviderDefinition {
        id: "antigravity-cli",
        display_name: "Antigravity CLI",
        executable: "agy",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::AntigravityStreamJson,
        usage: ProviderUsageCapability::HeadlessReported,
        reasoning: true,
        sandbox: true,
    },
];

impl CompatibilityCliProvider {
    fn from_definition(
        definition: CompatibilityProviderDefinition,
    ) -> Result<Self, AgentProviderError> {
        let provider_id = definition.id.to_string();
        let metadata = ProviderMetadata::new(
            definition.id,
            definition.display_name,
            ProviderFamily::CodingCli,
        )
        .map_err(|error| preparation_error(&provider_id, error))?;
        let capabilities = ProviderCapabilities::new(ProviderCapabilityInput {
            interaction_modes: vec![InteractionMode::Cli],
            session_resume: true,
            structured_output: true,
            terminal: true,
            usage: definition.usage,
            permissions: true,
            model_selection: true,
            reasoning: definition.reasoning,
            sandbox: definition.sandbox,
        })
        .map_err(|error| preparation_error(&provider_id, error))?;
        let readiness = ProviderReadinessPrerequisites::new(
            vec![definition.executable.to_string()],
            definition.managed_sdk_dependency_id.map(str::to_string),
        )
        .map_err(|error| preparation_error(&provider_id, error))?;
        Ok(Self {
            metadata,
            capabilities,
            readiness,
            output_format: definition.output_format,
        })
    }

    fn external_session_id<'a>(
        &self,
        session: Option<&'a crate::contexts::agent_runtime::domain::ProviderSessionRef>,
    ) -> Result<Option<&'a str>, AgentProviderError> {
        session
            .map(|session| {
                if session.provider_id() != self.metadata.id() {
                    return Err(AgentProviderError::Preparation {
                        provider_id: self.metadata.id().as_str().to_string(),
                        message: "session reference belongs to another provider".to_string(),
                    });
                }
                Ok(session.external_id())
            })
            .transpose()
    }
}

impl AgentProvider for CompatibilityCliProvider {
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
        self.output_format
    }

    fn prepare_generation(
        &self,
        request: ProviderGenerationInvocationRequest<'_>,
    ) -> Result<ProviderInvocationSpec, AgentProviderError> {
        let external_id = self.external_session_id(request.provider_session)?;
        build_invocation_with_role(
            self.metadata.id().as_str(),
            request.executable,
            request.prompt,
            external_id,
            request.managed_args,
            request.role_briefing,
        )
        .map_err(|error| preparation_error(self.metadata.id().as_str(), error))
    }

    fn prepare_interactive(
        &self,
        request: ProviderInteractiveInvocationRequest<'_>,
    ) -> Result<ProviderInteractiveInvocationSpec, AgentProviderError> {
        let external_id = self.external_session_id(request.provider_session)?;
        build_interactive_invocation(
            self.metadata.id().as_str(),
            request.executable,
            external_id,
            request.managed_args,
        )
        .map_err(|error| preparation_error(self.metadata.id().as_str(), error))
    }
}

fn preparation_error(provider_id: &str, error: impl std::fmt::Display) -> AgentProviderError {
    AgentProviderError::Preparation {
        provider_id: provider_id.to_string(),
        message: error.to_string(),
    }
}

pub(crate) fn builtin_cli_provider_registry() -> Result<ProviderRegistry, AgentProviderError> {
    let providers = DEFINITIONS
        .into_iter()
        .map(CompatibilityCliProvider::from_definition)
        .map(|provider| provider.map(|provider| Arc::new(provider) as Arc<dyn AgentProvider>))
        .collect::<Result<Vec<_>, _>>()?;
    let registry = ProviderRegistry::new(providers)?;
    validate_builtin_contracts(&registry)?;
    Ok(registry)
}

fn validate_builtin_contracts(registry: &ProviderRegistry) -> Result<(), AgentProviderError> {
    let providers = registry.list();
    if providers.len() != DEFINITIONS.len() {
        return Err(preparation_error(
            "builtin-registry",
            "provider catalog and compatibility registry differ",
        ));
    }
    for provider in providers {
        let metadata = provider.metadata();
        let capabilities = provider.capabilities();
        let readiness = provider.readiness_prerequisites();
        let valid = !metadata.display_name().is_empty()
            && metadata.family() == ProviderFamily::CodingCli
            && capabilities
                .interaction_modes()
                .contains(&InteractionMode::Cli)
            && capabilities.session_resume()
            && capabilities.structured_output()
            && capabilities.terminal()
            && capabilities.permissions()
            && capabilities.model_selection()
            && matches!(
                capabilities.usage(),
                ProviderUsageCapability::HeadlessReported
                    | ProviderUsageCapability::HeadlessAndTerminalReported
            )
            && readiness.executable_names().len() == 1;
        if !valid {
            return Err(preparation_error(
                metadata.id().as_str(),
                "built-in provider declaration is incomplete",
            ));
        }
        let definition = DEFINITIONS
            .iter()
            .find(|definition| definition.id == metadata.id().as_str())
            .ok_or_else(|| {
                preparation_error(
                    metadata.id().as_str(),
                    "provider is absent from the catalog",
                )
            })?;
        if readiness.managed_sdk_dependency_id() != definition.managed_sdk_dependency_id
            || capabilities.reasoning() != definition.reasoning
            || capabilities.sandbox() != definition.sandbox
        {
            return Err(preparation_error(
                metadata.id().as_str(),
                "provider declaration differs from the built-in catalog",
            ));
        }
    }
    Ok(())
}
