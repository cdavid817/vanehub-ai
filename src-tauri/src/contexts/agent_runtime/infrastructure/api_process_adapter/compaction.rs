//! Context compaction: trigger predicates, the automatic path, and the optimizer path.

use super::super::context_projection::ContextWireShape;
use super::super::context_reduction::{build_structured_summary_turns, reconstruct_candidate};
use super::super::model_context_catalog;
use super::generation::{extract_memories_accounted, summarize_turns_accounted, GenerationOptions};
use super::invocation::WireFormat;
use super::prompt::GenerationPersonalization;
use super::{
    failed_retryable, COMPACTION_KEEP_RECENT_TURNS, COMPACTION_TRIGGER_CHARACTERS,
    OPTIMIZER_TARGET_CHARACTERS, SUMMARIZATION_INSTRUCTION,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort, AgentProcessEventSink,
    ApiProviderConfig, ContextAnalysisInput, ContextAnalysisService, ContextQualityRecorder,
    GenerationProcessEvent, GenerationProcessRequest, ToolDefinition,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    build_optimization_plan, select_authoritative_compaction, verify_optimization_candidate,
    AutomaticCompactionState, CompactionBypassReason, CompactionPath, CompactionTriggerSource,
    ContextAssessmentInvariants, ContextAssessmentOutcome, ContextAssessmentPath,
    ContextAssessmentReason, ContextAssessmentTriggerSource, ContextCompactionEvidence,
    ContextOptimizationBudget, ContextQualityAssessment, ContextQualityAssessmentInput,
    ContextQualityAssessmentRecord, ContextSnapshot, FallbackReason, OptimizationActionKind,
    OptimizationOutcome, RetentionClass, UsageAnchor, AUTOMATIC_COMPACTION_POLICY_VERSION,
    CONTEXT_OPTIMIZER_VERSION, CONTEXT_QUALITY_HISTORY_HARD_LIMIT, CONTEXT_VERIFIER_VERSION,
    STRUCTURED_SUMMARY_PROMPT,
};
use crate::contexts::sessions::api::{SessionsApi, UsagePurpose};
use serde_json::{json, Value};
use std::sync::atomic::AtomicBool;

/// Sums the length of every string value reachable within `turns`, recursively — a
/// wire-format-agnostic proxy for how large a turns list is. Both wire formats nest large
/// content (tool results, tool-call arguments) inside arrays/objects rather than as a flat
/// `content` string, so a shallow field-only count would miss exactly the payloads (e.g.
/// file-read tool output) that motivate compaction in the first place.
pub(super) fn turns_character_count(turns: &[Value]) -> usize {
    turns.iter().map(value_character_count).sum()
}

pub(super) fn value_character_count(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.iter().map(value_character_count).sum(),
        Value::Object(map) => map.values().map(value_character_count).sum(),
        _ => 0,
    }
}

pub(super) fn should_compact(character_count: usize) -> bool {
    character_count > COMPACTION_TRIGGER_CHARACTERS
}

pub(super) fn compaction_notice_block(
    message_id: &str,
    turns_before: usize,
    evidence: &ContextCompactionEvidence,
) -> Value {
    let token_value = |value: Option<u64>| {
        value.map_or_else(|| "Unavailable".to_string(), |value| value.to_string())
    };
    json!({
        "id": format!("compaction-{message_id}-{turns_before}"),
        "kind": "card",
        "v": 1,
        "title": "Conversation compacted",
        "bodyMarkdown": "Earlier context was compacted. This evidence contains measurements only and excludes conversation content.",
        "tone": "info",
        "fields": [
            { "label": "Before characters", "value": evidence.before_characters.to_string() },
            { "label": "After characters", "value": evidence.after_characters.to_string() },
            { "label": "Characters saved", "value": evidence.saved_characters.to_string() },
            { "label": "Before tokens", "value": token_value(evidence.before_tokens) },
            { "label": "After tokens", "value": token_value(evidence.after_tokens) },
            { "label": "Tokens saved", "value": token_value(evidence.saved_tokens) },
            { "label": "Measurement quality", "value": format!("{} → {}", evidence.before_quality, evidence.after_quality) },
            { "label": "Trigger source", "value": evidence.trigger_source },
            { "label": "Compaction path", "value": evidence.compaction_path },
            { "label": "Policy version", "value": evidence.policy_version },
        ],
        "meta": {
            "evidenceKind": "context-compaction",
            "attemptId": evidence.attempt_id,
            "beforeCharacters": evidence.before_characters,
            "afterCharacters": evidence.after_characters,
            "savedCharacters": evidence.saved_characters,
            "beforeTokens": evidence.before_tokens,
            "afterTokens": evidence.after_tokens,
            "savedTokens": evidence.saved_tokens,
            "beforeQuality": evidence.before_quality,
            "afterQuality": evidence.after_quality,
            "triggerSource": evidence.trigger_source,
            "compactionPath": evidence.compaction_path,
            "policyVersion": evidence.policy_version,
        },
    })
}

