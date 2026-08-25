//! Generation entry points, per-request options, summarization, streaming, and child turns.

use super::super::memory_actions::{apply_memory_actions, render_existing_manifest};
use super::super::tool_call_accumulator::ToolCallAccumulator;
use super::super::SqliteNativeToolRepository;
use super::compaction::turns_character_count;
use super::execution::execute_with_code_intelligence;
use super::invocation::{begin_api_invocation, finish_api_invocation, WireFormat};
use super::sinks::{EvidenceCountingSink, EvidenceToolCounts};
use super::{ExecutedToolCall, PendingApprovals};
use crate::contexts::agent_runtime::application::{
    AgentChatConfiguration, AgentClockPort, AgentCodeIntelligencePort, AgentCoreInstructionsPort,
    AgentLog, AgentLogLevel, AgentLoggingPort, AgentMcpToolPort, AgentMemoryPort,
    AgentPermissionPort, AgentPersonalizationSnapshotPort, AgentProcessEventSink,
    AgentRetrievalPort, AgentSkillPort, AgentWorkspaceMutationPort, ApiAgentGateway,
    ApiCredentialPort, ApiProviderConfig, ContextEngineOutcome, ContextEngineService,
    ContextQualityRecorder, ConversationHistoryPort, GenerationProcessEvent,
    GenerationProcessRequest, MemorySource, NativeToolRegistry, ReportedUsageTotals,
    ToolDefinition, ToolUseBlock, UtilityDelegationApplicationService,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE,
};
use crate::contexts::agent_runtime::domain::{
    parse_memory_actions, ContextBudget, ContextRequest, MEMORY_ACTIONS_INSTRUCTION,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::sessions::api::{SessionsApi, UsagePurpose, UsageStatus};
use crate::contexts::skill_evolution_evidence::application::{
    NativeExecutionFact, RuntimeEvidenceProjector,
};
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, FailureClass, ObservedSkillRevision, OperationClass, SafeCounts,
    SourceFidelity, TerminalOutcome,
};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolCatalogPort, SkillToolExecutionPort,
};
use serde_json::{json, Value};
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[allow(clippy::too_many_arguments)]
pub(super) fn run_generation(
    mut request: GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    history: Arc<dyn ConversationHistoryPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    skills: Arc<dyn AgentSkillPort>,
    core_instructions: Arc<dyn AgentCoreInstructionsPort>,
    memories: Arc<dyn AgentMemoryPort>,
    mcp: Arc<dyn AgentMcpToolPort>,
    permissions: Arc<dyn AgentPermissionPort>,
    retrieval: Arc<dyn AgentRetrievalPort>,
    code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
    workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    personalization: Arc<dyn AgentPersonalizationSnapshotPort>,
    context_quality: Option<Arc<ContextQualityRecorder>>,
    context_engine: Option<Arc<ContextEngineService>>,
    accounting: Option<SessionsApi>,
    native_tools: NativeToolRegistry,
    native_tool_operations: Option<Arc<SqliteNativeToolRepository>>,
    artifacts: Option<Arc<ArtifactService>>,
    native_tool_events: Option<tauri::AppHandle>,
    sink: Arc<dyn AgentProcessEventSink>,
    pending_approvals: PendingApprovals,
    evidence: RuntimeEvidenceProjector,
    utility_delegation: Option<UtilityDelegationApplicationService>,
    skill_tool_catalog: Option<Arc<dyn SkillToolCatalogPort>>,
    skill_tool_execution: Option<Arc<dyn SkillToolExecutionPort>>,
) {
    if request.agent.id == "onepiece" {
        if let Some(engine) = context_engine {
            let context_request = ContextRequest {
                session_id: request.session.id.clone(),
                turn_id: request.message_id.clone(),
                generation_id: request.operation_id.clone(),
                task: request.effective_prompt.clone(),
                workspace_ref: request.session.folder.clone(),
                explicit_refs: request
                    .file_references
                    .iter()
                    .map(|reference| reference.path.clone())
                    .collect(),
                model_capacity: Some(32_768),
            };
            let budget = ContextBudget {
                total: 32_768,
                reserved_system: 8_192,
                reserved_task: 4_096,
                reserved_recent_turns: 12_288,
                reserve: 2_048,
            };
            if let ContextEngineOutcome::Ready(projected) =
                engine.assemble(&context_request, &budget, cancelled.as_ref())
            {
                if !projected.provider_projection.is_empty() {
                    request
                        .effective_prompt
                        .push_str("\n\n<context-evidence>\n");
                    request
                        .effective_prompt
                        .push_str(&projected.provider_projection);
                    request.effective_prompt.push_str("\n</context-evidence>");
                }
            }
        }
    }
    let mut observed_skill_revisions = Vec::new();
    let counting_sink = EvidenceCountingSink::new(sink.clone());
    let terminal = execute_with_code_intelligence(
        &request,
        cancelled,
        credentials.as_ref(),
        config.as_ref(),
        history.as_ref(),
        &counting_sink,
        &pending_approvals,
        logging.as_ref(),
        clock.as_ref(),
        skills.as_ref(),
        core_instructions.as_ref(),
        memories.as_ref(),
        mcp.as_ref(),
        permissions.as_ref(),
        retrieval.as_ref(),
        code_intelligence.as_ref(),
        workspace_mutations.as_ref(),
        personalization.as_ref(),
        context_quality.as_deref(),
        utility_delegation.as_ref(),
        skill_tool_catalog.as_deref(),
        skill_tool_execution.as_deref(),
        &mut observed_skill_revisions,
        accounting.as_ref(),
        &native_tools,
        native_tool_operations.as_deref(),
        artifacts.as_deref(),
        native_tool_events.as_ref(),
    );
    project_native_outcomes(
        &evidence,
        &request,
        &terminal,
        observed_skill_revisions,
        counting_sink.counts(),
        clock.now(),
    );
    if let GenerationProcessEvent::Failed(failure) = &terminal {
        let _ = logging.record(AgentLog {
            level: AgentLogLevel::Error,
            category: "session.runtime.api".to_string(),
            message: failure.diagnostic.clone(),
            agent_id: Some(request.agent.id.clone()),
            session_id: Some(request.session.id.clone()),
            operation_id: Some(request.operation_id.clone()),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: clock.now(),
        });
    }
    let _ = sink.handle(terminal);
}

