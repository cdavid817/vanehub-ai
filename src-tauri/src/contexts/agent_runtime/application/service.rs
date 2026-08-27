use super::model_category::{is_chat_model, is_embedding_model};
use super::{
    format_memory_index, memory_from_ref, proposals_from_actions, render_existing_manifest,
    AgentCandidateSubmission, AgentChatConfiguration, AgentCliProfileGateway, AgentClockPort,
    AgentEvent, AgentEventPort, AgentGenerationPort, AgentInvocationUsage, AgentLog, AgentLogLevel,
    AgentLoggingPort, AgentMemory, AgentMemoryDelivery, AgentMessage, AgentMessageTerminal,
    AgentMessageTerminalCompletionPort, AgentMessageTerminalOutcome, AgentProcessEventSink,
    AgentProcessGateway, AgentProposalOrigin, AgentRegistryRepository,
    AgentRuntimeApplicationError, AgentSession, AgentSessionDetails, AgentSessionGateway,
    AgentTaskPort, AgentUsageAccountingKind, AgentUsageOverlap, AgentUsageRecord, AgentView,
    ApiAgentGateway, ApiCredentialPort, ApiProviderConfig, CanonicalRunLinks, CanonicalRunOutcome,
    CanonicalRunSignal, CliProfileSnapshot, CompleteAgentMessage, ConversationHistoryPort,
    DiscoverOnePieceProviderModelsInput, DurableAgentGenerationStart, EffectivePrompt,
    EffectivePromptGateway, EmbeddingEndpointView, FrozenEndpointProfile, GenerationLease,
    GenerationPersonalizationContext, GenerationProcessEvent, GenerationProcessRequest,
    HybridRoutePreview, HybridRoutePreviewInput, LaunchWorkflowResult, LoopGenerationControlPort,
    LoopRoleGenerationCompletionPort, LoopRoleGenerationOutcome, LoopRoleGenerationTerminal,
    LoopVerifierGenerationPort, LoopWorkerGenerationPort, MessageTokenUsage, NewAgentMessage,
    OnePieceModelDiscoveryPort, OnePieceModelDiscoveryRequest, OnePieceProviderConfig,
    OnePieceProviderModelDiscoveryResult, OnePieceProviderModelOption, OnePieceProviderPreset,
    OnePieceProviderProfile, OnePieceProviderProfiles, PendingPromptExecution,
    PromptExecutionOutcome, PromptExecutionReport, PromptVersionReference,
    ProviderCredentialProbeAuthentication, ProviderCredentialProbeProtocol,
    ProviderCredentialProbeRequest, ProviderCredentialValidationResult, ReadinessView,
    RecoverSessionResult, RegisterApiAgentInput, ReportedUsageTotals, RunnerDescriptor,
    RunnerDiscoveryPort, RunnerSelection, SaveCustomOnePieceProviderProfileInput,
    SaveOnePieceProviderConfigInput, SaveOnePieceProviderProfileInput, SeatTurnCompletionPort,
    SeatTurnTerminal, SendMessageRequest, StartedAgentMessage, StopGenerationResult,
    StoredEndpointProfileMetadata, StoredHybridRoutingRule, StoredOnePieceProviderConfig,
    StoredOnePieceProviderProfile, ToolApprovalDecision, ToolApprovalPort, ToolLifecycleEvent,
    ToolLifecyclePhase, UpdateApiAgentInput, ValidateOnePieceProviderCredentialInput,
    WorkflowLaunchRequest, WorkflowView, CLI_MEMORY_INDEX_BOUNDS, INTERFACE_FORMAT_ANTHROPIC,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    route_profile, AgentDefinition, AgentLifecycle, AgentOrigin, AgentReadiness, AgentWorkflow,
    AuthenticationMode, AutomaticCompactionMode, CapabilityProvenance, CapabilityState,
    ContextCapacityProvenance, DataPolicy, EndpointCapabilities, EndpointProfileSnapshot,
    EndpointSource, HybridRouteDecision, HybridRouteReason, HybridRouteRequest, HybridRoutingRule,
    InteractionMode, ProfileCapability, ProfileContextCapacity, ProfilePrivacy, ProfileRuntimeKind,
    RequiredCapabilities, RouteCandidate, TaskClass,
};
use crate::contexts::execution_observability::api::{
    ExecutionContext, ExecutionFidelity, ExecutionIdentityPort, ExecutionLink, ExecutionRun,
    ExecutionRunId, ExecutionSettingsPort, ExecutionSource, ExecutionSpan, ExecutionStatus,
    ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes, SpanId, TraceId,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use uuid::Uuid;

fn onepiece_profile_credential_key(profile_id: &str) -> String {
    format!("onepiece-profile:{profile_id}")
}

fn custom_profile_snapshot(
    id: &str,
    input: &SaveCustomOnePieceProviderProfileInput,
    credential_present: bool,
) -> Result<EndpointProfileSnapshot, AgentRuntimeApplicationError> {
    let runtime_kind = match input.runtime_kind.as_str() {
        "local" => ProfileRuntimeKind::Local,
        "private" => ProfileRuntimeKind::Private,
        _ => {
            return Err(AgentRuntimeApplicationError::Validation(
                "A custom endpoint runtime must be local or private.".to_string(),
            ))
        }
    };
    let authentication_mode = match input.authentication_mode.as_str() {
        "required" => AuthenticationMode::Required,
        "optional" => AuthenticationMode::Optional,
        "none" => AuthenticationMode::None,
        _ => {
            return Err(AgentRuntimeApplicationError::Validation(
                "Invalid endpoint authentication mode.".to_string(),
            ))
        }
    };
    let privacy = match input.privacy_classification.as_str() {
        "local" => ProfilePrivacy::Local,
        "private" => ProfilePrivacy::Private,
        _ => {
            return Err(AgentRuntimeApplicationError::Validation(
                "A custom endpoint privacy classification must match its runtime.".to_string(),
            ))
        }
    };
    let capability = |value: &str| match value {
        "supported" => Ok(ProfileCapability::configured(CapabilityState::Supported)),
        "unsupported" => Ok(ProfileCapability::configured(CapabilityState::Unsupported)),
        "unknown" => Ok(ProfileCapability::configured(CapabilityState::Unknown)),
        _ => Err(AgentRuntimeApplicationError::Validation(
            "Invalid endpoint capability state.".to_string(),
        )),
    };
    let capacity_provenance = if input.context_window_tokens.is_some() {
        ContextCapacityProvenance::ConfiguredEstimate
    } else {
        ContextCapacityProvenance::Unknown
    };
    EndpointProfileSnapshot::new(EndpointProfileSnapshot {
        id: id.to_string(),
        agent_id: "onepiece".to_string(),
        runtime_kind,
        endpoint_source: EndpointSource::Configured,
        base_url: input.base_url.clone(),
        interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
        model_id: input.model_id.clone(),
        authentication_mode,
        credential_present,
        timeout_ms: input.timeout_ms,
        privacy,
        capabilities: EndpointCapabilities {
            text_generation: ProfileCapability::configured(CapabilityState::Supported),
            tool_calling: capability(&input.tool_calling_capability)?,
            image_input: capability(&input.image_input_capability)?,
            structured_output: capability(&input.structured_output_capability)?,
            reasoning_field: capability(&input.reasoning_field_capability)?,
        },
        context_capacity: ProfileContextCapacity {
            context_window_tokens: input.context_window_tokens,
            reserved_output_tokens: input.reserved_output_tokens,
            provenance: capacity_provenance,
        },
    })
    .map_err(|_| {
        AgentRuntimeApplicationError::Validation(
            "The custom endpoint Profile is invalid or unsafe.".to_string(),
        )
    })
}

fn task_class(value: &str) -> Result<TaskClass, AgentRuntimeApplicationError> {
    match value {
        "summarization" => Ok(TaskClass::Summarization),
        "embeddings" => Ok(TaskClass::Embeddings),
        "classification" => Ok(TaskClass::Classification),
        "code-review" => Ok(TaskClass::CodeReview),
        "planning" => Ok(TaskClass::Planning),
        "unknown" => Ok(TaskClass::Unknown),
        _ => Err(AgentRuntimeApplicationError::Validation(
            "Invalid Hybrid Routing task class.".to_string(),
        )),
    }
}

fn classify_task(prompt: &str) -> &'static str {
    let prompt = prompt.trim().to_lowercase();
    if ["summarize", "summary", "总结", "摘要"]
        .iter()
        .any(|prefix| prompt.starts_with(prefix))
    {
        "summarization"
    } else {
        "unknown"
    }
}

fn data_policy(value: &str) -> Result<DataPolicy, AgentRuntimeApplicationError> {
    match value {
        "cloud-allowed" => Ok(DataPolicy::CloudAllowed),
        "local-preferred" => Ok(DataPolicy::LocalPreferred),
        "local-only" => Ok(DataPolicy::LocalOnly),
        _ => Err(AgentRuntimeApplicationError::Validation(
            "Invalid Hybrid Routing data policy.".to_string(),
        )),
    }
}

fn profile_snapshot(
    profile: &StoredOnePieceProviderProfile,
    metadata: StoredEndpointProfileMetadata,
    credential_present: bool,
) -> Result<EndpointProfileSnapshot, AgentRuntimeApplicationError> {
    let runtime_kind = match metadata.runtime_kind.as_str() {
        "cloud" => ProfileRuntimeKind::Cloud,
        "local" => ProfileRuntimeKind::Local,
        "private" => ProfileRuntimeKind::Private,
        _ => ProfileRuntimeKind::Private,
    };
    let endpoint_source = match metadata.endpoint_source.as_str() {
        "catalog" => EndpointSource::Catalog,
        "discovered" => EndpointSource::Discovered,
        _ => EndpointSource::Configured,
    };
    let authentication_mode = match metadata.authentication_mode.as_str() {
        "none" => AuthenticationMode::None,
        "optional" => AuthenticationMode::Optional,
        _ => AuthenticationMode::Required,
    };
    let privacy = match metadata.privacy_classification.as_str() {
        "cloud" => ProfilePrivacy::Cloud,
        "local" => ProfilePrivacy::Local,
        _ => ProfilePrivacy::Private,
    };
    let provenance = if metadata.capability_provenance == "verified" {
        CapabilityProvenance::Verified
    } else {
        CapabilityProvenance::Configured
    };
    let capability = |value: &str| ProfileCapability {
        state: match value {
            "supported" => CapabilityState::Supported,
            "unsupported" => CapabilityState::Unsupported,
            _ => CapabilityState::Unknown,
        },
        provenance,
    };
    let capacity_provenance = match metadata.context_capacity_provenance.as_str() {
        "verified" => ContextCapacityProvenance::Verified,
        "configured-estimate" => ContextCapacityProvenance::ConfiguredEstimate,
        _ => ContextCapacityProvenance::Unknown,
    };
    EndpointProfileSnapshot::new(EndpointProfileSnapshot {
        id: profile.id.clone(),
        agent_id: "onepiece".to_string(),
        runtime_kind,
        endpoint_source,
        base_url: profile.base_url.clone().unwrap_or_default(),
        interface_format: profile.interface_format.clone(),
        model_id: profile.model_id.clone(),
        authentication_mode,
        credential_present,
        timeout_ms: u64::try_from(metadata.timeout_ms).unwrap_or_default(),
        privacy,
        capabilities: EndpointCapabilities {
            text_generation: capability(&metadata.text_generation_capability),
            tool_calling: capability(&metadata.tool_calling_capability),
            image_input: capability(&metadata.image_input_capability),
            structured_output: capability(&metadata.structured_output_capability),
            reasoning_field: capability(&metadata.reasoning_field_capability),
        },
        context_capacity: ProfileContextCapacity {
            context_window_tokens: metadata
                .context_window_tokens
                .and_then(|value| u64::try_from(value).ok()),
            reserved_output_tokens: u64::try_from(metadata.reserved_output_tokens)
                .unwrap_or_default(),
            provenance: capacity_provenance,
        },
    })
    .map_err(|_| {
        AgentRuntimeApplicationError::Validation(
            "Stored endpoint Profile metadata is invalid.".to_string(),
        )
    })
}

fn route_reason(reason: HybridRouteReason) -> &'static str {
    match reason {
        HybridRouteReason::ActiveRoutingDisabled => "hybrid-disabled",
        HybridRouteReason::RulePreferred => "rule-preferred",
        HybridRouteReason::RuleFallbackUnavailable => "rule-fallback-unavailable",
        HybridRouteReason::RuleFallbackIncapable => "rule-fallback-incapable",
        HybridRouteReason::ActiveNoMatch => "no-matching-rule",
        HybridRouteReason::WaitingLocalOnly => "waiting-local-only",
        HybridRouteReason::NoUsableProfile => "no-usable-profile",
    }
}

fn record_telemetry_result(
    ports: &AgentRuntimeApplicationPorts,
    context: &ExecutionContext,
    transition: &'static str,
    result: Result<(), crate::contexts::execution_observability::api::ExecutionTelemetryError>,
) {
    if result.is_ok() {
        return;
    }
    let _ = ports.logging.record(AgentLog {
        level: AgentLogLevel::Warn,
        category: "execution_telemetry".to_owned(),
        message: format!("execution telemetry transition failed: {transition}"),
        agent_id: None,
        session_id: None,
        operation_id: None,
        run_id: Some(context.run_id.as_str().to_owned()),
        trace_id: Some(context.trace_id.as_str().to_owned()),
        span_id: Some(context.span_id.as_str().to_owned()),
        occurred_at: ports.clock.now(),
    });
}

fn restore_credential(credentials: &dyn ApiCredentialPort, key: &str, value: Option<&str>) {
    match value {
        Some(secret) => {
            let _ = credentials.store(key, secret);
        }
        None => {
            let _ = credentials.remove(key);
        }
    }
}

fn push_model_option(
    models: &mut Vec<OnePieceProviderModelOption>,
    seen: &mut BTreeSet<String>,
    id: &str,
    display_name: &str,
    source: &str,
) {
    let normalized = id.trim();
    if !normalized.is_empty() && seen.insert(normalized.to_ascii_lowercase()) {
        models.push(OnePieceProviderModelOption {
            id: normalized.to_string(),
            display_name: display_name.trim().to_string(),
            source: source.to_string(),
        });
    }
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeApplicationPorts {
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) workflows: Arc<dyn super::AgentWorkflowRepository>,
    pub(crate) sessions: Arc<dyn AgentSessionGateway>,
    pub(crate) cli_profiles: Arc<dyn AgentCliProfileGateway>,
    pub(crate) prompts: Arc<dyn EffectivePromptGateway>,
    pub(crate) processes: Arc<dyn AgentProcessGateway>,
    pub(crate) operations: Arc<dyn AgentTaskPort>,
    pub(crate) logging: Arc<dyn AgentLoggingPort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
    pub(crate) events: Arc<dyn AgentEventPort>,
    pub(crate) generations: Arc<dyn AgentGenerationPort>,
    pub(crate) execution_ids: Arc<dyn ExecutionIdentityPort>,
    pub(crate) execution_settings: Arc<dyn ExecutionSettingsPort>,
    pub(crate) telemetry: Arc<dyn ExecutionTelemetryPort>,
    pub(crate) loop_completions: Arc<dyn LoopRoleGenerationCompletionPort>,
    pub(crate) seat_completions: Arc<dyn SeatTurnCompletionPort>,
    pub(crate) expert_roles: Arc<dyn super::ExpertRolePort>,
    pub(crate) history: Arc<dyn ConversationHistoryPort>,
    pub(crate) message_completions: Arc<dyn AgentMessageTerminalCompletionPort>,
    pub(crate) api_agents: Arc<dyn ApiAgentGateway>,
    pub(crate) api_credentials: Arc<dyn ApiCredentialPort>,
    pub(crate) onepiece_model_discovery: Arc<dyn OnePieceModelDiscoveryPort>,
    pub(crate) tool_approvals: Arc<dyn ToolApprovalPort>,
    pub(crate) memory_extraction: Arc<dyn super::AgentMemoryExtractionPort>,
    /// The one governed answer per CLI message and per completed CLI turn. Replaces reading flat
    /// settings and then listing every memory: those were two decisions taken at two moments, and
    /// a policy change between them produced a message whose instructions and whose memory index
    /// disagreed about what was permitted.
    pub(crate) personalization: Arc<dyn super::AgentPersonalizationSnapshotPort>,
    pub(crate) runner_discovery: Arc<dyn RunnerDiscoveryPort>,
}

#[derive(Clone)]
pub(crate) struct AgentRuntimeApplicationService {
    pub(super) ports: AgentRuntimeApplicationPorts,
}

/// Whether the human who triggered this turn is positioned to answer a question it asks
/// (`add-agent-user-question` D4). A scheduled run has no human at all. An IM-sourced turn has one,
/// but they are in a chat connector that cannot render a choice card -- the question would surface
/// on a desktop surface they are not looking at, so the wait would burn the turn either way.
fn interactive_message_source(source: &super::AgentMessageSource) -> bool {
    matches!(source, super::AgentMessageSource::Desktop)
}

pub(super) struct MessageGenerationInput {
    pub(super) source: super::AgentMessageSource,
    pub(super) configuration: AgentChatConfiguration,
    pub(super) content: String,
    pub(super) file_references: Vec<super::AgentFileReference>,
    /// A multi-seat session's role briefing, placed in the CLI's own system-prompt channel.
    pub(super) role_briefing: Option<String>,
    pub(super) seat_ownership: Option<super::SeatTurnOwnership>,
    /// A handoff prompt is written by the runtime, not by the user. Recording it as a user message
    /// would put words the human never typed into the thread under their name.
    pub(super) record_user_message: bool,
    /// Whether a human can answer a question this generation asks (`add-agent-user-question`).
    pub(super) interactive: bool,
    pub(super) runner: RunnerSelection,
}

