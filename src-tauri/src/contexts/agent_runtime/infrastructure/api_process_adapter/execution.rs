//! The tool-use loop and the skill-tool dispatch it drives.

use super::super::agent_image::{AgentImage, MAX_IMAGES_PER_REQUEST};
use super::super::memory_selection_gateway::RuntimeAgentMemorySelectionAdapter;
use super::super::tool_call_accumulator::ToolCallAccumulator;
use super::super::tools::{execute_file_image_read, ToolExecutionOutcome};
use super::super::SqliteNativeToolRepository;
use super::compaction::maybe_compact_accounted;
use super::endpoint::{resolve_endpoint, resolve_image_support, ResolvedEndpoint};
use super::generation::{
    generation_options_from_configuration, is_plan_mode, reviewed_stream_usage_strategy,
};
use super::interactive::{
    ask_user_question, authorize_tool_call, request_plan_exit, ToolAuthorization,
};
use super::invocation::{
    analyze_round_context, begin_api_invocation, estimated_input_characters, finish_api_invocation,
    record_context_snapshot, WireFormat,
};
use super::native_tools::{
    execute_registered_native_tool, execute_tool_call_with_runtime_ports, is_image_read_request,
    log_image_attachment, resolve_tool_image,
};
use super::prompt::{
    resolve_generation_personalization, resolve_generation_skill_tools,
    resolve_generation_tool_catalog, resolve_system_prompt_with_settings,
};
use super::{
    failed_non_retryable, failed_retryable, ExecutedToolCall, PendingApprovals, HISTORY_LIMIT,
    MAX_TOOL_ROUND_TRIPS,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentCodeIntelligencePort, AgentCoreInstructionsPort, AgentLoggingPort,
    AgentMcpToolPort, AgentMemoryPort, AgentPermissionPort, AgentPersonalizationSnapshotPort,
    AgentProcessEventSink, AgentRetrievalPort, AgentRuntimeApplicationError, AgentSkillPort,
    AgentWorkspaceMutationPort, ApiAgentGateway, ApiCredentialPort, ContextAnalysisService,
    ContextQualityRecorder, ConversationHistoryPort, GenerationProcessEvent,
    GenerationProcessFailure, GenerationProcessRequest, NativeToolRegistry, ReportedUsageTotals,
    SkillToolUseProvenance, ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock,
    UtilityDelegationApplicationService, ASK_USER_QUESTION_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME,
    REMEMBER_TOOL_NAME,
};
use crate::contexts::agent_runtime::domain::{AutomaticCompactionState, UsageAnchor};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::sessions::api::{NewModelInvocation, SessionsApi, UsagePurpose, UsageStatus};
use crate::contexts::skill_evolution_evidence::domain::ObservedSkillRevision;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolCatalogMode, SkillToolCatalogPort, SkillToolDispatchOutcome,
    SkillToolExecutionLifecyclePhase, SkillToolExecutionLifecyclePort, SkillToolExecutionPort,
    SkillToolExecutionRequest,
};
use crate::platform::network::blocking_http_client;
use serde_json::Value;
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

