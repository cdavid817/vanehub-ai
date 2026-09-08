use super::definitions::{
    definition as catalog_definition, CompatibilityProviderDefinition, DEFINITIONS,
};
use super::{
    build_interactive_invocation, build_invocation_with_role, manifest::ValidatedProviderManifest,
    ProviderLaunchSegments,
};
#[cfg(test)]
use crate::contexts::agent_runtime::application::ProviderPromptDelivery;
use crate::contexts::agent_runtime::application::{
    AgentProvider, AgentProviderError, ProviderAcpInvocationRequest, ProviderAcpInvocationSpec,
    ProviderGenerationInvocationRequest, ProviderInteractiveInvocationRequest,
    ProviderInteractiveInvocationSpec, ProviderInvocationSpec, ProviderOptionRequest,
    ProviderOutputFormat, ProviderPermissionMode, ProviderRegistry,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, ProviderCancellationPolicy, ProviderCapabilities, ProviderCapability,
    ProviderFamily, ProviderHealth, ProviderMetadata, ProviderParserPolicy,
    ProviderReadinessPrerequisites, ProviderTransport, ProviderUsageCapability,
    ProviderVersionProbe,
};
use std::collections::BTreeMap;
use std::sync::Arc;

struct CompatibilityCliProvider {
    metadata: ProviderMetadata,
    capabilities: ProviderCapabilities,
    readiness: ProviderReadinessPrerequisites,
    output_format: ProviderOutputFormat,
    parser_policy: ProviderParserPolicy,
    version_probe: ProviderVersionProbe,
    cancellation_policy: ProviderCancellationPolicy,
    definition: &'static CompatibilityProviderDefinition,
}