struct GenerationFailure {
    safe_error: String,
    diagnostic: String,
}

fn generation_failure(
    safe_error: impl Into<String>,
    diagnostic: impl Into<String>,
) -> GenerationFailure {
    GenerationFailure {
        safe_error: safe_error.into(),
        diagnostic: diagnostic.into(),
    }
}

fn normalize_api_provider_config(
    provider: &str,
    model_id: &str,
    interface_format: &str,
    base_url: Option<&str>,
) -> Result<(String, String, String, Option<String>), AgentRuntimeApplicationError> {
    let provider = provider.trim().to_string();
    if provider.is_empty() {
        return Err(AgentRuntimeApplicationError::Validation(
            "Agent provider cannot be empty.".to_string(),
        ));
    }
    let model_id = model_id.trim().to_string();
    if model_id.is_empty() {
        return Err(AgentRuntimeApplicationError::Validation(
            "Model id cannot be empty.".to_string(),
        ));
    }
    let interface_format = interface_format.trim().to_string();
    if interface_format != INTERFACE_FORMAT_ANTHROPIC
        && interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE
    {
        return Err(AgentRuntimeApplicationError::Validation(
            "Interface format must be either \"anthropic\" or \"openai-compatible\".".to_string(),
        ));
    }
    let base_url = if interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        let value = base_url.unwrap_or_default().trim().to_string();
        if value.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Base URL is required for an OpenAI-compatible agent.".to_string(),
            ));
        }
        Some(value)
    } else {
        None
    };
    Ok((provider, model_id, interface_format, base_url))
}