#[derive(Debug)]
pub(super) enum AutomaticCompactionOutcome {
    NotEligible,
    Bypassed,
    Compacted(CompactionPath),
    Failed,
    TerminalFailure(Box<GenerationProcessEvent>),
}

#[allow(clippy::too_many_arguments)]
pub(super) fn maybe_compact_accounted(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    sink: &dyn AgentProcessEventSink,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    personalization: GenerationPersonalization<'_>,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
    usage_anchor: Option<&UsageAnchor>,
    state: &mut AutomaticCompactionState,
    context_quality: Option<&ContextQualityRecorder>,
    context_quality_retention_days: i64,
) -> Option<GenerationProcessEvent> {
    let outcome = run_automatic_compaction(
        turns,
        wire_format,
        client,
        api_key,
        model,
        provider_config,
        tools,
        generation_options,
        system,
        cancelled,
        sink,
        logging,
        clock,
        request,
        personalization,
        tool_assisted,
        accounting,
        request_sequence,
        usage_anchor,
        state,
        context_quality,
        context_quality_retention_days,
    );
    match outcome {
        AutomaticCompactionOutcome::TerminalFailure(failure) => Some(*failure),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_automatic_compaction(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    sink: &dyn AgentProcessEventSink,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    personalization: GenerationPersonalization<'_>,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
    usage_anchor: Option<&UsageAnchor>,
    state: &mut AutomaticCompactionState,
    context_quality: Option<&ContextQualityRecorder>,
    context_quality_retention_days: i64,
) -> AutomaticCompactionOutcome {
    let turn_characters = turns_character_count(turns) as u64;
    let character_decision = should_compact(turn_characters as usize);
    let body = (wire_format.build_request_body)(
        &provider_config.model_id,
        turns,
        tools,
        system,
        generation_options,
    );
    let snapshot = prepared_context_snapshot(
        wire_format,
        &body,
        provider_config,
        *request_sequence,
        character_decision,
        usage_anchor,
    );
    let decision = select_authoritative_compaction(
        &snapshot.compaction_decision,
        snapshot.active_character_compaction,
    );
    let decision_sequence = *request_sequence;
    if !decision.should_compact {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "not-eligible",
            None,
            state,
            turn_characters,
        );
        return AutomaticCompactionOutcome::NotEligible;
    }
    if turns.len() <= COMPACTION_KEEP_RECENT_TURNS {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "insufficient-context",
            None,
            state,
            turn_characters,
        );
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Bypassed,
            None,
            Some(ContextAssessmentReason::InsufficientReclaimableContext),
            None,
        );
        return AutomaticCompactionOutcome::Bypassed;
    }
    if let Some(reason) = state.bypass_reason(request.automatic_compaction, turn_characters) {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "bypassed",
            Some(reason),
            state,
            turn_characters,
        );
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Bypassed,
            None,
            Some(reason.into()),
            None,
        );
        return AutomaticCompactionOutcome::Bypassed;
    }
    let original_turns = turns.clone();
    let (outcome, fallback_reason) = match optimize_compaction_accounted(
        &original_turns,
        &snapshot,
        wire_format,
        client,
        api_key,
        provider_config,
        tools,
        generation_options,
        system,
        cancelled,
        accounting,
        request,
        request_sequence,
        clock,
        logging,
    ) {
        Ok(candidate) => {
            *turns = candidate;
            (
                AutomaticCompactionOutcome::Compacted(CompactionPath::Optimizer),
                None,
            )
        }
        Err(reason) => {
            record_optimizer_fallback(logging, clock, request, *request_sequence, reason);
            let fallback_outcome = compatibility_compact_accounted(
                turns,
                wire_format,
                client,
                api_key,
                model,
                provider_config,
                system,
                cancelled,
                logging,
                clock,
                request,
                personalization,
                tool_assisted,
                accounting,
                request_sequence,
            );
            (fallback_outcome, Some(reason))
        }
    };
    if let AutomaticCompactionOutcome::Compacted(path) = &outcome {
        let turns_before = original_turns.len();
        let post_body = (wire_format.build_request_body)(
            &provider_config.model_id,
            turns,
            tools,
            system,
            generation_options,
        );
        let post_snapshot = prepared_context_snapshot(
            wire_format,
            &post_body,
            provider_config,
            decision_sequence,
            should_compact(turns_character_count(turns)),
            usage_anchor,
        );
        let assessment = record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &post_snapshot,
            decision.source,
            if *path == CompactionPath::Optimizer {
                ContextAssessmentOutcome::Compacted
            } else {
                ContextAssessmentOutcome::Fallback
            },
            Some((*path).into()),
            fallback_reason.map(Into::into),
            (*path == CompactionPath::Optimizer).then_some(ContextAssessmentInvariants::passed()),
        );
        let evidence = ContextCompactionEvidence::project(
            &snapshot,
            &post_snapshot,
            decision.source,
            *path,
            assessment.attempt_id.clone(),
        );
        if sink
            .handle(GenerationProcessEvent::RichBlock(compaction_notice_block(
                &request.message_id,
                turns_before,
                &evidence,
            )))
            .is_err()
        {
            return AutomaticCompactionOutcome::TerminalFailure(Box::new(failed_retryable(
                "Agent generation event handling failed.",
            )));
        }
    }
    if matches!(outcome, AutomaticCompactionOutcome::Failed) {
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Failed,
            Some(ContextAssessmentPath::Compatibility),
            Some(
                fallback_reason
                    .map(Into::into)
                    .unwrap_or(ContextAssessmentReason::ProviderFailure),
            ),
            None,
        );
    }
    match &outcome {
        AutomaticCompactionOutcome::Compacted(_) => {
            state.record_success(turns_character_count(turns) as u64);
        }
        AutomaticCompactionOutcome::Failed => state.record_failure(),
        _ => {}
    }
    record_compaction_control(
        logging,
        clock,
        request,
        *request_sequence,
        &snapshot,
        decision.source,
        match &outcome {
            AutomaticCompactionOutcome::Compacted(_) => "compacted",
            AutomaticCompactionOutcome::Failed => "failed",
            AutomaticCompactionOutcome::TerminalFailure(_) => "terminal-failure",
            AutomaticCompactionOutcome::NotEligible => "not-eligible",
            AutomaticCompactionOutcome::Bypassed => "bypassed",
        },
        None,
        state,
        turn_characters,
    );
    outcome
}

