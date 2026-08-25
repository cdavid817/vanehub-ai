use super::{
    build_interactive_invocation, build_invocation_with_role, manifest::ValidatedProviderManifest,
    ProviderLaunchSegments,
};
#[cfg(test)]
use crate::contexts::agent_runtime::application::ProviderPromptDelivery;
use crate::contexts::agent_runtime::application::{
    AgentProvider, AgentProviderError, ProviderGenerationInvocationRequest,
    ProviderInteractiveInvocationRequest, ProviderInteractiveInvocationSpec,
    ProviderInvocationSpec, ProviderOptionRequest, ProviderOutputFormat, ProviderPermissionMode,
    ProviderRegistry,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, ProviderCancellationPolicy, ProviderCapabilities, ProviderCapability,
    ProviderFamily, ProviderHealth, ProviderMetadata, ProviderParserPolicy,
    ProviderReadinessPrerequisites, ProviderUsageCapability, ProviderVersionProbe,
};
use std::sync::Arc;

struct CompatibilityCliProvider {
    metadata: ProviderMetadata,
    capabilities: ProviderCapabilities,
    readiness: ProviderReadinessPrerequisites,
    output_format: ProviderOutputFormat,
    parser_policy: ProviderParserPolicy,
    version_probe: ProviderVersionProbe,
    cancellation_policy: ProviderCancellationPolicy,
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
        let manifest = ValidatedProviderManifest::parse_json(&manifest_json(&definition))?;
        let metadata = manifest.metadata;
        let capabilities = manifest
            .capabilities
            .with_usage_capability(definition.usage);
        let readiness = ProviderReadinessPrerequisites::new(
            manifest.readiness.executable_names().to_vec(),
            definition.managed_sdk_dependency_id.map(str::to_string),
        )
        .map_err(|error| preparation_error(&provider_id, error))?;
        Ok(Self {
            metadata,
            capabilities,
            readiness,
            output_format: definition.output_format,
            // 256 KiB starved real seat turns: a claude-code tool_result carrying one large file
            // read exceeds it and kills the whole generation. Use the domain-validated maximum
            // (ProviderParserPolicy caps the buffer at 1 MiB).
            parser_policy: ProviderParserPolicy::new(1_048_576, true)
                .map_err(|error| preparation_error(&provider_id, error))?,
            version_probe: ProviderVersionProbe::new(vec!["--version".to_string()], 5_000)
                .map_err(|error| preparation_error(&provider_id, error))?,
            cancellation_policy: ProviderCancellationPolicy::process_tree(2_000)
                .map_err(|error| preparation_error(&provider_id, error))?,
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

fn manifest_json(definition: &CompatibilityProviderDefinition) -> String {
    format!(
        r#"{{"schemaVersion":1,"id":"{}","name":"{}","runtime":"cli","executables":["{}"],"capabilities":{{"terminal":true,"resume":true,"structuredOutput":true,"images":false,"usage":true,"permissions":true,"modelSelection":true,"reasoning":{},"sandbox":{}}}}}"#,
        definition.id,
        definition.display_name,
        definition.executable,
        definition.reasoning,
        definition.sandbox
    )
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

    fn parser_policy(&self) -> ProviderParserPolicy {
        self.parser_policy
    }

    fn version_probe(&self) -> &ProviderVersionProbe {
        &self.version_probe
    }

    fn cancellation_policy(&self) -> ProviderCancellationPolicy {
        self.cancellation_policy
    }

    fn classify_health(&self, executable_available: bool, version_valid: bool) -> ProviderHealth {
        match (executable_available, version_valid) {
            (true, true) => ProviderHealth::Ready,
            (true, false) => ProviderHealth::Degraded,
            (false, _) => ProviderHealth::Unavailable,
        }
    }

    fn map_options(
        &self,
        request: ProviderOptionRequest<'_>,
    ) -> Result<Vec<String>, AgentProviderError> {
        let mut args = Vec::new();
        for (capability, value) in [
            (
                ProviderCapability::Permissions,
                request.permission.map(|_| "permission"),
            ),
            (ProviderCapability::ModelSelection, request.model),
            (ProviderCapability::Reasoning, request.reasoning),
        ] {
            if value.is_some() && !self.capabilities.supports(capability) {
                return Err(AgentProviderError::UnsupportedCapability {
                    provider_id: self.metadata.id().as_str().to_string(),
                    capability: capability.as_str().to_string(),
                });
            }
        }
        if let Some(model) = request.model.filter(|value| !value.trim().is_empty()) {
            args.extend(["--model".to_string(), model.to_string()]);
        }
        if let Some(reasoning) = request.reasoning.filter(|value| !value.trim().is_empty()) {
            args.extend(["--reasoning".to_string(), reasoning.to_string()]);
        }
        Ok(args)
    }

    fn prepare_generation(
        &self,
        request: ProviderGenerationInvocationRequest<'_>,
    ) -> Result<ProviderInvocationSpec, AgentProviderError> {
        let external_id = self.external_session_id(request.provider_session)?;
        #[cfg(test)]
        if self.metadata.id().as_str() == "fixture-cli" {
            let mut args = vec!["--json".to_string()];
            if let Some(external_id) = external_id {
                args.extend(["--resume".to_string(), external_id.to_string()]);
            }
            args.extend_from_slice(request.global_args);
            args.extend_from_slice(request.invocation_args);
            return Ok(ProviderInvocationSpec {
                executable: request.executable,
                args,
                prompt_delivery: ProviderPromptDelivery::Stdin,
            });
        }
        build_invocation_with_role(
            self.metadata.id().as_str(),
            request.executable,
            request.prompt,
            external_id,
            ProviderLaunchSegments {
                global: request.global_args,
                invocation: request.invocation_args,
            },
            request.role_briefing,
        )
        .map_err(|error| preparation_error(self.metadata.id().as_str(), error))
    }

    fn prepare_interactive(
        &self,
        request: ProviderInteractiveInvocationRequest<'_>,
    ) -> Result<ProviderInteractiveInvocationSpec, AgentProviderError> {
        let external_id = self.external_session_id(request.provider_session)?;
        #[cfg(test)]
        if self.metadata.id().as_str() == "fixture-cli" {
            let mut args = request.global_args.to_vec();
            args.extend_from_slice(request.invocation_args);
            if let Some(external_id) = external_id {
                args.extend(["--resume".to_string(), external_id.to_string()]);
            }
            return Ok(ProviderInteractiveInvocationSpec {
                executable: request.executable,
                args,
                assigned_runtime_session_id: external_id.map(str::to_string),
            });
        }
        build_interactive_invocation(
            self.metadata.id().as_str(),
            request.executable,
            external_id,
            ProviderLaunchSegments {
                global: request.global_args,
                invocation: request.invocation_args,
            },
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

#[cfg(test)]
pub(super) fn fixture_provider() -> Arc<dyn AgentProvider> {
    Arc::new(
        CompatibilityCliProvider::from_definition(CompatibilityProviderDefinition {
            id: "fixture-cli",
            display_name: "Fixture CLI",
            executable: "fixture",
            managed_sdk_dependency_id: None,
            output_format: ProviderOutputFormat::StructuredJsonLines,
            usage: ProviderUsageCapability::HeadlessReported,
            reasoning: false,
            sandbox: false,
        })
        .expect("valid fixture provider"),
    )
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
        let parser_policy = provider.parser_policy();
        let cancellation = provider.cancellation_policy();
        let version_probe = provider.version_probe();
        let sdk_contract_valid = parser_policy.max_buffer_bytes() >= 1_024
            && parser_policy.text_fallback()
            && !version_probe.args().is_empty()
            && version_probe.timeout_ms() <= 15_000
            && cancellation.grace_period_ms() <= 30_000
            && cancellation.uses_process_tree()
            && provider.classify_health(true, true) == ProviderHealth::Ready
            && provider
                .map_options(ProviderOptionRequest {
                    permission: Some(ProviderPermissionMode::Standard),
                    model: None,
                    reasoning: None,
                })
                .is_ok()
            && [
                ProviderPermissionMode::Readonly,
                ProviderPermissionMode::Unrestricted,
            ]
            .into_iter()
            .all(|permission| {
                provider
                    .map_options(ProviderOptionRequest {
                        permission: Some(permission),
                        model: None,
                        reasoning: None,
                    })
                    .is_ok()
            })
            && registry
                .require(metadata.id().as_str(), ProviderCapability::Cancellation)
                .is_ok();
        if !sdk_contract_valid {
            return Err(preparation_error(
                metadata.id().as_str(),
                "built-in provider SDK declaration is incomplete",
            ));
        }
        for capability in [
            ProviderCapability::Resume,
            ProviderCapability::StructuredOutput,
            ProviderCapability::Terminal,
            ProviderCapability::Usage,
            ProviderCapability::Permissions,
            ProviderCapability::ModelSelection,
            ProviderCapability::Reasoning,
            ProviderCapability::Sandbox,
            ProviderCapability::Cancellation,
        ] {
            let _ = capabilities.supports(capability);
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