impl AgentRuntimeApplicationService {
    pub(crate) fn new(ports: AgentRuntimeApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn take_seat_turn_completion(
        &self,
        session_id: &str,
    ) -> Result<Option<SeatTurnTerminal>, AgentRuntimeApplicationError> {
        self.ports.seat_completions.take_for_session(session_id)
    }

    #[cfg(test)]
    pub(crate) fn take_loop_role_completion(
        &self,
        session_id: &str,
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError> {
        self.ports.loop_completions.take_for_session(session_id)
    }

    fn start_loop_role_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        if session.loop_ownership.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Loop role generation requires an owned role session.".to_string(),
            ));
        }
        let agent = self
            .ports
            .registry
            .find(&session.agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(session.agent_id.clone()))?;
        let interaction_mode = if agent.supports(InteractionMode::Cli) {
            InteractionMode::Cli
        } else {
            InteractionMode::Api
        };
        let message = self.send_message(SendMessageRequest {
            session_id: session_id.to_string(),
            content: prompt.to_string(),
            source: super::AgentMessageSource::Desktop,
            configuration: AgentChatConfiguration {
                agent_id: session.agent_id,
                interaction_mode,
                execution_mode: "inherit".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            file_references: Vec::new(),
        })?;
        Ok(message.id)
    }

    pub(crate) fn register_api_agent(
        &self,
        input: RegisterApiAgentInput,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        let display_name = input.display_name.trim().to_string();
        if display_name.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Agent display name cannot be empty.".to_string(),
            ));
        }
        let api_key = input.api_key.trim().to_string();
        if input.authentication_mode == "required" && api_key.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "API key cannot be empty.".to_string(),
            ));
        }
        if input.authentication_mode == "none" && !api_key.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "An unauthenticated API Agent cannot store an API key.".to_string(),
            ));
        }
        let (provider, model_id, interface_format, base_url) = normalize_api_provider_config(
            &input.provider,
            &input.model_id,
            &input.interface_format,
            input.base_url.as_deref(),
        )?;

        if input.interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE
            && input.authentication_mode != "required"
        {
            return Err(AgentRuntimeApplicationError::Validation(
                "Only OpenAI-compatible endpoints can use optional authentication.".to_string(),
            ));
        }
        let runtime_kind = match input.runtime_kind.as_str() {
            "cloud" => ProfileRuntimeKind::Cloud,
            "local" => ProfileRuntimeKind::Local,
            "private" => ProfileRuntimeKind::Private,
            _ => {
                return Err(AgentRuntimeApplicationError::Validation(
                    "Invalid API endpoint runtime kind.".to_string(),
                ))
            }
        };
        let privacy = match input.privacy_classification.as_str() {
            "cloud" => ProfilePrivacy::Cloud,
            "local" => ProfilePrivacy::Local,
            "private" => ProfilePrivacy::Private,
            _ => {
                return Err(AgentRuntimeApplicationError::Validation(
                    "Invalid API endpoint privacy classification.".to_string(),
                ))
            }
        };
        if runtime_kind == ProfileRuntimeKind::Cloud && input.authentication_mode != "required" {
            return Err(AgentRuntimeApplicationError::Validation(
                "Cloud API endpoints require authentication.".to_string(),
            ));
        }
        if input.interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
            let authentication_mode = match input.authentication_mode.as_str() {
                "required" => AuthenticationMode::Required,
                "optional" => AuthenticationMode::Optional,
                "none" => AuthenticationMode::None,
                _ => {
                    return Err(AgentRuntimeApplicationError::Validation(
                        "Invalid API endpoint authentication mode.".to_string(),
                    ))
                }
            };
            EndpointProfileSnapshot::new(EndpointProfileSnapshot {
                id: "api-agent-registration".to_string(),
                agent_id: "api-agent-registration".to_string(),
                runtime_kind,
                endpoint_source: EndpointSource::Configured,
                base_url: base_url.clone().unwrap_or_default(),
                interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
                model_id: model_id.clone(),
                authentication_mode,
                credential_present: !api_key.is_empty(),
                timeout_ms: input.timeout_ms,
                privacy,
                capabilities: EndpointCapabilities {
                    text_generation: ProfileCapability::configured(CapabilityState::Supported),
                    tool_calling: ProfileCapability::configured(CapabilityState::Unknown),
                    image_input: ProfileCapability::configured(CapabilityState::Unknown),
                    structured_output: ProfileCapability::configured(CapabilityState::Unknown),
                    reasoning_field: ProfileCapability::configured(CapabilityState::Unknown),
                },
                context_capacity: ProfileContextCapacity {
                    context_window_tokens: None,
                    reserved_output_tokens: 0,
                    provenance: ContextCapacityProvenance::Unknown,
                },
            })
            .map_err(|_| {
                AgentRuntimeApplicationError::Validation(
                    "The API endpoint configuration is invalid or unsafe.".to_string(),
                )
            })?;
        }

        let agent_id = self.unique_api_agent_id(&display_name)?;
        if !api_key.is_empty() {
            self.ports.api_credentials.store(&agent_id, &api_key)?;
        }

        let normalized = RegisterApiAgentInput {
            display_name,
            provider,
            api_key,
            model_id,
            interface_format,
            base_url,
            runtime_kind: input.runtime_kind.clone(),
            authentication_mode: input.authentication_mode.clone(),
            timeout_ms: input.timeout_ms,
            privacy_classification: input.privacy_classification.clone(),
        };
        match self.ports.api_agents.register(&agent_id, &normalized) {
            Ok(definition) => {
                if let Err(error) = self.ports.api_agents.save_api_endpoint_metadata(
                    &agent_id,
                    &input.runtime_kind,
                    &input.authentication_mode,
                    input.timeout_ms,
                    &input.privacy_classification,
                ) {
                    let _ = self.ports.api_agents.delete(&agent_id);
                    let _ = self.ports.api_credentials.remove(&agent_id);
                    return Err(error);
                }
                Ok(AgentView::from(&definition))
            }
            Err(error) => {
                let _ = self.ports.api_credentials.remove(&agent_id);
                Err(error)
            }
        }
    }

    /// Reads back an API agent's current `model_id`/`interface_format`/`base_url` —
    /// `AgentView`/`AgentRegistryEntry` never carry these (they're CLI/API-agnostic view
    /// types), so this is the only way the frontend can pre-fill an edit form with the
    /// agent's current values, or know whether `base_url` is required, before submitting
    /// `update_api_agent` (`add-agent-lifecycle-management`, discovered as a necessary
    /// addition while wiring the edit UI — see tasks.md 5.1's note).
    pub(crate) fn api_agent_provider_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        self.ports.api_agents.provider_config(agent_id)
    }

    pub(crate) fn onepiece_provider_config(
        &self,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        let stored = self.ports.api_agents.onepiece_provider_config()?;
        let credential_present = self.ports.api_credentials.fetch("onepiece")?.is_some();
        Ok(OnePieceProviderConfig {
            provider: stored.provider,
            model_id: stored.model_id,
            interface_format: stored.interface_format,
            base_url: stored.base_url,
            auto_approve_tools: stored.auto_approve_tools,
            credential_present,
        })
    }

    pub(crate) fn save_onepiece_provider_config(
        &self,
        input: SaveOnePieceProviderConfigInput,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        let (provider, model_id, interface_format, base_url) = normalize_api_provider_config(
            &input.provider,
            &input.model_id,
            &input.interface_format,
            input.base_url.as_deref(),
        )?;
        let replacement = input
            .api_key
            .as_deref()
            .map(str::trim)
            .map(str::to_string)
            .filter(|value| !value.is_empty());
        if input.api_key.is_some() && replacement.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "API key cannot be empty.".to_string(),
            ));
        }
        let previous_credential = self.ports.api_credentials.fetch("onepiece")?;
        if replacement.is_none() && previous_credential.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "API key is required for the first OnePiece configuration.".to_string(),
            ));
        }
        let current = self.ports.api_agents.onepiece_provider_config()?;
        if let Some(api_key) = replacement.as_deref() {
            self.ports.api_credentials.store("onepiece", api_key)?;
        }
        let stored = StoredOnePieceProviderConfig {
            provider,
            model_id: Some(model_id),
            interface_format: Some(interface_format),
            base_url,
            auto_approve_tools: current.auto_approve_tools,
        };
        if let Err(error) = self.ports.api_agents.save_onepiece_provider_config(&stored) {
            if replacement.is_some() {
                match previous_credential.as_deref() {
                    Some(previous) => {
                        let _ = self.ports.api_credentials.store("onepiece", previous);
                    }
                    None => {
                        let _ = self.ports.api_credentials.remove("onepiece");
                    }
                }
            }
            return Err(error);
        }
        self.onepiece_provider_config()
    }

    pub(crate) fn reset_onepiece_provider_config(
        &self,
    ) -> Result<OnePieceProviderConfig, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        self.ports.api_agents.reset_onepiece_provider_config()?;
        self.ports.api_credentials.remove("onepiece")?;
        for profile in profiles {
            self.ports
                .api_credentials
                .remove(&onepiece_profile_credential_key(&profile.id))?;
        }
        self.onepiece_provider_config()
    }

    pub(crate) fn onepiece_provider_profiles(
        &self,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        let stored = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let active_profile_id = stored
            .iter()
            .find(|profile| profile.active)
            .map(|profile| profile.id.clone());
        let mut profiles = Vec::with_capacity(stored.len());
        for profile in stored {
            let authentication_none = self
                .ports
                .api_agents
                .endpoint_profile_metadata(&profile.id)?
                .is_some_and(|metadata| metadata.authentication_mode == "none");
            let mut credential_present = false;
            if !authentication_none {
                let credential_key = onepiece_profile_credential_key(&profile.id);
                credential_present = self.ports.api_credentials.fetch(&credential_key)?.is_some();
                if profile.active && !credential_present {
                    if let Some(legacy) = self.ports.api_credentials.fetch("onepiece")? {
                        self.ports.api_credentials.store(&credential_key, &legacy)?;
                        credential_present = true;
                    }
                }
            }
            profiles.push(OnePieceProviderProfile {
                id: profile.id,
                name: profile.name,
                source_provider_id: profile.source_provider_id,
                source_endpoint_type: profile.source_endpoint_type,
                source_preset_version: profile.source_preset_version,
                provider: profile.provider,
                model_id: profile.model_id,
                interface_format: profile.interface_format,
                base_url: profile.base_url,
                active: profile.active,
                credential_present,
            });
        }
        Ok(OnePieceProviderProfiles {
            profiles,
            active_profile_id,
        })
    }

    pub(crate) fn onepiece_provider_presets(&self) -> Vec<OnePieceProviderPreset> {
        super::onepiece_provider_catalog::list()
    }

    pub(crate) fn discover_onepiece_provider_models(
        &self,
        input: DiscoverOnePieceProviderModelsInput,
    ) -> Result<OnePieceProviderModelDiscoveryResult, AgentRuntimeApplicationError> {
        let provider_id = input.provider_id.trim().to_string();
        let endpoint_type = input.endpoint_type.trim().to_string();
        let preset = super::onepiece_provider_catalog::resolve(&provider_id, &endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece provider endpoint was not found.".to_string(),
                )
            })?;
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let profile = input
            .profile_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|profile_id| {
                profiles
                    .iter()
                    .find(|profile| profile.id == profile_id)
                    .ok_or_else(|| {
                        AgentRuntimeApplicationError::Validation(
                            "OnePiece provider profile was not found.".to_string(),
                        )
                    })
            })
            .transpose()?;
        if profile.is_some_and(|value| {
            value.source_provider_id.as_deref() != Some(provider_id.as_str())
                || value.source_endpoint_type.as_deref() != Some(endpoint_type.as_str())
        }) {
            return Err(AgentRuntimeApplicationError::Validation(
                "The OnePiece profile does not belong to the selected provider.".to_string(),
            ));
        }

        let transient_key = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let stored_key = profile
            .map(|value| {
                self.ports
                    .api_credentials
                    .fetch(&onepiece_profile_credential_key(&value.id))
            })
            .transpose()?
            .flatten();
        let credential = transient_key.or(stored_key);

        let mut models = Vec::new();
        let mut seen = BTreeSet::new();
        for id in &preset.fallback_models {
            push_model_option(&mut models, &mut seen, id, id, "catalog");
        }
        if let Some(value) = profile {
            push_model_option(
                &mut models,
                &mut seen,
                &value.model_id,
                &value.model_id,
                "profile",
            );
        }
        if preset.model_discovery_strategy == "catalog" {
            return Ok(OnePieceProviderModelDiscoveryResult {
                provider_id,
                endpoint_type,
                models,
                source: "catalog".to_string(),
                warning: None,
            });
        }
        let credential = credential.ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "API key is required to fetch models for this OnePiece provider.".to_string(),
            )
        })?;
        let url = super::onepiece_provider_catalog::discovery_url(&provider_id, &endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The OnePiece provider has no model discovery endpoint.".to_string(),
                )
            })?;
        match self
            .ports
            .onepiece_model_discovery
            .list_models(OnePieceModelDiscoveryRequest {
                strategy: preset.model_discovery_strategy,
                url,
                api_key: credential,
            }) {
            Ok(discovered) => {
                let discovered_count = discovered.len();
                for model in discovered
                    .into_iter()
                    .filter(|model| is_chat_model(&model.id))
                    .take(1_000)
                {
                    push_model_option(
                        &mut models,
                        &mut seen,
                        &model.id,
                        &model.display_name,
                        "api",
                    );
                }
                self.record_log(
                    AgentLogLevel::Info,
                    "onepiece.model-discovery",
                    format!(
                        "Fetched {discovered_count} model entries for provider {}.",
                        preset.id
                    ),
                    Some("onepiece"),
                    None,
                    None,
                );
                Ok(OnePieceProviderModelDiscoveryResult {
                    provider_id,
                    endpoint_type,
                    source: if models.iter().any(|model| model.source == "api") {
                        "merged".to_string()
                    } else {
                        "catalog".to_string()
                    },
                    models,
                    warning: None,
                })
            }
            Err(_) => {
                self.record_log(
                    AgentLogLevel::Warn,
                    "onepiece.model-discovery",
                    format!(
                        "Model discovery was unavailable for provider {}; catalog fallback used.",
                        preset.id
                    ),
                    Some("onepiece"),
                    None,
                    None,
                );
                Ok(OnePieceProviderModelDiscoveryResult {
                    provider_id,
                    endpoint_type,
                    models,
                    source: "catalog".to_string(),
                    warning: Some("live-unavailable".to_string()),
                })
            }
        }
    }

    pub(crate) fn validate_onepiece_provider_credential(
        &self,
        input: ValidateOnePieceProviderCredentialInput,
    ) -> Result<ProviderCredentialValidationResult, AgentRuntimeApplicationError> {
        let provider_id = input.provider_id.trim();
        let endpoint_type = input.endpoint_type.trim();
        let model_id = input.model_id.trim();
        if model_id.is_empty()
            || model_id.len() > 256
            || model_id.contains("..")
            || model_id.contains('\\')
            || model_id.chars().any(char::is_control)
        {
            return Err(AgentRuntimeApplicationError::Validation(
                "A valid model is required to verify the OnePiece API key.".to_string(),
            ));
        }
        let preset = super::onepiece_provider_catalog::resolve(provider_id, endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece provider endpoint was not found.".to_string(),
                )
            })?;
        let endpoint = preset
            .endpoints
            .iter()
            .find(|endpoint| endpoint.endpoint_type == endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece provider endpoint was not found.".to_string(),
                )
            })?;
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let profile = input
            .profile_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|profile_id| {
                profiles
                    .iter()
                    .find(|profile| profile.id == profile_id)
                    .ok_or_else(|| {
                        AgentRuntimeApplicationError::Validation(
                            "OnePiece provider profile was not found.".to_string(),
                        )
                    })
            })
            .transpose()?;
        if profile.is_some_and(|value| {
            value.source_provider_id.as_deref() != Some(provider_id)
                || value.source_endpoint_type.as_deref() != Some(endpoint_type)
        }) {
            return Err(AgentRuntimeApplicationError::Validation(
                "The OnePiece profile does not belong to the selected provider.".to_string(),
            ));
        }
        let transient_key = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        let stored_key = match (transient_key, profile) {
            (Some(_), _) => None,
            (None, Some(profile)) => self
                .ports
                .api_credentials
                .fetch(&onepiece_profile_credential_key(&profile.id))?,
            (None, None) => None,
        };
        let credential = transient_key.or(stored_key.as_deref()).ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "API key is required to verify this OnePiece provider.".to_string(),
            )
        })?;
        let protocol = match endpoint.endpoint_type.as_str() {
            "anthropic-messages" => ProviderCredentialProbeProtocol::AnthropicMessages,
            "openai-chat-completions" => ProviderCredentialProbeProtocol::OpenAiChatCompletions,
            "openai-responses" => ProviderCredentialProbeProtocol::OpenAiResponses,
            _ => {
                return Err(AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece endpoint cannot verify API credentials.".to_string(),
                ));
            }
        };
        let authentication = match endpoint.auth_strategy.as_str() {
            "x-api-key" => ProviderCredentialProbeAuthentication::AnthropicApiKey,
            "bearer" => ProviderCredentialProbeAuthentication::Bearer,
            _ => {
                return Err(AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece authentication strategy is unsupported.".to_string(),
                ));
            }
        };
        let result = self.ports.onepiece_model_discovery.validate_credential(
            ProviderCredentialProbeRequest {
                base_url: endpoint.base_url.clone(),
                model: model_id.to_string(),
                protocol,
                authentication,
                credential: credential.to_string(),
            },
        )?;
        self.record_log(
            AgentLogLevel::Info,
            "onepiece.credential-validation",
            format!(
                "OnePiece provider credential validation completed for {}: status={:?}.",
                preset.id, result.status
            ),
            Some("onepiece"),
            None,
            None,
        );
        Ok(result)
    }

    pub(crate) fn save_onepiece_provider_profile(
        &self,
        input: SaveOnePieceProviderProfileInput,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        let name = input.name.trim().to_string();
        if name.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Profile name cannot be empty.".to_string(),
            ));
        }
        let provider_id = input.provider_id.trim().to_string();
        let endpoint_type = input.endpoint_type.trim().to_string();
        let preset = super::onepiece_provider_catalog::resolve(&provider_id, &endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider endpoint was not found.".to_string(),
                )
            })?;
        let model_id = input.model_id.trim().to_string();
        if model_id.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Model id cannot be empty.".to_string(),
            ));
        }
        let existing = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let id = input
            .id
            .unwrap_or_else(|| format!("onepiece-profile-{}", Uuid::new_v4()));
        let previous = existing.iter().find(|profile| profile.id == id).cloned();
        if previous.as_ref().is_some_and(|profile| {
            profile
                .source_provider_id
                .as_deref()
                .is_some_and(|value| value != provider_id)
                || profile
                    .source_endpoint_type
                    .as_deref()
                    .is_some_and(|value| value != endpoint_type)
        }) {
            return Err(AgentRuntimeApplicationError::Validation(
                "The provider of an existing OnePiece Profile cannot be changed.".to_string(),
            ));
        }
        let active = previous.as_ref().is_some_and(|profile| profile.active) || existing.is_empty();
        let credential_key = onepiece_profile_credential_key(&id);
        let previous_scoped = self.ports.api_credentials.fetch(&credential_key)?;
        let previous_runtime = active
            .then(|| self.ports.api_credentials.fetch("onepiece"))
            .transpose()?
            .flatten();
        let replacement = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if input.api_key.is_some() && replacement.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "API key cannot be empty.".to_string(),
            ));
        }
        let effective = replacement
            .clone()
            .or(previous_scoped.clone())
            .or_else(|| active.then_some(previous_runtime.clone()).flatten());
        let Some(effective_credential) = effective else {
            return Err(AgentRuntimeApplicationError::Validation(
                "API key is required for a new OnePiece provider Profile.".to_string(),
            ));
        };
        let scoped_credential_changed = replacement.is_some() || previous_scoped.is_none();
        if scoped_credential_changed {
            self.ports
                .api_credentials
                .store(&credential_key, &effective_credential)?;
        }
        if active {
            if let Err(error) = self
                .ports
                .api_credentials
                .store("onepiece", &effective_credential)
            {
                if scoped_credential_changed {
                    restore_credential(
                        self.ports.api_credentials.as_ref(),
                        &credential_key,
                        previous_scoped.as_deref(),
                    );
                }
                restore_credential(
                    self.ports.api_credentials.as_ref(),
                    "onepiece",
                    previous_runtime.as_deref(),
                );
                return Err(error);
            }
        }
        let stored = StoredOnePieceProviderProfile {
            id,
            name,
            source_preset_id: Some(provider_id.clone()),
            source_provider_id: Some(provider_id),
            source_endpoint_type: Some(endpoint_type),
            source_preset_version: Some(preset.catalog_version),
            provider: preset.provider,
            model_id,
            interface_format: preset.interface_format,
            base_url: preset.base_url,
            active,
        };
        if let Err(error) = self
            .ports
            .api_agents
            .save_onepiece_provider_profile(&stored)
        {
            restore_credential(
                self.ports.api_credentials.as_ref(),
                &credential_key,
                previous_scoped.as_deref(),
            );
            if active {
                restore_credential(
                    self.ports.api_credentials.as_ref(),
                    "onepiece",
                    previous_runtime.as_deref(),
                );
            }
            return Err(error);
        }
        self.onepiece_provider_profiles()
    }

    pub(crate) fn save_custom_onepiece_provider_profile(
        &self,
        input: SaveCustomOnePieceProviderProfileInput,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        let existing = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| format!("onepiece-profile-{}", Uuid::new_v4()));
        let previous = existing.iter().find(|profile| profile.id == id).cloned();
        if previous
            .as_ref()
            .is_some_and(|profile| profile.source_provider_id.is_some())
        {
            return Err(AgentRuntimeApplicationError::Validation(
                "A catalog Profile cannot be converted to a custom endpoint.".to_string(),
            ));
        }
        let active = previous.as_ref().is_some_and(|profile| profile.active) || existing.is_empty();
        let replacement = input
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let credential_key = onepiece_profile_credential_key(&id);
        let previous_credential = if input.authentication_mode == "none" {
            None
        } else {
            self.ports.api_credentials.fetch(&credential_key)?
        };
        let effective_credential = replacement.clone().or(previous_credential.clone());
        let snapshot = custom_profile_snapshot(&id, &input, effective_credential.is_some())?;
        match snapshot.authentication_mode {
            AuthenticationMode::None => {
                if previous.is_some() {
                    self.ports.api_credentials.remove(&credential_key)?;
                    if active {
                        self.ports.api_credentials.remove("onepiece")?;
                    }
                }
            }
            AuthenticationMode::Required | AuthenticationMode::Optional => {
                if let Some(credential) = effective_credential.as_deref() {
                    self.ports
                        .api_credentials
                        .store(&credential_key, credential)?;
                    if active {
                        self.ports.api_credentials.store("onepiece", credential)?;
                    }
                }
            }
        }
        let stored = StoredOnePieceProviderProfile {
            id: id.clone(),
            name: input.name.trim().to_string(),
            source_preset_id: None,
            source_provider_id: None,
            source_endpoint_type: None,
            source_preset_version: None,
            provider: if snapshot.is_local() {
                "Local endpoint".to_string()
            } else {
                "Private endpoint".to_string()
            },
            model_id: snapshot.model_id.clone(),
            interface_format: snapshot.interface_format.clone(),
            base_url: Some(snapshot.base_url.clone()),
            active,
        };
        self.ports
            .api_agents
            .save_onepiece_provider_profile(&stored)?;
        self.ports
            .api_agents
            .save_endpoint_profile_metadata(&StoredEndpointProfileMetadata {
                profile_id: id,
                runtime_kind: input.runtime_kind,
                endpoint_source: "configured".to_string(),
                authentication_mode: input.authentication_mode,
                timeout_ms: i64::try_from(input.timeout_ms).map_err(|_| {
                    AgentRuntimeApplicationError::Validation(
                        "Invalid endpoint timeout.".to_string(),
                    )
                })?,
                privacy_classification: input.privacy_classification,
                text_generation_capability: "supported".to_string(),
                tool_calling_capability: input.tool_calling_capability,
                image_input_capability: input.image_input_capability,
                structured_output_capability: input.structured_output_capability,
                reasoning_field_capability: input.reasoning_field_capability,
                capability_provenance: "configured".to_string(),
                context_window_tokens: input
                    .context_window_tokens
                    .map(i64::try_from)
                    .transpose()
                    .map_err(|_| {
                    AgentRuntimeApplicationError::Validation(
                        "Invalid context capacity.".to_string(),
                    )
                })?,
                reserved_output_tokens: i64::try_from(input.reserved_output_tokens).map_err(
                    |_| {
                        AgentRuntimeApplicationError::Validation(
                            "Invalid context reserve.".to_string(),
                        )
                    },
                )?,
                context_capacity_provenance: if input.context_window_tokens.is_some() {
                    "configured-estimate"
                } else {
                    "unknown"
                }
                .to_string(),
            })?;
        self.onepiece_provider_profiles()
    }

    pub(crate) fn endpoint_profile_metadata(
        &self,
        profile_id: &str,
    ) -> Result<Option<StoredEndpointProfileMetadata>, AgentRuntimeApplicationError> {
        self.ports.api_agents.endpoint_profile_metadata(profile_id)
    }

    pub(crate) fn hybrid_routing_rules(
        &self,
    ) -> Result<Vec<StoredHybridRoutingRule>, AgentRuntimeApplicationError> {
        self.ports.api_agents.list_hybrid_routing_rules()
    }

    pub(crate) fn replace_hybrid_routing_rules(
        &self,
        rules: Vec<StoredHybridRoutingRule>,
    ) -> Result<Vec<StoredHybridRoutingRule>, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let mut ids = BTreeSet::new();
        for (expected_order, rule) in rules.iter().enumerate() {
            if rule.id.trim().is_empty()
                || !ids.insert(rule.id.as_str())
                || rule.position != u32::try_from(expected_order).unwrap_or(u32::MAX)
                || task_class(&rule.task_class).is_err()
                || data_policy(&rule.data_policy).is_err()
                || !profiles
                    .iter()
                    .any(|profile| profile.id == rule.preferred_profile_id)
                || rule.fallback_profile_id.as_ref().is_some_and(|id| {
                    !profiles.iter().any(|profile| profile.id == *id)
                        || id == &rule.preferred_profile_id
                })
            {
                return Err(AgentRuntimeApplicationError::Validation(
                    "Hybrid Routing rules are invalid or reference missing Profiles.".to_string(),
                ));
            }
        }
        self.ports.api_agents.replace_hybrid_routing_rules(&rules)?;
        self.hybrid_routing_rules()
    }

    pub(crate) fn preview_hybrid_route(
        &self,
        input: HybridRoutePreviewInput,
    ) -> Result<HybridRoutePreview, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let mut snapshots = Vec::with_capacity(profiles.len());
        let mut readiness = Vec::with_capacity(profiles.len());
        for profile in &profiles {
            let metadata = self
                .ports
                .api_agents
                .endpoint_profile_metadata(&profile.id)?
                .unwrap_or(StoredEndpointProfileMetadata {
                    profile_id: profile.id.clone(),
                    runtime_kind: "cloud".to_string(),
                    endpoint_source: "catalog".to_string(),
                    authentication_mode: "required".to_string(),
                    timeout_ms: 30_000,
                    privacy_classification: "cloud".to_string(),
                    text_generation_capability: "supported".to_string(),
                    tool_calling_capability: "unknown".to_string(),
                    image_input_capability: "unknown".to_string(),
                    structured_output_capability: "unknown".to_string(),
                    reasoning_field_capability: "unknown".to_string(),
                    capability_provenance: "configured".to_string(),
                    context_window_tokens: None,
                    reserved_output_tokens: 0,
                    context_capacity_provenance: "unknown".to_string(),
                });
            let credential_present = if metadata.authentication_mode == "required" {
                self.ports
                    .api_credentials
                    .fetch(&onepiece_profile_credential_key(&profile.id))?
                    .is_some()
            } else {
                false
            };
            let ready = metadata.authentication_mode != "required" || credential_present;
            let invariant_credential =
                credential_present || metadata.authentication_mode == "required";
            snapshots.push(profile_snapshot(profile, metadata, invariant_credential)?);
            readiness.push(ready);
        }
        let candidates = snapshots
            .iter()
            .zip(readiness)
            .map(|(profile, ready)| RouteCandidate { profile, ready })
            .collect::<Vec<_>>();
        let stored_rules = self.ports.api_agents.list_hybrid_routing_rules()?;
        let rules = stored_rules
            .iter()
            .map(|rule| {
                Ok(HybridRoutingRule {
                    id: rule.id.clone(),
                    enabled: rule.enabled,
                    order: rule.position,
                    task_class: task_class(&rule.task_class)?,
                    preferred_profile_id: rule.preferred_profile_id.clone(),
                    fallback_profile_id: rule.fallback_profile_id.clone(),
                })
            })
            .collect::<Result<Vec<_>, AgentRuntimeApplicationError>>()?;
        let decision = route_profile(HybridRouteRequest {
            routing_enabled: input.hybrid_enabled,
            task_class: task_class(&input.task_class)?,
            data_policy: data_policy(&input.data_policy)?,
            active_profile_id: input.active_profile_id.as_deref(),
            required_capabilities: RequiredCapabilities {
                tools: input.requires_tools,
                image: input.requires_image_input,
                structured_output: input.requires_structured_output,
                reasoning: input.requests_reasoning_field,
            },
            rules: &rules,
            candidates: &candidates,
        });
        Ok(match decision {
            HybridRouteDecision::Selected {
                profile_id,
                rule_id,
                reason,
            } => HybridRoutePreview {
                profile_id: Some(profile_id),
                rule_id,
                reason: route_reason(reason).to_string(),
                waiting_for_user_choice: false,
            },
            HybridRouteDecision::WaitingForUserChoice { rule_id, reason } => HybridRoutePreview {
                profile_id: None,
                rule_id,
                reason: route_reason(reason).to_string(),
                waiting_for_user_choice: true,
            },
            HybridRouteDecision::Rejected { reason } => HybridRoutePreview {
                profile_id: None,
                rule_id: None,
                reason: route_reason(reason).to_string(),
                waiting_for_user_choice: false,
            },
        })
    }

    pub(super) fn freeze_endpoint_profile(
        &self,
        agent_id: &str,
        prompt: &str,
    ) -> Result<Option<FrozenEndpointProfile>, AgentRuntimeApplicationError> {
        if agent_id != "onepiece" {
            return Ok(None);
        }
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let active_profile_id = profiles
            .iter()
            .find(|profile| profile.active)
            .map(|profile| profile.id.clone());
        let rules = self.ports.api_agents.list_hybrid_routing_rules()?;
        let task = classify_task(prompt);
        let matching_rule = rules
            .iter()
            .filter(|rule| rule.enabled && rule.task_class == task)
            .min_by_key(|rule| (rule.position, rule.id.as_str()));
        let policy = matching_rule
            .map(|rule| rule.data_policy.as_str())
            .unwrap_or("cloud-allowed");
        let decision = self.preview_hybrid_route(HybridRoutePreviewInput {
            task_class: task.to_string(),
            data_policy: policy.to_string(),
            active_profile_id,
            hybrid_enabled: rules.iter().any(|rule| rule.enabled),
            requires_tools: false,
            requires_image_input: false,
            requires_structured_output: false,
            requests_reasoning_field: false,
        })?;
        let Some(profile_id) = decision.profile_id else {
            return Err(AgentRuntimeApplicationError::Validation(
                if decision.waiting_for_user_choice {
                    "Local-only routing is waiting for a usable local Profile.".to_string()
                } else {
                    "Hybrid Routing found no usable Profile.".to_string()
                },
            ));
        };
        let profile = profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The routed endpoint Profile no longer exists.".to_string(),
                )
            })?;
        let metadata = self
            .ports
            .api_agents
            .endpoint_profile_metadata(&profile.id)?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The routed endpoint Profile has no metadata.".to_string(),
                )
            })?;
        Ok(Some(FrozenEndpointProfile {
            profile_id: profile.id,
            source_provider_id: profile.source_provider_id,
            model_id: profile.model_id,
            interface_format: profile.interface_format,
            base_url: profile.base_url,
            authentication_mode: metadata.authentication_mode,
            timeout_ms: u64::try_from(metadata.timeout_ms).map_err(|_| {
                AgentRuntimeApplicationError::Validation("Invalid endpoint timeout.".to_string())
            })?,
            image_input_capability: metadata.image_input_capability,
            tool_calling_capability: metadata.tool_calling_capability,
            structured_output_capability: metadata.structured_output_capability,
            reasoning_field_capability: metadata.reasoning_field_capability,
            context_window_tokens: metadata
                .context_window_tokens
                .map(u64::try_from)
                .transpose()
                .map_err(|_| {
                    AgentRuntimeApplicationError::Validation(
                        "Invalid endpoint context capacity.".to_string(),
                    )
                })?,
            reserved_output_tokens: u64::try_from(metadata.reserved_output_tokens).map_err(
                |_| {
                    AgentRuntimeApplicationError::Validation(
                        "Invalid endpoint context reserve.".to_string(),
                    )
                },
            )?,
            context_capacity_provenance: metadata.context_capacity_provenance,
            routing_rule_id: decision.rule_id,
            routing_reason: decision.reason,
        }))
    }

    pub(crate) fn activate_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let target = profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })?;
        let target_key = onepiece_profile_credential_key(profile_id);
        let authentication_mode = self
            .ports
            .api_agents
            .endpoint_profile_metadata(profile_id)?
            .map(|metadata| metadata.authentication_mode)
            .unwrap_or_else(|| "required".to_string());
        if authentication_mode != "required" {
            self.ports
                .api_agents
                .activate_onepiece_provider_profile(&target.id)?;
            return self.onepiece_provider_profiles();
        }
        let target_credential = self.ports.api_credentials.fetch(&target_key)?;
        if authentication_mode == "required" && target_credential.is_none() {
            return Err(AgentRuntimeApplicationError::Validation(
                "The selected OnePiece provider Profile has no API key.".to_string(),
            ));
        }
        let previous_runtime = self.ports.api_credentials.fetch("onepiece")?;
        if let Some(current) = profiles.iter().find(|profile| profile.active) {
            let current_key = onepiece_profile_credential_key(&current.id);
            if self.ports.api_credentials.fetch(&current_key)?.is_none() {
                if let Some(secret) = previous_runtime.as_deref() {
                    self.ports.api_credentials.store(&current_key, secret)?;
                }
            }
        }
        let credential_update = if let Some(target_credential) = target_credential.as_deref() {
            self.ports
                .api_credentials
                .store("onepiece", target_credential)
        } else {
            self.ports.api_credentials.remove("onepiece")
        };
        if let Err(error) = credential_update {
            restore_credential(
                self.ports.api_credentials.as_ref(),
                "onepiece",
                previous_runtime.as_deref(),
            );
            return Err(error);
        }
        if let Err(error) = self
            .ports
            .api_agents
            .activate_onepiece_provider_profile(&target.id)
        {
            restore_credential(
                self.ports.api_credentials.as_ref(),
                "onepiece",
                previous_runtime.as_deref(),
            );
            return Err(error);
        }
        self.onepiece_provider_profiles()
    }

    pub(crate) fn delete_onepiece_provider_profile(
        &self,
        profile_id: &str,
    ) -> Result<OnePieceProviderProfiles, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let profile = profiles
            .iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })?;
        let credential_key = onepiece_profile_credential_key(profile_id);
        let previous_scoped = self.ports.api_credentials.fetch(&credential_key)?;
        let previous_runtime = profile
            .active
            .then(|| self.ports.api_credentials.fetch("onepiece"))
            .transpose()?
            .flatten();
        self.ports.api_credentials.remove(&credential_key)?;
        if profile.active {
            if let Err(error) = self.ports.api_credentials.remove("onepiece") {
                restore_credential(
                    self.ports.api_credentials.as_ref(),
                    &credential_key,
                    previous_scoped.as_deref(),
                );
                restore_credential(
                    self.ports.api_credentials.as_ref(),
                    "onepiece",
                    previous_runtime.as_deref(),
                );
                return Err(error);
            }
        }
        if let Err(error) = self
            .ports
            .api_agents
            .delete_onepiece_provider_profile(profile_id)
        {
            restore_credential(
                self.ports.api_credentials.as_ref(),
                &credential_key,
                previous_scoped.as_deref(),
            );
            if profile.active {
                restore_credential(
                    self.ports.api_credentials.as_ref(),
                    "onepiece",
                    previous_runtime.as_deref(),
                );
            }
            return Err(error);
        }
        self.onepiece_provider_profiles()
    }

    // 凭据只读出一次、原样放进返回值供进程内传递给 bootstrap 的 embedding 端点适配器——
    // 不写日志、不拼进错误消息（见下方各分支，全部是不含凭据的静态字符串）。
    pub(crate) fn resolve_embedding_endpoint(
        &self,
        profile_id: &str,
    ) -> Result<EmbeddingEndpointView, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let profile = profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })?;
        if profile.interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE {
            return Err(AgentRuntimeApplicationError::Validation(
                "Only openai-compatible OnePiece profiles support embeddings.".to_string(),
            ));
        }
        let base_url = profile.base_url.ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "The OnePiece profile has no base URL configured.".to_string(),
            )
        })?;
        let credential = self
            .ports
            .api_credentials
            .fetch(&onepiece_profile_credential_key(&profile.id))?
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece provider Profile has no API key.".to_string(),
                )
            })?;
        Ok(EmbeddingEndpointView {
            base_url,
            interface_format: profile.interface_format,
            credential,
        })
    }

    // 凭据只用于组装发往 HttpOnePieceModelDiscoveryAdapter 的请求，从不进入返回值或
    // 错误消息——发现失败时直接把底层 Err 冒泡（该错误本身也不含凭据，见
    // onepiece_model_discovery.rs 的 discovery_error），不像 discover_onepiece_provider_models
    // 那样回退到目录：目录里没有任何 embedding 模型数据可回退。
    pub(crate) fn list_embedding_models(
        &self,
        profile_id: &str,
        transient_credential: Option<&str>,
    ) -> Result<Vec<OnePieceProviderModelOption>, AgentRuntimeApplicationError> {
        let profiles = self.ports.api_agents.list_onepiece_provider_profiles()?;
        let profile = profiles
            .into_iter()
            .find(|profile| profile.id == profile_id)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "OnePiece provider profile was not found.".to_string(),
                )
            })?;
        if profile.interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE {
            return Err(AgentRuntimeApplicationError::Validation(
                "Only openai-compatible OnePiece profiles support embedding model discovery."
                    .to_string(),
            ));
        }
        let provider_id = profile.source_provider_id.ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "The OnePiece profile is missing its source provider.".to_string(),
            )
        })?;
        let endpoint_type = profile.source_endpoint_type.ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "The OnePiece profile is missing its source endpoint.".to_string(),
            )
        })?;
        let preset = super::onepiece_provider_catalog::resolve(&provider_id, &endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The selected OnePiece provider endpoint was not found.".to_string(),
                )
            })?;
        let transient = transient_credential
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let stored = self
            .ports
            .api_credentials
            .fetch(&onepiece_profile_credential_key(&profile.id))?;
        let credential = transient.or(stored).ok_or_else(|| {
            AgentRuntimeApplicationError::Validation(
                "API key is required to list embedding models for this OnePiece provider."
                    .to_string(),
            )
        })?;
        let url = super::onepiece_provider_catalog::discovery_url(&provider_id, &endpoint_type)
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation(
                    "The OnePiece provider has no model discovery endpoint.".to_string(),
                )
            })?;
        let discovered =
            self.ports
                .onepiece_model_discovery
                .list_models(OnePieceModelDiscoveryRequest {
                    strategy: preset.model_discovery_strategy,
                    url,
                    api_key: credential,
                })?;
        let mut models = Vec::new();
        let mut seen = BTreeSet::new();
        for model in discovered
            .into_iter()
            .filter(|model| is_embedding_model(&model.id))
            .take(1_000)
        {
            push_model_option(
                &mut models,
                &mut seen,
                &model.id,
                &model.display_name,
                "api",
            );
        }
        Ok(models)
    }

    /// Edits an existing API agent's `display_name`/`model_id`/`base_url`, and optionally
    /// rotates its stored API key. `provider`/`interface_format` are immutable after
    /// registration (`add-agent-lifecycle-management` design.md Decision 1) — re-validates like
    /// `register_api_agent`, but against the agent's *current* `interface_format` (read via
    /// `provider_config`), since that isn't part of the update payload.
    pub(crate) fn update_api_agent(
        &self,
        agent_id: &str,
        input: UpdateApiAgentInput,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        let display_name = input.display_name.trim().to_string();
        if display_name.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Agent display name cannot be empty.".to_string(),
            ));
        }
        let model_id = input.model_id.trim().to_string();
        if model_id.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Model id cannot be empty.".to_string(),
            ));
        }
        let current = self
            .ports
            .api_agents
            .provider_config(agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))?;
        let base_url = if current.interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
            let base_url = input
                .base_url
                .as_deref()
                .unwrap_or_default()
                .trim()
                .to_string();
            if base_url.is_empty() {
                return Err(AgentRuntimeApplicationError::Validation(
                    "Base URL is required for an OpenAI-compatible agent.".to_string(),
                ));
            }
            Some(base_url)
        } else {
            None
        };
        if let Some(new_api_key) = input.new_api_key.as_deref() {
            let new_api_key = new_api_key.trim();
            if new_api_key.is_empty() {
                return Err(AgentRuntimeApplicationError::Validation(
                    "API key cannot be empty.".to_string(),
                ));
            }
            self.ports.api_credentials.store(agent_id, new_api_key)?;
        }
        let normalized = UpdateApiAgentInput {
            display_name,
            model_id,
            base_url,
            new_api_key: None,
        };
        let definition = self.ports.api_agents.update(agent_id, &normalized)?;
        Ok(AgentView::from(&definition))
    }

    /// Deletes a registered API agent and its stored credential. The repository rejects (and
    /// changes nothing) if the agent is still referenced by other stored data
    /// (`add-agent-lifecycle-management` design.md Decision 2) — `credentials.remove` only runs
    /// after that succeeds; its own failure is logged and otherwise ignored rather than
    /// reported back as the operation's result, matching this codebase's existing best-effort
    /// posture for credential cleanup (`register_api_agent`'s registration-failure rollback).
    pub(crate) fn delete_api_agent(
        &self,
        agent_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if self
            .ports
            .registry
            .find(agent_id)?
            .is_some_and(|agent| agent.origin() == AgentOrigin::Builtin)
        {
            return Err(AgentRuntimeApplicationError::Validation(
                "Built-in agents cannot be deleted; reset their provider configuration instead."
                    .to_string(),
            ));
        }
        self.ports.api_agents.delete(agent_id)?;
        if let Err(error) = self.ports.api_credentials.remove(agent_id) {
            self.record_log(
                AgentLogLevel::Warn,
                "session.runtime.api.credentials",
                format!("Failed to remove stored credential after deleting the agent: {error}"),
                Some(agent_id),
                None,
                None,
            );
        }
        Ok(())
    }

    /// Delivers a user's approve/deny decision for a native-agent tool call awaiting approval.
    /// Returns `false` if no such pending approval exists.
    pub(crate) fn resolve_tool_approval(
        &self,
        session_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let Some(process_id) = self.ports.generations.active_process_id(session_id)? else {
            return Ok(false);
        };
        self.ports
            .tool_approvals
            .resolve(&process_id, call_id, decision)
    }

    fn unique_api_agent_id(
        &self,
        display_name: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let base = slugify(display_name);
        let base = if base.is_empty() {
            "api-agent".to_string()
        } else {
            base
        };
        for attempt in 0..50 {
            let candidate = if attempt == 0 {
                base.clone()
            } else {
                format!("{base}-{}", attempt + 1)
            };
            if self.ports.registry.find(&candidate)?.is_none() {
                return Ok(candidate);
            }
        }
        Err(AgentRuntimeApplicationError::Validation(
            "Could not derive a unique agent id from the display name.".to_string(),
        ))
    }

    pub(crate) fn list_agents(
        &self,
        capability_tag: Option<&str>,
    ) -> Result<Vec<AgentView>, AgentRuntimeApplicationError> {
        let agents = self.ports.registry.list()?;
        Ok(agents
            .iter()
            .filter(|agent| {
                capability_tag
                    .map(|tag| agent.has_capability(tag))
                    .unwrap_or(true)
            })
            .map(AgentView::from)
            .collect())
    }

    pub(crate) fn get_agent(
        &self,
        agent_id: &str,
    ) -> Result<AgentView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        Ok(AgentView::from(&agent))
    }

    pub(crate) fn workflow(&self) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        Ok(WorkflowView::from(&self.ports.workflows.load()?))
    }

    pub(crate) fn select_agent(
        &self,
        agent_id: &str,
        interaction_mode: InteractionMode,
    ) -> Result<WorkflowView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        let current = self.ports.workflows.load()?;
        let mut workflow = AgentWorkflow::rehydrate(
            current
                .active_agent_id()
                .map(|active| active.as_str().to_string()),
            current.active_interaction_mode(),
            current.lifecycle(),
            current.intent().to_string(),
        )?;
        workflow.select(&agent, interaction_mode)?;
        self.ports.workflows.save(&workflow)?;
        let view = WorkflowView::from(&workflow);
        let _ = self
            .ports
            .events
            .publish(AgentEvent::WorkflowChanged(view.clone()));
        Ok(view)
    }

    pub(crate) fn browser_readiness(
        &self,
        agent_id: &str,
    ) -> Result<ReadinessView, AgentRuntimeApplicationError> {
        let agent = self.require_agent(agent_id)?;
        Ok(AgentReadiness::for_browser(&agent).into())
    }

    pub(crate) fn session_details(
        &self,
    ) -> Result<AgentSessionDetails, AgentRuntimeApplicationError> {
        let workflow = self.workflow()?;
        let (adapter, details) = self.ports.workflows.load_details()?;
        Ok(AgentSessionDetails {
            workflow,
            adapter,
            details,
        })
    }

    pub(crate) fn launch_active_workflow(
        &self,
    ) -> Result<LaunchWorkflowResult, AgentRuntimeApplicationError> {
        let mut workflow = self.ports.workflows.load()?;
        let agent_id = workflow
            .active_agent_id()
            .map(|value| value.as_str().to_string())
            .ok_or(AgentRuntimeApplicationError::NoActiveAgent)?;
        let interaction_mode = workflow
            .active_interaction_mode()
            .ok_or(AgentRuntimeApplicationError::NoActiveAgent)?;
        let agent = self.require_agent(&agent_id)?;
        let operation = self
            .ports
            .operations
            .start_agent_launch(&agent_id, &format!("Launching {}", agent.display_name()))?;

        workflow.begin_launch()?;
        self.ports.workflows.save(&workflow)?;
        let launch = self.ports.processes.launch_workflow(WorkflowLaunchRequest {
            operation_id: operation.id.clone(),
            agent: AgentView::from(&agent),
            interaction_mode,
        });
        let outcome = match launch {
            Ok(outcome) => outcome,
            Err(error) => {
                let _ = workflow.mark_failed();
                let _ = self.ports.workflows.save(&workflow);
                let _ = self.ports.operations.fail(&operation.id, error.to_string());
                self.record_log(
                    AgentLogLevel::Error,
                    "agent.launch",
                    error.to_string(),
                    Some(&agent_id),
                    None,
                    Some(&operation.id),
                );
                return Err(error);
            }
        };

        self.ports
            .workflows
            .save_details(&outcome.adapter, &outcome.message)?;
        workflow.mark_running()?;
        self.ports.workflows.save(&workflow)?;
        let _ = self
            .ports
            .operations
            .append_log(&operation.id, outcome.message.clone());
        let _ = self.ports.operations.complete(&operation.id);
        self.record_log(
            AgentLogLevel::Info,
            "agent.launch",
            outcome.message.clone(),
            Some(&agent_id),
            None,
            Some(&operation.id),
        );
        let workflow = WorkflowView::from(&workflow);
        let _ = self
            .ports
            .events
            .publish(AgentEvent::WorkflowChanged(workflow.clone()));
        Ok(LaunchWorkflowResult {
            operation_id: operation.id,
            workflow,
            message: outcome.message,
        })
    }

    pub(crate) fn send_message(
        &self,
        request: SendMessageRequest,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.send_message_internal(request, RunnerSelection::local(), false, true)
            .map(|(message, _)| message)
    }

    pub(crate) fn send_message_with_runner(
        &self,
        request: SendMessageRequest,
        runner: RunnerSelection,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        runner
            .validate()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.code().to_string()))?;
        self.send_message_internal(request, runner, false, true)
            .map(|(message, _)| message)
    }

    pub(crate) fn list_runners(
        &self,
        session_id: &str,
        agent_id: &str,
    ) -> Result<Vec<RunnerDescriptor>, AgentRuntimeApplicationError> {
        self.ports.runner_discovery.list(session_id, agent_id)
    }

    pub(crate) fn send_message_with_completion(
        &self,
        request: SendMessageRequest,
    ) -> Result<StartedAgentMessage, AgentRuntimeApplicationError> {
        let (message, terminal) =
            self.send_message_internal(request, RunnerSelection::local(), true, true)?;
        let terminal = terminal.ok_or_else(|| {
            AgentRuntimeApplicationError::Generation(
                "message completion registration was not created".to_string(),
            )
        })?;
        Ok(StartedAgentMessage { message, terminal })
    }

    pub(crate) fn active_generation_correlation(
        &self,
        session_id: &str,
    ) -> Result<Option<super::ActiveGenerationCorrelation>, AgentRuntimeApplicationError> {
        self.ports.generations.active_correlation(session_id)
    }

    /// Orchestration entry point (`add-agent-user-question`). Plan attempts, repairs, and
    /// discovery all send with `AgentMessageSource::Desktop` and own no Loop session, so they are
    /// indistinguishable from an ordinary chat turn by inspection -- the caller has to say so.
    pub(crate) fn send_non_interactive_message_with_completion(
        &self,
        request: SendMessageRequest,
    ) -> Result<StartedAgentMessage, AgentRuntimeApplicationError> {
        let (message, terminal) =
            self.send_message_internal(request, RunnerSelection::local(), true, false)?;
        let terminal = terminal.ok_or_else(|| {
            AgentRuntimeApplicationError::Generation(
                "message completion registration was not created".to_string(),
            )
        })?;
        Ok(StartedAgentMessage { message, terminal })
    }

    fn send_message_internal(
        &self,
        request: SendMessageRequest,
        runner: RunnerSelection,
        register_completion: bool,
        caller_is_interactive: bool,
    ) -> Result<
        (AgentMessage, Option<super::AgentMessageTerminalReceiver>),
        AgentRuntimeApplicationError,
    > {
        let content = request.content.trim().to_string();
        if content.is_empty() {
            return Err(AgentRuntimeApplicationError::Validation(
                "Message content cannot be empty.".to_string(),
            ));
        }
        let session = self.require_session(&request.session_id)?;
        if session.archived {
            return Err(AgentRuntimeApplicationError::Validation(
                "Archived sessions cannot accept messages.".to_string(),
            ));
        }
        let initial_seat_context =
            self.initial_seat_turn_context(&session, &content, &request.source)?;
        // The addressed seat runs its own Agent, which is not necessarily the session's: the
        // session mirrors the *first* seat, so honouring a mention while still invoking
        // `session.agent_id` would answer as one participant in another's name.
        let (agent, mut configuration) = match initial_seat_context.as_ref() {
            Some((seat, _, _)) if seat.agent_id != session.agent_id => {
                let agent = self.require_agent(&seat.agent_id)?;
                let configuration = self.seat_chat_configuration(&session, &agent)?;
                (agent, configuration)
            }
            _ => (
                self.require_agent(&session.agent_id)?,
                self.ports
                    .sessions
                    .validate_configuration(&session, request.configuration)?,
            ),
        };
        if session.read_only {
            configuration.execution_mode = "plan".to_string();
        }
        if !agent.supports(configuration.interaction_mode) {
            return Err(AgentRuntimeApplicationError::UnsupportedInteractionMode(
                configuration.interaction_mode.as_str().to_string(),
            ));
        }
        let (seat_ownership, role_briefing) = initial_seat_context
            .map(|(_, ownership, briefing)| (Some(ownership), Some(briefing)))
            .unwrap_or((None, None));
        let terminal = register_completion
            .then(|| self.ports.message_completions.register(&session.id))
            .transpose()?;
        let result = self.start_message_generation(
            &session,
            &agent,
            MessageGenerationInput {
                interactive: caller_is_interactive
                    && interactive_message_source(&request.source)
                    // A Loop-owned session is driven by the Loop scheduler, not by someone
                    // watching the thread.
                    && session.loop_ownership.is_none(),
                source: request.source,
                configuration,
                content,
                file_references: request.file_references,
                role_briefing,
                seat_ownership,
                record_user_message: true,
                runner,
            },
        );
        if result.is_err() && terminal.is_some() {
            let _ = self.ports.message_completions.remove(&session.id);
        }
        result.map(|message| (message, terminal))
    }

    pub(super) fn start_message_generation(
        &self,
        session: &AgentSession,
        agent: &AgentDefinition,
        input: MessageGenerationInput,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        let MessageGenerationInput {
            source,
            configuration,
            content,
            file_references,
            role_briefing,
            seat_ownership,
            record_user_message,
            interactive,
            runner,
        } = input;
        let originated_from_im =
            matches!(&source, super::AgentMessageSource::InstantMessage { .. });
        let endpoint_profile = self.freeze_endpoint_profile(agent.id().as_str(), &content)?;
        let settings = self.ports.execution_settings.load_settings().map_err(|_| {
            AgentRuntimeApplicationError::Process(
                "execution observability settings are unavailable".to_string(),
            )
        })?;
        let root_context = self.ports.execution_ids.next_context(
            settings.capture_policy,
            settings.sampling_ratio,
            settings.mcp_relay_enabled,
        );
        let started_at = self.ports.clock.now();
        let mut root_attributes = vec![
            (
                "vanehub.stage".to_string(),
                SafeAttributeValue::String("task_execution".to_string()),
            ),
            (
                "vanehub.agent.id".to_string(),
                SafeAttributeValue::String(agent.id().as_str().to_string()),
            ),
        ];
        if let Some(ownership) = &seat_ownership {
            root_attributes.extend([
                (
                    "vanehub.seat.round.id".to_string(),
                    SafeAttributeValue::String(ownership.round_id.clone()),
                ),
                (
                    "vanehub.seat.index".to_string(),
                    SafeAttributeValue::Integer(
                        i64::try_from(ownership.seat_index).unwrap_or(i64::MAX),
                    ),
                ),
            ]);
            if let Some(parent_run_id) = &ownership.parent_execution_run_id {
                root_attributes.push((
                    "vanehub.seat.parent_run.id".to_string(),
                    SafeAttributeValue::String(parent_run_id.clone()),
                ));
            }
        }
        let mut run = ExecutionRun {
            context: root_context.clone(),
            source: execution_source(source),
            status: ExecutionStatus::Running,
            started_at: started_at.clone(),
            ended_at: None,
            error_classification: None,
            session_id: Some(session.id.clone()),
            user_message_id: None,
            assistant_message_id: None,
            operation_id: None,
            agent_id: Some(agent.id().as_str().to_string()),
            provider_session_id: session.runtime_session_id.clone(),
            attributes: safe_attributes(root_attributes.clone()),
            links: Vec::new(),
        };
        let root_span = ExecutionSpan {
            context: root_context.clone(),
            parent_span_id: None,
            name: "vanehub.task.execute".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: started_at.clone(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes(root_attributes),
            links: Vec::new(),
        };
        record_telemetry_result(
            &self.ports,
            &root_context,
            "start_run",
            self.ports.telemetry.start_run(&run),
        );
        record_telemetry_result(
            &self.ports,
            &root_context,
            "start_root_span",
            self.ports.telemetry.start_span(&root_span),
        );
        let prompt_context = child_context(&root_context, self.ports.execution_ids.next_span_id());
        let prompt_span = ExecutionSpan {
            context: prompt_context.clone(),
            parent_span_id: Some(root_context.span_id.clone()),
            name: "vanehub.prompt.assemble".to_string(),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: self.ports.clock.now(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes([(
                "vanehub.stage".to_string(),
                SafeAttributeValue::String("prompt_assembly".to_string()),
            )]),
            links: Vec::new(),
        };
        record_telemetry_result(
            &self.ports,
            &prompt_context,
            "start_prompt_span",
            self.ports.telemetry.start_span(&prompt_span),
        );
        let prompt =
            match self
                .ports
                .sessions
                .compose_prompt(&session.id, &content, &file_references)
            {
                Ok(prompt) => prompt,
                Err(error) => {
                    let ended_at = self.ports.clock.now();
                    record_telemetry_result(
                        &self.ports,
                        &prompt_context,
                        "finish_prompt_span",
                        self.ports.telemetry.finish_span(
                            &prompt_context.run_id,
                            &prompt_context.span_id,
                            ExecutionStatus::Failed,
                            &ended_at,
                            Some("prompt_compose_failed"),
                        ),
                    );
                    self.finish_execution_root(
                        &root_context,
                        ExecutionStatus::Failed,
                        Some("prompt_compose_failed"),
                    );
                    return Err(error);
                }
            };
        record_telemetry_result(
            &self.ports,
            &prompt_context,
            "finish_prompt_span",
            self.ports.telemetry.finish_span(
                &prompt_context.run_id,
                &prompt_context.span_id,
                ExecutionStatus::Succeeded,
                &self.ports.clock.now(),
                None,
            ),
        );
        let generation_file_references = file_references.clone();
        let durable_messages =
            match self
                .ports
                .sessions
                .start_generation(DurableAgentGenerationStart {
                    session_id: session.id.clone(),
                    execution_run_id: root_context.run_id.as_str().to_string(),
                    seat_round_id: seat_ownership
                        .as_ref()
                        .map(|ownership| ownership.round_id.clone()),
                    parent_execution_run_id: seat_ownership
                        .as_ref()
                        .and_then(|ownership| ownership.parent_execution_run_id.clone()),
                    user_message: record_user_message.then_some(NewAgentMessage {
                        session_id: session.id.clone(),
                        speaker_seat_id: None,
                        seat_index: None,
                        role: "user".to_string(),
                        status: "completed".to_string(),
                        content,
                        file_references,
                    }),
                    assistant_message: NewAgentMessage {
                        session_id: session.id.clone(),
                        speaker_seat_id: seat_ownership
                            .as_ref()
                            .map(|ownership| ownership.seat_id.clone()),
                        // Stable seat identity owns new writes; the numeric index remains
                        // read-only compatibility data for messages created before migration 59.
                        seat_index: None,
                        role: "assistant".to_string(),
                        status: "streaming".to_string(),
                        content: String::new(),
                        file_references: Vec::new(),
                    },
                }) {
                Ok(messages) => messages,
                Err(error) => {
                    self.finish_execution_root(
                        &root_context,
                        ExecutionStatus::Failed,
                        Some("generation_start_persistence_failed"),
                    );
                    return Err(error);
                }
            };
        let user_message = durable_messages.user_message;
        let assistant = durable_messages.assistant_message;
        let lease = match self.ports.generations.reserve(&session.id) {
            Ok(lease) => lease,
            Err(error) => {
                let _ = self.ports.sessions.fail_message(
                    &assistant.id,
                    &session.id,
                    "Generation control reservation failed.",
                );
                self.finish_execution_root(
                    &root_context,
                    ExecutionStatus::Failed,
                    Some("generation_control_reservation_failed"),
                );
                return Err(error);
            }
        };
        if let Err(error) = self.ports.generations.correlate(&lease, &root_context) {
            let _ = self.ports.sessions.fail_message(
                &assistant.id,
                &session.id,
                "Generation control correlation failed.",
            );
            let _ = self.ports.generations.release(&lease);
            self.finish_execution_root(
                &root_context,
                ExecutionStatus::Failed,
                Some("generation_correlation_failed"),
            );
            return Err(error);
        }
        let operation = match self.ports.operations.start_agent_generation(
            agent.id().as_str(),
            &session.id,
            &assistant.id,
        ) {
            Ok(operation) => operation,
            Err(error) => {
                return self.fail_prepared_message(
                    &root_context,
                    session,
                    &assistant,
                    &lease,
                    None,
                    generation_failure(
                        format!("{} command failed", agent.display_name()),
                        error.to_string(),
                    ),
                );
            }
        };
        run.user_message_id = user_message.as_ref().map(|message| message.id.clone());
        run.assistant_message_id = Some(assistant.id.clone());
        run.operation_id = Some(operation.id.clone());
        record_telemetry_result(
            &self.ports,
            &run.context,
            "restart_run_correlation",
            self.ports.telemetry.start_run(&run),
        );
        if let Err(error) = self.ports.operations.correlate_execution(
            &operation.id,
            root_context.run_id.as_str(),
            root_context.trace_id.as_str(),
        ) {
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    "Generation recovery evidence could not be persisted.",
                    error.to_string(),
                ),
            );
        }
        let _ = self.ports.events.publish(AgentEvent::MessageStarted {
            session_id: session.id.clone(),
            message_id: assistant.id.clone(),
        });
        if let Err(error) = self
            .ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Running)
        {
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }

        // Prompt Hooks are CLI-only (`ManagedCliAgentId` only recognizes the built-in CLI ids) —
        // mirrors the `cli_profiles` gate below. Calling `assemble` for a non-CLI agent id would
        // fail to parse it and abort the whole send, so it's skipped in favor of the prompt
        // composed above, passed through unchanged.
        let effective_prompt = if agent.launch().kind_str() == "cli" {
            let assembled =
                match self
                    .ports
                    .prompts
                    .assemble(agent.id().as_str(), &session.id, &prompt)
                {
                    Ok(prompt) => prompt,
                    Err(error) => {
                        return self.fail_prepared_message(
                            &root_context,
                            session,
                            &assistant,
                            &lease,
                            Some(&operation.id),
                            generation_failure("Prompt Hook assembly failed", error.to_string()),
                        );
                    }
                };
            // Custom instructions (`add-cli-custom-instructions-injection`) and the shared memory
            // pool (`add-cli-memory-support`) are combined here, after Prompt Hook assembly rather
            // than before it, so hook templates' own `{{sample_input}}` rendering still reflects
            // only the user's original message. A personalization-settings lookup failure
            // degrades to safe defaults (mirroring `resolve_personalization_settings` on the
            // OnePiece side) rather than blocking the message — this codebase's established
            // philosophy of never letting an optional personalization lookup fail delivery.
            let governed = self
                .ports
                .personalization
                .snapshot(GenerationPersonalizationContext {
                    agent_id: agent.id().as_str().to_string(),
                    session_id: session.id.clone(),
                    folder: session.folder.clone(),
                    personalization_mode: session.personalization_mode.clone(),
                });
            if let Some(reason) = governed.memory.blocked_reason.as_deref() {
                self.record_log(
                    AgentLogLevel::Warn,
                    "session.runtime.personalization",
                    format!(
                        "Personalization unavailable ({reason}); continuing without custom instructions or memory."
                    ),
                    Some(agent.id().as_str()),
                    Some(&session.id),
                    None,
                );
            }
            let custom_instructions = governed.instruction_block.clone();
            // The index and nothing more. A CLI has nowhere to put a memory body that is not also
            // the message it is about to answer, so the resolver downgrades delivery for a runtime
            // that has not declared the capability — and this surface never asks for bodies even
            // when it has. That stays true here rather than being re-decided per Agent.
            let memory_section = match governed.memory.delivery {
                AgentMemoryDelivery::None => None,
                AgentMemoryDelivery::IndexOnly | AgentMemoryDelivery::IndexWithSelectedBodies => {
                    let eligible: Vec<AgentMemory> = governed
                        .memory
                        .eligible
                        .iter()
                        .map(memory_from_ref)
                        .collect();
                    format_memory_index(&eligible, CLI_MEMORY_INDEX_BOUNDS)
                }
            };
            let leading_sections: Vec<String> = [custom_instructions, memory_section]
                .into_iter()
                .flatten()
                .collect();
            if leading_sections.is_empty() {
                assembled
            } else {
                EffectivePrompt {
                    content: format!("{}\n\n{}", leading_sections.join("\n\n"), assembled.content),
                    trace: assembled.trace,
                }
            }
        } else {
            EffectivePrompt {
                content: prompt.clone(),
                trace: Vec::new(),
            }
        };
        for trace in &effective_prompt.trace {
            self.record_log(
                AgentLogLevel::Debug,
                "session.runtime.prompt-hook",
                format!(
                    "Prompt Hook {} {} hash={} tokens={} reason={}",
                    trace.hook_id,
                    trace.status,
                    trace.content_hash.as_deref().unwrap_or("none"),
                    trace
                        .token_estimate
                        .map(|value| value.to_string())
                        .unwrap_or_else(|| "none".to_string()),
                    trace.reason.as_deref().unwrap_or("none")
                ),
                Some(agent.id().as_str()),
                Some(&session.id),
                None,
            );
        }
        let prompt_started_at = Instant::now();
        let prompt_versions = effective_prompt
            .trace
            .iter()
            .filter_map(|trace| {
                (trace.status == "fired" || trace.status == "applied")
                    .then_some(trace.version)
                    .flatten()
                    .map(|version| PromptVersionReference {
                        hook_id: trace.hook_id.clone(),
                        version,
                    })
            })
            .collect::<Vec<_>>();
        if let Err(error) = self.ports.generations.correlate_prompt(
            &lease,
            &PendingPromptExecution {
                invocation_id: operation.id.clone(),
                agent_id: agent.id().as_str().to_string(),
                versions: prompt_versions.clone(),
                started_at: prompt_started_at,
            },
        ) {
            self.record_prompt_execution(
                &operation.id,
                agent.id().as_str(),
                &prompt_versions,
                PromptExecutionOutcome::Failed,
                prompt_started_at,
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        let profile = if agent.launch().kind_str() == "cli" {
            match self.ports.cli_profiles.load(
                agent.id().as_str(),
                &configuration,
                Some(&operation.id),
            ) {
                Ok(profile) => profile,
                Err(error) => {
                    self.record_prompt_execution(
                        &operation.id,
                        agent.id().as_str(),
                        &prompt_versions,
                        PromptExecutionOutcome::Failed,
                        prompt_started_at,
                    );
                    return self.fail_prepared_message(
                        &root_context,
                        session,
                        &assistant,
                        &lease,
                        Some(&operation.id),
                        generation_failure(
                            format!("{} command failed", agent.display_name()),
                            error.to_string(),
                        ),
                    );
                }
            }
        } else {
            // API agents launch no CLI process, so there is no profile to resolve.
            CliProfileSnapshot::default()
        };
        let input_count = effective_prompt.content.chars().count();
        let agent_context = child_context(&root_context, self.ports.execution_ids.next_span_id());
        let mut agent_attributes = vec![
            (
                "gen_ai.operation.name".to_string(),
                SafeAttributeValue::String("invoke_agent".to_string()),
            ),
            (
                "vanehub.agent.id".to_string(),
                SafeAttributeValue::String(agent.id().as_str().to_string()),
            ),
        ];
        if let Some(provider_id) = &configuration.provider_id {
            agent_attributes.push((
                "gen_ai.provider.name".to_string(),
                SafeAttributeValue::String(provider_id.clone()),
            ));
        }
        if let Some(model_id) = &configuration.model_id {
            agent_attributes.push((
                "gen_ai.request.model".to_string(),
                SafeAttributeValue::String(model_id.clone()),
            ));
        }
        // The trace stays session-scoped and shows a whole round including the handoffs between
        // seats, so it has to say which seat each Agent span belongs to.
        if let Some(ownership) = &seat_ownership {
            agent_attributes.push((
                "vanehub.seat.id".to_string(),
                SafeAttributeValue::String(ownership.seat_id.clone()),
            ));
            agent_attributes.push((
                "vanehub.seat.index".to_string(),
                SafeAttributeValue::String(ownership.seat_index.to_string()),
            ));
            agent_attributes.push((
                "vanehub.seat.mention".to_string(),
                SafeAttributeValue::String(ownership.seat_mention.clone()),
            ));
        }
        let agent_span = ExecutionSpan {
            context: agent_context.clone(),
            parent_span_id: Some(root_context.span_id.clone()),
            name: format!("invoke_agent {}", agent.id().as_str()),
            status: ExecutionStatus::Running,
            fidelity: ExecutionFidelity::Native,
            started_at: self.ports.clock.now(),
            ended_at: None,
            error_classification: None,
            attributes: safe_attributes(agent_attributes),
            links: Vec::new(),
        };
        record_telemetry_result(
            &self.ports,
            &agent_context,
            "start_agent_span",
            self.ports.telemetry.start_span(&agent_span),
        );
        // A seat resumes only its own Agent's thread. Without seat ownership this is a plain
        // single-Agent turn, whose thread is the session's and must keep resuming exactly as it
        // did before seats carried threads of their own.
        let resume_thread_id = match seat_ownership.as_ref() {
            Some(ownership) => super::resume_thread_for(
                &session.seats,
                &ownership.seat_id,
                session.runtime_session_id.as_deref(),
            )
            .map(str::to_string),
            None => session.runtime_session_id.clone(),
        };
        let started = match self
            .ports
            .processes
            .start_generation(GenerationProcessRequest {
                execution_context: agent_context.clone(),
                session: session.clone(),
                agent: AgentView::from(agent),
                message_id: assistant.id.clone(),
                operation_id: operation.id.clone(),
                configuration: configuration.clone(),
                effective_prompt: effective_prompt.content,
                file_references: generation_file_references,
                automatic_compaction: AutomaticCompactionMode::Automatic,
                // Single-Agent sessions carry no briefing, so their invocation is unchanged.
                role_briefing: role_briefing.clone(),
                cli_profile: profile,
                interactive,
                runner,
                endpoint_profile,
                resume_thread_id: resume_thread_id.clone(),
            }) {
            Ok(started) => started,
            Err(error) => {
                self.record_prompt_execution(
                    &operation.id,
                    agent.id().as_str(),
                    &prompt_versions,
                    PromptExecutionOutcome::Failed,
                    prompt_started_at,
                );
                record_telemetry_result(
                    &self.ports,
                    &agent_context,
                    "finish_agent_span",
                    self.ports.telemetry.finish_span(
                        &agent_context.run_id,
                        &agent_context.span_id,
                        ExecutionStatus::Failed,
                        &self.ports.clock.now(),
                        Some("process_start_failed"),
                    ),
                );
                return self.fail_prepared_message(
                    &root_context,
                    session,
                    &assistant,
                    &lease,
                    Some(&operation.id),
                    generation_failure(
                        format!("{} command failed", agent.display_name()),
                        error.to_string(),
                    ),
                );
            }
        };
        if let Err(error) = self.ports.operations.start_canonical_run(
            root_context.run_id.as_str(),
            &assistant.id,
            seat_ownership
                .as_ref()
                .and_then(|ownership| ownership.parent_execution_run_id.as_deref()),
            CanonicalRunLinks {
                session_id: &session.id,
                user_message_id: run.user_message_id.as_deref(),
                assistant_message_id: &assistant.id,
                operation_id: &operation.id,
            },
            &started.runner_reference,
            started.process_reference.as_deref(),
        ) {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    "Agent run initialization failed".to_string(),
                    error.to_string(),
                ),
            );
        }
        if let Err(error) = self
            .ports
            .operations
            .signal_canonical_run(root_context.run_id.as_str(), CanonicalRunSignal::Active)
        {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    "Agent run start could not be persisted.".to_string(),
                    error.to_string(),
                ),
            );
        }
        if let Err(error) =
            self.ports
                .generations
                .attach(&lease, &assistant.id, &started.process_id, &operation.id)
        {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            record_telemetry_result(
                &self.ports,
                &agent_context,
                "finish_agent_span",
                self.ports.telemetry.finish_span(
                    &agent_context.run_id,
                    &agent_context.span_id,
                    ExecutionStatus::Failed,
                    &self.ports.clock.now(),
                    Some("generation_attach_failed"),
                ),
            );
            self.record_prompt_execution(
                &operation.id,
                agent.id().as_str(),
                &prompt_versions,
                PromptExecutionOutcome::Failed,
                prompt_started_at,
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        let sink: Arc<dyn AgentProcessEventSink> = Arc::new(GenerationEventHandler::new(
            self.ports.clone(),
            GenerationEventHandlerInput {
                session_id: session.id.clone(),
                agent_id: agent.id().as_str().to_string(),
                message_id: assistant.id.clone(),
                operation_id: operation.id.clone(),
                safe_error: format!("{} command failed", agent.display_name()),
                configuration,
                input_count,
                root_context: root_context.clone(),
                agent_context: agent_context.clone(),
                loop_ownership: session.loop_ownership.clone(),
                personalization_mode: session.personalization_mode.clone(),
                seat_ownership,
                prompt_versions: prompt_versions.clone(),
                prompt_started_at,
                is_cli_kind: agent.launch().kind_str() == "cli",
                folder: session.folder.clone(),
                user_prompt: prompt.clone(),
                originated_from_im,
                resumed_thread_id: resume_thread_id,
            },
        ));
        if let Err(error) = self
            .ports
            .processes
            .monitor_generation(&started.process_id, sink)
        {
            let _ = self.ports.processes.stop_generation(
                &started.process_id,
                super::ProcessStopInitiator::RuntimeCleanup,
            );
            record_telemetry_result(
                &self.ports,
                &agent_context,
                "finish_agent_span",
                self.ports.telemetry.finish_span(
                    &agent_context.run_id,
                    &agent_context.span_id,
                    ExecutionStatus::Failed,
                    &self.ports.clock.now(),
                    Some("generation_monitor_failed"),
                ),
            );
            self.record_prompt_execution(
                &operation.id,
                agent.id().as_str(),
                &prompt_versions,
                PromptExecutionOutcome::Failed,
                prompt_started_at,
            );
            return self.fail_prepared_message(
                &root_context,
                session,
                &assistant,
                &lease,
                Some(&operation.id),
                generation_failure(
                    format!("{} command failed", agent.display_name()),
                    error.to_string(),
                ),
            );
        }
        Ok(assistant)
    }

    fn fail_prepared_message(
        &self,
        execution_context: &ExecutionContext,
        session: &AgentSession,
        assistant: &AgentMessage,
        lease: &GenerationLease,
        operation_id: Option<&str>,
        failure: GenerationFailure,
    ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.finish_execution_root(
            execution_context,
            ExecutionStatus::Failed,
            Some("agent_generation_failed"),
        );
        self.record_log(
            AgentLogLevel::Error,
            "session.runtime",
            failure.diagnostic,
            Some(&session.agent_id),
            Some(&session.id),
            operation_id,
        );
        let failed =
            self.ports
                .sessions
                .fail_message(&assistant.id, &session.id, &failure.safe_error)?;
        let _ = self
            .ports
            .message_completions
            .deliver(AgentMessageTerminal {
                session_id: session.id.clone(),
                message_id: assistant.id.clone(),
                outcome: AgentMessageTerminalOutcome::Failed,
                content: None,
            });
        self.ports
            .sessions
            .update_lifecycle(&session.id, AgentLifecycle::Failed)?;
        let _ = self.ports.generations.release(lease);
        if let Some(operation_id) = operation_id {
            let _ = self
                .ports
                .operations
                .fail(operation_id, failure.safe_error.clone());
        }
        let _ = self.ports.events.publish(AgentEvent::MessageFailed {
            session_id: session.id.clone(),
            message_id: assistant.id.clone(),
            error: failure.safe_error,
        });
        self.deliver_loop_terminal(
            session,
            &assistant.id,
            LoopRoleGenerationOutcome::Failed,
            None,
            Some(format!("{} command failed", session.agent_id)),
        )?;
        Ok(failed)
    }

    fn finish_execution_root(
        &self,
        context: &ExecutionContext,
        status: ExecutionStatus,
        error_classification: Option<&str>,
    ) {
        let outcome = match status {
            ExecutionStatus::Succeeded => CanonicalRunOutcome::Completed,
            ExecutionStatus::Cancelled => CanonicalRunOutcome::Cancelled,
            _ => CanonicalRunOutcome::Failed,
        };
        let _ = self.ports.operations.finish_canonical_run(
            context.run_id.as_str(),
            outcome,
            error_classification,
        );
        let ended_at = self.ports.clock.now();
        record_telemetry_result(
            &self.ports,
            context,
            "finish_root_span",
            self.ports.telemetry.finish_span(
                &context.run_id,
                &context.span_id,
                status,
                &ended_at,
                error_classification,
            ),
        );
        record_telemetry_result(
            &self.ports,
            context,
            "finish_run",
            self.ports.telemetry.finish_run(
                &context.run_id,
                status,
                &ended_at,
                error_classification,
            ),
        );
    }

    pub(crate) fn stop_generation(
        &self,
        session_id: &str,
    ) -> Result<StopGenerationResult, AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        let cancellation = self.ports.generations.cancel(session_id)?;
        let streaming_ids = self.ports.sessions.cancel_streaming_messages(session_id)?;
        let mut message_ids = BTreeSet::new();
        if let Some(message_id) = cancellation
            .as_ref()
            .and_then(|outcome| outcome.message_id.clone())
        {
            message_ids.insert(message_id);
        }
        message_ids.extend(streaming_ids);
        for message_id in &message_ids {
            let _ = self
                .ports
                .message_completions
                .deliver(AgentMessageTerminal {
                    session_id: session_id.to_string(),
                    message_id: message_id.clone(),
                    outcome: AgentMessageTerminalOutcome::Cancelled,
                    content: None,
                });
        }
        let has_process = cancellation
            .as_ref()
            .and_then(|outcome| outcome.process_id.as_deref())
            .is_some();
        if message_ids.is_empty() && !has_process {
            return Ok(StopGenerationResult {
                cancelled_message_ids: Vec::new(),
                process_stopped: false,
            });
        }
        let operation_id = cancellation
            .as_ref()
            .and_then(|outcome| outcome.operation_id.as_deref());
        self.ports
            .sessions
            .update_lifecycle(session_id, AgentLifecycle::Stopped)?;
        if let Some(operation_id) = operation_id {
            let _ = self.ports.operations.cancel(operation_id);
        }
        self.record_log(
            AgentLogLevel::Warn,
            "session.runtime",
            "session generation cancelled".to_string(),
            Some(&session.agent_id),
            Some(session_id),
            operation_id,
        );
        let process_stopped = match cancellation
            .as_ref()
            .and_then(|outcome| outcome.process_id.as_deref())
        {
            Some(process_id) => self
                .ports
                .processes
                .stop_generation(process_id, super::ProcessStopInitiator::User)?,
            None => false,
        };
        if let Some(execution_context) = cancellation
            .as_ref()
            .and_then(|outcome| outcome.execution_context.as_ref())
        {
            self.finish_execution_root(
                execution_context,
                ExecutionStatus::Cancelled,
                Some("user_cancelled"),
            );
        }
        if let Some(prompt_execution) = cancellation
            .as_ref()
            .and_then(|outcome| outcome.prompt_execution.as_ref())
        {
            self.record_prompt_execution(
                &prompt_execution.invocation_id,
                &prompt_execution.agent_id,
                &prompt_execution.versions,
                PromptExecutionOutcome::Cancelled,
                prompt_execution.started_at,
            );
        }
        for message_id in &message_ids {
            let _ = self.ports.events.publish(AgentEvent::MessageCancelled {
                session_id: session_id.to_string(),
                message_id: message_id.clone(),
            });
            self.deliver_loop_terminal(
                &session,
                message_id,
                LoopRoleGenerationOutcome::Cancelled,
                None,
                None,
            )?;
        }
        Ok(StopGenerationResult {
            cancelled_message_ids: message_ids.into_iter().collect(),
            process_stopped,
        })
    }

    /// Returns a stuck session to a state that accepts messages.
    ///
    /// Deliberately does not launch anything: CLI generations start per message, so an idle
    /// session answers the user's next message on its own. Spawning a process here would turn an
    /// error the user is reporting into work they did not ask for.
    pub(crate) fn recover_session(
        &self,
        session_id: &str,
    ) -> Result<RecoverSessionResult, AgentRuntimeApplicationError> {
        let session = self.require_session(session_id)?;
        if session.archived {
            return Err(AgentRuntimeApplicationError::Validation(
                "Archived sessions cannot be recovered.".to_string(),
            ));
        }
        // `stop_generation` returns early without touching the lifecycle when there is nothing to
        // cancel, which is exactly the case a `failed` session is usually in, so the reset below
        // is unconditional rather than a fallback.
        let stopped = self.stop_generation(session_id)?;
        self.ports
            .sessions
            .update_lifecycle(session_id, AgentLifecycle::Idle)?;
        self.record_log(
            AgentLogLevel::Info,
            "session.runtime",
            format!(
                "session runtime recovered (cancelled {} message(s), process stopped: {})",
                stopped.cancelled_message_ids.len(),
                stopped.process_stopped
            ),
            Some(&session.agent_id),
            Some(session_id),
            None,
        );
        Ok(RecoverSessionResult {
            cancelled_message_ids: stopped.cancelled_message_ids,
            process_stopped: stopped.process_stopped,
            lifecycle: AgentLifecycle::Idle,
        })
    }

    pub(crate) fn shutdown_generations(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        let session_ids = self.ports.generations.begin_shutdown()?;
        let mut stopped = Vec::new();
        for session_id in session_ids {
            self.stop_generation(&session_id)?;
            stopped.push(session_id);
        }
        Ok(stopped)
    }

    fn deliver_loop_terminal(
        &self,
        session: &AgentSession,
        message_id: &str,
        outcome: LoopRoleGenerationOutcome,
        content: Option<String>,
        error: Option<String>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(ownership) = &session.loop_ownership else {
            return Ok(());
        };
        self.ports
            .loop_completions
            .deliver(LoopRoleGenerationTerminal {
                run_id: ownership.run_id.clone(),
                iteration_id: ownership.iteration_id.clone(),
                role: ownership.role.clone(),
                session_id: session.id.clone(),
                message_id: message_id.to_string(),
                outcome,
                content,
                error,
            })?;
        Ok(())
    }

    pub(super) fn require_agent(
        &self,
        agent_id: &str,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        self.ports
            .registry
            .find(agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))
    }

    pub(super) fn require_session(
        &self,
        session_id: &str,
    ) -> Result<AgentSession, AgentRuntimeApplicationError> {
        self.ports
            .sessions
            .find_session(session_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::SessionNotFound(session_id.to_string()))
    }

    fn record_log(
        &self,
        level: AgentLogLevel,
        category: &str,
        message: String,
        agent_id: Option<&str>,
        session_id: Option<&str>,
        operation_id: Option<&str>,
    ) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: category.to_string(),
            message,
            agent_id: agent_id.map(str::to_string),
            session_id: session_id.map(str::to_string),
            operation_id: operation_id.map(str::to_string),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: self.ports.clock.now(),
        });
    }

    fn record_prompt_execution(
        &self,
        invocation_id: &str,
        agent_id: &str,
        versions: &[PromptVersionReference],
        outcome: PromptExecutionOutcome,
        started_at: Instant,
    ) {
        if versions.is_empty() {
            return;
        }
        let elapsed_ms = i64::try_from(started_at.elapsed().as_millis()).unwrap_or(i64::MAX);
        let _ = self.ports.prompts.record_execution(PromptExecutionReport {
            invocation_id: invocation_id.to_string(),
            agent_id: agent_id.to_string(),
            versions: versions.to_vec(),
            outcome,
            elapsed_ms,
            created_at: self.ports.clock.now(),
        });
    }
}