#[allow(clippy::too_many_arguments)]
fn record_context_quality_assessment(
    recorder: Option<&ContextQualityRecorder>,
    retention_days: i64,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    decision_sequence: u32,
    before: &ContextSnapshot,
    after: &ContextSnapshot,
    trigger_source: CompactionTriggerSource,
    outcome: ContextAssessmentOutcome,
    path: Option<ContextAssessmentPath>,
    reason: Option<ContextAssessmentReason>,
    invariants: Option<ContextAssessmentInvariants>,
) -> ContextQualityAssessment {
    let recorded_at = clock.now();
    let assessment = ContextQualityAssessment::new(ContextQualityAssessmentInput {
        generation_correlation: request.execution_context.run_id.as_str(),
        decision_sequence: u64::from(decision_sequence),
        outcome,
        path,
        reason,
        trigger_source: Some(ContextAssessmentTriggerSource::from(trigger_source)),
        before_characters: before.characters,
        after_characters: after.characters,
        before_tokens: before.tokens,
        after_tokens: after.tokens,
        measurement_quality: before.quality.into(),
        invariants,
        context_policy_version: AUTOMATIC_COMPACTION_POLICY_VERSION,
        optimizer_version: CONTEXT_OPTIMIZER_VERSION,
        verifier_version: CONTEXT_VERIFIER_VERSION,
    });
    if let Some(recorder) = recorder {
        recorder.record_with_retention_days(
            &ContextQualityAssessmentRecord {
                session_correlation: None,
                recorded_at,
                assessment: assessment.clone(),
            },
            retention_days,
            CONTEXT_QUALITY_HISTORY_HARD_LIMIT,
        );
    }
    assessment
}

