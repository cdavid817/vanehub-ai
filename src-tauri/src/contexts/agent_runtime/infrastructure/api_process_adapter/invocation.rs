//! Accounting lifecycle for one API invocation, and the per-provider wire format it drives.

use super::super::context_projection::PreparedContextProjection;
use super::super::model_context_catalog;
use super::super::tool_call_accumulator::ToolCallAccumulator;
use super::super::{anthropic_provider, openai_compatible_provider};
use super::compaction::{should_compact, turns_character_count, value_character_count};
use super::generation::GenerationOptions;
use super::ExecutedToolCall;
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentMessage, ApiProviderConfig,
    ContextAnalysisInput, ContextAnalysisService, GenerationProcessEvent, GenerationProcessFailure,
    GenerationProcessRequest, ReportedUsageTotals, StoredEndpointProfileMetadata, ToolDefinition,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    ContextCapacity, ContextSnapshot, SemanticClass, UsageAnchor,
};
use crate::contexts::sessions::api::{
    AccountingUnit, MeasurementKind, MeasurementQuality, NewModelInvocation, NewUsageObservation,
    SessionsApi, TokenDimensions, TokenOverlap, UsageInteractionKind, UsagePurpose, UsageStatus,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

/// The wire-protocol-specific pieces `execute` needs: where to send the request, what body to
/// build, how to authenticate, and how to translate the response and build tool-reply turns.
/// Selected once per generation from the agent's `interface_format`; everything else in
/// `execute` — the tool-use loop, risk-tiered approval, and sandboxed tool execution — is
/// format-agnostic.
type BuildRequestBody =
    fn(&str, &[Value], &[ToolDefinition], Option<&str>, &GenerationOptions) -> Value;
type ProjectRequestContext = fn(&Value) -> PreparedContextProjection;

pub(crate) struct WireFormat {
    pub(super) endpoint: String,
    pub(super) history_to_turns: fn(&[AgentMessage]) -> Vec<Value>,
    pub(super) build_request_body: BuildRequestBody,
    pub(super) project_request_context: ProjectRequestContext,
    pub(super) translate_sse_data:
        fn(&str, &mut ToolCallAccumulator) -> Option<GenerationProcessEvent>,
    pub(super) build_reply_turns: fn(&str, &[ExecutedToolCall]) -> Vec<Value>,
    pub(super) failure_from_http_status: fn(u16, &str) -> GenerationProcessFailure,
    pub(super) apply_auth:
        fn(reqwest::blocking::RequestBuilder, &str) -> reqwest::blocking::RequestBuilder,
}

pub(super) fn begin_api_invocation(
    accounting: Option<&SessionsApi>,
    request: &GenerationProcessRequest,
    config: &ApiProviderConfig,
    request_sequence: u32,
    purpose: UsagePurpose,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Option<NewModelInvocation> {
    let accounting = accounting?;
    let invocation = api_invocation_snapshot(request, config, request_sequence, purpose, clock);
    if accounting.start_model_invocation(&invocation).is_err() {
        record_accounting_diagnostic(logging, clock, &invocation, "start_failed");
        return None;
    }
    Some(invocation)
}

/// Starts an accounted invocation for one subagent child turn.
///
/// A child is not a message, so it carries no message or run identity to borrow; those invocation
/// fields are optional and stay `None` rather than being filled with the parent's, which would
/// make the child's spend look like the parent's own turn (`add-onepiece-subagents`).
pub(crate) fn begin_child_invocation(
    accounting: Option<&SessionsApi>,
    identity: ChildInvocationIdentity<'_>,
    config: &ApiProviderConfig,
    request_sequence: u32,
    clock: &dyn AgentClockPort,
) -> Option<NewModelInvocation> {
    let accounting = accounting?;
    let invocation = NewModelInvocation {
        id: format!("native-subagent:{}:{}", identity.call_id, request_sequence),
        generation_id: None,
        run_id: None,
        operation_id: Some(identity.operation_id.to_owned()),
        session_id: identity.session_id.to_owned(),
        message_id: None,
        agent_id: identity.agent_id.to_owned(),
        provider_id: Some(config.interface_format.clone()),
        profile_id: None,
        endpoint_id: config
            .base_url
            .as_deref()
            .map(|value| format!("endpoint-{}", bounded_hash(value))),
        model_id: Some(config.model_id.clone()),
        interaction_kind: UsageInteractionKind::NativeApi,
        purpose: UsagePurpose::SubagentDelegation,
        request_sequence,
        attempt: 0,
        started_at: clock.now(),
    };
    if accounting.start_model_invocation(&invocation).is_err() {
        return None;
    }
    Some(invocation)
}

/// Who a child turn is spending on behalf of.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ChildInvocationIdentity<'a> {
    pub(crate) call_id: &'a str,
    pub(crate) session_id: &'a str,
    pub(crate) agent_id: &'a str,
    pub(crate) operation_id: &'a str,
}

/// Records a child turn's outcome against its invocation. Reported provider usage is used when
/// present; there is deliberately no character-count estimate, because a child turn's body carries
/// tool schemas and prior tool output whose length says nothing useful about its token cost.
pub(crate) fn finish_child_invocation(
    accounting: Option<&SessionsApi>,
    invocation: Option<&NewModelInvocation>,
    session_id: &str,
    usage: Option<&ReportedUsageTotals>,
    status: UsageStatus,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) {
    let _ = session_id;
    finish_api_invocation(accounting, invocation, usage, None, status, clock, logging);
}

pub(super) fn api_invocation_snapshot(
    request: &GenerationProcessRequest,
    config: &ApiProviderConfig,
    request_sequence: u32,
    purpose: UsagePurpose,
    clock: &dyn AgentClockPort,
) -> NewModelInvocation {
    NewModelInvocation {
        id: format!(
            "native-api:{}:{}:{}",
            request.message_id, request_sequence, 0
        ),
        generation_id: Some(request.message_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        operation_id: Some(request.operation_id.clone()),
        session_id: request.session.id.clone(),
        message_id: Some(request.message_id.clone()),
        agent_id: request.agent.id.clone(),
        provider_id: request
            .configuration
            .provider_id
            .clone()
            .or_else(|| Some(config.interface_format.clone())),
        profile_id: request.configuration.provider_id.clone(),
        endpoint_id: config
            .base_url
            .as_deref()
            .map(|value| format!("endpoint-{}", bounded_hash(value))),
        model_id: Some(config.model_id.clone()),
        interaction_kind: UsageInteractionKind::NativeApi,
        purpose,
        request_sequence,
        attempt: 0,
        started_at: clock.now(),
    }
}

/// The character-count estimate for a request body, or `None` when the body carries an image.
///
/// The estimator counts characters, and a base64 image payload is millions of them, so an
/// image-bearing request would report a wildly inflated input estimate. An image's real cost
/// depends on the provider's own tiling of its dimensions and is not derivable from length at
/// all, so reduced reported coverage beats a confident wrong number
/// (`add-agent-image-input`).
pub(super) fn estimated_input_characters(body: &Value, images_in_request: usize) -> Option<usize> {
    (images_in_request == 0).then(|| value_character_count(body))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn finish_api_invocation(
    accounting: Option<&SessionsApi>,
    invocation: Option<&NewModelInvocation>,
    usage: Option<&ReportedUsageTotals>,
    estimated_characters: Option<(usize, usize)>,
    status: UsageStatus,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) {
    let (Some(accounting), Some(invocation)) = (accounting, invocation) else {
        return;
    };
    let observed_at = clock.now();
    let observation = usage
        .map(|usage| {
            let overlap = |value| match value {
                crate::contexts::agent_runtime::application::AgentUsageOverlap::Subset => {
                    TokenOverlap::Subset
                }
                crate::contexts::agent_runtime::application::AgentUsageOverlap::Exclusive => {
                    TokenOverlap::Exclusive
                }
                crate::contexts::agent_runtime::application::AgentUsageOverlap::Unknown => {
                    TokenOverlap::Unknown
                }
            };
            NewUsageObservation {
                id: format!("{}:reported", invocation.id),
                invocation_id: invocation.id.clone(),
                quality: MeasurementQuality::Reported,
                unit: AccountingUnit::Tokens,
                measurement_kind: MeasurementKind::Interval,
                dimensions: TokenDimensions {
                    input: usage.input_tokens,
                    output: usage.output_tokens,
                    cached_input: usage.cache_read_tokens,
                    cache_write_input: usage.cache_creation_tokens,
                    reasoning_output: usage.reasoning_output_tokens,
                    provider_total: usage.provider_total_tokens,
                },
                cache_overlap: overlap(usage.cache_overlap),
                reasoning_overlap: overlap(usage.reasoning_overlap),
                normalization_version: usage.normalization_version.to_string(),
                source: "provider-api-stream".to_string(),
                source_key: format!("{}:reported", invocation.id),
                source_revision: None,
                supersedes_observation_id: None,
                event_at: None,
                observed_at: observed_at.clone(),
                provenance_hash: None,
            }
        })
        .or_else(|| {
            let (input, output) = estimated_characters?;
            Some(NewUsageObservation {
                id: format!("{}:estimated", invocation.id),
                invocation_id: invocation.id.clone(),
                quality: MeasurementQuality::Estimated,
                unit: AccountingUnit::Characters,
                measurement_kind: MeasurementKind::Interval,
                dimensions: TokenDimensions {
                    input: i64::try_from(input).unwrap_or(i64::MAX),
                    output: i64::try_from(output).unwrap_or(i64::MAX),
                    ..TokenDimensions::default()
                },
                cache_overlap: TokenOverlap::Unknown,
                reasoning_overlap: TokenOverlap::Unknown,
                normalization_version: "api-character-count-v1".to_string(),
                source: "character-count".to_string(),
                source_key: format!("{}:estimated", invocation.id),
                source_revision: None,
                supersedes_observation_id: None,
                event_at: None,
                observed_at: observed_at.clone(),
                provenance_hash: None,
            })
        });
    if observation
        .as_ref()
        .is_some_and(|observation| accounting.record_token_observation(observation).is_err())
    {
        record_accounting_diagnostic(logging, clock, invocation, "observation_failed");
    }
    if accounting
        .finalize_model_invocation(&invocation.id, status, &observed_at)
        .is_err()
    {
        record_accounting_diagnostic(logging, clock, invocation, "finalize_failed");
    }
}

pub(super) fn record_accounting_diagnostic(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    invocation: &NewModelInvocation,
    reason: &str,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Warn,
        category: "token.accounting.api".to_string(),
        message: format!(
            "API accounting degraded reason={reason} request_sequence={} adapter=v1",
            invocation.request_sequence
        ),
        agent_id: Some(invocation.agent_id.clone()),
        session_id: Some(invocation.session_id.clone()),
        operation_id: invocation.operation_id.clone(),
        run_id: invocation.run_id.clone(),
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn bounded_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// One round trip's measurement of its own request, analyzed before the send and handed to
/// `record_context_snapshot` below. The model catalog supplies the capacity only when neither a
/// frozen endpoint Profile nor stored Profile metadata did — an endpoint whose metadata says
/// nothing about its window is left without one rather than given the catalog's guess.
#[allow(clippy::too_many_arguments)]
pub(super) fn analyze_round_context(
    body: &Value,
    wire_format: &WireFormat,
    provider_config: &ApiProviderConfig,
    endpoint_capacity: Option<&ContextCapacity>,
    endpoint_metadata: Option<&StoredEndpointProfileMetadata>,
    request: &GenerationProcessRequest,
    turns: &[Value],
    request_sequence: u32,
    usage_anchor: Option<&UsageAnchor>,
) -> ContextSnapshot {
    let projection = (wire_format.project_request_context)(body);
    ContextAnalysisService::analyze(
        ContextAnalysisInput {
            provider_id: provider_config.source_provider_id.clone(),
            model_id: provider_config.model_id.clone(),
            request_fingerprint: projection.request_fingerprint,
            characters: projection.characters,
            components: projection.components,
            rounds: projection.rounds,
            token_estimate_complete: projection.token_estimate_complete,
            capacity: endpoint_capacity.cloned().or_else(|| {
                (endpoint_metadata.is_none() && request.endpoint_profile.is_none()).then(|| {
                    model_context_catalog::resolve_capacity(
                        provider_config.source_provider_id.as_deref(),
                        &provider_config.model_id,
                    )
                })?
            }),
            active_character_compaction: should_compact(turns_character_count(turns)),
            invocation_sequence: request_sequence,
            overflow_count: projection.overflow_count,
        },
        usage_anchor,
    )
}

pub(super) fn record_context_snapshot(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    snapshot: &ContextSnapshot,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Debug,
        category: "agent.context.measurement".to_string(),
        message: context_snapshot_diagnostic(request_sequence, snapshot),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        trace_id: Some(request.execution_context.trace_id.as_str().to_string()),
        span_id: Some(request.execution_context.span_id.as_str().to_string()),
        occurred_at: clock.now(),
    });
}

pub(crate) fn context_snapshot_diagnostic(
    request_sequence: u32,
    snapshot: &ContextSnapshot,
) -> String {
    let capacity = snapshot
        .capacity
        .as_ref()
        .map(|value| value.context_window_tokens.to_string())
        .unwrap_or_else(|| "unknown".to_string());
    let disagreement = snapshot
        .compaction_decision
        .should_compact
        .is_some_and(|shadow| shadow != snapshot.active_character_compaction);
    let class_count = |class| {
        snapshot
            .components
            .iter()
            .filter(|component| component.semantic_class == class)
            .count()
    };
    format!(
            "snapshot={} estimator={} policy={} sequence={} request_hash={} quality={:?} characters={} tokens={} capacity={} reserved={:?} remaining={:?} utilization_bps={:?} components={} rounds={} classes=system:{},schemas:{},user:{},assistant:{},tool_requests:{},tool_results:{},attachments:{},memory:{},unknown:{} legacy_character_compact={} token_compact={:?} token_threshold={:?} token_reason={:?} disagreement={} disagreement_reason={} overflows={}",
            snapshot.version,
            snapshot.estimator_version,
            snapshot.policy_version,
            request_sequence,
            snapshot.request_fingerprint,
            snapshot.quality,
            snapshot.characters,
            snapshot.tokens.map_or_else(|| "unknown".to_string(), |value| value.to_string()),
            capacity,
            snapshot.reserved_tokens,
            snapshot.remaining_tokens,
            snapshot.utilization_basis_points,
            snapshot.components.len(),
            snapshot.rounds.len(),
            class_count(SemanticClass::SystemInstruction),
            class_count(SemanticClass::ToolSchema),
            class_count(SemanticClass::UserIntent),
            class_count(SemanticClass::AssistantResponse),
            class_count(SemanticClass::ToolRequest),
            class_count(SemanticClass::ToolResult),
            class_count(SemanticClass::Attachment),
            class_count(SemanticClass::Memory),
            class_count(SemanticClass::Unknown),
            snapshot.active_character_compaction,
            snapshot.compaction_decision.should_compact,
            snapshot.compaction_decision.threshold_tokens,
            snapshot.compaction_decision.reason,
            disagreement,
            if disagreement { "legacy-character-production-token" } else { "none" },
            snapshot.overflow_count,
        )
}

/// `Err` carries a plain diagnostic message rather than `GenerationProcessEvent` — that enum
/// has a large `ToolLifecycle`/`RichBlock`-sized variant, and this function's only failure case
/// is a short, statically-known string, so the caller wraps it into a `Failed` event itself.
pub(crate) fn wire_format_for(config: &ApiProviderConfig) -> Result<WireFormat, &'static str> {
    if config.interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        let base_url = config
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or("No base URL is configured for this OpenAI-compatible agent.")?;
        Ok(WireFormat {
            endpoint: format!("{}/chat/completions", base_url.trim_end_matches('/')),
            history_to_turns: openai_compatible_provider::history_to_turns,
            build_request_body: openai_compatible_provider::build_request_body,
            project_request_context: openai_compatible_provider::project_request_context,
            translate_sse_data: openai_compatible_provider::translate_sse_data,
            build_reply_turns: openai_compatible_provider::build_reply_turns,
            failure_from_http_status: openai_compatible_provider::failure_from_http_status,
            apply_auth: |builder, api_key| {
                if api_key.is_empty() {
                    builder
                } else {
                    builder.header("Authorization", format!("Bearer {api_key}"))
                }
            },
        })
    } else {
        let base_url = config
            .base_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("https://api.anthropic.com");
        let endpoint = if base_url.ends_with("/v1/messages") {
            base_url.to_string()
        } else {
            format!("{}/v1/messages", base_url.trim_end_matches('/'))
        };
        let official_anthropic = base_url.trim_end_matches('/') == "https://api.anthropic.com";
        Ok(WireFormat {
            endpoint,
            history_to_turns: anthropic_provider::history_to_turns,
            build_request_body: anthropic_provider::build_request_body,
            project_request_context: anthropic_provider::project_request_context,
            translate_sse_data: anthropic_provider::translate_sse_data,
            build_reply_turns: anthropic_provider::build_reply_turns,
            failure_from_http_status: anthropic_provider::failure_from_http_status,
            apply_auth: if official_anthropic {
                |builder, api_key| {
                    let builder = if api_key.is_empty() {
                        builder
                    } else {
                        builder.header("x-api-key", api_key)
                    };
                    builder.header("anthropic-version", anthropic_provider::ANTHROPIC_VERSION)
                }
            } else {
                |builder, api_key| {
                    let builder = if api_key.is_empty() {
                        builder
                    } else {
                        builder.header("Authorization", format!("Bearer {api_key}"))
                    };
                    builder.header("anthropic-version", anthropic_provider::ANTHROPIC_VERSION)
                }
            },
        })
    }
}