impl LoopWorkerGenerationPort for AgentRuntimeApplicationService {
    fn start_worker_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.start_loop_role_generation(session_id, prompt)
    }
}

impl LoopVerifierGenerationPort for AgentRuntimeApplicationService {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        prompt: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        self.start_loop_role_generation(session_id, prompt)
    }
}

impl LoopGenerationControlPort for AgentRuntimeApplicationService {
    fn stop_loop_generation(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.stop_generation(session_id).map(|_| ())
    }
}

struct GenerationEventHandler {
    ports: AgentRuntimeApplicationPorts,
    session_id: String,
    agent_id: String,
    message_id: String,
    operation_id: String,
    safe_error: String,
    configuration: AgentChatConfiguration,
    input_count: usize,
    root_context: ExecutionContext,
    agent_context: ExecutionContext,
    loop_ownership: Option<super::LoopRoleGenerationOwnership>,
    seat_ownership: Option<super::SeatTurnOwnership>,
    prompt_versions: Vec<PromptVersionReference>,
    prompt_started_at: Instant,
    /// `add-cli-memory-support` — gates the post-completion memory-extraction attempt to
    /// CLI-wrapped agents only (`agent.launch().kind_str() == "cli"` at construction time),
    /// mirroring the same gate the CLI send path already uses for injection.
    is_cli_kind: bool,
    folder: Option<String>,
    /// Carried so the completed turn resolves under the same mode the message went out under. A
    /// second read of the session could see a different one if the user switched it mid-turn.
    personalization_mode: String,
    user_prompt: String,
    originated_from_im: bool,
    resumed_thread_id: Option<String>,
    state: Mutex<GenerationStreamState>,
}

