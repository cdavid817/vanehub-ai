use super::providers::{
    codex_session_root, find_codex_rollout_since, find_gemini_chat_session,
    find_opencode_session_since, opencode_database_path,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentRuntimeApplicationError, AgentSessionGateway, AgentUsageAccountingKind,
    AgentUsageRecord, CompleteAgentMessage, MessageTokenUsage, NewAgentMessage,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TerminalUsageTotals {
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
}

/// Reads the claude-code session JSONL and upserts one aggregated usage record under
/// `message_id`. Safe to call repeatedly (e.g. every few seconds while the terminal is
/// open, and once more at exit): the shared message state is recovered across restarts
/// and created only when non-zero usage exists, so each call updates the same row.
pub(crate) fn ingest_claude_terminal_usage(
    session_folder: Option<&str>,
    runtime_session_id: &str,
    sessions: &dyn AgentSessionGateway,
    message_id: &Mutex<Option<String>>,
    session_id: &str,
    agent_id: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(jsonl_path) = claude_session_jsonl_path(session_folder, runtime_session_id) else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&jsonl_path) else {
        return Ok(false);
    };
    let totals = aggregate_claude_usage(&jsonl_path, file)?;
    persist_terminal_usage(
        totals,
        sessions,
        message_id,
        session_id,
        agent_id,
        "cli-session-log",
        clock,
    )
}

/// Finds the opencode session that was created in this working directory during this
/// terminal's lifetime, then reads the running per-session token totals opencode itself
/// maintains in its own SQLite database, upserting them under `message_id`.
///
/// This intentionally does *not* rely on `ProviderSessionCapture`'s live poll (which
/// only samples while fresh PTY output is arriving and can miss a session whose DB row
/// appears after the last visible output, before the user stops the terminal). Doing
/// the directory+time lookup here instead has no such race and is cheap enough to repeat
/// on every periodic poll as well as once more at exit; repeated calls update the same
/// recovered or lazily-created row.
pub(crate) fn ingest_opencode_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    sessions: &dyn AgentSessionGateway,
    message_id: &Mutex<Option<String>>,
    session_id: &str,
    agent_id: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(database_path) = opencode_database_path() else {
        return Ok(false);
    };
    if !database_path.exists() {
        return Ok(false);
    }
    let Some(cwd) = session_folder
        .filter(|f| !f.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(runtime_session_id) = find_opencode_session_since(&database_path, &cwd, started_at_ms)
        .map_err(|e| AgentRuntimeApplicationError::Process(e.to_string()))?
    else {
        return Ok(false);
    };
    let Some(totals) = read_opencode_session_totals(&database_path, &runtime_session_id)? else {
        return Ok(false);
    };
    persist_terminal_usage(
        totals,
        sessions,
        message_id,
        session_id,
        agent_id,
        "cli-session-log",
        clock,
    )
}

/// Finds the rollout file codex itself wrote for this working directory during this
/// terminal's lifetime, then reads the last `token_count` event's cumulative
/// `total_token_usage` — codex reports a running total per rollout, not a per-turn
/// delta (verified empirically: monotonically non-decreasing across 82 consecutive
/// `token_count` events in a real rollout) — and upserts it under `message_id`.
///
/// Uses the same post-hoc lookup as opencode rather than `ProviderSessionCapture`'s
/// live poll, for the same reason: no race, and cheap enough to repeat on every
/// periodic poll as well as once more at exit; repeated calls update the same recovered
/// or lazily-created row.
pub(crate) fn ingest_codex_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    sessions: &dyn AgentSessionGateway,
    message_id: &Mutex<Option<String>>,
    session_id: &str,
    agent_id: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(session_root) = codex_session_root() else {
        return Ok(false);
    };
    let Some(cwd) = session_folder
        .filter(|f| !f.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(rollout_path) = find_codex_rollout_since(&session_root, &cwd, started_at_ms)
        .map_err(|e| AgentRuntimeApplicationError::Process(e.to_string()))?
    else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&rollout_path) else {
        return Ok(false);
    };
    let totals = aggregate_codex_usage(&rollout_path, file)?;
    persist_terminal_usage(
        totals,
        sessions,
        message_id,
        session_id,
        agent_id,
        "cli-session-log",
        clock,
    )
}