pub(super) fn project_native_outcomes(
    evidence: &RuntimeEvidenceProjector,
    request: &GenerationProcessRequest,
    terminal: &GenerationProcessEvent,
    observed_skill_revisions: Vec<ObservedSkillRevision>,
    tools: EvidenceToolCounts,
    occurred_at: String,
) {
    let (outcome, failure_class) = match terminal {
        GenerationProcessEvent::Completed(_) => (TerminalOutcome::Succeeded, None),
        GenerationProcessEvent::Failed(_) => (TerminalOutcome::Failed, Some(FailureClass::Agent)),
        _ => (TerminalOutcome::Incomplete, Some(FailureClass::Agent)),
    };
    let common = EnvelopeCommon {
        source_event_id: format!(
            "native:{}:generation",
            request.execution_context.run_id.as_str()
        ),
        occurred_at: occurred_at.clone(),
        stable_agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        message_id: Some(request.message_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        attempt_id: Some(request.operation_id.clone()),
        workspace: evidence.workspace_scope(request.session.folder.as_deref()),
        fidelity: SourceFidelity::Native,
        observed_skill_revisions,
    };
    let _ = evidence.native(NativeExecutionFact {
        common: common.clone(),
        operation_class: OperationClass::Generation,
        outcome,
        failure_class,
        safe_counts: SafeCounts {
            attempts: 1,
            failures: u32::from(outcome == TerminalOutcome::Failed),
        },
    });
    if tools.attempts > 0 {
        let mut tool_common = common;
        tool_common.source_event_id =
            format!("native:{}:tools", request.execution_context.run_id.as_str());
        let _ = evidence.native(NativeExecutionFact {
            common: tool_common,
            operation_class: OperationClass::Tool,
            outcome: if tools.failures == 0 {
                TerminalOutcome::Succeeded
            } else {
                TerminalOutcome::Failed
            },
            failure_class: (tools.failures > 0).then_some(FailureClass::Tool),
            safe_counts: SafeCounts {
                attempts: tools.attempts,
                failures: tools.failures,
            },
        });
    }
}

/// Provider-agnostic knobs from `AgentChatConfiguration` that map onto a single generation
/// request (`add-agent-chat-configuration`). Each provider's `build_request_body` reads only the
/// field(s) meaningful to its own wire format — mirrors how `WireFormat`'s other function
/// pointers already share one signature across providers with different per-provider bodies.
pub(crate) struct GenerationOptions<'a> {
    pub(crate) thinking: bool,
    pub(crate) reasoning_depth: Option<&'a str>,
    pub(crate) include_stream_usage: bool,
}