struct GenerationEventHandlerInput {
    session_id: String,
    agent_id: String,
    message_id: String,
    operation_id: String,
    safe_error: String,
    configuration: AgentChatConfiguration,
    input_count: usize,
    root_context: ExecutionContext,
    agent_context: ExecutionContext,
    loop_ownership: Option<super::LoopRoleGenerationOwnership>,
    seat_ownership: Option<super::SeatTurnOwnership>,
    prompt_versions: Vec<PromptVersionReference>,
    prompt_started_at: Instant,
    is_cli_kind: bool,
    folder: Option<String>,
    personalization_mode: String,
    user_prompt: String,
    originated_from_im: bool,
    /// The provider thread this turn asked to resume, if any. Kept so a turn that fails without
    /// producing a word can forget an id that no longer refers to anything.
    resumed_thread_id: Option<String>,
}

// Streaming deltas are persisted for crash/live-reload durability only — the terminal
// path rewrites the full message content anyway. Persisting every token meant an
// O(N²) load-full-row + rewrite-full-content per token; instead we coalesce deltas and
// flush at most this often (bounding the flush count by wall-clock, not token count) or
// once the un-persisted buffer grows past the byte cap.
const STREAM_FLUSH_INTERVAL: Duration = Duration::from_millis(250);
const STREAM_FLUSH_MAX_PENDING_BYTES: usize = 8 * 1024;