/// Finds the `chats/*.jsonl` file gemini-cli's own `ChatRecordingService` wrote for this
/// working directory during this terminal's lifetime, then sums every `type: "gemini"`
/// message's per-turn `tokens` object — verified directly against the installed
/// `@google/gemini-cli` package's bundled source, not assumed. Unlike codex-cli's
/// cumulative `total_token_usage`, gemini-cli's local per-message tokens are genuinely
/// per-turn deltas, so every matching message is summed rather than taking the last one.
///
/// Uses the same post-hoc lookup as opencode/codex-cli; gemini-cli has no live
/// `ProviderSessionCapture` poll to race against in the first place, so this is simply
/// the only lookup. Repeated calls update the same recovered or lazily-created row.
pub(crate) fn ingest_gemini_terminal_usage(
    session_folder: Option<&str>,
    runtime_session_id: &str,
    sessions: &dyn AgentSessionGateway,
    message_id: &Mutex<Option<String>>,
    session_id: &str,
    agent_id: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(cwd) = session_folder
        .filter(|f| !f.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(chat_path) = find_gemini_chat_session(&cwd, runtime_session_id)
        .map_err(|e| AgentRuntimeApplicationError::Process(e.to_string()))?
    else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&chat_path) else {
        return Ok(false);
    };
    let totals = aggregate_gemini_usage(&chat_path, file)?;
    persist_terminal_usage(
        totals,
        sessions,
        message_id,
        session_id,
        agent_id,
        "cli-session-log",
        clock,
    )
}

pub(crate) fn load_terminal_usage_message_id(
    sessions: &dyn AgentSessionGateway,
    session_id: &str,
    agent_id: &str,
) -> Result<Option<String>, AgentRuntimeApplicationError> {
    sessions.find_terminal_usage_message(session_id, agent_id)
}

/// Returns whether a usage record was actually persisted, so callers can log the
/// difference between "correctly found nothing to report" and "wrote real data" —
/// both are `Ok`, but only one means the panel will show anything.
fn persist_terminal_usage(
    totals: TerminalUsageTotals,
    sessions: &dyn AgentSessionGateway,
    message_id: &Mutex<Option<String>>,
    session_id: &str,
    agent_id: &str,
    source: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    if totals.input_tokens == 0
        && totals.output_tokens == 0
        && totals.cache_read_tokens == 0
        && totals.cache_creation_tokens == 0
    {
        return Ok(false);
    }
    let mut message_id = message_id.lock().map_err(|_| {
        AgentRuntimeApplicationError::Process(
            "Terminal usage message state is unavailable.".to_string(),
        )
    })?;
    if message_id.is_none() {
        *message_id = match sessions.find_terminal_usage_message(session_id, agent_id)? {
            Some(existing) => Some(existing),
            None => Some(
                sessions
                    .create_message(NewAgentMessage {
                        session_id: session_id.to_string(),
                        seat_index: None,
                        role: "assistant".to_string(),
                        status: "completed".to_string(),
                        content: String::new(),
                        file_references: Vec::new(),
                    })?
                    .id,
            ),
        };
    }
    let message_id = message_id.clone().ok_or_else(|| {
        AgentRuntimeApplicationError::Process(
            "Failed to create terminal usage message.".to_string(),
        )
    })?;
    let usage = AgentUsageRecord {
        message_id: message_id.clone(),
        session_id: session_id.to_string(),
        agent_id: agent_id.to_string(),
        provider_id: None,
        model_id: None,
        accounting_kind: AgentUsageAccountingKind::Reported,
        input_count: totals.input_tokens,
        output_count: totals.output_tokens,
        cache_read_count: totals.cache_read_tokens,
        cache_creation_count: totals.cache_creation_tokens,
        source: source.to_string(),
        occurred_at: clock.now(),
    };
    sessions.complete_message(CompleteAgentMessage {
        message_id,
        session_id: session_id.to_string(),
        content: String::new(),
        thinking_content: None,
        tool_use: Vec::new(),
        rich_blocks: Vec::new(),
        token_usage: Some(MessageTokenUsage {
            input: totals.input_tokens,
            output: totals.output_tokens,
        }),
        usage: Some(usage),
    })?;
    Ok(true)
}