impl GenerationOptions<'_> {
    /// Used for requests that are not the user-facing turn (context compaction's own internal
    /// summarization call) — never inherits the user's turn-level settings.
    pub(crate) fn disabled() -> GenerationOptions<'static> {
        GenerationOptions {
            thinking: false,
            reasoning_depth: None,
            include_stream_usage: false,
        }
    }
}

pub(super) fn generation_options_from_configuration(
    configuration: &AgentChatConfiguration,
    include_stream_usage: bool,
) -> GenerationOptions<'_> {
    GenerationOptions {
        thinking: configuration.thinking,
        reasoning_depth: configuration.reasoning_depth.as_deref(),
        include_stream_usage,
    }
}

/// Whether the session narrows the durable Agent policy to read-only planning behavior.
pub(super) fn is_plan_mode(configuration: &AgentChatConfiguration) -> bool {
    configuration.execution_mode == "plan"
}

pub(super) fn reviewed_stream_usage_strategy(config: &ApiProviderConfig) -> bool {
    if config.interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        return false;
    }
    config.base_url.as_deref().is_some_and(|base_url| {
        [
            "https://api.openai.com/",
            "https://openrouter.ai/",
            "https://api.deepseek.com/",
        ]
        .iter()
        .any(|prefix| base_url.starts_with(prefix))
    })
}

/// Calls the model once to reduce `turns_to_summarize` to short text per `instruction`, reusing
/// the same wire-format request/response machinery as regular generation — except the full text
/// response is accumulated instead of streamed to the sink, and no tools are declared. `Ok(None)`
/// means the call completed but produced nothing (empty `turns_to_summarize`, or a blank/
/// whitespace-only response — a valid outcome, e.g. "nothing worth remembering" for extraction).
/// `Err` means the call itself didn't complete: network error, non-2xx status, cancellation, or a
/// provider-reported failure. Shared by compaction's own summarization and automatic memory
/// extraction (`extract_memories`) — both are best-effort steps that must never fail the
/// generation they're attached to, but the `Ok`/`Err` split lets `extract_memories` log only the
/// latter, matching `add-agent-cross-session-memory`'s spec.
#[allow(clippy::too_many_arguments)]
pub(crate) fn summarize_turns(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_summarize: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    max_output_tokens: Option<u32>,
) -> Result<Option<String>, String> {
    summarize_turns_with_usage(
        wire_format,
        client,
        api_key,
        model,
        system,
        turns_to_summarize,
        instruction,
        cancelled,
        max_output_tokens,
    )
    .map(|(summary, _usage)| summary)
}