struct GenerationStreamState {
    response: String,
    phase: GenerationStreamPhase,
    active_tool_spans: BTreeMap<String, crate::contexts::execution_observability::api::SpanId>,
    terminal_tool_calls: BTreeSet<String>,
    pending_content: String,
    pending_thinking: String,
    last_flush: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum GenerationStreamPhase {
    #[default]
    Active,
    ApplyingTerminal,
    Terminal,
}

impl Default for GenerationStreamState {
    fn default() -> Self {
        Self {
            response: String::new(),
            phase: GenerationStreamPhase::Active,
            active_tool_spans: BTreeMap::new(),
            terminal_tool_calls: BTreeSet::new(),
            pending_content: String::new(),
            pending_thinking: String::new(),
            last_flush: Instant::now(),
        }
    }
}

impl GenerationStreamState {
    fn should_flush(&self) -> bool {
        self.last_flush.elapsed() >= STREAM_FLUSH_INTERVAL
            || self.pending_content.len() >= STREAM_FLUSH_MAX_PENDING_BYTES
            || self.pending_thinking.len() >= STREAM_FLUSH_MAX_PENDING_BYTES
    }

    fn take_pending_content(&mut self) -> String {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending_content)
    }

    fn take_pending_thinking(&mut self) -> String {
        self.last_flush = Instant::now();
        std::mem::take(&mut self.pending_thinking)
    }
}

