use super::tool_call_accumulator::ToolCallAccumulator;
use super::tools::{execute_file, execute_shell, ToolExecutionOutcome};
use super::{anthropic_provider, openai_compatible_provider};
use crate::contexts::agent_runtime::application::{
    risk_tier_for, tool_catalog, AgentClockPort, AgentLog, AgentLogLevel, AgentLoggingPort,
    AgentMemory, AgentMemoryPort, AgentMessage, AgentProcessEventSink, AgentProcessGateway,
    AgentRuntimeApplicationError, AgentSkillPort, ApiAgentGateway, ApiCredentialPort,
    ApiProviderConfig, BoundSkillPrompt, ConversationHistoryPort, GenerationProcessEvent,
    GenerationProcessFailure, GenerationProcessRequest, MemorySource, ProcessStopInitiator,
    StartedGenerationProcess, ToolApprovalDecision, ToolApprovalPort, ToolDefinition, ToolRiskTier,
    ToolUseBlock, WorkflowLaunchOutcome, WorkflowLaunchRequest, FILE_TOOL_NAME,
    INTERFACE_FORMAT_OPENAI_COMPATIBLE, REMEMBER_TOOL_NAME, SHELL_TOOL_NAME,
};
use crate::platform::network::blocking_http_client;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::BufRead;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

const HISTORY_LIMIT: i64 = 50;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MESSAGES_ENDPOINT: &str = "https://api.anthropic.com/v1/messages";
const MAX_TOOL_ROUND_TRIPS: u32 = 25;
const APPROVAL_POLL_INTERVAL: Duration = Duration::from_millis(200);
/// A conservative proxy for "the turns list is getting large enough to risk exceeding the
/// model's real context window" — character count, not a real token count, matching the
/// codebase's existing `source: "character-count"` approximation used for usage accounting.
/// ~60,000 characters stays comfortably under typical 128K-token context windows even at a
/// pessimistic ~1 char/token ratio, leaving headroom for system/tool-definition overhead and the
/// response's own `DEFAULT_MAX_TOKENS`.
const COMPACTION_TRIGGER_CHARACTERS: usize = 60_000;
/// How many of the most recent turns stay untouched (verbatim) when compaction triggers;
/// everything older is replaced by one synthetic summary turn.
const COMPACTION_KEEP_RECENT_TURNS: usize = 6;
const SUMMARIZATION_INSTRUCTION: &str = "Summarize the conversation above concisely for your own future reference. Preserve key facts, decisions, and any outstanding tasks. Respond with only the summary text, no preamble.";
/// Deliberately asks for one fact per line with no numbering/bullets/preamble, since the
/// response is parsed by splitting on newlines (`extract_memories`) rather than an additional
/// structured-output round trip.
const MEMORY_EXTRACTION_INSTRUCTION: &str = "Review the conversation above for any facts, decisions, or preferences worth remembering in future, separate sessions working on this same project. Respond with one per line, plain text, no numbering, bullets, or preamble. If nothing is worth remembering, respond with nothing at all.";
/// Conservative bound on injected memory content, well under `COMPACTION_TRIGGER_CHARACTERS` —
/// memories share the same system prompt as Skills and, unlike a turn, are never eligible for
/// compaction, so they must not by themselves risk crowding out the context window.
const MEMORY_INJECTION_CHARACTER_BUDGET: usize = 4_000;

type PendingApprovals = Arc<Mutex<HashMap<String, mpsc::Sender<ToolApprovalDecision>>>>;
/// A tool call's block, its output text, and whether execution failed — the shape both wire
/// formats need to build a reply turn from.
type ExecutedToolCall = (ToolUseBlock, String, bool);

/// `AgentProcessGateway` implementation for `launch_kind = "api"` agents: no subprocess is
/// spawned, generation is a direct streaming HTTP call to the provider's Messages API.
#[derive(Clone)]
pub(crate) struct RuntimeAgentApiAdapter {
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    history: Arc<dyn ConversationHistoryPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    skills: Arc<dyn AgentSkillPort>,
    memories: Arc<dyn AgentMemoryPort>,
    generations: Arc<Mutex<HashMap<String, ManagedApiGeneration>>>,
    ids: Arc<AtomicU64>,
}

struct ManagedApiGeneration {
    request: GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    pending_approvals: PendingApprovals,
}