#[allow(clippy::too_many_arguments)]
pub(super) fn execute_with_code_intelligence(
    request: &GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    credentials: &dyn ApiCredentialPort,
    config: &dyn ApiAgentGateway,
    history: &dyn ConversationHistoryPort,
    sink: &dyn AgentProcessEventSink,
    pending_approvals: &PendingApprovals,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    skills: &dyn AgentSkillPort,
    core_instructions: &dyn AgentCoreInstructionsPort,
    memories: &dyn AgentMemoryPort,
    mcp: &dyn AgentMcpToolPort,
    permissions: &dyn AgentPermissionPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: &dyn AgentCodeIntelligencePort,
    workspace_mutations: &dyn AgentWorkspaceMutationPort,
    personalization: &dyn AgentPersonalizationSnapshotPort,
    context_quality: Option<&ContextQualityRecorder>,
    utility_delegation: Option<&UtilityDelegationApplicationService>,
    skill_tool_catalog: Option<&dyn SkillToolCatalogPort>,
    skill_tool_execution: Option<&dyn SkillToolExecutionPort>,
    observed_skill_revisions: &mut Vec<ObservedSkillRevision>,
    accounting: Option<&SessionsApi>,
    native_tools: &NativeToolRegistry,
    native_tool_operations: Option<&SqliteNativeToolRepository>,
    artifacts: Option<&ArtifactService>,
    native_tool_events: Option<&tauri::AppHandle>,
) -> GenerationProcessEvent {
    let agent_id = request.agent.id.as_str();
    let ResolvedEndpoint {
        provider_config,
        endpoint_metadata,
        endpoint_capacity,
        api_key,
        wire_format,
    } = match resolve_endpoint(request, agent_id, config, credentials) {
        Ok(endpoint) => endpoint,
        Err(failure) => return failure,
    };
    let generation_personalization =
        resolve_generation_personalization(personalization, logging, clock, request);
    // Built here, and the prompt resolved once, before the round-trip loop below. That is what
    // makes the system prompt byte-identical across every round trip of this generation.
    let selection = RuntimeAgentMemorySelectionAdapter::new(credentials, config);
    let system = resolve_system_prompt_with_settings(
        agent_id,
        core_instructions,
        &generation_personalization,
        skills,
        personalization,
        &selection,
        logging,
        clock,
        request,
        observed_skill_revisions,
    );
    let recent = match history.recent_messages(&request.session.id, HISTORY_LIMIT) {
        Ok(messages) => messages,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    // Signal for the tool-assisted extraction sub-policy, seeded from the persisted message
    // history rather than from wire-format `turns`, so it needs no per-provider parsing and no
    // index alignment with whatever `maybe_compact` later slices off. Mutable because this
    // generation's own tool round trips can still flip it before the in-loop `maybe_compact` call:
    // seeding it from `recent` alone would miss a session's very first tool call when compaction
    // triggers within that same generation.
    let mut tool_assisted_session = recent.iter().any(|message| !message.tool_use.is_empty());
    let request_timeout = if let Some(profile) = request.endpoint_profile.as_ref() {
        Duration::from_millis(profile.timeout_ms)
    } else {
        match config.api_endpoint_timeout_ms(agent_id) {
            Ok(timeout_ms) => Duration::from_millis(timeout_ms),
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let client = match blocking_http_client(request_timeout) {
        Ok(client) => client,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    let plan_mode = is_plan_mode(&request.configuration);
    let mut tools = resolve_generation_tool_catalog(
        request,
        mcp,
        logging,
        clock,
        retrieval,
        code_intelligence,
        native_tools,
        utility_delegation,
        plan_mode,
        generation_personalization.memory.read,
    );
    // Declared here, not returned as one value: `_skill_tool_catalog_lease` is an `Arc` held for
    // the rest of the generation, and these three drop in this order at the end of it.
    let mut skill_tool_keys = HashMap::new();
    let mut _skill_tool_catalog_lease = None;
    let mut _skill_tool_catalog_generation = None;
    if let Some(catalog) = skill_tool_catalog {
        if let Some(resolved) = resolve_generation_skill_tools(
            catalog,
            request,
            &provider_config,
            observed_skill_revisions,
            &tools,
            logging,
            clock,
            plan_mode,
        ) {
            tools.extend(resolved.definitions);
            skill_tool_keys = resolved.keys_by_name;
            _skill_tool_catalog_generation = Some(resolved.generation);
            _skill_tool_catalog_lease = Some(resolved.lease);
        }
    }
    let mut generation_options = generation_options_from_configuration(
        &request.configuration,
        reviewed_stream_usage_strategy(&provider_config),
    );
    if request
        .endpoint_profile
        .as_ref()
        .is_some_and(|profile| profile.reasoning_field_capability != "supported")
    {
        generation_options.thinking = false;
        generation_options.reasoning_depth = None;
    }
    let mut turns = (wire_format.history_to_turns)(&recent);
    let mut request_sequence = 0u32;
    let mut context_usage_anchor: Option<UsageAnchor> = None;
    let mut automatic_compaction_state = AutomaticCompactionState::with_user_preference(
        generation_personalization.automatic_context_compaction_enabled,
    );
    if let Some(failure) = maybe_compact_accounted(
        &mut turns,
        &wire_format,
        &client,
        &api_key,
        &provider_config.model_id,
        &provider_config,
        &tools,
        &generation_options,
        system.as_deref(),
        &cancelled,
        sink,
        logging,
        clock,
        request,
        memories,
        &generation_personalization,
        tool_assisted_session,
        accounting,
        &mut request_sequence,
        context_usage_anchor.as_ref(),
        &mut automatic_compaction_state,
        context_quality,
        generation_personalization.context_quality_retention_days,
    ) {
        return failure;
    }

    let mut emitted_visible_content = false;
    let images_supported =
        resolve_image_support(request, endpoint_metadata.as_ref(), &provider_config);
    let mut images_in_request = 0_usize;
    let mut context_recovery_attempted = false;
    for round_trip in 0..MAX_TOOL_ROUND_TRIPS {
        if cancelled.load(Ordering::SeqCst) {
            return failed_non_retryable("Generation was cancelled.");
        }
        let sequence = request_sequence;
        request_sequence = request_sequence.saturating_add(1);
        let purpose = if round_trip == 0 {
            UsagePurpose::AssistantInitial
        } else {
            UsagePurpose::ToolContinuation
        };
        let invocation = begin_api_invocation(
            accounting,
            request,
            &provider_config,
            sequence,
            purpose,
            clock,
            logging,
        );
        let body = (wire_format.build_request_body)(
            &provider_config.model_id,
            &turns,
            &tools,
            system.as_deref(),
            &generation_options,
        );
        let mut context_snapshot = analyze_round_context(
            &body,
            &wire_format,
            &provider_config,
            endpoint_capacity.as_ref(),
            endpoint_metadata.as_ref(),
            request,
            &turns,
            sequence,
            context_usage_anchor.as_ref(),
        );
        record_context_snapshot(logging, clock, request, sequence, &context_snapshot);
        if context_snapshot.capacity.as_ref().is_some_and(|capacity| {
            context_snapshot.tokens.is_some_and(|tokens| {
                tokens.saturating_add(context_snapshot.reserved_tokens.unwrap_or_default())
                    > capacity.context_window_tokens
            })
        }) {
            finish_api_invocation(
                accounting,
                invocation.as_ref(),
                None,
                None,
                UsageStatus::Failed,
                clock,
                logging,
            );
            return failed_non_retryable(
                "The protected request content exceeds the selected endpoint Profile context budget.",
            );
        }
        let request_builder =
            (wire_format.apply_auth)(client.post(&wire_format.endpoint), &api_key);
        let estimated_input_characters = estimated_input_characters(&body, images_in_request);
        let response = match request_builder
            .header("content-type", "application/json")
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                finish_api_invocation(
                    accounting,
                    invocation.as_ref(),
                    None,
                    None,
                    UsageStatus::Failed,
                    clock,
                    logging,
                );
                return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                    error.to_string(),
                ));
            }
        };
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            finish_api_invocation(
                accounting,
                invocation.as_ref(),
                None,
                None,
                UsageStatus::Failed,
                clock,
                logging,
            );
            let context_failure = matches!(status.as_u16(), 400 | 413 | 422)
                && body_text.to_lowercase().contains("context");
            if context_failure && !context_recovery_attempted && turns.len() > 1 {
                context_recovery_attempted = true;
                turns.remove(0);
                continue;
            }
            return GenerationProcessEvent::Failed((wire_format.failure_from_http_status)(
                status.as_u16(),
                &body_text,
            ));
        }

        let StreamedRound {
            mut accumulator,
            assistant_text,
            round_usage,
        } = match stream_round(
            response,
            &wire_format,
            sink,
            &cancelled,
            &mut emitted_visible_content,
            accounting,
            invocation.as_ref(),
            clock,
            logging,
        ) {
            Ok(round) => round,
            Err(failure) => return failure,
        };

        finish_api_invocation(
            accounting,
            invocation.as_ref(),
            round_usage.as_ref(),
            estimated_input_characters.map(|input| (input, assistant_text.chars().count())),
            UsageStatus::Succeeded,
            clock,
            logging,
        );
        if round_usage.as_ref().is_some_and(|usage| {
            ContextAnalysisService::finalize_reported_snapshot(
                &mut context_snapshot,
                usage.input_tokens,
            )
        }) {
            record_context_snapshot(logging, clock, request, sequence, &context_snapshot);
        }
        context_usage_anchor = round_usage.as_ref().and_then(|usage| {
            ContextAnalysisService::finalize_anchor(
                &context_snapshot,
                provider_config.source_provider_id.as_deref(),
                &provider_config.model_id,
                sequence,
                usage.input_tokens,
            )
        });

        let tool_calls = accumulator.take_completed();
        if tool_calls.is_empty() {
            return GenerationProcessEvent::Completed(None);
        }

        let mut executed: Vec<ExecutedToolCall> = Vec::with_capacity(tool_calls.len());
        for mut tool_use in tool_calls {
            if cancelled.load(Ordering::SeqCst) {
                return failed_non_retryable("Generation was cancelled.");
            }
            let input = tool_use.input.clone().unwrap_or(Value::Null);
            if native_tools.handler(&tool_use.name).is_some() {
                let outcome = match execute_registered_native_tool(
                    &mut tool_use,
                    &input,
                    request,
                    cancelled.clone(),
                    native_tools,
                    native_tool_operations,
                    native_tool_events,
                    permissions,
                    pending_approvals,
                    sink,
                    plan_mode,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                let (outcome, image_artifact_id) = outcome;
                if cancelled.load(Ordering::SeqCst) {
                    return failed_non_retryable("Generation was cancelled.");
                }
                // The tool named an Artifact, not bytes, so nothing base64 has entered the output
                // above or the operation record. Resolving it here is a local, hash-verified read.
                let image = image_artifact_id.as_deref().and_then(|artifact_id| {
                    resolve_tool_image(artifacts, artifact_id, images_supported, images_in_request)
                });
                if let Some(image) = image.as_ref() {
                    log_image_attachment(logging, clock, request, &tool_use.id, image);
                    images_in_request += 1;
                }
                match record_tool_outcome(sink, tool_use, outcome, image) {
                    Ok(entry) => executed.push(entry),
                    Err(failure) => return failure,
                }
                continue;
            }
            // Intercepted here for the same reason `ask_user_question` is: the image has to reach
            // `build_reply_turns`, and `execute_tool_call_impl` can only return text
            // (`add-agent-image-input`).
            if images_supported && is_image_read_request(&tool_use.name, &input) {
                let folder = request.session.folder.as_deref().unwrap_or_default();
                // Checked per call, not per round trip: the counter only moves here, so a
                // round-trip-scoped check would let every image in one batch through. Exceeding
                // the budget is an explicit error rather than a silent drop -- a question
                // answered about the image that got dropped would be confident nonsense.
                let (outcome, image) = if images_in_request >= MAX_IMAGES_PER_REQUEST {
                    (
                        ToolExecutionOutcome {
                            output: format!(
                                "This request already carries the maximum of {MAX_IMAGES_PER_REQUEST} images. Ask about the images already attached before reading another."
                            ),
                            is_error: true,
                        },
                        None,
                    )
                } else {
                    match execute_file_image_read(
                        input
                            .get("path")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        folder,
                    ) {
                        Ok((summary, image)) => (
                            ToolExecutionOutcome {
                                output: summary,
                                is_error: false,
                            },
                            Some(image),
                        ),
                        Err(outcome) => (outcome, None),
                    }
                };
                if let Some(image) = image.as_ref() {
                    log_image_attachment(logging, clock, request, &tool_use.id, image);
                    images_in_request += 1;
                }
                match record_tool_outcome(sink, tool_use, outcome, image) {
                    Ok(entry) => executed.push(entry),
                    Err(failure) => return failure,
                }
                continue;
            }
            // Handled here rather than in `execute_tool_call_impl` because asking needs the event
            // sink and the blocked-call channel, exactly as the approval gate below does
            // (`add-agent-user-question` D1).
            if tool_use.name == ASK_USER_QUESTION_TOOL_NAME {
                let outcome = match ask_user_question(
                    &mut tool_use,
                    &input,
                    request.interactive,
                    &cancelled,
                    pending_approvals,
                    sink,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                match record_tool_outcome(sink, tool_use, outcome, None) {
                    Ok(entry) => executed.push(entry),
                    Err(failure) => return failure,
                }
                continue;
            }
            // Same placement and reason as the question above: the sink and blocked-call channel
            // live here (`add-agent-plan-exit-request` D1).
            if tool_use.name == EXIT_PLAN_MODE_TOOL_NAME {
                let outcome = match request_plan_exit(
                    &mut tool_use,
                    &input,
                    request.interactive,
                    plan_mode,
                    &cancelled,
                    pending_approvals,
                    sink,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                match record_tool_outcome(sink, tool_use, outcome, None) {
                    Ok(entry) => executed.push(entry),
                    Err(failure) => return failure,
                }
                continue;
            }
            if tool_use.name.starts_with("skill__") {
                let skill_key = skill_tool_keys.get(&tool_use.name);
                tool_use.skill_provenance = skill_key.map(skill_tool_provenance);
                tool_use.status = "running".to_string();
                if emit_skill_tool_lifecycle(sink, &tool_use, ToolLifecyclePhase::Started).is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                let lifecycle = AgentSkillToolLifecycle {
                    sink,
                    tool_use: &tool_use,
                };
                let outcome = dispatch_skill_tool(
                    skill_tool_execution,
                    skill_key,
                    &tool_use.id,
                    agent_id,
                    request.session.folder.as_deref(),
                    &request.session.id,
                    &request.operation_id,
                    plan_mode,
                    &input,
                    &cancelled,
                    &lifecycle,
                );
                if matches!(outcome.output.as_str(), "cancelled")
                    || cancelled.load(Ordering::SeqCst)
                {
                    tool_use.status = "cancelled".to_string();
                    set_skill_result_summary(&mut tool_use, "cancelled");
                    let _ =
                        emit_skill_tool_lifecycle(sink, &tool_use, ToolLifecyclePhase::Cancelled);
                    return failed_non_retryable("Generation was cancelled.");
                }
                tool_use.status = if outcome.is_error {
                    "failed".to_string()
                } else {
                    "completed".to_string()
                };
                let terminal_phase = if outcome.is_error {
                    ToolLifecyclePhase::Failed
                } else {
                    ToolLifecyclePhase::Completed
                };
                let result_label = if outcome.is_error {
                    "failed"
                } else {
                    "completed"
                };
                set_skill_result_summary(&mut tool_use, result_label);
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if emit_skill_tool_lifecycle(sink, &tool_use, terminal_phase).is_err() {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
                continue;
            }
            match authorize_tool_call(
                &mut tool_use,
                &input,
                agent_id,
                request,
                permissions,
                pending_approvals,
                sink,
                &cancelled,
            ) {
                ToolAuthorization::Allowed => {}
                ToolAuthorization::Denied(denial) => {
                    executed.push((tool_use, denial, true, None));
                    continue;
                }
                ToolAuthorization::Failed(failure) => return failure,
            }
            tool_use.status = "running".to_string();
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return failed_retryable("Agent generation event handling failed.");
            }
            let outcome = if tool_use.name == REMEMBER_TOOL_NAME
                && !generation_personalization.memory.explicit_save
            {
                // From this generation's snapshot rather than re-resolved per tool call: a second
                // read could disagree with the one the prompt was built from, allowing a tool
                // against a policy the model was never told about. Rejected before dispatching, so
                // this never reaches `AgentMemoryPort::save`.
                ToolExecutionOutcome {
                    output: "Memory is disabled; nothing was remembered.".to_string(),
                    is_error: true,
                }
            } else {
                execute_tool_call_with_runtime_ports(
                    &tool_use.name,
                    &input,
                    request.session.folder.as_deref(),
                    cancelled.clone(),
                    agent_id,
                    memories,
                    mcp,
                    retrieval,
                    code_intelligence,
                    workspace_mutations,
                    plan_mode,
                    skills,
                    utility_delegation,
                    request,
                )
            };
            if cancelled.load(Ordering::SeqCst) {
                return failed_non_retryable("Generation was cancelled.");
            }
            match record_tool_outcome(sink, tool_use, outcome, None) {
                Ok(entry) => executed.push(entry),
                Err(failure) => return failure,
            }
        }

        turns.extend((wire_format.build_reply_turns)(&assistant_text, &executed));
        // `tool_calls` was non-empty to reach this point (checked above), and every branch through
        // the loop above either pushes to `executed` or returns the whole generation early — so
        // reaching here always means at least one tool call was attempted this round trip.
        if !executed.is_empty() {
            tool_assisted_session = true;
        }
        if let Some(failure) = maybe_compact_accounted(
            &mut turns,
            &wire_format,
            &client,
            &api_key,
            &provider_config.model_id,
            &provider_config,
            &tools,
            &generation_options,
            system.as_deref(),
            &cancelled,
            sink,
            logging,
            clock,
            request,
            memories,
            &generation_personalization,
            tool_assisted_session,
            accounting,
            &mut request_sequence,
            context_usage_anchor.as_ref(),
            &mut automatic_compaction_state,
            context_quality,
            generation_personalization.context_quality_retention_days,
        ) {
            return failure;
        }
    }

    failed_non_retryable("Tool-use loop exceeded the maximum number of round trips.")
}

/// What one round trip's SSE stream produced.
struct StreamedRound {
    accumulator: ToolCallAccumulator,
    assistant_text: String,
    round_usage: Option<ReportedUsageTotals>,
}

/// Reads one round trip's `text/event-stream` response to completion.
///
/// Each `Err` is the `GenerationProcessEvent` the caller returns unchanged. Note the asymmetry the
/// exits carry with them, unchanged from when this loop was inline: cancellation, a read error and
/// a translated failure each finish the accounting invocation before returning, while a rejected
/// event does **not** — a sink that cannot accept events cannot be trusted to have observed the
/// round at all, so the invocation is left open rather than recorded as a completed failure.
#[allow(clippy::too_many_arguments, clippy::result_large_err)]
fn stream_round(
    response: reqwest::blocking::Response,
    wire_format: &WireFormat,
    sink: &dyn AgentProcessEventSink,
    cancelled: &AtomicBool,
    emitted_visible_content: &mut bool,
    accounting: Option<&SessionsApi>,
    invocation: Option<&NewModelInvocation>,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Result<StreamedRound, GenerationProcessEvent> {
    let mut reader = std::io::BufReader::new(response);
    let mut current_data: Option<String> = None;
    let mut accumulator = ToolCallAccumulator::default();
    let mut assistant_text = String::new();
    let mut round_usage = None;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            finish_api_invocation(
                accounting,
                invocation,
                round_usage.as_ref(),
                None,
                UsageStatus::Cancelled,
                clock,
                logging,
            );
            return Err(failed_non_retryable("Generation was cancelled."));
        }
        let mut line = String::new();
        let read = match reader.read_line(&mut line) {
            Ok(read) => read,
            Err(error) => {
                finish_api_invocation(
                    accounting,
                    invocation,
                    round_usage.as_ref(),
                    None,
                    UsageStatus::Failed,
                    clock,
                    logging,
                );
                return Err(GenerationProcessEvent::Failed(
                    GenerationProcessFailure::retryable(format!(
                        "Failed to read the provider API response: {error}"
                    )),
                ));
            }
        };
        if read == 0 {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(data) = line.strip_prefix("data:") {
            current_data = Some(data.trim().to_string());
            continue;
        }
        if line.is_empty() {
            if let Some(data) = current_data.take() {
                match (wire_format.translate_sse_data)(&data, &mut accumulator) {
                    Some(GenerationProcessEvent::Completed(usage)) => {
                        round_usage = usage;
                        break;
                    }
                    Some(GenerationProcessEvent::Failed(failure)) => {
                        finish_api_invocation(
                            accounting,
                            invocation,
                            round_usage.as_ref(),
                            None,
                            UsageStatus::Failed,
                            clock,
                            logging,
                        );
                        return Err(GenerationProcessEvent::Failed(failure));
                    }
                    Some(GenerationProcessEvent::Token(text)) => {
                        let starts_new_round = assistant_text.is_empty();
                        assistant_text.push_str(&text);
                        let content_delta = if *emitted_visible_content && starts_new_round {
                            format!("\n{text}")
                        } else {
                            text
                        };
                        *emitted_visible_content = true;
                        if sink
                            .handle(GenerationProcessEvent::Token(content_delta))
                            .is_err()
                        {
                            return Err(failed_retryable(
                                "Agent generation event handling failed.",
                            ));
                        }
                    }
                    Some(event) => {
                        if sink.handle(event).is_err() {
                            return Err(failed_retryable(
                                "Agent generation event handling failed.",
                            ));
                        }
                    }
                    None => {}
                }
            }
        }
    }
    Ok(StreamedRound {
        accumulator,
        assistant_text,
        round_usage,
    })
}

/// The status/output/emit/push tail every tool-dispatch branch ends with, which each of the five
/// branches carried its own copy of. `Err` is the sink-rejection event the caller returns
/// unchanged; `Ok` is the entry the caller pushes to `executed` before continuing.
#[allow(clippy::result_large_err)]
fn record_tool_outcome(
    sink: &dyn AgentProcessEventSink,
    mut tool_use: ToolUseBlock,
    outcome: ToolExecutionOutcome,
    image: Option<AgentImage>,
) -> Result<ExecutedToolCall, GenerationProcessEvent> {
    tool_use.status = if outcome.is_error {
        "failed".to_owned()
    } else {
        "completed".to_owned()
    };
    tool_use.output = Some(Value::String(outcome.output.clone()));
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    Ok((tool_use, outcome.output, outcome.is_error, image))
}

fn skill_tool_outcome(outcome: SkillToolDispatchOutcome) -> ToolExecutionOutcome {
    match outcome {
        SkillToolDispatchOutcome::Completed(value) => ToolExecutionOutcome {
            output: value.to_string(),
            is_error: false,
        },
        SkillToolDispatchOutcome::Denied { reason } => ToolExecutionOutcome {
            output: format!("Skill tool denied: {reason}"),
            is_error: true,
        },
        SkillToolDispatchOutcome::Failed { code } => ToolExecutionOutcome {
            output: format!("Skill tool failed: {code}"),
            is_error: true,
        },
        SkillToolDispatchOutcome::Cancelled => ToolExecutionOutcome {
            output: "cancelled".to_string(),
            is_error: true,
        },
    }
}

pub(super) fn skill_tool_provenance(
    key: &crate::contexts::tooling::skill_tools::domain::SkillToolKey,
) -> SkillToolUseProvenance {
    SkillToolUseProvenance {
        skill_id: key.owner.as_str().to_string(),
        tool_id: key.tool.as_str().to_string(),
        revision: key.revision.as_str().to_string(),
        source_scope: key.source.scope.as_str().to_string(),
        workspace_path: key.source.workspace_path.clone(),
        redacted_result_summary: None,
    }
}

pub(super) fn set_skill_result_summary(tool_use: &mut ToolUseBlock, label: &str) {
    if let Some(provenance) = tool_use.skill_provenance.as_mut() {
        provenance.redacted_result_summary = Some(label.to_string());
    }
}

pub(super) fn emit_skill_tool_lifecycle(
    sink: &dyn AgentProcessEventSink,
    tool_use: &ToolUseBlock,
    phase: ToolLifecyclePhase,
) -> Result<(), AgentRuntimeApplicationError> {
    sink.handle(GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent {
        call_id: tool_use.id.clone(),
        phase,
        provider_timestamp: None,
        fidelity: crate::contexts::execution_observability::api::ExecutionFidelity::Native,
        parent_run_id: None,
        parent_trace_id: None,
        parent_span_id: None,
        delegation_id: None,
        attempt: None,
        tool_use: tool_use.clone(),
    }))
}

pub(super) struct AgentSkillToolLifecycle<'a> {
    pub(super) sink: &'a dyn AgentProcessEventSink,
    pub(super) tool_use: &'a ToolUseBlock,
}