impl CompatibilityCliProvider {
    fn from_definition(
        definition: &'static CompatibilityProviderDefinition,
    ) -> Result<Self, AgentProviderError> {
        let provider_id = definition.id.to_string();
        let manifest = ValidatedProviderManifest::parse_json(&definition.manifest_json())?;
        let metadata = manifest.metadata;
        // A version 1 manifest cannot express terminal-reported usage; the catalog refines it. A
        // version 2 manifest already carries the reviewed value, and the catalog must agree.
        let capabilities = if manifest.schema_version == 1 {
            manifest
                .capabilities
                .with_usage_capability(definition.usage)
        } else {
            if manifest.capabilities.usage() != definition.usage {
                return Err(preparation_error(
                    &provider_id,
                    "manifest usage disagrees with the catalog",
                ));
            }
            manifest.capabilities
        };
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
            definition,
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

    fn unsupported(&self, capability: &str) -> AgentProviderError {
        AgentProviderError::UnsupportedCapability {
            provider_id: self.metadata.id().as_str().to_string(),
            capability: capability.to_string(),
        }
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
        // A one-shot headless launch is only meaningful for the headless transport. An ACP
        // provider's prompt travels inside the protocol, and a legacy terminal provider has no
        // managed conversation at all; refusing here is what keeps a wrong route from spawning
        // `qwen --acp` with a prompt on stdin and reading protocol frames as prose.
        if !self
            .capabilities
            .supports_transport(ProviderTransport::Headless)
        {
            return Err(self.unsupported("headless-generation"));
        }
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

    fn prepare_acp(
        &self,
        request: ProviderAcpInvocationRequest<'_>,
    ) -> Result<ProviderAcpInvocationSpec, AgentProviderError> {
        let Some(grammar) = self.definition.acp else {
            return Err(self.unsupported("acp-stdio"));
        };
        let mut environment = BTreeMap::new();
        match request
            .account_profile
            .map(str::trim)
            .filter(|id| !id.is_empty())
        {
            None => {}
            Some(profile_id) => {
                let profile = grammar
                    .account_profiles
                    .iter()
                    .find(|profile| profile.id == profile_id)
                    .ok_or_else(|| AgentProviderError::Preparation {
                        provider_id: self.metadata.id().as_str().to_string(),
                        message: format!("unknown account profile '{profile_id}'"),
                    })?;
                if let Some((key, value)) = profile.environment {
                    environment.insert(key.to_string(), value.to_string());
                }
            }
        }
        // Profile tokens precede the reviewed ACP entry; invocation-slot tokens follow it. The
        // grammar owns the entry flag, and nothing in either segment may carry the prompt.
        let mut args = request.global_args.to_vec();
        args.extend(grammar.args.iter().map(|arg| (*arg).to_string()));
        args.extend_from_slice(request.invocation_args);
        Ok(ProviderAcpInvocationSpec {
            executable: request.executable,
            args,
            environment,
            adapter_revision: self.definition.adapter_revision,
        })
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
        .iter()
        .map(CompatibilityCliProvider::from_definition)
        .map(|provider| provider.map(|provider| Arc::new(provider) as Arc<dyn AgentProvider>))
        .collect::<Result<Vec<_>, _>>()?;
    let registry = ProviderRegistry::new(providers)?;
    validate_builtin_contracts(&registry)?;
    Ok(registry)
}

#[cfg(test)]
pub(super) fn fixture_provider() -> Arc<dyn AgentProvider> {
    use super::definitions::ProviderLifecycle;
    static FIXTURE: CompatibilityProviderDefinition = CompatibilityProviderDefinition {
        id: "fixture-cli",
        display_name: "Fixture CLI",
        executable: "fixture",
        managed_sdk_dependency_id: None,
        output_format: ProviderOutputFormat::StructuredJsonLines,
        usage: ProviderUsageCapability::HeadlessReported,
        reasoning: false,
        sandbox: false,
        transports: &[ProviderTransport::Terminal, ProviderTransport::Headless],
        resume: true,
        model_selection: true,
        permissions: true,
        structured_output: true,
        acp: None,
        lifecycle: ProviderLifecycle::Active,
        adapter_revision: "fixture-v1",
    };
    Arc::new(CompatibilityCliProvider::from_definition(&FIXTURE).expect("valid fixture provider"))
}

/// The mandatory contract every built-in provider must satisfy, per declared transport.
///
/// Common concerns are unconditional. Transport concerns apply only where the transport is
/// declared: a headless provider must parse text fallback and report usage; an ACP provider must
/// expose a grammar; a terminal-only provider must refuse every managed path. Optional features
/// (resume, usage, reasoning, model selection) may be declared unsupported, and that declaration
/// is accepted rather than treated as incompleteness.
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
        let definition = catalog_definition(metadata.id().as_str()).ok_or_else(|| {
            preparation_error(
                metadata.id().as_str(),
                "provider is absent from the catalog",
            )
        })?;
        let common_valid = !metadata.display_name().is_empty()
            && metadata.family() == ProviderFamily::CodingCli
            && capabilities
                .interaction_modes()
                .contains(&InteractionMode::Cli)
            && capabilities.terminal()
            && capabilities.supports_transport(ProviderTransport::Terminal)
            && readiness.executable_names().len() == 1
            && capabilities.transports() == definition.transports;
        if !common_valid {
            return Err(preparation_error(
                metadata.id().as_str(),
                "built-in provider declaration is incomplete",
            ));
        }
        let parser_policy = provider.parser_policy();
        let cancellation = provider.cancellation_policy();
        let version_probe = provider.version_probe();
        let sdk_contract_valid = parser_policy.max_buffer_bytes() >= 1_024
            && !version_probe.args().is_empty()
            && version_probe.timeout_ms() <= 15_000
            && cancellation.grace_period_ms() <= 30_000
            && cancellation.uses_process_tree()
            && provider.classify_health(true, true) == ProviderHealth::Ready
            && registry
                .require(metadata.id().as_str(), ProviderCapability::Cancellation)
                .is_ok();
        if !sdk_contract_valid {
            return Err(preparation_error(
                metadata.id().as_str(),
                "built-in provider SDK declaration is incomplete",
            ));
        }
        validate_transport_contract(registry, provider.as_ref(), definition)?;
        // The capability query must agree with the declaration it answers for: usage is a
        // supported capability exactly when it is reported, sandbox exactly when declared.
        if capabilities.supports(ProviderCapability::Usage) != definition.usage.is_reported()
            || capabilities.supports(ProviderCapability::Sandbox) != definition.sandbox
        {
            return Err(preparation_error(
                metadata.id().as_str(),
                "capability query disagrees with the declaration",
            ));
        }
        if readiness.managed_sdk_dependency_id() != definition.managed_sdk_dependency_id
            || capabilities.reasoning() != definition.reasoning
            || capabilities.sandbox() != definition.sandbox
            || capabilities.session_resume() != definition.resume
            || capabilities.model_selection() != definition.model_selection
            || capabilities.permissions() != definition.permissions
            || capabilities.structured_output() != definition.structured_output
            || capabilities.usage() != definition.usage
        {
            return Err(preparation_error(
                metadata.id().as_str(),
                "provider declaration differs from the built-in catalog",
            ));
        }
    }
    Ok(())
}

fn validate_transport_contract(
    registry: &ProviderRegistry,
    provider: &dyn AgentProvider,
    definition: &CompatibilityProviderDefinition,
) -> Result<(), AgentProviderError> {
    let id = provider.metadata().id().as_str();
    let capabilities = provider.capabilities();
    let permission_mapping_valid = [
        ProviderPermissionMode::Readonly,
        ProviderPermissionMode::Standard,
        ProviderPermissionMode::Unrestricted,
    ]
    .into_iter()
    .all(|permission| {
        let mapped = provider.map_options(ProviderOptionRequest {
            permission: Some(permission),
            model: None,
            reasoning: None,
        });
        // A provider that declares no permission model must classify the request, not accept it.
        if capabilities.permissions() {
            mapped.is_ok()
        } else {
            matches!(
                mapped,
                Err(AgentProviderError::UnsupportedCapability { .. })
            )
        }
    });
    if !permission_mapping_valid {
        return Err(preparation_error(id, "permission mapping is incomplete"));
    }
    if capabilities.supports_transport(ProviderTransport::Headless) {
        let headless_valid = capabilities.structured_output()
            && provider.parser_policy().text_fallback()
            && capabilities.usage().is_reported()
            && registry
                .require(id, ProviderCapability::StructuredOutput)
                .is_ok();
        if !headless_valid {
            return Err(preparation_error(
                id,
                "headless transport contract is incomplete",
            ));
        }
    }
    if capabilities.supports_transport(ProviderTransport::AcpStdio) {
        let acp_valid = capabilities.structured_output()
            && capabilities.permissions()
            && definition
                .acp
                .is_some_and(|grammar| !grammar.args.is_empty())
            && provider
                .prepare_acp(ProviderAcpInvocationRequest {
                    executable: definition.executable.to_string(),
                    global_args: &[],
                    invocation_args: &[],
                    account_profile: None,
                })
                .is_ok();
        if !acp_valid {
            return Err(preparation_error(
                id,
                "acp transport contract is incomplete",
            ));
        }
    }
    if capabilities.managed_transport().is_none() {
        // Terminal-only: every managed path must refuse with a classified result, and the usage
        // declaration must be truthful about there being nothing to report.
        let refuses_generation = matches!(
            provider.prepare_generation(ProviderGenerationInvocationRequest {
                executable: definition.executable.to_string(),
                prompt: "contract",
                provider_session: None,
                global_args: &[],
                invocation_args: &[],
                role_briefing: None,
            }),
            Err(AgentProviderError::UnsupportedCapability { .. })
        );
        let refuses_acp = matches!(
            provider.prepare_acp(ProviderAcpInvocationRequest {
                executable: definition.executable.to_string(),
                global_args: &[],
                invocation_args: &[],
                account_profile: None,
            }),
            Err(AgentProviderError::UnsupportedCapability { .. })
        );
        if !refuses_generation
            || !refuses_acp
            || capabilities.usage() != ProviderUsageCapability::Unavailable
        {
            return Err(preparation_error(
                id,
                "terminal-only transport contract is incomplete",
            ));
        }
    }
    Ok(())
}