impl RuntimeAgentApiAdapter {
    pub(crate) fn new(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
        history: Arc<dyn ConversationHistoryPort>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        skills: Arc<dyn AgentSkillPort>,
        memories: Arc<dyn AgentMemoryPort>,
    ) -> Self {
        Self {
            credentials,
            config,
            history,
            logging,
            clock,
            skills,
            memories,
            generations: Arc::new(Mutex::new(HashMap::new())),
            ids: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl AgentProcessGateway for RuntimeAgentApiAdapter {
    fn launch_workflow(
        &self,
        _request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        Ok(WorkflowLaunchOutcome {
            adapter: "api".to_string(),
            message: "API-based agent workflow ready.".to_string(),
        })
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        if request.agent.launch.kind != "api" {
            return Err(AgentRuntimeApplicationError::Process(format!(
                "{} launch kind '{}' is unsupported for the API runtime.",
                request.agent.display_name, request.agent.launch.kind
            )));
        }
        let process_id = format!(
            "agent-api-process-{}",
            self.ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        generations.insert(
            process_id.clone(),
            ManagedApiGeneration {
                request,
                cancelled: Arc::new(AtomicBool::new(false)),
                pending_approvals: Arc::new(Mutex::new(HashMap::new())),
            },
        );
        Ok(StartedGenerationProcess { process_id })
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let (request, cancelled, pending_approvals) = {
            let generations = self
                .generations
                .lock()
                .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
            let managed = generations.get(process_id).ok_or_else(|| {
                AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is not active."
                ))
            })?;
            (
                managed.request.clone(),
                managed.cancelled.clone(),
                managed.pending_approvals.clone(),
            )
        };
        let generations = self.generations.clone();
        let process_id = process_id.to_string();
        let credentials = self.credentials.clone();
        let config = self.config.clone();
        let history = self.history.clone();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let skills = self.skills.clone();
        let memories = self.memories.clone();
        thread::spawn(move || {
            run_generation(
                request,
                cancelled,
                credentials,
                config,
                history,
                logging,
                clock,
                skills,
                memories,
                sink,
                pending_approvals,
            );
            if let Ok(mut generations) = generations.lock() {
                generations.remove(&process_id);
            }
        });
        Ok(())
    }

    fn stop_generation(
        &self,
        process_id: &str,
        _initiator: ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        match generations.get(process_id) {
            Some(managed) => {
                managed.cancelled.store(true, Ordering::SeqCst);
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl ToolApprovalPort for RuntimeAgentApiAdapter {
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let Some(managed) = generations.get(process_id) else {
            return Ok(false);
        };
        let mut pending = managed
            .pending_approvals
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        match pending.remove(call_id) {
            Some(sender) => {
                let _ = sender.send(decision);
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_generation(
    request: GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    history: Arc<dyn ConversationHistoryPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    skills: Arc<dyn AgentSkillPort>,
    memories: Arc<dyn AgentMemoryPort>,
    sink: Arc<dyn AgentProcessEventSink>,
    pending_approvals: PendingApprovals,
) {
    let terminal = execute(
        &request,
        cancelled,
        credentials.as_ref(),
        config.as_ref(),
        history.as_ref(),
        sink.as_ref(),
        &pending_approvals,
        logging.as_ref(),
        clock.as_ref(),
        skills.as_ref(),
        memories.as_ref(),
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

/// The wire-protocol-specific pieces `execute` needs: where to send the request, what body to
/// build, how to authenticate, and how to translate the response and build tool-reply turns.
/// Selected once per generation from the agent's `interface_format`; everything else in
/// `execute` — the tool-use loop, risk-tiered approval, and sandboxed tool execution — is
/// format-agnostic.
struct WireFormat {
    endpoint: String,
    history_to_turns: fn(&[AgentMessage]) -> Vec<Value>,
    build_request_body: fn(&str, &[Value], &[ToolDefinition], Option<&str>) -> Value,
    translate_sse_data: fn(&str, &mut ToolCallAccumulator) -> Option<GenerationProcessEvent>,
    build_reply_turns: fn(&str, &[ExecutedToolCall]) -> Vec<Value>,
    failure_from_http_status: fn(u16, &str) -> GenerationProcessFailure,
    apply_auth: fn(reqwest::blocking::RequestBuilder, &str) -> reqwest::blocking::RequestBuilder,
}

/// `Err` carries a plain diagnostic message rather than `GenerationProcessEvent` — that enum
/// has a large `ToolLifecycle`/`RichBlock`-sized variant, and this function's only failure case
/// is a short, statically-known string, so the caller wraps it into a `Failed` event itself.
fn wire_format_for(config: &ApiProviderConfig) -> Result<WireFormat, &'static str> {
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
            translate_sse_data: openai_compatible_provider::translate_sse_data,
            build_reply_turns: openai_compatible_provider::build_reply_turns,
            failure_from_http_status: openai_compatible_provider::failure_from_http_status,
            apply_auth: |builder, api_key| {
                builder.header("Authorization", format!("Bearer {api_key}"))
            },
        })
    } else {
        Ok(WireFormat {
            endpoint: MESSAGES_ENDPOINT.to_string(),
            history_to_turns: anthropic_provider::history_to_turns,
            build_request_body: anthropic_provider::build_request_body,
            translate_sse_data: anthropic_provider::translate_sse_data,
            build_reply_turns: anthropic_provider::build_reply_turns,
            failure_from_http_status: anthropic_provider::failure_from_http_status,
            apply_auth: |builder, api_key| {
                builder
                    .header("x-api-key", api_key)
                    .header("anthropic-version", anthropic_provider::ANTHROPIC_VERSION)
            },
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn execute(
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
    memories: &dyn AgentMemoryPort,
) -> GenerationProcessEvent {
    let agent_id = request.agent.id.as_str();
    let api_key = match credentials.fetch(agent_id) {
        Ok(Some(key)) => key,
        Ok(None) => {
            return failed_non_retryable("No API key is stored for this agent.");
        }
        Err(error) => return failed_non_retryable(&error.to_string()),
    };
    let provider_config = match config.provider_config(agent_id) {
        Ok(Some(config)) => config,
        Ok(None) => return failed_non_retryable("No model is configured for this agent."),
        Err(error) => return failed_non_retryable(&error.to_string()),
    };
    let wire_format = match wire_format_for(&provider_config) {
        Ok(wire_format) => wire_format,
        Err(message) => return failed_non_retryable(message),
    };
    let system = resolve_system_prompt(agent_id, skills, memories, logging, clock, request);
    let recent = match history.recent_messages(&request.session.id, HISTORY_LIMIT) {
        Ok(messages) => messages,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    let client = match blocking_http_client(REQUEST_TIMEOUT) {
        Ok(client) => client,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    let tools = tool_catalog();
    let mut turns = (wire_format.history_to_turns)(&recent);
    if let Some(failure) = maybe_compact(
        &mut turns,
        &wire_format,
        &client,
        &api_key,
        &provider_config.model_id,
        system.as_deref(),
        &cancelled,
        sink,
        logging,
        clock,
        request,
        memories,
    ) {
        return failure;
    }

    for _round_trip in 0..MAX_TOOL_ROUND_TRIPS {
        if cancelled.load(Ordering::SeqCst) {
            return failed_non_retryable("Generation was cancelled.");
        }
        let body = (wire_format.build_request_body)(
            &provider_config.model_id,
            &turns,
            &tools,
            system.as_deref(),
        );
        let request_builder =
            (wire_format.apply_auth)(client.post(&wire_format.endpoint), &api_key);
        let response = match request_builder
            .header("content-type", "application/json")
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                    error.to_string(),
                ))
            }
        };
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            return GenerationProcessEvent::Failed((wire_format.failure_from_http_status)(
                status.as_u16(),
                &body_text,
            ));
        }

        let mut reader = std::io::BufReader::new(response);
        let mut current_data: Option<String> = None;
        let mut accumulator = ToolCallAccumulator::default();
        let mut assistant_text = String::new();
        loop {
            if cancelled.load(Ordering::SeqCst) {
                return failed_non_retryable("Generation was cancelled.");
            }
            let mut line = String::new();
            let read = match reader.read_line(&mut line) {
                Ok(read) => read,
                Err(error) => {
                    return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                        format!("Failed to read the provider API response: {error}"),
                    ))
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
                        Some(GenerationProcessEvent::Completed(_)) => break,
                        Some(GenerationProcessEvent::Failed(failure)) => {
                            return GenerationProcessEvent::Failed(failure)
                        }
                        Some(GenerationProcessEvent::Token(text)) => {
                            assistant_text.push_str(&text);
                            if sink.handle(GenerationProcessEvent::Token(text)).is_err() {
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
            if risk_tier_for(&tool_use.name, &input) == ToolRiskTier::RequiresApproval {
                tool_use.status = "awaiting_approval".to_string();
                if sink
                    .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                    .is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
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
                        executed.push((tool_use, denial, true));
                        continue;
                    }
                    ApprovalOutcome::Cancelled => {
                        return failed_non_retryable(
                            "Generation was cancelled while a tool call was awaiting approval.",
                        );
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
            let outcome = execute_tool_call(
                &tool_use.name,
                &input,
                request.session.folder.as_deref(),
                cancelled.clone(),
                agent_id,
                memories,
            );
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
            executed.push((tool_use, outcome.output, outcome.is_error));
        }

        turns.extend((wire_format.build_reply_turns)(&assistant_text, &executed));
        if let Some(failure) = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            &api_key,
            &provider_config.model_id,
            system.as_deref(),
            &cancelled,
            sink,
            logging,
            clock,
            request,
            memories,
        ) {
            return failure;
        }
    }

    failed_non_retryable("Tool-use loop exceeded the maximum number of round trips.")
}

/// Sums the length of every string value reachable within `turns`, recursively — a
/// wire-format-agnostic proxy for how large a turns list is. Both wire formats nest large
/// content (tool results, tool-call arguments) inside arrays/objects rather than as a flat
/// `content` string, so a shallow field-only count would miss exactly the payloads (e.g.
/// file-read tool output) that motivate compaction in the first place.
fn turns_character_count(turns: &[Value]) -> usize {
    turns.iter().map(value_character_count).sum()
}

fn value_character_count(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.iter().map(value_character_count).sum(),
        Value::Object(map) => map.values().map(value_character_count).sum(),
        _ => 0,
    }
}

fn should_compact(character_count: usize) -> bool {
    character_count > COMPACTION_TRIGGER_CHARACTERS
}

fn compaction_notice_block(message_id: &str, turns_before: usize) -> Value {
    json!({
        "id": format!("compaction-{message_id}-{turns_before}"),
        "kind": "card",
        "v": 1,
        "title": "Conversation compacted",
        "bodyMarkdown": "Earlier turns in this conversation were summarized to stay within the model's context window.",
        "tone": "info",
    })
}

/// Resolves the agent's bound, enabled Skills (`add-agent-skill-support`) and stored memories
/// scoped to `(agent_id, request.session.folder)` (`add-agent-cross-session-memory`) into one
/// system-prompt string, or `None` if both are empty. Neither source can fail the generation on
/// lookup error — each logs its own warning and falls back to contributing nothing, matching
/// context compaction's own established best-effort-enhancement philosophy (design.md Decision 3
/// in `add-agent-skill-support`).
fn resolve_system_prompt(
    agent_id: &str,
    skills: &dyn AgentSkillPort,
    memories: &dyn AgentMemoryPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> Option<String> {
    let skill_section = match skills.bound_skill_prompts(agent_id) {
        Ok(prompts) if prompts.is_empty() => None,
        Ok(prompts) => Some(format_system_prompt(&prompts)),
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skills".to_string(),
                message: format!(
                    "Failed to resolve bound Skills; continuing without them in the system prompt: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    let memory_section = match memories.list(agent_id, request.session.folder.as_deref()) {
        Ok(scoped_memories) => format_memory_section(&scoped_memories),
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Failed to resolve stored memories; continuing without them in the system prompt: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    let sections: Vec<String> = [skill_section, memory_section]
        .into_iter()
        .flatten()
        .collect();
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

fn format_system_prompt(prompts: &[BoundSkillPrompt]) -> String {
    prompts
        .iter()
        .map(|prompt| format!("## {}\n{}", prompt.name, prompt.body))
        .collect::<Vec<_>>()
        .join("\n\n")
}

/// Formats `memories` (already recency-ordered — most recent first — by the port's `list`
/// contract) as one `## Memory` section, one bullet per memory, greedily included up to
/// `MEMORY_INJECTION_CHARACTER_BUDGET`. An individual memory too large to fit is skipped rather
/// than stopping the whole pass, so one oversized entry can't crowd out every smaller, older one
/// behind it. Returns `None` when there are no memories or none fit — a bounded substitute for
/// real retrieval (design.md defers vector search/embeddings unless this proves inadequate).
fn format_memory_section(memories: &[AgentMemory]) -> Option<String> {
    let mut budget = MEMORY_INJECTION_CHARACTER_BUDGET;
    let mut lines = Vec::new();
    for memory in memories {
        let line = format!("- {}", memory.content);
        let line_length = line.chars().count();
        if line_length > budget {
            continue;
        }
        budget -= line_length;
        lines.push(line);
    }
    if lines.is_empty() {
        None
    } else {
        Some(format!("## Memory\n{}", lines.join("\n")))
    }
}

/// If `turns`' accumulated size crosses the trigger threshold, replaces everything except the
/// most recent `COMPACTION_KEEP_RECENT_TURNS` turns with one synthetic summary turn and emits a
/// visible `RichBlock` notice. Leaves `turns` untouched when below threshold, when there's
/// nothing old enough to summarize, or when the summarization call itself fails — a failed
/// summarization attempt falls back to sending the request uncompacted rather than breaking the
/// generation. `system`, if present, is forwarded to the summarization call itself (it must never
/// be written into `turns`, or it would be eligible to be summarized away — see design.md
/// Decision 2 in `add-agent-skill-support`).
#[allow(clippy::too_many_arguments)]
fn maybe_compact(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    cancelled: &AtomicBool,
    sink: &dyn AgentProcessEventSink,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    memories: &dyn AgentMemoryPort,
) -> Option<GenerationProcessEvent> {
    if !should_compact(turns_character_count(turns)) || turns.len() <= COMPACTION_KEEP_RECENT_TURNS
    {
        return None;
    }
    let split_at = turns.len() - COMPACTION_KEEP_RECENT_TURNS;
    let turns_before = turns.len();
    let summary = match summarize_turns(
        wire_format,
        client,
        api_key,
        model,
        system,
        &turns[..split_at],
        SUMMARIZATION_INSTRUCTION,
        cancelled,
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
            return None;
        }
    };
    // Piggybacks on compaction's own trigger as a cost/latency control (design.md) — runs only
    // when the session has already gotten substantial enough to compact, using the identical
    // pre-mutation slice.
    extract_memories(
        wire_format,
        client,
        api_key,
        model,
        system,
        &turns[..split_at],
        cancelled,
        request.agent.id.as_str(),
        request.session.folder.as_deref(),
        memories,
        logging,
        clock,
        request,
    );
    let mut compacted = vec![json!({ "role": "user", "content": summary })];
    compacted.extend(turns.split_off(split_at));
    *turns = compacted;
    if sink
        .handle(GenerationProcessEvent::RichBlock(compaction_notice_block(
            &request.message_id,
            turns_before,
        )))
        .is_err()
    {
        return Some(failed_retryable("Agent generation event handling failed."));
    }
    None
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
fn summarize_turns(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_summarize: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
) -> Result<Option<String>, String> {
    if turns_to_summarize.is_empty() {
        return Ok(None);
    }
    let mut prompt_turns = turns_to_summarize.to_vec();
    prompt_turns.push(json!({ "role": "user", "content": instruction }));
    let body = (wire_format.build_request_body)(model, &prompt_turns, &[], system);
    let request_builder = (wire_format.apply_auth)(client.post(&wire_format.endpoint), api_key);
    let response = request_builder
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("received HTTP {}", response.status()));
    }

    let mut reader = std::io::BufReader::new(response);
    let mut current_data: Option<String> = None;
    let mut accumulator = ToolCallAccumulator::default();
    let mut summary = String::new();
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
                    Some(GenerationProcessEvent::Completed(_)) => break,
                    Some(GenerationProcessEvent::Failed(failure)) => return Err(failure.diagnostic),
                    Some(GenerationProcessEvent::Token(text)) => summary.push_str(&text),
                    _ => {}
                }
            }
        }
    }

    let trimmed = summary.trim();
    Ok((!trimmed.is_empty()).then(|| trimmed.to_string()))
}

/// Parses `summarize_turns`'s response as zero or more memories, one per non-empty line, and
/// saves each as `MemorySource::Automatic`. "Nothing worth remembering" (`Ok(None)`) saves
/// nothing and logs nothing — a normal, expected outcome, not a failure. An actual call failure
/// (`Err`) is logged and otherwise ignored, exactly like compaction's own summarization failure,
/// so it's visible to an operator without affecting the generation or its compaction.
#[allow(clippy::too_many_arguments)]
fn extract_memories(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_extract_from: &[Value],
    cancelled: &AtomicBool,
    agent_id: &str,
    folder: Option<&str>,
    memories: &dyn AgentMemoryPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) {
    let response = match summarize_turns(
        wire_format,
        client,
        api_key,
        model,
        system,
        turns_to_extract_from,
        MEMORY_EXTRACTION_INSTRUCTION,
        cancelled,
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
    for line in response.lines() {
        let line = line.trim();
        if !line.is_empty() {
            let _ = memories.save(agent_id, folder, line, MemorySource::Automatic);
        }
    }
}

enum ApprovalOutcome {
    Approved,
    Denied,
    Cancelled,
}

fn await_approval(
    call_id: &str,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
) -> ApprovalOutcome {
    let rx = {
        let (tx, rx) = mpsc::channel();
        match pending_approvals.lock() {
            Ok(mut pending) => {
                pending.insert(call_id.to_string(), tx);
            }
            Err(_) => return ApprovalOutcome::Cancelled,
        }
        rx
    };
    loop {
        if cancelled.load(Ordering::SeqCst) {
            if let Ok(mut pending) = pending_approvals.lock() {
                pending.remove(call_id);
            }
            return ApprovalOutcome::Cancelled;
        }
        match rx.recv_timeout(APPROVAL_POLL_INTERVAL) {
            Ok(ToolApprovalDecision::Approved) => return ApprovalOutcome::Approved,
            Ok(ToolApprovalDecision::Denied) => return ApprovalOutcome::Denied,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return ApprovalOutcome::Cancelled,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_tool_call(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    cancelled: Arc<AtomicBool>,
    agent_id: &str,
    memories: &dyn AgentMemoryPort,
) -> ToolExecutionOutcome {
    // `remember` has no dependency on a workspace folder — unlike shell/file, it only ever
    // touches this app's own storage — so it's handled before the workspace-folder gate below,
    // and a folder-less session can still save agent-global memories (`add-agent-cross-session-memory`).
    if name == REMEMBER_TOOL_NAME {
        return execute_remember(input, agent_id, workspace_folder, memories);
    }
    let Some(folder) = workspace_folder else {
        return ToolExecutionOutcome {
            output: "This session has no workspace folder configured.".to_string(),
            is_error: true,
        };
    };
    match name {
        SHELL_TOOL_NAME => {
            let command = input
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default();
            execute_shell(command, folder, cancelled)
        }
        FILE_TOOL_NAME => {
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let content = input.get("content").and_then(Value::as_str);
            execute_file(operation, path, content, folder)
        }
        other => ToolExecutionOutcome {
            output: format!("Unknown tool \"{other}\"."),
            is_error: true,
        },
    }
}

fn execute_remember(
    input: &Value,
    agent_id: &str,
    folder: Option<&str>,
    memories: &dyn AgentMemoryPort,
) -> ToolExecutionOutcome {
    let content = input
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if content.is_empty() {
        return ToolExecutionOutcome {
            output: "No content was provided to remember.".to_string(),
            is_error: true,
        };
    }
    match memories.save(agent_id, folder, content, MemorySource::Explicit) {
        Ok(()) => ToolExecutionOutcome {
            output: "Saved.".to_string(),
            is_error: false,
        },
        Err(error) => ToolExecutionOutcome {
            output: format!("Failed to save memory: {error}"),
            is_error: true,
        },
    }
}

fn failed_non_retryable(message: &str) -> GenerationProcessEvent {
    GenerationProcessEvent::Failed(GenerationProcessFailure::non_retryable(message.to_string()))
}

fn failed_retryable(message: &str) -> GenerationProcessEvent {
    GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(message.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        AgentChatConfiguration, AgentLaunchView, AgentSession, AgentView, CliProfileSnapshot,
        GenerationProcessFailureKind, INTERFACE_FORMAT_ANTHROPIC,
    };
    use crate::contexts::agent_runtime::domain::{
        AgentAvailability, AgentDefinition, AgentLifecycle, InteractionMode,
    };
    use crate::contexts::execution_observability::api::CapturePolicy;
    use crate::contexts::execution_observability::application::ExecutionIdentityPort;
    use crate::contexts::execution_observability::infrastructure::RandomExecutionIdentity;
    use std::collections::BTreeMap;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};

    #[derive(Default)]
    struct FakeCredentials {
        value: Option<String>,
    }

    impl ApiCredentialPort for FakeCredentials {
        fn store(
            &self,
            _agent_id: &str,
            _api_key: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
        fn fetch(&self, _agent_id: &str) -> Result<Option<String>, AgentRuntimeApplicationError> {
            Ok(self.value.clone())
        }
        fn remove(&self, _agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeConfig {
        provider_config: Option<ApiProviderConfig>,
    }

    fn anthropic_config(model_id: &str) -> FakeConfig {
        FakeConfig {
            provider_config: Some(ApiProviderConfig {
                model_id: model_id.to_string(),
                interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
                base_url: None,
            }),
        }
    }

    fn openai_compatible_config(model_id: &str, base_url: Option<&str>) -> FakeConfig {
        FakeConfig {
            provider_config: Some(ApiProviderConfig {
                model_id: model_id.to_string(),
                interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
                base_url: base_url.map(str::to_string),
            }),
        }
    }

    impl ApiAgentGateway for FakeConfig {
        fn register(
            &self,
            _agent_id: &str,
            _input: &crate::contexts::agent_runtime::application::RegisterApiAgentInput,
        ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
            unimplemented!("not exercised by RuntimeAgentApiAdapter tests")
        }
        fn provider_config(
            &self,
            _agent_id: &str,
        ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
            Ok(self.provider_config.clone())
        }
    }

    enum FakeHistoryOutcome {
        Messages(Vec<crate::contexts::agent_runtime::application::AgentMessage>),
        Error,
    }

    struct FakeHistory(FakeHistoryOutcome);

    impl ConversationHistoryPort for FakeHistory {
        fn recent_messages(
            &self,
            _session_id: &str,
            _limit: i64,
        ) -> Result<
            Vec<crate::contexts::agent_runtime::application::AgentMessage>,
            AgentRuntimeApplicationError,
        > {
            match &self.0 {
                FakeHistoryOutcome::Messages(messages) => Ok(messages.clone()),
                FakeHistoryOutcome::Error => Err(AgentRuntimeApplicationError::Session(
                    "history unavailable".to_string(),
                )),
            }
        }
    }

    #[derive(Default)]
    struct NoopLogging;

    impl AgentLoggingPort for NoopLogging {
        fn record(&self, _log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    #[derive(Default)]
    struct RecordingLogging {
        logs: Mutex<Vec<AgentLog>>,
    }

    impl AgentLoggingPort for RecordingLogging {
        fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            self.logs.lock().expect("logs").push(log);
            Ok(())
        }
    }

    struct FixedClock;

    impl AgentClockPort for FixedClock {
        fn now(&self) -> String {
            "2026-01-01T00:00:00Z".to_string()
        }
    }

    struct NoopSkills;

    impl AgentSkillPort for NoopSkills {
        fn bound_skill_prompts(
            &self,
            _agent_id: &str,
        ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError> {
            Ok(Vec::new())
        }
    }

    /// `(agent_id, folder, content, source)`, as recorded by `FakeMemories::save`.
    type SavedMemory = (String, Option<String>, String, MemorySource);

    #[derive(Default)]
    struct FakeMemories {
        saved: Mutex<Vec<SavedMemory>>,
        /// What `list`/`list_all_for_agent` hand back — empty by default (the shape every
        /// pre-existing call site outside this section's own tests relies on), seeded via
        /// `FakeMemories::seeded` where a test needs `resolve_system_prompt` to see memories.
        to_list: Vec<AgentMemory>,
    }

    impl FakeMemories {
        fn seeded(to_list: Vec<AgentMemory>) -> Self {
            Self {
                saved: Mutex::new(Vec::new()),
                to_list,
            }
        }
    }

    impl AgentMemoryPort for FakeMemories {
        fn save(
            &self,
            agent_id: &str,
            folder: Option<&str>,
            content: &str,
            source: MemorySource,
        ) -> Result<(), AgentRuntimeApplicationError> {
            self.saved.lock().expect("saved memories").push((
                agent_id.to_string(),
                folder.map(str::to_string),
                content.to_string(),
                source,
            ));
            Ok(())
        }

        fn list(
            &self,
            _agent_id: &str,
            _folder: Option<&str>,
        ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
            Ok(self.to_list.clone())
        }

        fn list_all_for_agent(
            &self,
            _agent_id: &str,
        ) -> Result<Vec<AgentMemory>, AgentRuntimeApplicationError> {
            Ok(self.to_list.clone())
        }

        fn delete(&self, _memory_id: &str) -> Result<(), AgentRuntimeApplicationError> {
            Ok(())
        }
    }

    fn fake_memory(id: &str, content: &str) -> AgentMemory {
        AgentMemory {
            id: id.to_string(),
            agent_id: "my-agent".to_string(),
            folder: None,
            content: content.to_string(),
            source: MemorySource::Explicit,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    struct FakeSkills(Result<Vec<BoundSkillPrompt>, &'static str>);

    impl AgentSkillPort for FakeSkills {
        fn bound_skill_prompts(
            &self,
            _agent_id: &str,
        ) -> Result<Vec<BoundSkillPrompt>, AgentRuntimeApplicationError> {
            self.0
                .clone()
                .map_err(|message| AgentRuntimeApplicationError::Skill(message.to_string()))
        }
    }

    #[derive(Default)]
    struct CapturingSink {
        events: Mutex<Vec<GenerationProcessEvent>>,
    }

    impl AgentProcessEventSink for CapturingSink {
        fn handle(
            &self,
            event: GenerationProcessEvent,
        ) -> Result<(), AgentRuntimeApplicationError> {
            self.events.lock().expect("events").push(event);
            Ok(())
        }
    }

    fn sample_request(launch_kind: &str) -> GenerationProcessRequest {
        GenerationProcessRequest {
            execution_context: RandomExecutionIdentity.next_context(
                CapturePolicy::MetadataOnly,
                0.0,
                false,
            ),
            session: AgentSession {
                id: "session-1".to_string(),
                agent_id: "my-claude-agent".to_string(),
                interaction_mode: InteractionMode::Api,
                lifecycle: AgentLifecycle::Running,
                folder: None,
                runtime_session_id: None,
                archived: false,
                read_only: false,
                loop_ownership: None,
            },
            agent: AgentView {
                id: "my-claude-agent".to_string(),
                display_name: "My Claude Agent".to_string(),
                provider: "Anthropic".to_string(),
                managed_sdk_dependency_id: None,
                launch: AgentLaunchView {
                    kind: launch_kind.to_string(),
                    command: None,
                    url: None,
                    executable_name: None,
                },
                supported_interaction_modes: vec![InteractionMode::Api],
                availability: AgentAvailability::Available,
                unavailable_reason: None,
                capability_tags: vec!["api".to_string()],
            },
            message_id: "message-1".to_string(),
            operation_id: "operation-1".to_string(),
            configuration: AgentChatConfiguration {
                agent_id: "my-claude-agent".to_string(),
                interaction_mode: InteractionMode::Api,
                permission_mode: "default".to_string(),
                provider_id: None,
                model_id: None,
                reasoning_depth: None,
                streaming: true,
                thinking: false,
                long_context: false,
            },
            effective_prompt: "hello".to_string(),
            cli_profile: CliProfileSnapshot {
                executable: String::new(),
                selections: BTreeMap::new(),
                managed_args: Vec::new(),
            },
        }
    }

    fn adapter() -> RuntimeAgentApiAdapter {
        RuntimeAgentApiAdapter::new(
            Arc::new(FakeCredentials::default()),
            Arc::new(FakeConfig::default()),
            Arc::new(FakeHistory(FakeHistoryOutcome::Messages(Vec::new()))),
            Arc::new(NoopLogging),
            Arc::new(FixedClock),
            Arc::new(NoopSkills),
            Arc::new(FakeMemories::default()),
        )
    }

    #[test]
    fn start_generation_rejects_non_api_launch_kind() {
        let result = adapter().start_generation(sample_request("cli"));
        assert!(result.is_err());
    }

    #[test]
    fn start_generation_registers_with_api_process_prefix() {
        let started = adapter()
            .start_generation(sample_request("api"))
            .expect("start generation");
        assert!(started.process_id.starts_with("agent-api-process-"));
    }

    #[test]
    fn stop_generation_returns_false_for_unknown_process() {
        let stopped = adapter()
            .stop_generation(
                "agent-api-process-does-not-exist",
                ProcessStopInitiator::User,
            )
            .expect("stop generation");
        assert!(!stopped);
    }

    #[test]
    fn stop_generation_returns_true_for_a_registered_process() {
        let adapter = adapter();
        let started = adapter
            .start_generation(sample_request("api"))
            .expect("start generation");
        let stopped = adapter
            .stop_generation(&started.process_id, ProcessStopInitiator::User)
            .expect("stop generation");
        assert!(stopped);
    }

    #[test]
    fn monitor_generation_errors_for_unknown_process() {
        let result = adapter().monitor_generation(
            "agent-api-process-does-not-exist",
            Arc::new(CapturingSink::default()),
        );
        assert!(result.is_err());
    }

    fn not_cancelled() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn no_pending_approvals() -> PendingApprovals {
        Arc::new(Mutex::new(HashMap::new()))
    }

    #[test]
    fn execute_fails_non_retryably_when_no_credential_is_stored() {
        let request = sample_request("api");
        let sink = CapturingSink::default();
        let event = execute(
            &request,
            not_cancelled(),
            &FakeCredentials::default(),
            &anthropic_config("claude-opus-4-8"),
            &FakeHistory(FakeHistoryOutcome::Messages(Vec::new())),
            &sink,
            &no_pending_approvals(),
            &NoopLogging,
            &FixedClock,
            &NoopSkills,
            &FakeMemories::default(),
        );
        match event {
            GenerationProcessEvent::Failed(failure) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
                assert!(failure.diagnostic.contains("API key"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
        assert!(sink.events.lock().expect("events").is_empty());
    }

    #[test]
    fn execute_fails_non_retryably_when_no_model_is_configured() {
        let request = sample_request("api");
        let event = execute(
            &request,
            not_cancelled(),
            &FakeCredentials {
                value: Some("sk-ant-test".to_string()),
            },
            &FakeConfig::default(),
            &FakeHistory(FakeHistoryOutcome::Messages(Vec::new())),
            &CapturingSink::default(),
            &no_pending_approvals(),
            &NoopLogging,
            &FixedClock,
            &NoopSkills,
            &FakeMemories::default(),
        );
        match event {
            GenerationProcessEvent::Failed(failure) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
                assert!(failure.diagnostic.contains("model"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn execute_fails_retryably_when_history_lookup_errors() {
        let request = sample_request("api");
        let event = execute(
            &request,
            not_cancelled(),
            &FakeCredentials {
                value: Some("sk-ant-test".to_string()),
            },
            &anthropic_config("claude-opus-4-8"),
            &FakeHistory(FakeHistoryOutcome::Error),
            &CapturingSink::default(),
            &no_pending_approvals(),
            &NoopLogging,
            &FixedClock,
            &NoopSkills,
            &FakeMemories::default(),
        );
        match event {
            GenerationProcessEvent::Failed(failure) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn execute_fails_non_retryably_when_openai_compatible_agent_has_no_base_url() {
        let request = sample_request("api");
        let event = execute(
            &request,
            not_cancelled(),
            &FakeCredentials {
                value: Some("sk-test".to_string()),
            },
            &openai_compatible_config("deepseek-chat", None),
            &FakeHistory(FakeHistoryOutcome::Messages(Vec::new())),
            &CapturingSink::default(),
            &no_pending_approvals(),
            &NoopLogging,
            &FixedClock,
            &NoopSkills,
            &FakeMemories::default(),
        );
        match event {
            GenerationProcessEvent::Failed(failure) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
                assert!(failure.diagnostic.contains("base URL"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn execute_fails_non_retryably_when_openai_compatible_base_url_is_blank() {
        let request = sample_request("api");
        let event = execute(
            &request,
            not_cancelled(),
            &FakeCredentials {
                value: Some("sk-test".to_string()),
            },
            &openai_compatible_config("deepseek-chat", Some("   ")),
            &FakeHistory(FakeHistoryOutcome::Messages(Vec::new())),
            &CapturingSink::default(),
            &no_pending_approvals(),
            &NoopLogging,
            &FixedClock,
            &NoopSkills,
            &FakeMemories::default(),
        );
        match event {
            GenerationProcessEvent::Failed(failure) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
                assert!(failure.diagnostic.contains("base URL"));
            }
            other => panic!("expected Failed, got {other:?}"),
        }
    }

    #[test]
    fn wire_format_for_openai_compatible_builds_chat_completions_endpoint() {
        let config = ApiProviderConfig {
            model_id: "deepseek-chat".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some("https://api.deepseek.com/v1/".to_string()),
        };
        let wire_format = wire_format_for(&config).expect("wire format");
        assert_eq!(
            wire_format.endpoint,
            "https://api.deepseek.com/v1/chat/completions"
        );
    }

    #[test]
    fn wire_format_for_anthropic_uses_fixed_endpoint() {
        let config = ApiProviderConfig {
            model_id: "claude-opus-4-8".to_string(),
            interface_format: INTERFACE_FORMAT_ANTHROPIC.to_string(),
            base_url: None,
        };
        let wire_format = wire_format_for(&config).expect("wire format");
        assert_eq!(wire_format.endpoint, MESSAGES_ENDPOINT);
    }

    #[test]
    fn await_approval_returns_approved_when_resolved_with_approved() {
        let pending = no_pending_approvals();
        let cancelled = not_cancelled();
        let cancelled_for_resolver = cancelled.clone();
        let pending_for_resolver = pending.clone();
        let resolver = thread::spawn(move || {
            // Give await_approval a moment to register the pending entry first.
            thread::sleep(Duration::from_millis(20));
            let sender = pending_for_resolver
                .lock()
                .expect("lock")
                .get("call-1")
                .expect("registered")
                .clone();
            let _ = sender.send(ToolApprovalDecision::Approved);
            let _ = cancelled_for_resolver;
        });
        let outcome = await_approval("call-1", &cancelled, &pending);
        resolver.join().expect("resolver thread");
        assert!(matches!(outcome, ApprovalOutcome::Approved));
    }

    #[test]
    fn await_approval_returns_denied_when_resolved_with_denied() {
        let pending = no_pending_approvals();
        let cancelled = not_cancelled();
        let pending_for_resolver = pending.clone();
        let resolver = thread::spawn(move || {
            thread::sleep(Duration::from_millis(20));
            let sender = pending_for_resolver
                .lock()
                .expect("lock")
                .get("call-1")
                .expect("registered")
                .clone();
            let _ = sender.send(ToolApprovalDecision::Denied);
        });
        let outcome = await_approval("call-1", &cancelled, &pending);
        resolver.join().expect("resolver thread");
        assert!(matches!(outcome, ApprovalOutcome::Denied));
    }

    #[test]
    fn await_approval_returns_cancelled_when_already_cancelled() {
        let pending = no_pending_approvals();
        let cancelled = Arc::new(AtomicBool::new(true));
        let outcome = await_approval("call-1", &cancelled, &pending);
        assert!(matches!(outcome, ApprovalOutcome::Cancelled));
        assert!(!pending.lock().expect("lock").contains_key("call-1"));
    }

    #[test]
    fn execute_tool_call_rejects_unknown_tool_names() {
        let outcome = execute_tool_call(
            "mystery",
            &json!({}),
            Some("."),
            not_cancelled(),
            "test-agent",
            &FakeMemories::default(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn execute_tool_call_fails_closed_without_a_workspace_folder() {
        let outcome = execute_tool_call(
            SHELL_TOOL_NAME,
            &json!({"command": "echo hi"}),
            None,
            not_cancelled(),
            "test-agent",
            &FakeMemories::default(),
        );
        assert!(outcome.is_error);
        assert!(outcome.output.contains("workspace folder"));
    }

    #[test]
    fn execute_tool_call_routes_shell_and_file_by_name() {
        let directory = crate::test_support::TempDirectory::new("execute-tool-call-routing");
        std::fs::write(directory.path().join("a.txt"), "hello").expect("fixture");
        let folder = directory.path().to_string_lossy().to_string();

        let shell_outcome = execute_tool_call(
            SHELL_TOOL_NAME,
            &json!({"command": "echo hi"}),
            Some(&folder),
            not_cancelled(),
            "test-agent",
            &FakeMemories::default(),
        );
        assert!(!shell_outcome.is_error);

        let file_outcome = execute_tool_call(
            FILE_TOOL_NAME,
            &json!({"operation": "read", "path": "a.txt"}),
            Some(&folder),
            not_cancelled(),
            "test-agent",
            &FakeMemories::default(),
        );
        assert!(!file_outcome.is_error);
        assert_eq!(file_outcome.output, "hello");
    }

    #[test]
    fn execute_tool_call_routes_remember_and_works_without_a_workspace_folder() {
        let memories = FakeMemories::default();

        let outcome = execute_tool_call(
            REMEMBER_TOOL_NAME,
            &json!({"content": "Uses pnpm."}),
            None,
            not_cancelled(),
            "test-agent",
            &memories,
        );

        assert!(!outcome.is_error);
        let saved = memories.saved.lock().expect("saved memories");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].0, "test-agent");
        assert_eq!(saved[0].1, None);
        assert_eq!(saved[0].2, "Uses pnpm.");
        assert_eq!(saved[0].3, MemorySource::Explicit);
    }

    #[test]
    fn execute_tool_call_remember_rejects_empty_content() {
        let outcome = execute_tool_call(
            REMEMBER_TOOL_NAME,
            &json!({"content": "   "}),
            Some("."),
            not_cancelled(),
            "test-agent",
            &FakeMemories::default(),
        );
        assert!(outcome.is_error);
    }

    #[test]
    fn tool_approval_port_resolve_returns_false_for_unknown_process() {
        let adapter = adapter();
        let resolved = ToolApprovalPort::resolve(
            &adapter,
            "agent-api-process-does-not-exist",
            "call-1",
            ToolApprovalDecision::Approved,
        )
        .expect("resolve");
        assert!(!resolved);
    }

    #[test]
    fn tool_approval_port_resolve_returns_false_when_call_id_has_no_pending_approval() {
        let adapter = adapter();
        let started = adapter
            .start_generation(sample_request("api"))
            .expect("start generation");
        let resolved = ToolApprovalPort::resolve(
            &adapter,
            &started.process_id,
            "call-never-registered",
            ToolApprovalDecision::Approved,
        )
        .expect("resolve");
        assert!(!resolved);
    }

    #[test]
    fn turns_character_count_sums_nested_string_values_not_just_the_content_field() {
        // Both wire formats' tool-loop turns nest large payloads (e.g. file-read output) inside
        // arrays of blocks rather than a flat `content` string — a shallow field read would
        // undercount exactly the case compaction exists for. The walk picks up every string
        // leaf, so "role"/"type" contribute too, not just the 100-character payload.
        let turns = vec![json!({
            "role": "user",
            "content": [
                { "type": "tool_result", "content": "a".repeat(100), "is_error": false }
            ]
        })];
        assert_eq!(
            turns_character_count(&turns),
            "user".len() + "tool_result".len() + 100
        );
    }

    #[test]
    fn should_compact_triggers_only_strictly_above_the_threshold() {
        assert!(!should_compact(COMPACTION_TRIGGER_CHARACTERS));
        assert!(should_compact(COMPACTION_TRIGGER_CHARACTERS + 1));
    }

    #[test]
    fn format_system_prompt_joins_multiple_skills_with_headers() {
        let prompts = vec![
            BoundSkillPrompt {
                name: "First".to_string(),
                body: "Do the first thing.".to_string(),
            },
            BoundSkillPrompt {
                name: "Second".to_string(),
                body: "Do the second thing.".to_string(),
            },
        ];
        assert_eq!(
            format_system_prompt(&prompts),
            "## First\nDo the first thing.\n\n## Second\nDo the second thing."
        );
    }

    #[test]
    fn resolve_system_prompt_returns_none_when_no_skills_are_bound() {
        let request = sample_request("api");
        let system = resolve_system_prompt(
            "my-agent",
            &FakeSkills(Ok(Vec::new())),
            &FakeMemories::default(),
            &NoopLogging,
            &FixedClock,
            &request,
        );
        assert_eq!(system, None);
    }

    #[test]
    fn resolve_system_prompt_formats_bound_skills() {
        let request = sample_request("api");
        let system = resolve_system_prompt(
            "my-agent",
            &FakeSkills(Ok(vec![BoundSkillPrompt {
                name: "Reviewer".to_string(),
                body: "Review the diff.".to_string(),
            }])),
            &FakeMemories::default(),
            &NoopLogging,
            &FixedClock,
            &request,
        );
        assert_eq!(system, Some("## Reviewer\nReview the diff.".to_string()));
    }

    #[test]
    fn resolve_system_prompt_falls_back_to_none_when_skill_lookup_fails() {
        let request = sample_request("api");
        let system = resolve_system_prompt(
            "my-agent",
            &FakeSkills(Err("lookup failed")),
            &FakeMemories::default(),
            &NoopLogging,
            &FixedClock,
            &request,
        );
        assert_eq!(system, None);
    }

    #[test]
    fn resolve_system_prompt_combines_skills_and_memory_sections() {
        let request = sample_request("api");
        let memories = FakeMemories::seeded(vec![fake_memory("memory-1", "Uses pnpm.")]);
        let system = resolve_system_prompt(
            "my-agent",
            &FakeSkills(Ok(vec![BoundSkillPrompt {
                name: "Reviewer".to_string(),
                body: "Review the diff.".to_string(),
            }])),
            &memories,
            &NoopLogging,
            &FixedClock,
            &request,
        );
        assert_eq!(
            system,
            Some("## Reviewer\nReview the diff.\n\n## Memory\n- Uses pnpm.".to_string())
        );
    }

    #[test]
    fn resolve_system_prompt_returns_only_memory_when_no_skills_are_bound() {
        let request = sample_request("api");
        let memories = FakeMemories::seeded(vec![fake_memory("memory-1", "Uses pnpm.")]);
        let system = resolve_system_prompt(
            "my-agent",
            &FakeSkills(Ok(Vec::new())),
            &memories,
            &NoopLogging,
            &FixedClock,
            &request,
        );
        assert_eq!(system, Some("## Memory\n- Uses pnpm.".to_string()));
    }

    #[test]
    fn format_memory_section_truncates_by_recency_when_over_budget() {
        let recent = fake_memory(
            "recent",
            &"x".repeat(MEMORY_INJECTION_CHARACTER_BUDGET - 10),
        );
        let older = fake_memory("older", "This one no longer fits.");
        // `list`'s contract is recency order (most recent first) — `recent` is deliberately
        // sized to consume nearly the whole budget, leaving no room for `older` behind it.
        let section = format_memory_section(&[recent.clone(), older]);
        assert_eq!(section, Some(format!("## Memory\n- {}", recent.content)));
    }

    #[test]
    fn format_memory_section_skips_an_oversized_entry_and_keeps_checking_smaller_ones_behind_it() {
        let oversized = fake_memory("big", &"x".repeat(MEMORY_INJECTION_CHARACTER_BUDGET + 1));
        let fits = fake_memory("small", "Uses pnpm.");
        let section = format_memory_section(&[oversized, fits]);
        assert_eq!(section, Some("## Memory\n- Uses pnpm.".to_string()));
    }

    #[test]
    fn format_memory_section_returns_none_for_no_memories() {
        assert_eq!(format_memory_section(&[]), None);
    }

    fn openai_compatible_wire_format(base_url: &str) -> WireFormat {
        wire_format_for(&ApiProviderConfig {
            model_id: "deepseek-chat".to_string(),
            interface_format: INTERFACE_FORMAT_OPENAI_COMPATIBLE.to_string(),
            base_url: Some(base_url.to_string()),
        })
        .expect("wire format")
    }

    fn sse_body(events: &[&str]) -> String {
        events
            .iter()
            .map(|event| format!("data: {event}\n\n"))
            .collect()
    }

    /// Spins up a one-shot local HTTP server returning `status`/`body`, and returns the raw
    /// bytes of the request it received (so tests can assert on what was actually sent) via
    /// `JoinHandle::join`. Mirrors the `TcpListener`-based fixture pattern already established in
    /// `contexts::tooling::mcp::infrastructure::relay_tests`.
    fn http_fixture(status: &'static str, body: String) -> (String, thread::JoinHandle<Vec<u8>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind summarization fixture");
        let address = listener.local_addr().expect("fixture address");
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept summarization request");
            let request = read_fixture_request(&mut stream);
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            stream
                .write_all(response.as_bytes())
                .expect("write summarization response");
            request
        });
        (format!("http://{address}"), handle)
    }

    /// Like `http_fixture`, but accepts and answers `bodies.len()` requests in sequence on the
    /// same address — for call sites that make more than one HTTP request against the same
    /// wire-format endpoint, such as `maybe_compact`'s own summarization call followed by
    /// `extract_memories`'s.
    fn http_fixture_sequence(
        status: &'static str,
        bodies: Vec<String>,
    ) -> (String, thread::JoinHandle<Vec<Vec<u8>>>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture sequence");
        let address = listener.local_addr().expect("fixture address");
        let handle = thread::spawn(move || {
            bodies
                .into_iter()
                .map(|body| {
                    let (mut stream, _) = listener.accept().expect("accept fixture request");
                    let request = read_fixture_request(&mut stream);
                    let response = format!(
                        "HTTP/1.1 {status}\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream
                        .write_all(response.as_bytes())
                        .expect("write fixture response");
                    request
                })
                .collect()
        });
        (format!("http://{address}"), handle)
    }

    fn read_fixture_request(stream: &mut TcpStream) -> Vec<u8> {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 4096];
        loop {
            let count = stream.read(&mut buffer).expect("read fixture request");
            if count == 0 {
                break;
            }
            request.extend_from_slice(&buffer[..count]);
            let Some(header_end) = request.windows(4).position(|value| value == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&request[..header_end]);
            let content_length = headers
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            if request.len() >= header_end + 4 + content_length {
                break;
            }
        }
        request
    }

    fn request_json_body(request: &[u8]) -> Value {
        let text = String::from_utf8_lossy(request);
        let body_start = text.find("\r\n\r\n").map(|index| index + 4).unwrap_or(0);
        serde_json::from_str(&text[body_start..]).expect("request body json")
    }

    #[test]
    fn summarize_turns_accumulates_text_across_streamed_chunks_and_omits_tools() {
        let (address, server) = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"This "},"finish_reason":null}]}"#,
                r#"{"choices":[{"index":0,"delta":{"content":"is a summary."},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let cancelled = not_cancelled();

        let summary = summarize_turns(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[json!({ "role": "user", "content": "hello" })],
            SUMMARIZATION_INSTRUCTION,
            &cancelled,
        );

        let request = server.join().expect("fixture server");
        assert_eq!(summary, Ok(Some("This is a summary.".to_string())));
        assert!(request_json_body(&request).get("tools").is_none());
    }

    #[test]
    fn summarize_turns_returns_ok_none_when_the_turns_to_summarize_are_empty() {
        let wire_format = openai_compatible_wire_format("http://127.0.0.1:1");
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let summary = summarize_turns(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[],
            SUMMARIZATION_INSTRUCTION,
            &not_cancelled(),
        );
        assert_eq!(summary, Ok(None));
    }

    #[test]
    fn summarize_turns_returns_err_when_the_http_call_fails() {
        let (address, server) = http_fixture("500 Internal Server Error", String::new());
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let cancelled = not_cancelled();

        let summary = summarize_turns(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[json!({ "role": "user", "content": "hello" })],
            SUMMARIZATION_INSTRUCTION,
            &cancelled,
        );

        server.join().expect("fixture server");
        assert!(summary.is_err());
    }

    #[test]
    fn extract_memories_saves_one_memory_per_non_empty_line() {
        let (address, server) = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"Uses pnpm.\nPrefers dark mode."},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let cancelled = not_cancelled();
        let memories = FakeMemories::default();
        let logging = RecordingLogging::default();
        let request = sample_request("api");

        extract_memories(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[json!({ "role": "user", "content": "hello" })],
            &cancelled,
            "my-agent",
            Some("my-folder"),
            &memories,
            &logging,
            &FixedClock,
            &request,
        );

        server.join().expect("fixture server");
        let saved = memories.saved.lock().expect("saved memories");
        assert_eq!(saved.len(), 2);
        assert_eq!(
            saved[0],
            (
                "my-agent".to_string(),
                Some("my-folder".to_string()),
                "Uses pnpm.".to_string(),
                MemorySource::Automatic,
            )
        );
        assert_eq!(saved[1].2, "Prefers dark mode.");
        assert!(logging.logs.lock().expect("logs").is_empty());
    }

    #[test]
    fn extract_memories_saves_nothing_and_logs_nothing_when_the_response_is_empty() {
        let (address, server) = http_fixture("200 OK", sse_body(&["[DONE]"]));
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let cancelled = not_cancelled();
        let memories = FakeMemories::default();
        let logging = RecordingLogging::default();
        let request = sample_request("api");

        extract_memories(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[json!({ "role": "user", "content": "hello" })],
            &cancelled,
            "my-agent",
            None,
            &memories,
            &logging,
            &FixedClock,
            &request,
        );

        server.join().expect("fixture server");
        assert!(memories.saved.lock().expect("saved memories").is_empty());
        // "Nothing worth remembering" is a normal outcome, not a failure — unlike the HTTP
        // failure case below, it must not be logged.
        assert!(logging.logs.lock().expect("logs").is_empty());
    }

    #[test]
    fn extract_memories_saves_nothing_and_logs_a_warning_when_the_http_call_fails() {
        let (address, server) = http_fixture("500 Internal Server Error", String::new());
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let cancelled = not_cancelled();
        let memories = FakeMemories::default();
        let logging = RecordingLogging::default();
        let request = sample_request("api");

        extract_memories(
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &[json!({ "role": "user", "content": "hello" })],
            &cancelled,
            "my-agent",
            None,
            &memories,
            &logging,
            &FixedClock,
            &request,
        );

        server.join().expect("fixture server");
        assert!(memories.saved.lock().expect("saved memories").is_empty());
        let logs = logging.logs.lock().expect("logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].level, AgentLogLevel::Warn);
        assert!(logs[0].message.contains("extraction"));
    }

    #[test]
    fn maybe_compact_leaves_turns_untouched_below_threshold() {
        let mut turns = vec![json!({ "role": "user", "content": "hi" })];
        let sink = CapturingSink::default();
        let wire_format = openai_compatible_wire_format("http://127.0.0.1:1");
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let request = sample_request("api");
        let cancelled = not_cancelled();

        let result = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &cancelled,
            &sink,
            &NoopLogging,
            &FixedClock,
            &request,
            &FakeMemories::default(),
        );

        assert!(result.is_none());
        assert_eq!(turns.len(), 1);
        assert!(sink.events.lock().expect("events").is_empty());
    }

    #[test]
    fn maybe_compact_replaces_older_turns_and_emits_a_rich_block_notice_when_triggered() {
        let (address, server) = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"Condensed summary."},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let sink = CapturingSink::default();
        let request = sample_request("api");
        let cancelled = not_cancelled();

        let mut turns = Vec::new();
        for index in 0..3 {
            turns.push(json!({
                "role": "user",
                "content": format!("{}-{index}", "x".repeat(COMPACTION_TRIGGER_CHARACTERS / 2)),
            }));
        }
        for index in 0..COMPACTION_KEEP_RECENT_TURNS {
            turns.push(json!({ "role": "user", "content": format!("recent-{index}") }));
        }

        let result = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &cancelled,
            &sink,
            &NoopLogging,
            &FixedClock,
            &request,
            &FakeMemories::default(),
        );
        server.join().expect("fixture server");

        assert!(result.is_none());
        assert_eq!(turns.len(), 1 + COMPACTION_KEEP_RECENT_TURNS);
        assert_eq!(turns[0]["content"], "Condensed summary.");
        for index in 0..COMPACTION_KEEP_RECENT_TURNS {
            assert_eq!(turns[index + 1]["content"], format!("recent-{index}"));
        }
        let events = sink.events.lock().expect("events");
        assert_eq!(events.len(), 1);
        match &events[0] {
            GenerationProcessEvent::RichBlock(block) => {
                assert_eq!(block["kind"], "card");
                assert_eq!(block["tone"], "info");
            }
            other => panic!("expected RichBlock, got {other:?}"),
        }
    }

    #[test]
    fn maybe_compact_preserves_the_system_prompt_across_a_trigger() {
        let (address, server) = http_fixture(
            "200 OK",
            sse_body(&[
                r#"{"choices":[{"index":0,"delta":{"content":"Condensed summary."},"finish_reason":null}]}"#,
                "[DONE]",
            ]),
        );
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let sink = CapturingSink::default();
        let request = sample_request("api");
        let cancelled = not_cancelled();
        let system = "Be concise.";

        let mut turns = Vec::new();
        for index in 0..3 {
            turns.push(json!({
                "role": "user",
                "content": format!("{}-{index}", "x".repeat(COMPACTION_TRIGGER_CHARACTERS / 2)),
            }));
        }
        for index in 0..COMPACTION_KEEP_RECENT_TURNS {
            turns.push(json!({ "role": "user", "content": format!("recent-{index}") }));
        }

        let result = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            Some(system),
            &cancelled,
            &sink,
            &NoopLogging,
            &FixedClock,
            &request,
            &FakeMemories::default(),
        );
        let summarization_request = server.join().expect("fixture server");
        assert!(result.is_none());

        // The system prompt reached the summarization call itself...
        let summarization_body = request_json_body(&summarization_request);
        assert_eq!(summarization_body["messages"][0]["role"], "system");
        assert_eq!(summarization_body["messages"][0]["content"], system);

        // ...and was never written into the turns compaction rewrote, so it can't be
        // mistaken for a turn a later compaction pass could summarize away.
        for turn in &turns {
            assert_ne!(turn["content"], system);
        }

        // A request built after compaction still carries the same system prompt, unaffected.
        let body_after =
            (wire_format.build_request_body)("deepseek-chat", &turns, &[], Some(system));
        assert_eq!(body_after["messages"][0]["role"], "system");
        assert_eq!(body_after["messages"][0]["content"], system);
    }

    #[test]
    fn maybe_compact_falls_back_to_leaving_turns_untouched_when_summarization_fails() {
        let (address, server) = http_fixture("500 Internal Server Error", String::new());
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let sink = CapturingSink::default();
        let request = sample_request("api");
        let cancelled = not_cancelled();

        let big = "x".repeat(COMPACTION_TRIGGER_CHARACTERS + 1);
        let mut turns = vec![json!({ "role": "user", "content": big.clone() })];
        for index in 0..COMPACTION_KEEP_RECENT_TURNS {
            turns.push(json!({ "role": "user", "content": format!("recent-{index}") }));
        }
        let original_len = turns.len();

        let result = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &cancelled,
            &sink,
            &NoopLogging,
            &FixedClock,
            &request,
            &FakeMemories::default(),
        );
        server.join().expect("fixture server");

        assert!(result.is_none());
        assert_eq!(turns.len(), original_len);
        assert_eq!(turns[0]["content"], big);
        assert!(sink.events.lock().expect("events").is_empty());
    }

    #[test]
    fn maybe_compact_triggers_extraction_and_saves_memories_when_it_succeeds() {
        let (address, server) = http_fixture_sequence(
            "200 OK",
            vec![
                sse_body(&[
                    r#"{"choices":[{"index":0,"delta":{"content":"Condensed summary."},"finish_reason":null}]}"#,
                    "[DONE]",
                ]),
                sse_body(&[
                    r#"{"choices":[{"index":0,"delta":{"content":"Uses pnpm."},"finish_reason":null}]}"#,
                    "[DONE]",
                ]),
            ],
        );
        let wire_format = openai_compatible_wire_format(&address);
        let client = blocking_http_client(Duration::from_secs(5)).expect("client");
        let sink = CapturingSink::default();
        let request = sample_request("api");
        let cancelled = not_cancelled();
        let memories = FakeMemories::default();

        let mut turns = Vec::new();
        for index in 0..3 {
            turns.push(json!({
                "role": "user",
                "content": format!("{}-{index}", "x".repeat(COMPACTION_TRIGGER_CHARACTERS / 2)),
            }));
        }
        for index in 0..COMPACTION_KEEP_RECENT_TURNS {
            turns.push(json!({ "role": "user", "content": format!("recent-{index}") }));
        }

        let result = maybe_compact(
            &mut turns,
            &wire_format,
            &client,
            "sk-test",
            "deepseek-chat",
            None,
            &cancelled,
            &sink,
            &NoopLogging,
            &FixedClock,
            &request,
            &memories,
        );
        let requests = server.join().expect("fixture server");

        assert!(result.is_none());
        assert_eq!(
            requests.len(),
            2,
            "compaction's own summarization call, then extraction's"
        );
        let saved = memories.saved.lock().expect("saved memories");
        assert_eq!(saved.len(), 1);
        assert_eq!(saved[0].0, request.agent.id);
        assert_eq!(saved[0].1, request.session.folder);
        assert_eq!(saved[0].2, "Uses pnpm.");
        assert_eq!(saved[0].3, MemorySource::Automatic);
    }
}