impl SkillToolExecutionLifecyclePort for AgentSkillToolLifecycle<'_> {
    fn transition(&self, phase: SkillToolExecutionLifecyclePhase) {
        let phase = match phase {
            SkillToolExecutionLifecyclePhase::AwaitingApproval => {
                ToolLifecyclePhase::AwaitingApproval
            }
        };
        let mut tool_use = self.tool_use.clone();
        tool_use.status = "awaiting_approval".to_string();
        let _ = emit_skill_tool_lifecycle(self.sink, &tool_use, phase);
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_skill_tool(
    execution: Option<&dyn SkillToolExecutionPort>,
    key: Option<&crate::contexts::tooling::skill_tools::domain::SkillToolKey>,
    call_id: &str,
    parent_agent_id: &str,
    workspace_path: Option<&str>,
    session_id: &str,
    generation_id: &str,
    plan_mode: bool,
    input: &Value,
    cancelled: &AtomicBool,
    lifecycle: &dyn SkillToolExecutionLifecyclePort,
) -> ToolExecutionOutcome {
    let (Some(execution), Some(key)) = (execution, key) else {
        return ToolExecutionOutcome {
            output: "Skill tool is unknown or stale for this generation.".to_string(),
            is_error: true,
        };
    };
    execution
        .execute(SkillToolExecutionRequest {
            call_id,
            key,
            parent_agent_id,
            workspace_path,
            session_id,
            generation_id,
            mode: if plan_mode {
                SkillToolCatalogMode::Plan
            } else {
                SkillToolCatalogMode::Execute
            },
            input,
            cancelled,
            lifecycle,
        })
        .map(skill_tool_outcome)
        .unwrap_or_else(|error| ToolExecutionOutcome {
            output: format!("Skill tool failed: {}", error.code()),
            is_error: true,
        })
}