#[allow(clippy::too_many_arguments)]
fn summarize_turns_with_usage(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_summarize: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    max_output_tokens: Option<u32>,
) -> Result<(Option<String>, Option<ReportedUsageTotals>), String> {
    if turns_to_summarize.is_empty() {
        return Ok((None, None));
    }
    let mut prompt_turns = turns_to_summarize.to_vec();
    prompt_turns.push(json!({ "role": "user", "content": instruction }));
    // Never inherits the user's turn-level `thinking`/`reasoning_depth` — this is an internal,
    // mechanical summarization call, not the user-facing turn (`add-agent-chat-configuration`).
    let body = (wire_format.build_request_body)(
        model,
        &prompt_turns,
        &[],
        system,
        &GenerationOptions::disabled(),
    );
    // Applied after the provider builder rather than inside it: the builders serve the main
    // generation path too, where a summarization-shaped cap would be wrong. Anthropic's builder
    // sets its own default here and this deliberately overrides it for callers that opt in;
    // callers passing `None` — compaction and extraction — are left byte-identical.
    let mut body = body;
    if let Some(limit) = max_output_tokens {
        if let Some(object) = body.as_object_mut() {
            object.insert("max_tokens".to_string(), json!(limit));
        }
    }
    let (text, _tool_calls, usage) =
        stream_completion(wire_format, client, api_key, &body, cancelled)?;
    let trimmed = text.trim();
    Ok(((!trimmed.is_empty()).then(|| trimmed.to_string()), usage))
}

/// Sends one request and drains its SSE stream into assistant text, any completed tool calls, and
/// the provider's reported usage.
///
/// Shared by the internal summarization calls (which declare no tools and discard the tool-call
/// half) and by the subagent child loop (which needs it). Extracted rather than copied because a
/// second SSE reader is a second place for the `data:`/blank-line framing, cancellation, and
/// terminal-event handling to drift.
fn stream_completion(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    body: &Value,
    cancelled: &AtomicBool,
) -> Result<(String, Vec<ToolUseBlock>, Option<ReportedUsageTotals>), String> {
    let request_builder = (wire_format.apply_auth)(client.post(&wire_format.endpoint), api_key);
    let response = request_builder
        .header("content-type", "application/json")
        .json(body)
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("received HTTP {}", response.status()));
    }

    let mut reader = std::io::BufReader::new(response);
    let mut current_data: Option<String> = None;
    let mut accumulator = ToolCallAccumulator::default();
    let mut text = String::new();
    let mut usage = None;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err("cancelled".to_string());
        }
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
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
                    Some(GenerationProcessEvent::Completed(reported)) => {
                        usage = reported;
                        break;
                    }
                    Some(GenerationProcessEvent::Failed(failure)) => return Err(failure.diagnostic),
                    Some(GenerationProcessEvent::Token(text_delta)) => text.push_str(&text_delta),
                    _ => {}
                }
            }
        }
    }

    Ok((text, accumulator.take_completed(), usage))
}

/// Builds the reply turns that carry a child's executed tool results back into its next request.
///
/// Exists so `WireFormat`'s function pointers stay private and the subagent module never has to
/// know about `ExecutedToolCall`'s image slot, which a read-only child can never fill
/// (`add-onepiece-subagents`).
pub(crate) fn child_reply_turns(
    wire_format: &WireFormat,
    assistant_text: &str,
    executed: &[(ToolUseBlock, String, bool)],
) -> Vec<Value> {
    let executed: Vec<ExecutedToolCall> = executed
        .iter()
        .map(|(call, output, is_error)| (call.clone(), output.clone(), *is_error, None))
        .collect();
    (wire_format.build_reply_turns)(assistant_text, &executed)
}