#[allow(clippy::too_many_arguments)]
fn optimize_compaction_accounted(
    original_turns: &[Value],
    original: &ContextSnapshot,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    accounting: Option<&SessionsApi>,
    request: &GenerationProcessRequest,
    request_sequence: &mut u32,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Result<Vec<Value>, FallbackReason> {
    let original_body = (wire_format.build_request_body)(
        &provider_config.model_id,
        original_turns,
        tools,
        system,
        generation_options,
    );
    let target_characters = OPTIMIZER_TARGET_CHARACTERS.min(original.characters.saturating_sub(1));
    let target_tokens = original
        .tokens
        .map(|tokens| tokens.saturating_mul(target_characters) / original.characters.max(1));
    let plan = build_optimization_plan(
        original,
        ContextOptimizationBudget {
            original_characters: original.characters,
            original_tokens: original.tokens,
            target_characters,
            target_tokens,
        },
    )
    .map_err(|_| FallbackReason::InvalidPlan)?;
    if plan.outcome == OptimizationOutcome::InsufficientReclaimableContext {
        return Err(FallbackReason::InsufficientReclaimableContext);
    }
    if plan
        .actions
        .iter()
        .any(|action| action.kind == OptimizationActionKind::ReplaceReinjectable)
    {
        return Err(FallbackReason::ReinjectionUnavailable);
    }
    let shape = context_wire_shape(provider_config);
    let summary = if let Some(boundary) = plan.summary_boundary.as_ref() {
        let selected = build_structured_summary_turns(&original_body, shape, boundary)
            .map_err(|_| FallbackReason::ReconstructionFailed)?;
        let sequence = *request_sequence;
        *request_sequence = request_sequence.saturating_add(1);
        summarize_turns_accounted(
            wire_format,
            client,
            api_key,
            provider_config,
            None,
            &selected,
            STRUCTURED_SUMMARY_PROMPT,
            cancelled,
            accounting,
            request,
            UsagePurpose::ContextCompaction,
            sequence,
            clock,
            logging,
        )
        .map_err(|_| FallbackReason::SummaryFailed)?
        .ok_or(FallbackReason::SummaryFailed)?
    } else {
        String::new()
    };
    let candidate_body = reconstruct_candidate(
        &original_body,
        shape,
        &plan,
        (!summary.is_empty()).then_some(summary.as_str()),
        &[],
    )
    .map_err(|_| FallbackReason::ReconstructionFailed)?;
    let candidate = optimizer_snapshot(
        wire_format,
        &candidate_body,
        provider_config,
        *request_sequence,
    );
    let verification = verify_optimization_candidate(original, &candidate, &plan, &[]);
    if !verification.accepted {
        return Err(FallbackReason::VerificationFailed);
    }
    record_optimizer_success(
        logging,
        clock,
        request,
        *request_sequence,
        &plan,
        original,
        &candidate,
    );
    candidate_turns(&candidate_body, shape, system).ok_or(FallbackReason::ReconstructionFailed)
}

fn record_optimizer_success(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    plan: &crate::contexts::agent_runtime::domain::ContextOptimizationPlan,
    original: &ContextSnapshot,
    candidate: &ContextSnapshot,
) {
    let action_count = |kind| {
        plan.actions
            .iter()
            .filter(|action| action.kind == kind)
            .count()
    };
    let class_count = |class| {
        original
            .components
            .iter()
            .filter(|component| component.retention_class == class)
            .count()
    };
    let fingerprints = plan
        .actions
        .iter()
        .flat_map(|action| action.source_fingerprints.iter())
        .take(8)
        .map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Info,
        category: "session.runtime.api.context-optimizer".to_string(),
        message: format!(
            "optimizer={} verifier={} sequence={} result=accepted actions={} discard={} reinject={} microcompact={} summarize={} classes=protected:{},verbatim:{},summarizable:{},microcompactable:{},reinjectable:{},discardable:{} before_quality={:?} before_characters={} before_tokens={:?} after_quality={:?} after_characters={} after_tokens={:?} saved_characters={} fingerprints={} original_hash={} candidate_hash={} protocol_complete=true coverage_complete=true",
            CONTEXT_OPTIMIZER_VERSION,
            CONTEXT_VERIFIER_VERSION,
            request_sequence,
            plan.actions.len(),
            action_count(OptimizationActionKind::DiscardTransient),
            action_count(OptimizationActionKind::ReplaceReinjectable),
            action_count(OptimizationActionKind::MicrocompactToolResult),
            action_count(OptimizationActionKind::SummarizeRound),
            class_count(RetentionClass::Protected),
            class_count(RetentionClass::Verbatim),
            class_count(RetentionClass::Summarizable),
            class_count(RetentionClass::Microcompactable),
            class_count(RetentionClass::Reinjectable),
            class_count(RetentionClass::Discardable),
            original.quality,
            original.characters,
            original.tokens,
            candidate.quality,
            candidate.characters,
            candidate.tokens,
            original.characters.saturating_sub(candidate.characters),
            fingerprints,
            original.request_fingerprint,
            candidate.request_fingerprint,
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn record_optimizer_fallback(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    reason: FallbackReason,
) {
    let stage = match reason {
        FallbackReason::InvalidPlan | FallbackReason::InsufficientReclaimableContext => "planning",
        FallbackReason::ReductionFailed => "reduction",
        FallbackReason::ReinjectionUnavailable => "reinjection",
        FallbackReason::SummaryFailed => "summary",
        FallbackReason::ReconstructionFailed => "reconstruction",
        FallbackReason::VerificationFailed => "verification",
    };
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Warn,
        category: "session.runtime.api.context-optimizer".to_string(),
        message: format!(
            "optimizer={} verifier={} sequence={} result=fallback stage={} reason={:?}",
            CONTEXT_OPTIMIZER_VERSION, CONTEXT_VERIFIER_VERSION, request_sequence, stage, reason,
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

#[allow(clippy::too_many_arguments)]
fn record_compaction_control(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    snapshot: &ContextSnapshot,
    source: CompactionTriggerSource,
    result: &'static str,
    bypass_reason: Option<CompactionBypassReason>,
    state: &AutomaticCompactionState,
    turn_characters: u64,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Debug,
        category: "agent.context.compaction.control".to_string(),
        message: format!(
            "policy={} sequence={} result={} trigger_source={:?} quality={:?} tokens={:?} token_threshold={:?} request_characters={} turn_characters={} legacy_character_compact={} token_compact={:?} bypass_reason={:?} cooldown_growth={} consecutive_failures={} circuit_open={}",
            AUTOMATIC_COMPACTION_POLICY_VERSION,
            request_sequence,
            result,
            source,
            snapshot.quality,
            snapshot.tokens,
            snapshot.compaction_decision.threshold_tokens,
            snapshot.characters,
            turn_characters,
            snapshot.active_character_compaction,
            snapshot.compaction_decision.should_compact,
            bypass_reason,
            state.growth_since_success(turn_characters),
            state.consecutive_failures(),
            state.circuit_open(),
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        trace_id: Some(request.execution_context.trace_id.as_str().to_string()),
        span_id: Some(request.execution_context.span_id.as_str().to_string()),
        occurred_at: clock.now(),
    });
}

fn optimizer_snapshot(
    wire_format: &WireFormat,
    body: &Value,
    provider_config: &ApiProviderConfig,
    invocation_sequence: u32,
) -> ContextSnapshot {
    prepared_context_snapshot(
        wire_format,
        body,
        provider_config,
        invocation_sequence,
        true,
        None,
    )
}

fn prepared_context_snapshot(
    wire_format: &WireFormat,
    body: &Value,
    provider_config: &ApiProviderConfig,
    invocation_sequence: u32,
    character_decision: bool,
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
            capacity: model_context_catalog::resolve_capacity(
                provider_config.source_provider_id.as_deref(),
                &provider_config.model_id,
            ),
            active_character_compaction: character_decision,
            invocation_sequence,
            overflow_count: projection.overflow_count,
        },
        usage_anchor,
    )
}

fn context_wire_shape(provider_config: &ApiProviderConfig) -> ContextWireShape {
    if provider_config.interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        ContextWireShape::OpenAiCompatible
    } else {
        ContextWireShape::Anthropic
    }
}

fn candidate_turns(
    body: &Value,
    shape: ContextWireShape,
    system: Option<&str>,
) -> Option<Vec<Value>> {
    let mut messages = body.get("messages")?.as_array()?.clone();
    if shape == ContextWireShape::OpenAiCompatible
        && system.is_some()
        && messages
            .first()
            .and_then(|message| message.get("role"))
            .and_then(Value::as_str)
            == Some("system")
    {
        messages.remove(0);
    }
    Some(messages)
}

/// Preserves the pre-optimizer summary-only compaction path as an untouched compatibility
/// fallback. Optimizer-first orchestration calls this only with the original turns.
#[allow(clippy::too_many_arguments)]
pub(super) fn compatibility_compact_accounted(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    system: Option<&str>,
    cancelled: &AtomicBool,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    personalization: GenerationPersonalization<'_>,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
) -> AutomaticCompactionOutcome {
    if turns.len() <= COMPACTION_KEEP_RECENT_TURNS {
        return AutomaticCompactionOutcome::Failed;
    }
    let split_at = turns.len() - COMPACTION_KEEP_RECENT_TURNS;
    let compaction_sequence = *request_sequence;
    *request_sequence = request_sequence.saturating_add(1);
    let summary = match summarize_turns_accounted(
        wire_format,
        client,
        api_key,
        provider_config,
        system,
        &turns[..split_at],
        SUMMARIZATION_INSTRUCTION,
        cancelled,
        accounting,
        request,
        UsagePurpose::ContextCompaction,
        compaction_sequence,
        clock,
        logging,
    ) {
        Ok(Some(summary)) => summary,
        Ok(None) | Err(_) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.compaction".to_string(),
                message:
                    "Context compaction summarization call failed; continuing without compaction."
                        .to_string(),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return AutomaticCompactionOutcome::Failed;
        }
    };
    // Piggybacks on compaction's own trigger as a cost/latency control (design.md) — runs only
    // when the session has already gotten substantial enough to compact, using the identical
    // pre-mutation slice. Gated by the memory master switch and, only for sessions where tools
    // were used, the tool-assisted-chats sub-switch (`add-personalization-settings` D4/D5) — the
    // explicit `remember` tool is unaffected by either gate, this only skips the passive,
    // automatic extraction call itself.
    // Two questions, both answered by this generation's snapshot: whether extraction runs at all,
    // and whether it runs on a compaction that included tool calls. The sub-policy narrows the
    // second only — it must not suppress extraction on turns that used no tool. Neither is
    // re-resolved here: compaction happens partway through a generation, and a policy edit made in
    // the meantime must not extract under a rule the rest of the turn never saw.
    let extraction_allowed = personalization.snapshot.memory.automatic_extraction
        && (!tool_assisted
            || personalization
                .snapshot
                .memory
                .automatic_extraction_in_tool_assisted_turns);
    if extraction_allowed {
        extract_memories_accounted(
            wire_format,
            client,
            api_key,
            model,
            provider_config,
            system,
            &turns[..split_at],
            cancelled,
            personalization.port,
            personalization.snapshot,
            logging,
            clock,
            request,
            accounting,
            request_sequence,
        );
    }
    let mut compacted = vec![json!({ "role": "user", "content": summary })];
    compacted.extend(turns.split_off(split_at));
    *turns = compacted;
    AutomaticCompactionOutcome::Compacted(CompactionPath::Compatibility)
}
