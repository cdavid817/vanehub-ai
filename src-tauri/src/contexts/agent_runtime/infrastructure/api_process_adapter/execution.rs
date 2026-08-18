//! The tool-use loop and the skill-tool dispatch it drives.

use super::super::agent_image::MAX_IMAGES_PER_REQUEST;
use super::super::memory_selection_gateway::RuntimeAgentMemorySelectionAdapter;
use super::super::model_context_catalog;
use super::super::skill_tool_catalog_adapter::resolve_skill_tool_catalog;
use super::super::tool_call_accumulator::ToolCallAccumulator;
use super::super::tools::{execute_file_image_read, ToolExecutionOutcome};
use super::super::SqliteNativeToolRepository;
use super::compaction::{maybe_compact_accounted, should_compact, turns_character_count};
use super::generation::{
    generation_options_from_configuration, is_plan_mode, reviewed_stream_usage_strategy,
};
use super::interactive::{
    ask_user_question, await_approval, permission_action_and_resource, request_plan_exit,
    ApprovalOutcome,
};
use super::invocation::{
    begin_api_invocation, estimated_input_characters, finish_api_invocation,
    record_context_snapshot, wire_format_for,
};
use super::native_tools::{
    execute_registered_native_tool, execute_tool_call_with_runtime_ports, is_image_read_request,
    log_image_attachment, resolve_tool_image,
};
use super::prompt::{
    resolve_personalization_settings, resolve_system_prompt_with_settings,
    resolve_tool_catalog_with_code_intelligence,
};
use super::{
    failed_configuration, failed_non_retryable, failed_retryable, ExecutedToolCall,
    PendingApprovals, HISTORY_LIMIT, MAX_TOOL_ROUND_TRIPS,
};
use crate::contexts::agent_runtime::application::{
    delegate_utility_skill_tool_definition, AgentClockPort, AgentCodeIntelligenceContext,
    AgentCodeIntelligencePort, AgentCoreInstructionsPort, AgentLog, AgentLogLevel,
    AgentLoggingPort, AgentMcpToolPort, AgentMemoryPort, AgentPermissionPort,
    AgentPersonalizationPort, AgentProcessEventSink, AgentRetrievalPort,
    AgentRuntimeApplicationError, AgentSkillPort, AgentWorkspaceMutationPort, ApiAgentGateway,
    ApiCredentialPort, ApiProviderConfig, ContextAnalysisInput, ContextAnalysisService,
    ContextQualityRecorder, ConversationHistoryPort, GenerationProcessEvent,
    GenerationProcessFailure, GenerationProcessRequest, NativeToolExecutionMode,
    NativeToolRegistry, SkillToolUseProvenance, ToolEligibilityContext, ToolLifecycleEvent,
    ToolLifecyclePhase, ToolUseBlock, UtilityDelegationApplicationService,
    ASK_USER_QUESTION_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME, REMEMBER_TOOL_NAME,
};
use crate::contexts::agent_runtime::domain::{
    AutomaticCompactionState, ContextCapacity, UsageAnchor,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::permissions::domain::Effect;
use crate::contexts::sessions::api::{SessionsApi, UsagePurpose, UsageStatus};
use crate::contexts::skill_evolution_evidence::domain::ObservedSkillRevision;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolBinding, SkillToolCatalogContext, SkillToolCatalogMode, SkillToolCatalogPort,
    SkillToolDispatchOutcome, SkillToolExecutionLifecyclePhase, SkillToolExecutionLifecyclePort,
    SkillToolExecutionPort, SkillToolExecutionRequest,
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
    personalization: &dyn AgentPersonalizationPort,
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
    let provider_config = if let Some(profile) = request.endpoint_profile.as_ref() {
        ApiProviderConfig {
            source_provider_id: profile.source_provider_id.clone(),
            model_id: profile.model_id.clone(),
            interface_format: profile.interface_format.clone(),
            base_url: profile.base_url.clone(),
            auto_approve_tools: false,
        }
    } else {
        match config.provider_config(agent_id) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return failed_configuration(agent_id, "No model is configured for this agent.");
            }
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let endpoint_metadata = if request.endpoint_profile.is_some() {
        None
    } else {
        match config.active_endpoint_profile_metadata(agent_id) {
            Ok(metadata) => metadata,
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let endpoint_capacity = request
        .endpoint_profile
        .as_ref()
        .and_then(|profile| {
            let window = profile.context_window_tokens?;
            (profile.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                context_window_tokens: window,
                maximum_output_tokens: Some(profile.reserved_output_tokens),
                metadata_revision: profile.context_capacity_provenance.clone(),
                source_identity: format!("endpoint-profile:{}", profile.profile_id),
            })
        })
        .or_else(|| {
            endpoint_metadata.as_ref().and_then(|metadata| {
                let window = metadata
                    .context_window_tokens
                    .and_then(|value| u64::try_from(value).ok())?;
                (metadata.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                    context_window_tokens: window,
                    maximum_output_tokens: u64::try_from(metadata.reserved_output_tokens).ok(),
                    metadata_revision: metadata.context_capacity_provenance.clone(),
                    source_identity: format!("endpoint-profile:{}", metadata.profile_id),
                })
            })
        });
    let authentication_mode = if let Some(profile) = request.endpoint_profile.as_ref() {
        profile.authentication_mode.clone()
    } else {
        match config.api_endpoint_authentication_mode(agent_id) {
            Ok(mode) => mode,
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let credential_id = request.endpoint_profile.as_ref().map_or_else(
        || agent_id.to_string(),
        |profile| format!("onepiece-profile:{}", profile.profile_id),
    );
    let fetched_credential = if authentication_mode == "required" {
        match credentials.fetch(&credential_id) {
            Ok(Some(key)) => Ok(Some(key)),
            Ok(None) if request.endpoint_profile.is_some() => credentials.fetch(agent_id),
            other => other,
        }
    } else {
        Ok(None)
    };
    let api_key = match fetched_credential {
        Ok(Some(key)) => key,
        Ok(None) if authentication_mode != "required" => String::new(),
        Ok(None) => {
            return failed_configuration(agent_id, "No API key is stored for this agent.");
        }
        Err(error) => return failed_non_retryable(&error.to_string()),
    };
    let wire_format = match wire_format_for(&provider_config) {
        Ok(wire_format) => wire_format,
        Err(message) => return failed_configuration(agent_id, message),
    };
    let generation_personalization =
        resolve_personalization_settings(personalization, logging, clock, request);
    // Built here, and the prompt resolved once, before the round-trip loop below. That is what
    // makes the system prompt byte-identical across every round trip of this generation.
    let selection = RuntimeAgentMemorySelectionAdapter::new(credentials, config);
    let system = resolve_system_prompt_with_settings(
        agent_id,
        core_instructions,
        &generation_personalization,
        skills,
        memories,
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
    // Signal for the tool-assisted memory-extraction gate (`add-personalization-settings` design.md
    // D5) — seeded from the persisted message history, not from wire-format `turns`, so it needs no
    // per-provider parsing and no index-alignment with whatever `maybe_compact` later slices off
    // `turns`. Mutable: this generation's own tool round trips (below) can still flip it from
    // `false` to `true` before the in-loop `maybe_compact` call — seeding it from `recent` alone
    // would miss a session's very first tool call if compaction also triggers within that same
    // generation.
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
    // Never blocks, never errors (`AgentRetrievalPort::is_configured`'s own contract) — safe to
    // call unconditionally on every generation's catalog resolution, matching how `plan_mode`
    // itself is derived just above.
    let retrieval_available = retrieval.is_configured();
    let code_search_available = request
        .session
        .folder
        .as_deref()
        .and_then(|folder| {
            retrieval
                .code_retrieval()
                .map(|code| code.is_available(folder))
        })
        .unwrap_or(false);
    let code_intelligence_context = request
        .session
        .folder
        .as_deref()
        .map(AgentCodeIntelligenceContext::from_session_workspace);
    let code_intelligence_available = code_intelligence_context
        .as_ref()
        .is_some_and(|context| code_intelligence.is_available(context));
    let mut tools = resolve_tool_catalog_with_code_intelligence(
        request,
        mcp,
        logging,
        clock,
        plan_mode,
        retrieval_available,
        code_search_available,
        code_intelligence_available,
    );
    if utility_delegation.is_some() && !plan_mode {
        tools.push(delegate_utility_skill_tool_definition());
    }
    tools.extend(
        native_tools.eligible_tool_definitions(&ToolEligibilityContext {
            agent_id: request.agent.id.clone(),
            session_id: request.session.id.clone(),
            generation_id: request.operation_id.clone(),
            canonical_workspace: request.session.folder.as_deref().map(Into::into),
            execution_mode: if plan_mode {
                NativeToolExecutionMode::Plan
            } else {
                NativeToolExecutionMode::Execute
            },
            readiness: native_tools.readiness_snapshot(),
        }),
    );
    if request
        .endpoint_profile
        .as_ref()
        .is_some_and(|profile| profile.tool_calling_capability != "supported")
    {
        tools.clear();
    }
    let mut skill_tool_keys = HashMap::new();
    let mut _skill_tool_catalog_lease = None;
    let mut _skill_tool_catalog_generation = None;
    if let Some(catalog) = skill_tool_catalog {
        let loaded_roles = observed_skill_revisions
            .iter()
            .map(|observed| SkillToolBinding {
                skill_id: observed.skill_id.clone(),
                revision: observed.revision.clone(),
            })
            .collect::<Vec<_>>();
        let context = SkillToolCatalogContext::RoleGeneration {
            workspace_path: request.session.folder.clone(),
            loaded_roles,
            mode: if plan_mode {
                SkillToolCatalogMode::Plan
            } else {
                SkillToolCatalogMode::Execute
            },
        };
        let existing_names = tools.iter().map(|tool| tool.name.clone());
        match resolve_skill_tool_catalog(
            catalog,
            &context,
            existing_names,
            &provider_config.interface_format,
        ) {
            Ok(resolved) => {
                tools.extend(resolved.definitions);
                skill_tool_keys = resolved.keys_by_name;
                _skill_tool_catalog_generation = Some(resolved.generation);
                _skill_tool_catalog_lease = Some(resolved.lease);
            }
            Err(error) => {
                let _ = logging.record(AgentLog {
                    level: AgentLogLevel::Warn,
                    category: "session.runtime.api.skill-tools".to_string(),
                    message: format!("Skill tool catalog rejected: {}", error.code()),
                    agent_id: Some(request.agent.id.clone()),
                    session_id: Some(request.session.id.clone()),
                    operation_id: Some(request.operation_id.clone()),
                    run_id: None,
                    trace_id: None,
                    span_id: None,
                    occurred_at: clock.now(),
                });
            }
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
        personalization,
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
    // Capability comes from reviewed catalog metadata, never from trying and seeing: a provider
    // that rejects an image-bearing request fails the whole generation after the user has already
    // waited, and the failure text varies by vendor (`add-agent-image-input` D3).
    let images_supported = request.endpoint_profile.as_ref().map_or_else(
        || {
            endpoint_metadata.as_ref().map_or_else(
                || {
                    model_context_catalog::accepts_image_input(
                        provider_config.source_provider_id.as_deref(),
                        &provider_config.model_id,
                    )
                },
                |metadata| metadata.image_input_capability == "supported",
            )
        },
        |profile| profile.image_input_capability == "supported",
    );
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
        let projection = (wire_format.project_request_context)(&body);
        let mut context_snapshot = ContextAnalysisService::analyze(
            ContextAnalysisInput {
                provider_id: provider_config.source_provider_id.clone(),
                model_id: provider_config.model_id.clone(),
                request_fingerprint: projection.request_fingerprint,
                characters: projection.characters,
                components: projection.components,
                rounds: projection.rounds,
                token_estimate_complete: projection.token_estimate_complete,
                capacity: endpoint_capacity.clone().or_else(|| {
                    (endpoint_metadata.is_none() && request.endpoint_profile.is_none()).then(
                        || {
                            model_context_catalog::resolve_capacity(
                                provider_config.source_provider_id.as_deref(),
                                &provider_config.model_id,
                            )
                        },
                    )?
                }),
                active_character_compaction: should_compact(turns_character_count(&turns)),
                invocation_sequence: sequence,
                overflow_count: projection.overflow_count,
            },
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

        let mut reader = std::io::BufReader::new(response);
        let mut current_data: Option<String> = None;
        let mut accumulator = ToolCallAccumulator::default();
        let mut assistant_text = String::new();
        let mut round_usage = None;
        loop {
            if cancelled.load(Ordering::SeqCst) {
                finish_api_invocation(
                    accounting,
                    invocation.as_ref(),
                    round_usage.as_ref(),
                    None,
                    UsageStatus::Cancelled,
                    clock,
                    logging,
                );
                return failed_non_retryable("Generation was cancelled.");
            }
            let mut line = String::new();
            let read = match reader.read_line(&mut line) {
                Ok(read) => read,
                Err(error) => {
                    finish_api_invocation(
                        accounting,
                        invocation.as_ref(),
                        round_usage.as_ref(),
                        None,
                        UsageStatus::Failed,
                        clock,
                        logging,
                    );
                    return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                        format!("Failed to read the provider API response: {error}"),
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
                                invocation.as_ref(),
                                round_usage.as_ref(),
                                None,
                                UsageStatus::Failed,
                                clock,
                                logging,
                            );
                            return GenerationProcessEvent::Failed(failure);
                        }
                        Some(GenerationProcessEvent::Token(text)) => {
                            let starts_new_round = assistant_text.is_empty();
                            assistant_text.push_str(&text);
                            let content_delta = if emitted_visible_content && starts_new_round {
                                format!("\n{text}")
                            } else {
                                text
                            };
                            emitted_visible_content = true;
                            if sink
                                .handle(GenerationProcessEvent::Token(content_delta))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                        }
                        Some(event) => {
                            if sink.handle(event).is_err() {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                        }
                        None => {}
                    }
                }
            }
        }

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
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, image));
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
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, image));
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
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
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
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
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
            let (permission_action, permission_resource) =
                permission_action_and_resource(&tool_use.name, &input);
            let project_key = request.session.folder.as_deref().unwrap_or("");
            let effect = permissions.evaluate(
                agent_id,
                permission_action.clone(),
                permission_resource.clone(),
                &request.session.id,
                &request.operation_id,
                project_key,
            );
            match effect {
                Effect::Allow => {}
                Effect::Deny => {
                    let denial = "Denied by policy.".to_string();
                    tool_use.status = "failed".to_string();
                    tool_use.output = Some(Value::String(denial.clone()));
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return failed_retryable("Agent generation event handling failed.");
                    }
                    executed.push((tool_use, denial, true, None));
                    continue;
                }
                Effect::Ask => {
                    tool_use.status = "awaiting_approval".to_string();
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return failed_retryable("Agent generation event handling failed.");
                    }
                    if let Err(error) = permissions.create_pending_approval(
                        agent_id,
                        permission_action,
                        permission_resource,
                        &request.session.id,
                        &request.operation_id,
                        &tool_use.id,
                        project_key,
                    ) {
                        return failed_non_retryable(&error.to_string());
                    }
                    match await_approval(&tool_use.id, &cancelled, pending_approvals) {
                        ApprovalOutcome::Approved => {}
                        ApprovalOutcome::Denied => {
                            let denial = "Denied by user.".to_string();
                            tool_use.status = "failed".to_string();
                            tool_use.output = Some(Value::String(denial.clone()));
                            if sink
                                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                            executed.push((tool_use, denial, true, None));
                            continue;
                        }
                        ApprovalOutcome::Cancelled => {
                            return failed_non_retryable(
                                "Generation was cancelled while a tool call was awaiting approval.",
                            );
                        }
                        // An answer delivered to a call that asked for permission means the two
                        // resolutions were crossed; fail closed rather than treat it as consent.
                        ApprovalOutcome::Answered(_) => {
                            let denial = "Denied by user.".to_string();
                            tool_use.status = "failed".to_string();
                            tool_use.output = Some(Value::String(denial.clone()));
                            if sink
                                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                            executed.push((tool_use, denial, true, None));
                            continue;
                        }
                    }
                }
            }
            tool_use.status = "running".to_string();
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return failed_retryable("Agent generation event handling failed.");
            }
            let outcome = if tool_use.name == REMEMBER_TOOL_NAME
                && !resolve_personalization_settings(personalization, logging, clock, request)
                    .memory_enabled
            {
                // Memory master switch off (`add-personalization-settings`) — reject before
                // dispatching, matching `execute_remember`'s own empty-content rejection shape, so
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
            tool_use.status = if outcome.is_error {
                "failed".to_string()
            } else {
                "completed".to_string()
            };
            tool_use.output = Some(Value::String(outcome.output.clone()));
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return failed_retryable("Agent generation event handling failed.");
            }
            executed.push((tool_use, outcome.output, outcome.is_error, None));
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
            personalization,
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