/// One turn of a subagent child loop: sends `turns` with the child's restricted tool catalog and
/// returns what the model said plus any tool calls it wants executed
/// (`add-onepiece-subagents`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_turn(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns: &[Value],
    tools: &[ToolDefinition],
    cancelled: &AtomicBool,
) -> Result<(String, Vec<ToolUseBlock>, Option<ReportedUsageTotals>), String> {
    // Never inherits the parent turn's thinking/reasoning settings: a child is an internal,
    // bounded investigation, not the user-facing turn.
    let body = (wire_format.build_request_body)(
        model,
        turns,
        tools,
        system,
        &GenerationOptions::disabled(),
    );
    stream_completion(wire_format, client, api_key, &body, cancelled)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn summarize_turns_accounted(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    config: &ApiProviderConfig,
    system: Option<&str>,
    turns: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    accounting: Option<&SessionsApi>,
    request: &GenerationProcessRequest,
    purpose: UsagePurpose,
    request_sequence: u32,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Result<Option<String>, String> {
    let invocation = begin_api_invocation(
        accounting,
        request,
        config,
        request_sequence,
        purpose,
        clock,
        logging,
    );
    let estimated_input = turns_character_count(turns).saturating_add(instruction.chars().count());
    let result = summarize_turns_with_usage(
        wire_format,
        client,
        api_key,
        &config.model_id,
        system,
        turns,
        instruction,
        cancelled,
        // Compaction summaries and extraction are unbounded here exactly as before; capping a
        // compaction summary would truncate the context it exists to preserve.
        None,
    );
    match &result {
        Ok((summary, usage)) => finish_api_invocation(
            accounting,
            invocation.as_ref(),
            usage.as_ref(),
            Some((
                estimated_input,
                summary.as_ref().map_or(0, |value| value.chars().count()),
            )),
            UsageStatus::Succeeded,
            clock,
            logging,
        ),
        Err(_) => finish_api_invocation(
            accounting,
            invocation.as_ref(),
            None,
            None,
            if cancelled.load(Ordering::SeqCst) {
                UsageStatus::Cancelled
            } else {
                UsageStatus::Failed
            },
            clock,
            logging,
        ),
    }
    result.map(|(summary, _usage)| summary)
}

/// Parses `summarize_turns`'s response as zero or more memories, one per non-empty line, and
/// saves each as `MemorySource::Automatic`. "Nothing worth remembering" (`Ok(None)`) saves
/// nothing and logs nothing — a normal, expected outcome, not a failure. An actual call failure
/// (`Err`) is logged and otherwise ignored, exactly like compaction's own summarization failure,
/// so it's visible to an operator without affecting the generation or its compaction.
#[allow(clippy::too_many_arguments)]
pub(super) fn extract_memories_accounted(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    _model: &str,
    provider_config: &ApiProviderConfig,
    system: Option<&str>,
    turns_to_extract_from: &[Value],
    cancelled: &AtomicBool,
    agent_id: &str,
    folder: Option<&str>,
    memories: &dyn AgentMemoryPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
) {
    let memory_sequence = *request_sequence;
    *request_sequence = request_sequence.saturating_add(1);
    let response = match summarize_turns_accounted(
        wire_format,
        client,
        api_key,
        provider_config,
        system,
        turns_to_extract_from,
        &memory_extraction_instruction(memories),
        cancelled,
        accounting,
        request,
        UsagePurpose::MemoryExtraction,
        memory_sequence,
        clock,
        logging,
    ) {
        Ok(Some(response)) => response,
        Ok(None) => return,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Automatic memory extraction call failed; continuing without it: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return;
        }
    };
    // A malformed response is logged and dropped, never propagated: extraction is best-effort work
    // hanging off a compaction, and the generation that triggered it must be unaffected.
    match parse_memory_actions(&response) {
        Ok(parsed) => {
            apply_memory_actions(memories, agent_id, folder, MemorySource::Automatic, &parsed);
        }
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Automatic memory extraction returned an unusable response: {error}"
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
    }
}

/// Extraction instruction plus the existing pool's manifest. Built per call because the pool
/// changes between compactions, and without it the model cannot name a memory to update.
fn memory_extraction_instruction(memories: &dyn AgentMemoryPort) -> String {
    let existing = render_existing_manifest(memories);
    if existing.trim().is_empty() {
        MEMORY_ACTIONS_INSTRUCTION.to_string()
    } else {
        format!("{MEMORY_ACTIONS_INSTRUCTION}\n\nExisting memories:\n{existing}")
    }
}
