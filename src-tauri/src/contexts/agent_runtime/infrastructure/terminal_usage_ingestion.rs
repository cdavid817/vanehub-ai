use super::providers::{
    codex_session_root, find_codex_rollout_since, find_opencode_session_since,
    opencode_database_path,
};
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentRuntimeApplicationError, AgentSessionGateway, AgentUsageAccountingKind,
    AgentUsageRecord, CompleteAgentMessage, MessageTokenUsage, NewAgentMessage,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TerminalUsageTotals {
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_creation_tokens: i64,
}

/// Reads the claude-code session JSONL and upserts one aggregated usage record under
/// `message_id`. Safe to call repeatedly (e.g. every few seconds while the terminal is
/// open, and once more at exit): `message_id` is a stable id created once per terminal
/// session by `create_terminal_usage_placeholder`, so each call updates the same row
/// via `upsert_usage`'s `ON CONFLICT(message_id)` instead of accumulating duplicates.
pub(crate) fn ingest_claude_terminal_usage(
    session_folder: Option<&str>,
    runtime_session_id: &str,
    sessions: &dyn AgentSessionGateway,
    message_id: &str,
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
/// on every periodic poll as well as once more at exit; `message_id` is the stable id
/// from `create_terminal_usage_placeholder`, so repeated calls update the same row.
pub(crate) fn ingest_opencode_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    sessions: &dyn AgentSessionGateway,
    message_id: &str,
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
/// periodic poll as well as once more at exit; `message_id` is the stable id from
/// `create_terminal_usage_placeholder`, so repeated calls update the same row.
pub(crate) fn ingest_codex_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    sessions: &dyn AgentSessionGateway,
    message_id: &str,
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

/// Creates the placeholder assistant message that periodic and exit-time usage polls
/// repeatedly update in place. A single stable id is required so `upsert_usage`'s
/// `ON CONFLICT(message_id)` updates the same row on every poll instead of
/// accumulating a new row every few seconds; `MessageStatus::can_transition_to`
/// explicitly allows `Completed -> Completed`, so re-completing this same message
/// on each poll is a valid, non-erroring transition.
pub(crate) fn create_terminal_usage_placeholder(
    sessions: &dyn AgentSessionGateway,
    session_id: &str,
) -> Result<String, AgentRuntimeApplicationError> {
    let message = sessions.create_message(NewAgentMessage {
        session_id: session_id.to_string(),
        role: "assistant".to_string(),
        status: "streaming".to_string(),
        content: String::new(),
        file_references: Vec::new(),
    })?;
    Ok(message.id)
}

/// Returns whether a usage record was actually persisted, so callers can log the
/// difference between "correctly found nothing to report" and "wrote real data" —
/// both are `Ok`, but only one means the panel will show anything.
fn persist_terminal_usage(
    totals: TerminalUsageTotals,
    sessions: &dyn AgentSessionGateway,
    message_id: &str,
    session_id: &str,
    agent_id: &str,
    source: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    if totals.input_tokens == 0 && totals.output_tokens == 0 {
        return Ok(false);
    }
    let usage = AgentUsageRecord {
        message_id: message_id.to_string(),
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
    let _ = sessions.complete_message(CompleteAgentMessage {
        message_id: message_id.to_string(),
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
    });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{AgentMessage, ToolUseBlock};
    use serde_json::Value;
    use std::io::Write;
    use std::sync::Mutex;

    /// Minimal `AgentSessionGateway` fake exercising only the two methods the
    /// terminal usage ingestion path calls, to verify that polling reuses a single
    /// message id (one `create_message`, many `complete_message` calls) instead of
    /// creating a fresh message every poll.
    #[derive(Default)]
    struct FakeSessionGateway {
        created: Mutex<Vec<NewAgentMessage>>,
        completed: Mutex<Vec<CompleteAgentMessage>>,
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
            self.completed
                .lock()
                .expect("completed")
                .push(message.clone());
            Ok(AgentMessage {
                id: message.message_id,
                session_id: message.session_id,
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
    fn create_terminal_usage_placeholder_creates_exactly_one_streaming_message() {
        let gateway = FakeSessionGateway::default();

        let message_id = create_terminal_usage_placeholder(&gateway, "session-1").expect("create");

        assert_eq!(message_id, "placeholder-message");
        let created = gateway.created.lock().expect("created");
        assert_eq!(created.len(), 1);
        assert_eq!(created[0].session_id, "session-1");
        assert_eq!(created[0].role, "assistant");
        assert_eq!(created[0].status, "streaming");
    }

    #[test]
    fn repeated_persist_calls_with_the_same_message_id_update_in_place() {
        let gateway = FakeSessionGateway::default();
        let clock = FixedClock;
        let message_id =
            create_terminal_usage_placeholder(&gateway, "session-1").expect("placeholder");

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
        // Exactly one placeholder message was ever created…
        assert_eq!(gateway.created.lock().expect("created").len(), 1);
        // …and both polls completed that same message id, letting the DB's
        // `ON CONFLICT(message_id)` upsert keep only the latest totals.
        let completed = gateway.completed.lock().expect("completed");
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].message_id, message_id);
        assert_eq!(completed[1].message_id, message_id);
        assert_eq!(completed[1].usage.as_ref().expect("usage").input_count, 400);
    }

    #[test]
    fn zero_totals_are_not_persisted_and_do_not_touch_the_gateway() {
        let gateway = FakeSessionGateway::default();
        let clock = FixedClock;

        let persisted = persist_terminal_usage(
            TerminalUsageTotals::default(),
            &gateway,
            "some-message-id",
            "session-1",
            "codex-cli",
            "cli-session-log",
            &clock,
        )
        .expect("zero totals still return Ok");

        assert!(!persisted);
        assert!(gateway.completed.lock().expect("completed").is_empty());
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
}