impl GenerationEventHandler {
    fn new(ports: AgentRuntimeApplicationPorts, input: GenerationEventHandlerInput) -> Self {
        Self {
            ports,
            session_id: input.session_id,
            agent_id: input.agent_id,
            message_id: input.message_id,
            operation_id: input.operation_id,
            safe_error: input.safe_error,
            configuration: input.configuration,
            input_count: input.input_count,
            root_context: input.root_context,
            agent_context: input.agent_context,
            loop_ownership: input.loop_ownership,
            seat_ownership: input.seat_ownership,
            prompt_versions: input.prompt_versions,
            prompt_started_at: input.prompt_started_at,
            is_cli_kind: input.is_cli_kind,
            folder: input.folder,
            personalization_mode: input.personalization_mode,
            user_prompt: input.user_prompt,
            originated_from_im: input.originated_from_im,
            resumed_thread_id: input.resumed_thread_id,
            state: Mutex::new(GenerationStreamState::default()),
        }
    }

    fn token(&self, delta: String) -> Result<(), AgentRuntimeApplicationError> {
        let (content_delta, flushed) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            // Provider adapters own text boundaries. API streams emit exact token deltas,
            // while line-oriented CLI adapters restore the line break they consumed.
            let content_delta = delta;
            state.response.push_str(&content_delta);
            state.pending_content.push_str(&content_delta);
            let flushed = state.should_flush().then(|| state.take_pending_content());
            (content_delta, flushed)
        };
        // The frontend accumulates from the per-token event, so live rendering is
        // unaffected by how often we persist; the DB write is coalesced.
        if let Some(pending) = flushed {
            self.ports
                .sessions
                .append_content(&self.message_id, &pending)?;
        }
        let _ = self.ports.events.publish(AgentEvent::MessageToken {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            content_delta,
        });
        Ok(())
    }

    fn thinking(&self, content_delta: String) -> Result<(), AgentRuntimeApplicationError> {
        let flushed = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            state.pending_thinking.push_str(&content_delta);
            state.should_flush().then(|| state.take_pending_thinking())
        };
        if let Some(pending) = flushed {
            self.ports
                .sessions
                .append_thinking(&self.message_id, &pending)?;
        }
        let _ = self.ports.events.publish(AgentEvent::MessageThinking {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            content_delta,
        });
        Ok(())
    }

    fn tool_use(&self, tool_use: super::ToolUseBlock) -> Result<(), AgentRuntimeApplicationError> {
        let phase = if tool_use.status == "awaiting_approval" {
            ToolLifecyclePhase::AwaitingApproval
        } else if tool_use.status == "awaiting_input" {
            ToolLifecyclePhase::AwaitingInput
        } else {
            match tool_terminal_status(&tool_use.status) {
                Some(ExecutionStatus::Succeeded) => ToolLifecyclePhase::Completed,
                Some(_) => ToolLifecyclePhase::Failed,
                None => ToolLifecyclePhase::Started,
            }
        };
        self.tool_lifecycle(ToolLifecycleEvent {
            call_id: tool_use.id.clone(),
            phase,
            provider_timestamp: None,
            fidelity: ExecutionFidelity::Inferred,
            parent_run_id: None,
            parent_trace_id: None,
            parent_span_id: None,
            delegation_id: None,
            attempt: None,
            tool_use,
        })
    }

    fn tool_lifecycle(
        &self,
        event: ToolLifecycleEvent,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let signal = match event.phase {
            ToolLifecyclePhase::AwaitingApproval => Some(CanonicalRunSignal::WaitingApproval),
            ToolLifecyclePhase::AwaitingInput => Some(CanonicalRunSignal::WaitingUser),
            ToolLifecyclePhase::Started | ToolLifecyclePhase::Updated => {
                Some(CanonicalRunSignal::Active)
            }
            ToolLifecyclePhase::Completed
            | ToolLifecyclePhase::Failed
            | ToolLifecyclePhase::Cancelled => None,
        };
        if let Some(signal) = signal {
            self.ports
                .operations
                .signal_canonical_run(self.root_context.run_id.as_str(), signal)?;
        }
        let is_terminal = matches!(
            event.phase,
            ToolLifecyclePhase::Completed
                | ToolLifecyclePhase::Failed
                | ToolLifecyclePhase::Cancelled
        );
        let (span_id, is_new) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(());
            }
            if state.terminal_tool_calls.contains(&event.call_id) {
                return Ok(());
            }
            match state.active_tool_spans.get(&event.call_id) {
                Some(span_id) => (span_id.clone(), false),
                None => {
                    let span_id = self.ports.execution_ids.next_span_id();
                    state
                        .active_tool_spans
                        .insert(event.call_id.clone(), span_id.clone());
                    (span_id, true)
                }
            }
        };
        if is_new {
            let context = child_context(&self.agent_context, span_id.clone());
            let fidelity = if is_terminal {
                ExecutionFidelity::Opaque
            } else {
                event.fidelity
            };
            let mut attributes = vec![
                (
                    "gen_ai.tool.name".to_string(),
                    SafeAttributeValue::String(event.tool_use.name.clone()),
                ),
                (
                    "vanehub.tool.duration_known".to_string(),
                    SafeAttributeValue::Boolean(!is_terminal),
                ),
            ];
            if let Some(delegation_id) = &event.delegation_id {
                attributes.push((
                    "vanehub.delegation.id".to_string(),
                    SafeAttributeValue::String(delegation_id.clone()),
                ));
            }
            if let Some(attempt) = event.attempt {
                attributes.push((
                    "vanehub.execution.attempt".to_string(),
                    SafeAttributeValue::Integer(i64::from(attempt)),
                ));
            }
            let links = provider_parent_link(&event);
            let span = ExecutionSpan {
                context: context.clone(),
                parent_span_id: Some(self.agent_context.span_id.clone()),
                name: format!("execute_tool {}", event.tool_use.name),
                status: ExecutionStatus::Running,
                fidelity,
                started_at: event
                    .provider_timestamp
                    .clone()
                    .unwrap_or_else(|| self.ports.clock.now()),
                ended_at: None,
                error_classification: None,
                attributes: safe_attributes(attributes),
                links,
            };
            record_telemetry_result(
                &self.ports,
                &context,
                "start_tool_span",
                self.ports.telemetry.start_span(&span),
            );
        }
        if !is_new && event.phase == ToolLifecyclePhase::Started {
            return Ok(());
        }
        let terminal_status = match event.phase {
            ToolLifecyclePhase::Completed => Some(ExecutionStatus::Succeeded),
            ToolLifecyclePhase::Failed => Some(ExecutionStatus::Failed),
            ToolLifecyclePhase::Cancelled => Some(ExecutionStatus::Cancelled),
            ToolLifecyclePhase::Started
            | ToolLifecyclePhase::Updated
            | ToolLifecyclePhase::AwaitingApproval
            | ToolLifecyclePhase::AwaitingInput => None,
        };
        if let Some(status) = terminal_status {
            let error_classification =
                (status == ExecutionStatus::Failed).then_some("provider_tool_failed");
            let ended_at = event
                .provider_timestamp
                .clone()
                .unwrap_or_else(|| self.ports.clock.now());
            let context = child_context(&self.agent_context, span_id.clone());
            record_telemetry_result(
                &self.ports,
                &context,
                "finish_tool_span",
                self.ports.telemetry.finish_span(
                    &self.agent_context.run_id,
                    &span_id,
                    status,
                    &ended_at,
                    error_classification,
                ),
            );
            let mut state = self.state()?;
            state.active_tool_spans.remove(&event.call_id);
            state.terminal_tool_calls.insert(event.call_id.clone());
        }
        self.ports
            .sessions
            .append_tool_use(&self.message_id, event.tool_use.clone())?;
        let _ = self.ports.events.publish(AgentEvent::MessageToolUse {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            tool_use: Box::new(event.tool_use),
        });
        Ok(())
    }

    fn rich_block(&self, block: serde_json::Value) -> Result<(), AgentRuntimeApplicationError> {
        if self.state()?.phase != GenerationStreamPhase::Active {
            return Ok(());
        }
        self.ports
            .sessions
            .append_rich_block(&self.message_id, block.clone())?;
        let _ = self.ports.events.publish(AgentEvent::MessageRichBlock {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            block,
        });
        Ok(())
    }

    fn completed(
        &self,
        usage: Option<ReportedUsageTotals>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(response) = self.begin_terminal()? else {
            return Ok(());
        };
        let reply = response.clone();
        let result = self.complete_claimed(response, usage);
        if result.is_ok() {
            // Delivered only after the reply is persisted: the coordinator reads the thread to build
            // the next seat's context, and would otherwise race a message that is not there yet.
            self.deliver_seat_turn(Some(reply));
        }
        if result.is_err() {
            self.record_prompt_execution(PromptExecutionOutcome::Failed);
            self.finish_execution(
                ExecutionStatus::Failed,
                Some("completion_persistence_failed"),
            );
        }
        self.finish_terminal(result.is_ok())?;
        result
    }

    fn complete_claimed(
        &self,
        response: String,
        reported_usage: Option<ReportedUsageTotals>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let current = self.current_message()?;
        if current.status == "cancelled" {
            self.mark_cancelled();
            return Ok(());
        }
        let token_usage = MessageTokenUsage {
            input: bounded_count(self.input_count),
            output: bounded_count(response.chars().count()),
        };
        // Reported usage from the CLI's own completion line takes precedence over the
        // character-count estimate; an all-zero/degenerate payload is normalized to
        // `None` upstream in `output.rs`, so any `Some(...)` reaching here is genuine.
        // See `add-reported-usage-ingestion` design.md Decisions 2 and 4.
        let occurred_at = self.ports.clock.now();
        let source_identity = reported_usage
            .as_ref()
            .and_then(|reported| reported.source_identity.clone());
        let source_revision = reported_usage
            .as_ref()
            .and_then(|reported| reported.source_revision.clone());
        let usage = match reported_usage {
            Some(reported) => AgentUsageRecord {
                message_id: self.message_id.clone(),
                session_id: self.session_id.clone(),
                agent_id: self.agent_id.clone(),
                provider_id: self.configuration.provider_id.clone(),
                model_id: reported
                    .model_id
                    .clone()
                    .or_else(|| self.configuration.model_id.clone()),
                accounting_kind: AgentUsageAccountingKind::Reported,
                input_count: reported.input_tokens,
                output_count: reported.output_tokens,
                cache_read_count: reported.cache_read_tokens,
                cache_creation_count: reported.cache_creation_tokens,
                reasoning_output_count: reported.reasoning_output_tokens,
                provider_total_count: reported.provider_total_tokens,
                cache_overlap: reported.cache_overlap,
                reasoning_overlap: reported.reasoning_overlap,
                normalization_version: reported.normalization_version.to_string(),
                source: "cli-reported".to_string(),
                occurred_at: occurred_at.clone(),
            },
            None => AgentUsageRecord {
                message_id: self.message_id.clone(),
                session_id: self.session_id.clone(),
                agent_id: self.agent_id.clone(),
                provider_id: self.configuration.provider_id.clone(),
                model_id: self.configuration.model_id.clone(),
                accounting_kind: AgentUsageAccountingKind::Estimated,
                input_count: token_usage.input,
                output_count: token_usage.output,
                cache_read_count: 0,
                cache_creation_count: 0,
                reasoning_output_count: 0,
                provider_total_count: None,
                cache_overlap: AgentUsageOverlap::Subset,
                reasoning_overlap: AgentUsageOverlap::Subset,
                normalization_version: "character-count-v1".to_string(),
                source: "character-count".to_string(),
                occurred_at: occurred_at.clone(),
            },
        };
        let invocation_id = source_identity.as_ref().map_or_else(
            || format!("managed-cli:{}:invocation", self.message_id),
            |identity| format!("managed-cli:{}:step:{identity}:invocation", self.message_id),
        );
        let observation_id = match (source_identity.as_deref(), source_revision.as_deref()) {
            (Some(identity), Some(revision)) => format!(
                "managed-cli:{}:step:{identity}:revision:{revision}:observation",
                self.message_id
            ),
            (Some(identity), None) => format!(
                "managed-cli:{}:step:{identity}:observation",
                self.message_id
            ),
            (None, _) => format!("managed-cli:{}:observation", self.message_id),
        };
        let invocation_usage = AgentInvocationUsage {
            observation_id,
            invocation_id,
            generation_id: self.message_id.clone(),
            run_id: self.root_context.run_id.as_str().to_string(),
            operation_id: self.operation_id.clone(),
            source_identity,
            source_revision,
            usage,
        };
        self.ports.sessions.complete_message(CompleteAgentMessage {
            message_id: self.message_id.clone(),
            session_id: self.session_id.clone(),
            content: response.clone(),
            thinking_content: current.thinking_content,
            tool_use: current.tool_use,
            rich_blocks: current.rich_blocks,
            token_usage: Some(token_usage.clone()),
            usage: None,
            invocation_usage: (self.configuration.interaction_mode != InteractionMode::Api)
                .then_some(invocation_usage),
        })?;
        if self.seat_ownership.is_none() {
            let _ = self
                .ports
                .message_completions
                .deliver(AgentMessageTerminal {
                    session_id: self.session_id.clone(),
                    message_id: self.message_id.clone(),
                    outcome: AgentMessageTerminalOutcome::Completed,
                    content: Some(response.clone()),
                });
        }
        self.ports
            .sessions
            .update_lifecycle(&self.session_id, AgentLifecycle::Idle)?;
        self.ports.generations.complete(&self.session_id)?;
        let _ = self
            .ports
            .operations
            .append_log(&self.operation_id, "generation completed".to_string());
        let _ = self.ports.operations.complete(&self.operation_id);
        self.finish_execution(ExecutionStatus::Succeeded, None);
        self.record_log(AgentLogLevel::Info, "generation completed".to_string());
        let _ = self.ports.events.publish(AgentEvent::MessageCompleted {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            token_usage: Some(token_usage),
            originated_from_im: self.originated_from_im,
        });
        self.deliver_loop_terminal(
            LoopRoleGenerationOutcome::Completed,
            Some(response.clone()),
            None,
        )?;
        self.record_prompt_execution(PromptExecutionOutcome::Succeeded);
        // `add-cli-memory-support`: runs after everything above has already committed and
        // delivered the completed message — a slow or failing extraction call only extends this
        // background monitoring thread's own lifetime, never the user-visible completion, which
        // was already published by `message_completions.deliver`/`events.publish` earlier in this
        // function (design.md D3/task 6.3).
        if self.is_cli_kind {
            self.propose_memories_from_turn(&response);
        }
        Ok(())
    }

    /// Best-effort extraction for a CLI-wrapped Agent's just-completed turn, as proposals.
    ///
    /// It cannot change an active memory. Every failure mode — an unresolvable policy, a missing
    /// OnePiece credential, the extraction call itself, a review queue that will not take the
    /// batch — logs and returns. The CLI response this hangs off has already succeeded, and none
    /// of this may retract it.
    ///
    /// One snapshot, and two of its answers matter. `automatic_extraction` is the resolved policy
    /// of the Agent that actually ran — not OnePiece, whose model merely performs the extraction —
    /// and `candidate_creation` is separately false in a temporary session, which forbids
    /// proposing one even where the extractor would have been allowed to run.
    fn propose_memories_from_turn(&self, response: &str) {
        let governed = self
            .ports
            .personalization
            .snapshot(GenerationPersonalizationContext {
                agent_id: self.agent_id.clone(),
                session_id: self.session_id.clone(),
                folder: self.folder.clone(),
                personalization_mode: self.personalization_mode.clone(),
            });
        if !governed.memory.automatic_extraction || !governed.memory.candidate_creation {
            return;
        }
        let exchange = format!("User: {}\n\nAssistant: {response}", self.user_prompt);
        // The manifest lets the extraction name an existing memory to correct or retract, and is
        // built from what this session may actually see. It is part of a prompt: a manifest listing
        // everything would show a session the names and descriptions of memories its own policy
        // excluded, and would invite proposals against them.
        let existing = render_existing_manifest(&governed.memory.eligible);
        let parsed = match self.ports.memory_extraction.extract(&exchange, &existing) {
            Ok(parsed) => parsed,
            Err(AgentRuntimeApplicationError::Credential(message)) => {
                // Expected, common condition (OnePiece isn't configured) — worded differently from
                // a genuine call failure below, because an operator reading the log needs to tell
                // "nothing to extract with" from "extraction broke".
                self.record_memory_extraction_log(format!(
                    "Skipping CLI memory extraction; OnePiece has no usable credential: {message}"
                ));
                return;
            }
            Err(error) => {
                self.record_memory_extraction_log(format!(
                    "CLI memory extraction call failed; continuing without it: {error}"
                ));
                return;
            }
        };
        let proposals = proposals_from_actions(&parsed, &governed.memory.eligible);
        if proposals.is_empty() {
            return;
        }
        let outcome = self
            .ports
            .personalization
            .propose_memories(AgentCandidateSubmission {
                proposals,
                origin: AgentProposalOrigin::AutomaticExtraction,
                // The Agent that actually ran, taken from the session rather than from a built-in
                // list, so a dynamically registered CLI Agent is attributed to itself.
                agent_id: self.agent_id.clone(),
                session_id: self.session_id.clone(),
                folder: self.folder.clone(),
                source_message_id: Some(self.message_id.clone()),
                eligible: governed.memory.eligible.clone(),
            });
        // Counts, never content: a proposal is the text nobody has approved yet.
        match outcome {
            Ok(outcome) => self.record_memory_extraction_log(format!(
                "CLI memory extraction proposed {} candidate(s) for review; {} were refused.",
                outcome.accepted, outcome.rejected
            )),
            Err(error) => self.record_memory_extraction_log(format!(
                "CLI memory extraction could not queue its proposals: {error}"
            )),
        }
    }

    fn record_memory_extraction_log(&self, message: String) {
        let _ = self.ports.logging.record(AgentLog {
            level: AgentLogLevel::Warn,
            category: "session.runtime.memory-extraction".to_string(),
            message,
            agent_id: Some(self.agent_id.clone()),
            session_id: Some(self.session_id.clone()),
            operation_id: Some(self.operation_id.clone()),
            run_id: Some(self.root_context.run_id.as_str().to_string()),
            trace_id: Some(self.root_context.trace_id.as_str().to_string()),
            span_id: Some(self.agent_context.span_id.as_str().to_string()),
            occurred_at: self.ports.clock.now(),
        });
    }

    fn failed(
        &self,
        diagnostic: String,
        safe_error: Option<String>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if self.begin_terminal()?.is_none() {
            return Ok(());
        }
        let result = self.fail_claimed(
            diagnostic,
            safe_error.as_deref().unwrap_or(&self.safe_error),
        );
        if result.is_err() {
            self.finish_execution(ExecutionStatus::Failed, Some("failure_persistence_failed"));
        }
        self.finish_terminal(result.is_ok())?;
        result
    }

    /// Forgets a resumed provider thread after a turn that used it failed without saying anything.
    ///
    /// A thread can stop existing on the provider's side, and a stored id then refers to nothing.
    /// Every later turn resumes it, is rejected before producing a word, and fails identically --
    /// the seat is broken permanently and explains nothing. Forgetting the id makes the next turn
    /// start a new thread, so one bad turn stays one bad turn.
    ///
    /// Judged by behaviour rather than by matching the provider's wording, which differs per CLI
    /// (`no rollout found for thread id …` is codex's) and would be a list to maintain forever.
    /// The cost of being wrong is bounded and already a supported state: a seat that cannot resume
    /// is given the preceding conversation instead (`seat_turn.rs`), so what is lost is the CLI's
    /// own cached context and not the thread the user can see.
    ///
    /// Silence is the discriminating signal. A turn that produced any output resumed successfully,
    /// whatever went wrong afterwards, so its thread is real and is left alone.
    fn forget_unusable_resumed_thread(&self) {
        if self.resumed_thread_id.is_none() {
            return;
        }
        let said_nothing = self
            .state()
            .map(|state| state.response.trim().is_empty())
            .unwrap_or(false);
        if !said_nothing {
            return;
        }
        let cleared = match self.seat_ownership.as_ref() {
            Some(ownership) => {
                let seat = self
                    .ports
                    .sessions
                    .clear_seat_provider_thread_id(&self.session_id, &ownership.seat_id);
                // The first seat also answers to the session's own id, which is where a session
                // predating seat-owned threads keeps looking. Clearing one and not the other would
                // leave the dead id reachable.
                if ownership.seat_index == 0 {
                    seat.and_then(|()| {
                        self.ports
                            .sessions
                            .clear_runtime_session_id(&self.session_id)
                    })
                } else {
                    seat
                }
            }
            None => self
                .ports
                .sessions
                .clear_runtime_session_id(&self.session_id),
        };
        match cleared {
            Ok(()) => self.record_log(
                AgentLogLevel::Warn,
                "Forgetting the provider thread this turn tried to resume: the turn failed without \
                 producing output, so the thread may no longer exist. The next turn starts a new \
                 one."
                    .to_string(),
            ),
            Err(error) => self.record_log(
                AgentLogLevel::Warn,
                format!("Could not forget the resumed provider thread: {error}"),
            ),
        }
    }

    fn fail_claimed(
        &self,
        diagnostic: String,
        safe_error: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let current = self.current_message()?;
        if current.status == "cancelled" {
            self.mark_cancelled();
            return Ok(());
        }
        self.record_log(AgentLogLevel::Error, diagnostic);
        self.forget_unusable_resumed_thread();
        // A failed turn hands off nothing, but the coordinator still has to learn the round ended —
        // otherwise a chain waits forever on a seat that already gave up.
        self.deliver_seat_turn(None);
        self.ports
            .sessions
            .fail_message(&self.message_id, &self.session_id, safe_error)?;
        if self.seat_ownership.is_none() {
            let _ = self
                .ports
                .message_completions
                .deliver(AgentMessageTerminal {
                    session_id: self.session_id.clone(),
                    message_id: self.message_id.clone(),
                    outcome: AgentMessageTerminalOutcome::Failed,
                    content: None,
                });
        }
        self.ports
            .sessions
            .update_lifecycle(&self.session_id, AgentLifecycle::Failed)?;
        let _ = self.ports.generations.fail(&self.session_id);
        let _ = self
            .ports
            .operations
            .fail(&self.operation_id, safe_error.to_string());
        self.finish_execution(ExecutionStatus::Failed, Some("agent_generation_failed"));
        let _ = self.ports.events.publish(AgentEvent::MessageFailed {
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            error: safe_error.to_string(),
        });
        self.deliver_loop_terminal(
            LoopRoleGenerationOutcome::Failed,
            None,
            Some(safe_error.to_string()),
        )?;
        self.record_prompt_execution(PromptExecutionOutcome::Failed);
        Ok(())
    }

    fn stderr(&self, diagnostic: String) {
        let _ = self
            .ports
            .operations
            .append_log(&self.operation_id, diagnostic.clone());
        self.record_log(AgentLogLevel::Warn, diagnostic);
    }

    fn deliver_loop_terminal(
        &self,
        outcome: LoopRoleGenerationOutcome,
        content: Option<String>,
        error: Option<String>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let Some(ownership) = &self.loop_ownership else {
            return Ok(());
        };
        self.ports
            .loop_completions
            .deliver(LoopRoleGenerationTerminal {
                run_id: ownership.run_id.clone(),
                iteration_id: ownership.iteration_id.clone(),
                role: ownership.role.clone(),
                session_id: self.session_id.clone(),
                message_id: self.message_id.clone(),
                outcome,
                content,
                error,
            })?;
        Ok(())
    }

    /// Reports a finished seat turn so the coordinator can route the next one.
    ///
    /// Failures deliver `None`: a turn that did not produce a reply has nothing to hand off, so the
    /// chain stops rather than routing on an empty string.
    fn deliver_seat_turn(&self, reply: Option<String>) {
        let Some(ownership) = &self.seat_ownership else {
            return;
        };
        let _ = self.ports.seat_completions.deliver(SeatTurnTerminal {
            source: ownership.source.clone(),
            session_id: self.session_id.clone(),
            message_id: self.message_id.clone(),
            seat_id: ownership.seat_id.clone(),
            seat_index: ownership.seat_index,
            seat_mention: ownership.seat_mention.clone(),
            depth: ownership.depth,
            round_id: ownership.round_id.clone(),
            execution_run_id: self.root_context.run_id.as_str().to_string(),
            reply,
        });
    }

    fn current_message(&self) -> Result<AgentMessage, AgentRuntimeApplicationError> {
        self.ports
            .sessions
            .find_message(&self.message_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::MessageNotFound(self.message_id.clone()))
    }

    fn mark_cancelled(&self) {
        let _ = self.ports.operations.cancel(&self.operation_id);
        self.finish_execution(ExecutionStatus::Cancelled, Some("user_cancelled"));
        self.record_prompt_execution(PromptExecutionOutcome::Cancelled);
        if let Ok(mut state) = self.state() {
            state.phase = GenerationStreamPhase::Terminal;
        }
    }

    fn record_prompt_execution(&self, outcome: PromptExecutionOutcome) {
        if self.prompt_versions.is_empty() {
            return;
        }
        let elapsed_ms =
            i64::try_from(self.prompt_started_at.elapsed().as_millis()).unwrap_or(i64::MAX);
        let _ = self.ports.prompts.record_execution(PromptExecutionReport {
            invocation_id: self.operation_id.clone(),
            agent_id: self.agent_id.clone(),
            versions: self.prompt_versions.clone(),
            outcome,
            elapsed_ms,
            created_at: self.ports.clock.now(),
        });
    }

    fn begin_terminal(&self) -> Result<Option<String>, AgentRuntimeApplicationError> {
        let (response, pending_content, pending_thinking) = {
            let mut state = self.state()?;
            if state.phase != GenerationStreamPhase::Active {
                return Ok(None);
            }
            state.phase = GenerationStreamPhase::ApplyingTerminal;
            (
                state.response.clone(),
                std::mem::take(&mut state.pending_content),
                std::mem::take(&mut state.pending_thinking),
            )
        };
        // Flush the coalesced tail on the way into the terminal phase. Best-effort: the
        // success path rewrites full content via `complete_message`, but the failed path
        // and `complete_message`'s read of `thinking_content` depend on these appends.
        if !pending_content.is_empty() {
            let _ = self
                .ports
                .sessions
                .append_content(&self.message_id, &pending_content);
        }
        if !pending_thinking.is_empty() {
            let _ = self
                .ports
                .sessions
                .append_thinking(&self.message_id, &pending_thinking);
        }
        Ok(Some(response))
    }

    fn finish_terminal(&self, committed: bool) -> Result<(), AgentRuntimeApplicationError> {
        let mut state = self.state()?;
        if state.phase == GenerationStreamPhase::ApplyingTerminal {
            state.phase = if committed {
                GenerationStreamPhase::Terminal
            } else {
                GenerationStreamPhase::Active
            };
        }
        Ok(())
    }

    fn record_log(&self, level: AgentLogLevel, message: String) {
        let _ = self.ports.logging.record(AgentLog {
            level,
            category: "session.runtime".to_string(),
            message,
            agent_id: Some(self.agent_id.clone()),
            session_id: Some(self.session_id.clone()),
            operation_id: Some(self.operation_id.clone()),
            run_id: Some(self.root_context.run_id.as_str().to_string()),
            trace_id: Some(self.root_context.trace_id.as_str().to_string()),
            span_id: Some(self.agent_context.span_id.as_str().to_string()),
            occurred_at: self.ports.clock.now(),
        });
    }

    fn finish_execution(&self, status: ExecutionStatus, error_classification: Option<&str>) {
        let ended_at = self.ports.clock.now();
        if let Ok(mut state) = self.state() {
            for span_id in std::mem::take(&mut state.active_tool_spans).into_values() {
                let context = child_context(&self.agent_context, span_id.clone());
                record_telemetry_result(
                    &self.ports,
                    &context,
                    "finish_orphaned_tool_span",
                    self.ports.telemetry.finish_span(
                        &self.root_context.run_id,
                        &span_id,
                        ExecutionStatus::Incomplete,
                        &ended_at,
                        Some("provider_boundary_missing"),
                    ),
                );
            }
        }
        record_telemetry_result(
            &self.ports,
            &self.agent_context,
            "finish_agent_span",
            self.ports.telemetry.finish_span(
                &self.agent_context.run_id,
                &self.agent_context.span_id,
                status,
                &ended_at,
                error_classification,
            ),
        );
        record_telemetry_result(
            &self.ports,
            &self.root_context,
            "finish_root_span",
            self.ports.telemetry.finish_span(
                &self.root_context.run_id,
                &self.root_context.span_id,
                status,
                &ended_at,
                error_classification,
            ),
        );
        record_telemetry_result(
            &self.ports,
            &self.root_context,
            "finish_run",
            self.ports.telemetry.finish_run(
                &self.root_context.run_id,
                status,
                &ended_at,
                error_classification,
            ),
        );
    }

    fn state(
        &self,
    ) -> Result<std::sync::MutexGuard<'_, GenerationStreamState>, AgentRuntimeApplicationError>
    {
        self.state
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))
    }
}