/// Claude Code transforms the working-directory path into a deterministic segment
/// used as the parent directory under `~/.claude/projects/`. On Windows this is
/// effectively `X:-/path/-/to/-/project` (':' and '\\' become '-'), producing
/// names like `D--cdavid-Documents-code-vanehub-ai`.
///
/// Windows extended-length path prefixes (`\\\\?\\`, `\\\\?\\UNC\\`) are stripped
/// before hashing because claude-code normalises the cwd internally and writes
/// its session log under the normalised path, while VaneHub stores the canonical
/// extended-length form.
fn claude_project_dir_name(cwd: &Path) -> String {
    let raw = cwd.to_string_lossy();
    let normalised = raw.strip_prefix(r"\\?\").unwrap_or(&raw);
    normalised.replace([':', '\\'], "-")
}

fn claude_session_jsonl_path(
    session_folder: Option<&str>,
    runtime_session_id: &str,
) -> Option<PathBuf> {
    let cwd = session_folder
        .filter(|f| !f.trim().is_empty())
        .map(PathBuf::from)?;
    let home = dirs::home_dir()?;
    Some(
        home.join(".claude")
            .join("projects")
            .join(claude_project_dir_name(&cwd))
            .join(format!("{runtime_session_id}.jsonl")),
    )
}

#[derive(Debug, Deserialize)]
struct ClaudeAssistantUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct ClaudeAssistantMessage {
    usage: Option<ClaudeAssistantUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeSessionEvent {
    #[serde(rename = "type")]
    event_type: String,
    message: Option<ClaudeAssistantMessage>,
}

fn aggregate_claude_usage(
    path: &Path,
    file: fs::File,
) -> Result<TerminalUsageTotals, AgentRuntimeApplicationError> {
    let reader = std::io::BufReader::new(file);
    let mut totals = TerminalUsageTotals::default();
    for line in reader.lines() {
        let line = line.map_err(|e| {
            AgentRuntimeApplicationError::Process(format!(
                "Failed to read claude session log {}: {e}",
                path.display()
            ))
        })?;
        let Ok(event) = serde_json::from_str::<ClaudeSessionEvent>(&line) else {
            continue;
        };
        if event.event_type != "assistant" {
            continue;
        }
        let Some(msg) = event.message else {
            continue;
        };
        let Some(usage) = msg.usage else {
            continue;
        };
        let it = usage.input_tokens.unwrap_or(0).max(0);
        let ot = usage.output_tokens.unwrap_or(0).max(0);
        let cr = usage.cache_read_input_tokens.unwrap_or(0).max(0);
        let cc = usage.cache_creation_input_tokens.unwrap_or(0).max(0);
        if it == 0 && ot == 0 && cr == 0 && cc == 0 {
            continue;
        }
        totals.input_tokens += it;
        totals.output_tokens += ot;
        totals.cache_read_tokens += cr;
        totals.cache_creation_tokens += cc;
    }
    Ok(totals)
}

/// Reasoning tokens are folded into `output_tokens`, matching how the managed
/// (non-interactive) ingestion path treats reasoning/thinking tokens for the
/// other three CLIs — see `providers::output::structured_usage`.
fn read_opencode_session_totals(
    database_path: &Path,
    session_id: &str,
) -> Result<Option<TerminalUsageTotals>, AgentRuntimeApplicationError> {
    let connection = Connection::open_with_flags(
        database_path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|e| {
        AgentRuntimeApplicationError::Process(format!(
            "Failed to open opencode database {}: {e}",
            database_path.display()
        ))
    })?;
    connection
        .query_row(
            "SELECT tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write
             FROM session WHERE id = ?1",
            [session_id],
            |row| {
                let input: i64 = row.get(0)?;
                let output: i64 = row.get(1)?;
                let reasoning: i64 = row.get(2)?;
                let cache_read: i64 = row.get(3)?;
                let cache_write: i64 = row.get(4)?;
                Ok(TerminalUsageTotals {
                    input_tokens: input.max(0),
                    output_tokens: (output.max(0)) + (reasoning.max(0)),
                    cache_read_tokens: cache_read.max(0),
                    cache_creation_tokens: cache_write.max(0),
                })
            },
        )
        .optional()
        .map_err(|e| {
            AgentRuntimeApplicationError::Process(format!(
                "Failed to read opencode session usage from {}: {e}",
                database_path.display()
            ))
        })
}

#[derive(Debug, Deserialize)]
struct CodexRolloutLine {
    #[serde(rename = "type")]
    line_type: String,
    payload: Option<CodexEventPayload>,
}

#[derive(Debug, Deserialize)]
struct CodexEventPayload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    info: Option<CodexTokenCountInfo>,
}

#[derive(Debug, Deserialize)]
struct CodexTokenCountInfo {
    total_token_usage: Option<CodexTokenUsage>,
}

#[derive(Debug, Deserialize)]
struct CodexTokenUsage {
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    cache_write_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_output_tokens: Option<i64>,
}

/// Codex emits `token_count` events roughly every turn; `total_token_usage` is the
/// running cumulative total for the whole rollout, so only the last one is kept.
fn aggregate_codex_usage(
    path: &Path,
    file: fs::File,
) -> Result<TerminalUsageTotals, AgentRuntimeApplicationError> {
    let reader = std::io::BufReader::new(file);
    let mut latest = TerminalUsageTotals::default();
    for line in reader.lines() {
        let line = line.map_err(|e| {
            AgentRuntimeApplicationError::Process(format!(
                "Failed to read codex rollout {}: {e}",
                path.display()
            ))
        })?;
        let Ok(event) = serde_json::from_str::<CodexRolloutLine>(&line) else {
            continue;
        };
        if event.line_type != "event_msg" {
            continue;
        }
        let Some(payload) = event.payload else {
            continue;
        };
        if payload.payload_type.as_deref() != Some("token_count") {
            continue;
        }
        let Some(usage) = payload.info.and_then(|info| info.total_token_usage) else {
            continue;
        };
        latest = TerminalUsageTotals {
            input_tokens: usage.input_tokens.unwrap_or(0).max(0),
            output_tokens: usage.output_tokens.unwrap_or(0).max(0)
                + usage.reasoning_output_tokens.unwrap_or(0).max(0),
            cache_read_tokens: usage.cached_input_tokens.unwrap_or(0).max(0),
            cache_creation_tokens: usage.cache_write_input_tokens.unwrap_or(0).max(0),
        };
    }
    Ok(latest)
}

/// One line per event in gemini-cli's own `ChatRecordingService` output: an initial
/// metadata line and `{"$set": ...}` update lines have neither `type` nor `tokens`, so
/// they deserialize with both fields `None` (serde ignores their unrecognised keys) and
/// are skipped by the `type == "gemini"` filter below, same as `type: "user"` lines.
#[derive(Debug, Deserialize)]
struct GeminiChatMessageLine {
    id: Option<String>,
    #[serde(rename = "type")]
    message_type: Option<String>,
    tokens: Option<GeminiMessageTokens>,
    #[serde(rename = "$set")]
    set: Option<GeminiSetRecord>,
}

#[derive(Debug, Deserialize)]
struct GeminiSetRecord {
    messages: Option<Vec<GeminiChatMessageLine>>,
}

#[derive(Debug, Clone, Deserialize)]
struct GeminiMessageTokens {
    input: Option<i64>,
    output: Option<i64>,
    cached: Option<i64>,
    thoughts: Option<i64>,
}

/// gemini-cli's `ChatRecordingService` records one line per event. Only `type:
/// "gemini"` messages (assistant turns) carry a `tokens` object, and — verified
/// directly from the installed package's source — each is a genuine per-turn delta
/// rather than a running cumulative total (unlike codex-cli's `total_token_usage`), so
/// every matching message's tokens are summed. `thoughts` (reasoning tokens) are folded
/// into `output_tokens`, matching the same folding decision already made for codex-cli
/// and opencode. `tool` (tokens spent on tool-use prompt context) is deliberately left
/// unmapped: it is very likely already a subset of `input`, and folding it in on top
/// would risk double-counting rather than adding real information.
fn aggregate_gemini_usage(
    path: &Path,
    file: fs::File,
) -> Result<TerminalUsageTotals, AgentRuntimeApplicationError> {
    let reader = std::io::BufReader::new(file);
    let mut messages: HashMap<String, Option<GeminiMessageTokens>> = HashMap::new();
    for line in reader.lines() {
        let line = line.map_err(|e| {
            AgentRuntimeApplicationError::Process(format!(
                "Failed to read gemini chat session {}: {e}",
                path.display()
            ))
        })?;
        let Ok(record) = serde_json::from_str::<GeminiChatMessageLine>(&line) else {
            continue;
        };
        materialize_gemini_record(record, &mut messages);
    }
    let mut totals = TerminalUsageTotals::default();
    for tokens in messages.into_values().flatten() {
        let input = tokens.input.unwrap_or(0).max(0);
        let output = tokens
            .output
            .unwrap_or(0)
            .max(0)
            .saturating_add(tokens.thoughts.unwrap_or(0).max(0));
        let cached = tokens.cached.unwrap_or(0).max(0);
        if input == 0 && output == 0 && cached == 0 {
            continue;
        }
        totals.input_tokens = totals.input_tokens.saturating_add(input);
        totals.output_tokens = totals.output_tokens.saturating_add(output);
        totals.cache_read_tokens = totals.cache_read_tokens.saturating_add(cached);
    }
    Ok(totals)
}

fn materialize_gemini_record(
    record: GeminiChatMessageLine,
    messages: &mut HashMap<String, Option<GeminiMessageTokens>>,
) {
    if let Some(snapshot) = record.set.and_then(|set| set.messages) {
        messages.clear();
        for message in snapshot {
            materialize_gemini_record(message, messages);
        }
        return;
    }
    if record.message_type.as_deref() != Some("gemini") {
        return;
    }
    let Some(id) = record.id.filter(|id| !id.trim().is_empty()) else {
        return;
    };
    messages.insert(id, record.tokens);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{AgentMessage, ToolUseBlock};
    use serde_json::Value;
    use std::io::Write;
    use std::sync::Mutex;

    /// Minimal gateway fake for restart recovery and repeated-poll persistence.
    #[derive(Default)]
    struct FakeSessionGateway {
        created: Mutex<Vec<NewAgentMessage>>,
        completed: Mutex<Vec<CompleteAgentMessage>>,
        existing: Mutex<Option<String>>,
        fail_completion: bool,
    }

    impl AgentSessionGateway for FakeSessionGateway {
        fn find_session(
            &self,
            _session_id: &str,
        ) -> Result<
            Option<crate::contexts::agent_runtime::application::AgentSession>,
            AgentRuntimeApplicationError,
        > {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn validate_configuration(
            &self,
            _session: &crate::contexts::agent_runtime::application::AgentSession,
            _configuration: crate::contexts::agent_runtime::application::AgentChatConfiguration,
        ) -> Result<
            crate::contexts::agent_runtime::application::AgentChatConfiguration,
            AgentRuntimeApplicationError,
        > {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn validate_seat_configuration(
            &self,
            _session: &crate::contexts::agent_runtime::application::AgentSession,
            _configuration: crate::contexts::agent_runtime::application::AgentChatConfiguration,
        ) -> Result<
            crate::contexts::agent_runtime::application::AgentChatConfiguration,
            AgentRuntimeApplicationError,
        > {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn compose_prompt(
            &self,
            _session_id: &str,
            _content: &str,
            _file_references: &[crate::contexts::agent_runtime::application::AgentFileReference],
        ) -> Result<String, AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn create_message(
            &self,
            message: NewAgentMessage,
        ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
            self.created.lock().expect("created").push(message.clone());
            Ok(AgentMessage {
                id: "placeholder-message".to_string(),
                session_id: message.session_id,
                seat_index: None,
                role: message.role,
                content: message.content,
                status: message.status,
                tool_use: Vec::new(),
                thinking_content: None,
                rich_blocks: Vec::new(),
                token_usage: None,
                file_references: message.file_references,
                error: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            })
        }

        fn find_terminal_usage_message(
            &self,
            _session_id: &str,
            _agent_id: &str,
        ) -> Result<Option<String>, AgentRuntimeApplicationError> {
            Ok(self.existing.lock().expect("existing").clone())
        }

        fn find_message(
            &self,
            _message_id: &str,
        ) -> Result<Option<AgentMessage>, AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn append_content(
            &self,
            _message_id: &str,
            _content_delta: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn append_thinking(
            &self,
            _message_id: &str,
            _content_delta: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn append_tool_use(
            &self,
            _message_id: &str,
            _tool_use: ToolUseBlock,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn append_rich_block(
            &self,
            _message_id: &str,
            _block: Value,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn complete_message(
            &self,
            message: CompleteAgentMessage,
        ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
            if self.fail_completion {
                return Err(AgentRuntimeApplicationError::Process(
                    "completion failed".to_string(),
                ));
            }
            self.completed
                .lock()
                .expect("completed")
                .push(message.clone());
            Ok(AgentMessage {
                id: message.message_id,
                session_id: message.session_id,
                seat_index: None,
                role: "assistant".to_string(),
                content: message.content,
                status: "completed".to_string(),
                tool_use: Vec::new(),
                thinking_content: message.thinking_content,
                rich_blocks: Vec::new(),
                token_usage: message.token_usage,
                file_references: Vec::new(),
                error: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-01T00:00:00Z".to_string(),
            })
        }

        fn fail_message(
            &self,
            _message_id: &str,
            _session_id: &str,
            _error: &str,
        ) -> Result<AgentMessage, AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn cancel_streaming_messages(
            &self,
            _session_id: &str,
        ) -> Result<Vec<String>, AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn update_lifecycle(
            &self,
            _session_id: &str,
            _lifecycle: crate::contexts::agent_runtime::domain::AgentLifecycle,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }

        fn update_runtime_session_id(
            &self,
            _session_id: &str,
            _runtime_session_id: &str,
        ) -> Result<(), AgentRuntimeApplicationError> {
            unimplemented!("not exercised by terminal usage ingestion tests")
        }
    }

    struct FixedClock;

    impl AgentClockPort for FixedClock {
        fn now(&self) -> String {
            "2026-01-01T00:00:00Z".to_string()
        }
    }

    #[test]
    fn zero_totals_do_not_create_a_backing_message() {
        let gateway = FakeSessionGateway::default();
        let clock = FixedClock;
        let message_id = Mutex::new(None);

        let persisted = persist_terminal_usage(
            TerminalUsageTotals::default(),
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("zero totals still return Ok");

        assert!(!persisted);
        assert!(gateway.created.lock().expect("created").is_empty());
        assert!(gateway.completed.lock().expect("completed").is_empty());
    }

    #[test]
    fn repeated_persist_calls_with_the_same_message_id_update_in_place() {
        let gateway = FakeSessionGateway::default();
        let clock = FixedClock;
        let message_id = Mutex::new(None);

        let first = TerminalUsageTotals {
            input_tokens: 100,
            output_tokens: 20,
            cache_read_tokens: 0,
            cache_creation_tokens: 0,
        };
        let second = TerminalUsageTotals {
            input_tokens: 400,
            output_tokens: 80,
            cache_read_tokens: 5,
            cache_creation_tokens: 1,
        };

        let first_persisted = persist_terminal_usage(
            first,
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("first poll persists");
        let second_persisted = persist_terminal_usage(
            second,
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("second poll persists");

        assert!(first_persisted);
        assert!(second_persisted);
        // Exactly one completed backing message was ever created…
        assert_eq!(gateway.created.lock().expect("created").len(), 1);
        assert_eq!(
            gateway.created.lock().expect("created")[0].status,
            "completed"
        );
        // …and both polls completed that same message id, letting the DB's
        // `ON CONFLICT(message_id)` upsert keep only the latest totals.
        let completed = gateway.completed.lock().expect("completed");
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].message_id, "placeholder-message");
        assert_eq!(completed[1].message_id, "placeholder-message");
        assert_eq!(completed[1].usage.as_ref().expect("usage").input_count, 400);
    }

    #[test]
    fn restart_reuses_the_existing_terminal_usage_message() {
        let gateway = FakeSessionGateway {
            existing: Mutex::new(Some("persisted-message".to_string())),
            ..Default::default()
        };
        let clock = FixedClock;
        let message_id = Mutex::new(None);

        let persisted = persist_terminal_usage(
            TerminalUsageTotals {
                input_tokens: 1,
                ..Default::default()
            },
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("existing row is reused");

        assert!(persisted);
        assert!(gateway.created.lock().expect("created").is_empty());
        assert_eq!(
            gateway.completed.lock().expect("completed")[0].message_id,
            "persisted-message"
        );
    }

    #[test]
    fn cache_only_totals_are_persisted() {
        let gateway = FakeSessionGateway::default();
        let clock = FixedClock;
        let message_id = Mutex::new(None);

        let persisted = persist_terminal_usage(
            TerminalUsageTotals {
                cache_read_tokens: 5,
                ..Default::default()
            },
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("cache-only usage persists");

        assert!(persisted);
        assert_eq!(
            gateway.completed.lock().expect("completed")[0]
                .usage
                .as_ref()
                .expect("usage")
                .cache_read_count,
            5
        );
    }

    #[test]
    fn completion_failure_is_propagated() {
        let gateway = FakeSessionGateway {
            fail_completion: true,
            ..Default::default()
        };
        let clock = FixedClock;
        let message_id = Mutex::new(None);

        let error = persist_terminal_usage(
            TerminalUsageTotals {
                input_tokens: 1,
                ..Default::default()
            },
            &gateway,
            &message_id,
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect_err("persistence errors must not be swallowed");

        assert!(error.to_string().contains("completion failed"));
    }

    #[test]
    fn claude_project_dir_transforms_windows_absolute_paths() {
        let input = Path::new("D:\\cdavid\\Documents\\code\\vanehub-ai");
        assert_eq!(
            claude_project_dir_name(input),
            "D--cdavid-Documents-code-vanehub-ai"
        );
    }

    #[test]
    fn claude_project_dir_transforms_windows_user_profile_paths() {
        let input = Path::new("C:\\Users\\cdavid");
        assert_eq!(claude_project_dir_name(input), "C--Users-cdavid");
    }

    #[test]
    fn extended_length_prefix_is_stripped_before_hashing() {
        let input = Path::new("\\\\?\\D:\\cdavid\\Documents\\code\\claude-code");
        assert_eq!(
            claude_project_dir_name(input),
            "D--cdavid-Documents-code-claude-code"
        );
    }

    #[test]
    fn aggregates_non_zero_assistant_usage_and_skips_zero_events() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":10,"output_tokens":5,"cache_read_input_tokens":2,"cache_creation_input_tokens":1}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":0,"output_tokens":0}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"type":"assistant","message":{{"usage":{{"input_tokens":20,"output_tokens":15}}}}}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"type":"user"}}"#).unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_claude_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 30);
        assert_eq!(totals.output_tokens, 20);
        assert_eq!(totals.cache_read_tokens, 2);
        assert_eq!(totals.cache_creation_tokens, 1);
    }

    #[test]
    fn empty_or_missing_jsonl_is_graceful() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(file, r#"{{"type":"user"}}"#).unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_claude_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 0);
        assert_eq!(totals.output_tokens, 0);
    }

    fn opencode_fixture_db() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let connection = Connection::open(dir.path().join("opencode.db")).expect("open db");
        connection
            .execute_batch(
                "CREATE TABLE session (
                    id TEXT PRIMARY KEY,
                    tokens_input INTEGER NOT NULL DEFAULT 0,
                    tokens_output INTEGER NOT NULL DEFAULT 0,
                    tokens_reasoning INTEGER NOT NULL DEFAULT 0,
                    tokens_cache_read INTEGER NOT NULL DEFAULT 0,
                    tokens_cache_write INTEGER NOT NULL DEFAULT 0
                );
                INSERT INTO session (id, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write)
                VALUES ('ses_with_usage', 14681, 2, 19, 0, 0);
                INSERT INTO session (id, tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write)
                VALUES ('ses_all_zero', 0, 0, 0, 0, 0);",
            )
            .expect("seed fixture");
        dir
    }

    #[test]
    fn opencode_totals_fold_reasoning_into_output() {
        let dir = opencode_fixture_db();
        let totals =
            read_opencode_session_totals(&dir.path().join("opencode.db"), "ses_with_usage")
                .expect("query")
                .expect("row found");

        assert_eq!(totals.input_tokens, 14681);
        assert_eq!(totals.output_tokens, 21);
        assert_eq!(totals.cache_read_tokens, 0);
        assert_eq!(totals.cache_creation_tokens, 0);
    }

    #[test]
    fn opencode_missing_session_id_returns_none() {
        let dir = opencode_fixture_db();
        let totals =
            read_opencode_session_totals(&dir.path().join("opencode.db"), "ses_does_not_exist")
                .expect("query");

        assert_eq!(totals, None);
    }

    #[test]
    fn codex_usage_keeps_the_last_cumulative_token_count_and_folds_reasoning() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"timestamp":"t1","type":"session_meta","payload":{{"session_id":"s1","cwd":"D:\\proj"}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"timestamp":"t2","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":16441,"cached_input_tokens":0,"cache_write_input_tokens":0,"output_tokens":175,"reasoning_output_tokens":35,"total_tokens":16616}}}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"timestamp":"t3","type":"event_msg","payload":{{"type":"token_count","info":{{"total_token_usage":{{"input_tokens":33000,"cached_input_tokens":500,"cache_write_input_tokens":100,"output_tokens":400,"reasoning_output_tokens":60,"total_tokens":34060}}}}}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"timestamp":"t4","type":"event_msg","payload":{{"type":"task_complete"}}}}"#
        )
        .unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_codex_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 33000);
        assert_eq!(totals.output_tokens, 460);
        assert_eq!(totals.cache_read_tokens, 500);
        assert_eq!(totals.cache_creation_tokens, 100);
    }

    #[test]
    fn codex_rollout_without_token_count_events_yields_zero_totals() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"timestamp":"t1","type":"session_meta","payload":{{}}}}"#
        )
        .unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_codex_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals, TerminalUsageTotals::default());
    }

    #[test]
    fn gemini_usage_sums_across_all_gemini_messages_and_folds_thoughts_into_output() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"sessionId":"s1","projectHash":"h1","startTime":"t0","lastUpdated":"t0","kind":"interactive"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"id":"msg1","timestamp":"t1","type":"user","content":"hello","displayContent":"hello"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"id":"msg2","timestamp":"t2","type":"gemini","content":"hi","displayContent":"hi","thoughts":[],"tokens":{{"input":100,"output":50,"cached":10,"thoughts":20,"tool":5,"total":185}},"model":"gemini-2.5-pro"}}"#
        )
        .unwrap();
        writeln!(file, r#"{{"$set":{{"lastUpdated":"t3"}}}}"#).unwrap();
        writeln!(
            file,
            r#"{{"id":"msg3","timestamp":"t4","type":"gemini","content":"more","displayContent":"more","tokens":{{"input":50,"output":30,"cached":0,"thoughts":0,"tool":0,"total":80}}}}"#
        )
        .unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_gemini_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 150);
        assert_eq!(totals.output_tokens, 100);
        assert_eq!(totals.cache_read_tokens, 10);
        assert_eq!(totals.cache_creation_tokens, 0);
    }

    #[test]
    fn gemini_usage_keeps_only_the_latest_revision_for_each_message_id() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"id":"msg1","type":"gemini","tokens":{{"input":100,"output":20,"cached":5,"thoughts":10}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"id":"msg1","type":"gemini","toolCalls":[{{"id":"tool1"}}],"tokens":{{"input":100,"output":20,"cached":5,"thoughts":10}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"id":"msg2","type":"gemini","tokens":{{"input":50,"output":7,"cached":0,"thoughts":3}}}}"#
        )
        .unwrap();
        file.flush().unwrap();

        let totals =
            aggregate_gemini_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 150);
        assert_eq!(totals.output_tokens, 40);
        assert_eq!(totals.cache_read_tokens, 5);
    }

    #[test]
    fn gemini_usage_set_messages_snapshot_replaces_prior_materialized_messages() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"id":"obsolete","type":"gemini","tokens":{{"input":900,"output":900}}}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"$set":{{"messages":[{{"id":"current","type":"gemini","tokens":{{"input":12,"output":8,"cached":2,"thoughts":1}}}},{{"id":"user","type":"user"}}]}}}}"#
        )
        .unwrap();
        file.flush().unwrap();

        let totals =
            aggregate_gemini_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals.input_tokens, 12);
        assert_eq!(totals.output_tokens, 9);
        assert_eq!(totals.cache_read_tokens, 2);
    }

    #[test]
    fn gemini_usage_skips_gemini_messages_without_a_tokens_object() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"id":"msg1","timestamp":"t1","type":"gemini","content":"queued","displayContent":"queued"}}"#
        )
        .unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_gemini_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals, TerminalUsageTotals::default());
    }

    #[test]
    fn gemini_chat_session_without_any_gemini_messages_yields_zero_totals() {
        let mut file = tempfile::NamedTempFile::new().expect("temp file");
        writeln!(
            file,
            r#"{{"sessionId":"s1","projectHash":"h1","startTime":"t0","lastUpdated":"t0","kind":"interactive"}}"#
        )
        .unwrap();
        writeln!(
            file,
            r#"{{"id":"msg1","timestamp":"t1","type":"user","content":"hello","displayContent":"hello"}}"#
        )
        .unwrap();
        file.flush().unwrap();
        let totals =
            aggregate_gemini_usage(file.path(), std::fs::File::open(file.path()).expect("open"))
                .expect("aggregate");

        assert_eq!(totals, TerminalUsageTotals::default());
    }
}