fn provider_parent_link(event: &ToolLifecycleEvent) -> Vec<ExecutionLink> {
    let (Some(run_id), Some(trace_id)) = (&event.parent_run_id, &event.parent_trace_id) else {
        return Vec::new();
    };
    let (Ok(run_id), Ok(trace_id)) = (ExecutionRunId::parse(run_id), TraceId::parse(trace_id))
    else {
        return Vec::new();
    };
    let span_id = event
        .parent_span_id
        .as_deref()
        .and_then(|value| SpanId::parse(value).ok());
    vec![ExecutionLink {
        run_id,
        trace_id,
        span_id,
        relationship: "delegated_from".to_string(),
    }]
}

impl AgentProcessEventSink for GenerationEventHandler {
    fn handle(&self, event: GenerationProcessEvent) -> Result<(), AgentRuntimeApplicationError> {
        match event {
            GenerationProcessEvent::Token(delta) => self.token(delta),
            GenerationProcessEvent::Thinking(content_delta) => self.thinking(content_delta),
            GenerationProcessEvent::ToolUse(tool_use) => self.tool_use(tool_use),
            GenerationProcessEvent::ToolLifecycle(event) => self.tool_lifecycle(event),
            GenerationProcessEvent::RichBlock(block) => self.rich_block(block),
            // A provider thread belongs to the Agent that created it. When a seat owns this
            // generation the id is recorded against that seat; writing it to the session's single
            // slot is what left every other seat resuming an id its own CLI had never issued.
            //
            // The session slot is still written for the seat that mirrors `agent_id`, because
            // that is where a session created before seats carried threads keeps looking, and the
            // terminal path and telemetry read it too.
            GenerationProcessEvent::RuntimeSessionId(runtime_session_id) => {
                match self.seat_ownership.as_ref() {
                    Some(ownership) => {
                        self.ports.sessions.update_seat_provider_thread_id(
                            &self.session_id,
                            &ownership.seat_id,
                            &runtime_session_id,
                        )?;
                        if ownership.seat_index == 0 {
                            self.ports
                                .sessions
                                .update_runtime_session_id(&self.session_id, &runtime_session_id)?;
                        }
                        Ok(())
                    }
                    None => self
                        .ports
                        .sessions
                        .update_runtime_session_id(&self.session_id, &runtime_session_id),
                }
            }
            GenerationProcessEvent::Stderr(diagnostic) => {
                self.stderr(diagnostic);
                Ok(())
            }
            GenerationProcessEvent::Completed(usage) => self.completed(usage),
            GenerationProcessEvent::Failed(failure) => {
                self.failed(failure.diagnostic, failure.safe_error)
            }
        }
    }
}

fn bounded_count(value: usize) -> i64 {
    i64::try_from(value).unwrap_or(i64::MAX)
}

fn child_context(
    parent: &ExecutionContext,
    span_id: crate::contexts::execution_observability::api::SpanId,
) -> ExecutionContext {
    ExecutionContext {
        run_id: parent.run_id.clone(),
        trace_id: parent.trace_id.clone(),
        span_id,
        capture_policy: parent.capture_policy,
        sampling_per_million: parent.sampling_per_million,
        mcp_relay_enabled: parent.mcp_relay_enabled,
    }
}

fn safe_attributes(
    entries: impl IntoIterator<Item = (String, SafeAttributeValue)>,
) -> SafeAttributes {
    SafeAttributes::try_from_entries(entries).unwrap_or_default()
}

fn slugify(value: &str) -> String {
    let mut slug = String::with_capacity(value.len());
    let mut last_was_hyphen = true;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_hyphen = false;
        } else if !last_was_hyphen {
            slug.push('-');
            last_was_hyphen = true;
        }
    }
    while slug.ends_with('-') {
        slug.pop();
    }
    slug
}

fn execution_source(source: super::AgentMessageSource) -> ExecutionSource {
    match source {
        super::AgentMessageSource::Desktop => ExecutionSource::Desktop,
        super::AgentMessageSource::InstantMessage { connector_id } => {
            ExecutionSource::InstantMessage { connector_id }
        }
        super::AgentMessageSource::Scheduled { task_id } => ExecutionSource::Scheduled { task_id },
    }
}

fn tool_terminal_status(value: &str) -> Option<ExecutionStatus> {
    match value {
        "completed" | "succeeded" | "success" => Some(ExecutionStatus::Succeeded),
        "failed" | "error" => Some(ExecutionStatus::Failed),
        "cancelled" => Some(ExecutionStatus::Cancelled),
        _ => None,
    }
}
