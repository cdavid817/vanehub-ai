use super::providers::{
    codex_session_root, find_codex_rollout_since, find_gemini_chat_session,
    find_opencode_session_since, opencode_database_path,
};
use crate::contexts::agent_runtime::application::{AgentClockPort, AgentRuntimeApplicationError};
use crate::contexts::sessions::api::{
    AccountingUnit, MeasurementKind, MeasurementQuality, NewModelInvocation, NewUsageObservation,
    SessionsApi, TokenDimensions, TokenOverlap, UsageCursor, UsageCursorAdvance,
    UsageInteractionKind, UsagePurpose, UsageStatus,
};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct TerminalUsageTotals {
    input: i64,
    output: i64,
    cached_input: i64,
    cache_write_input: i64,
    reasoning_output: i64,
    provider_total: Option<i64>,
}

impl TerminalUsageTotals {
    fn dimensions(self) -> TokenDimensions {
        TokenDimensions {
            input: self.input,
            output: self.output,
            cached_input: self.cached_input,
            cache_write_input: self.cache_write_input,
            reasoning_output: self.reasoning_output,
            provider_total: self.provider_total,
        }
    }

    fn is_zero(self) -> bool {
        self.dimensions().is_zero()
    }
}

#[derive(Debug, Clone)]
struct TerminalUsageEvent {
    identity: String,
    revision: String,
    invocation_started_at: String,
    event_at: Option<String>,
    totals: TerminalUsageTotals,
    cache_overlap: TokenOverlap,
    reasoning_overlap: TokenOverlap,
    normalization_version: &'static str,
    supersedes_revision: Option<String>,
}

pub(crate) fn ingest_claude_terminal_usage(
    session_folder: Option<&str>,
    runtime_session_id: &str,
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    terminal_started_at: &str,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(path) = claude_session_jsonl_path(session_folder, runtime_session_id) else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&path) else {
        return Ok(false);
    };
    let events = read_claude_events(&path, file, terminal_started_at)?;
    persist_events(accounting, session_id, agent_id, runtime_session_id, events)
}

pub(crate) fn ingest_gemini_terminal_usage(
    session_folder: Option<&str>,
    runtime_session_id: &str,
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    terminal_started_at: &str,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(cwd) = session_folder
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(path) = find_gemini_chat_session(&cwd, runtime_session_id).map_err(runtime_error)?
    else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&path) else {
        return Ok(false);
    };
    let events = read_gemini_events(&path, file, terminal_started_at)?;
    persist_events(accounting, session_id, agent_id, runtime_session_id, events)
}

pub(crate) fn ingest_codex_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    terminal_started_at: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(root) = codex_session_root() else {
        return Ok(false);
    };
    let Some(cwd) = session_folder
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(path) = find_codex_rollout_since(&root, &cwd, started_at_ms).map_err(runtime_error)?
    else {
        return Ok(false);
    };
    let Ok(file) = fs::File::open(&path) else {
        return Ok(false);
    };
    let totals = read_codex_cumulative(&path, file)?;
    ingest_cumulative(
        accounting,
        session_id,
        agent_id,
        &format!("codex-{}", stable_hash(&path.to_string_lossy())),
        totals,
        source_ordering_key(&path),
        terminal_started_at,
        clock,
        "codex-terminal-cumulative-v1",
        TokenOverlap::Subset,
        TokenOverlap::Subset,
    )
}

pub(crate) fn ingest_opencode_terminal_usage(
    session_folder: Option<&str>,
    started_at_ms: i64,
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    terminal_started_at: &str,
    clock: &dyn AgentClockPort,
) -> Result<bool, AgentRuntimeApplicationError> {
    let Some(database_path) = opencode_database_path().filter(|path| path.exists()) else {
        return Ok(false);
    };
    let Some(cwd) = session_folder
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
    else {
        return Ok(false);
    };
    let Some(provider_session_id) =
        find_opencode_session_since(&database_path, &cwd, started_at_ms).map_err(runtime_error)?
    else {
        return Ok(false);
    };
    let Some(totals) = read_opencode_session_totals(&database_path, &provider_session_id)? else {
        return Ok(false);
    };
    ingest_cumulative(
        accounting,
        session_id,
        agent_id,
        &provider_session_id,
        totals,
        source_ordering_key(&database_path),
        terminal_started_at,
        clock,
        "opencode-terminal-cumulative-v1",
        TokenOverlap::Exclusive,
        TokenOverlap::Exclusive,
    )
}

pub(crate) fn record_unsupported_terminal_source(
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    terminal_started_at: &str,
) -> Result<bool, AgentRuntimeApplicationError> {
    let invocation = NewModelInvocation {
        id: format!("terminal-unsupported:{}:{}", session_id, agent_id),
        generation_id: None,
        run_id: None,
        operation_id: None,
        session_id: session_id.to_string(),
        message_id: None,
        agent_id: agent_id.to_string(),
        provider_id: Some(agent_id.to_string()),
        profile_id: None,
        endpoint_id: None,
        model_id: None,
        interaction_kind: UsageInteractionKind::TerminalCli,
        purpose: UsagePurpose::TerminalInterval,
        request_sequence: 0,
        attempt: 0,
        started_at: terminal_started_at.to_string(),
    };
    let saved = accounting
        .start_model_invocation(&invocation)
        .map_err(runtime_error)?;
    if saved.status == UsageStatus::Running {
        accounting
            .finalize_model_invocation(&invocation.id, UsageStatus::Succeeded, terminal_started_at)
            .map_err(runtime_error)?;
        return Ok(true);
    }
    Ok(false)
}

fn persist_events(
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    provider_session_id: &str,
    events: Vec<TerminalUsageEvent>,
) -> Result<bool, AgentRuntimeApplicationError> {
    let mut persisted = false;
    for event in events.into_iter().filter(|event| !event.totals.is_zero()) {
        let identity_hash = stable_hash(&event.identity);
        let invocation_id = format!(
            "terminal-event:{}:{}:{}",
            session_id, agent_id, identity_hash
        );
        let source_prefix = format!(
            "terminal:{}:{}:{}:{}",
            agent_id,
            stable_hash(provider_session_id),
            identity_hash,
            event.revision
        );
        let observation_id = format!("{source_prefix}:observation");
        let invocation = NewModelInvocation {
            id: invocation_id.clone(),
            generation_id: None,
            run_id: None,
            operation_id: None,
            session_id: session_id.to_string(),
            message_id: None,
            agent_id: agent_id.to_string(),
            provider_id: Some(agent_id.to_string()),
            profile_id: None,
            endpoint_id: None,
            model_id: None,
            interaction_kind: UsageInteractionKind::TerminalCli,
            purpose: UsagePurpose::TerminalInterval,
            request_sequence: 0,
            attempt: 0,
            started_at: event.invocation_started_at.clone(),
        };
        let saved = accounting
            .start_model_invocation(&invocation)
            .map_err(runtime_error)?;
        let observation = NewUsageObservation {
            id: observation_id,
            invocation_id: invocation_id.clone(),
            quality: MeasurementQuality::Reported,
            unit: AccountingUnit::Tokens,
            measurement_kind: MeasurementKind::Interval,
            dimensions: event.totals.dimensions(),
            cache_overlap: event.cache_overlap,
            reasoning_overlap: event.reasoning_overlap,
            normalization_version: event.normalization_version.to_string(),
            source: "provider-terminal-event".to_string(),
            source_key: source_prefix,
            source_revision: Some(event.revision.clone()),
            supersedes_observation_id: event.supersedes_revision.map(|revision| {
                format!(
                    "terminal:{}:{}:{}:{}:observation",
                    agent_id,
                    stable_hash(provider_session_id),
                    identity_hash,
                    revision
                )
            }),
            event_at: event.event_at,
            observed_at: event.invocation_started_at.clone(),
            provenance_hash: None,
        };
        accounting
            .record_token_observation(&observation)
            .map_err(runtime_error)?;
        if saved.status == UsageStatus::Running {
            accounting
                .finalize_model_invocation(
                    &invocation_id,
                    UsageStatus::Succeeded,
                    &event.invocation_started_at,
                )
                .map_err(runtime_error)?;
        }
        persisted = true;
    }
    Ok(persisted)
}

#[allow(clippy::too_many_arguments)]
fn ingest_cumulative(
    accounting: &SessionsApi,
    session_id: &str,
    agent_id: &str,
    provider_session_id: &str,
    totals: TerminalUsageTotals,
    ordering_key: String,
    terminal_started_at: &str,
    clock: &dyn AgentClockPort,
    normalization_version: &str,
    cache_overlap: TokenOverlap,
    reasoning_overlap: TokenOverlap,
) -> Result<bool, AgentRuntimeApplicationError> {
    if totals.is_zero() {
        return Ok(false);
    }
    let source_id = format!("terminal-cursor:{session_id}:{agent_id}");
    let previous = accounting
        .find_usage_cursor(&source_id)
        .map_err(runtime_error)?;
    if previous
        .as_ref()
        .is_some_and(|cursor| ordering_key <= cursor.ordering_key)
    {
        return Ok(false);
    }
    let current_dimensions = totals.dimensions();
    let reset = previous.as_ref().is_some_and(|cursor| {
        cursor.provider_session_id != provider_session_id
            || dimensions_decreased(cursor.dimensions, current_dimensions)
    });
    let epoch = previous
        .as_ref()
        .map_or(0, |cursor| cursor.epoch + u64::from(reset));
    let delta = match previous.as_ref().filter(|_| !reset) {
        Some(cursor) => subtract_dimensions(current_dimensions, cursor.dimensions),
        None => current_dimensions,
    };
    let revision = previous.as_ref().map_or(0, |cursor| cursor.revision + 1);
    let observed_at = clock.now();
    let invocation_id = format!("terminal-cursor:{session_id}:{agent_id}:{epoch}:{revision}");
    let observation = (!delta.is_zero()).then(|| NewUsageObservation {
        id: format!("{invocation_id}:observation"),
        invocation_id: invocation_id.clone(),
        quality: MeasurementQuality::ReportedDerived,
        unit: AccountingUnit::Tokens,
        measurement_kind: MeasurementKind::CumulativeSnapshot,
        dimensions: delta,
        cache_overlap,
        reasoning_overlap,
        normalization_version: normalization_version.to_string(),
        source: "provider-terminal-cumulative".to_string(),
        source_key: format!("{source_id}:epoch:{epoch}:revision:{revision}"),
        source_revision: Some(ordering_key.clone()),
        supersedes_observation_id: None,
        event_at: None,
        observed_at: observed_at.clone(),
        provenance_hash: Some(stable_hash(&format!(
            "{}:{}:{}",
            previous.as_ref().map_or_else(
                || "initial".to_string(),
                |cursor| cursor.revision.to_string()
            ),
            revision,
            ordering_key
        ))),
    });
    if observation.is_some() {
        accounting
            .start_model_invocation(&NewModelInvocation {
                id: invocation_id.clone(),
                generation_id: None,
                run_id: None,
                operation_id: None,
                session_id: session_id.to_string(),
                message_id: None,
                agent_id: agent_id.to_string(),
                provider_id: Some(agent_id.to_string()),
                profile_id: None,
                endpoint_id: None,
                model_id: None,
                interaction_kind: UsageInteractionKind::TerminalCli,
                purpose: UsagePurpose::TerminalInterval,
                request_sequence: revision.try_into().unwrap_or(u32::MAX),
                attempt: 0,
                started_at: terminal_started_at.to_string(),
            })
            .map_err(runtime_error)?;
    }
    accounting
        .advance_usage_cursor(&UsageCursorAdvance {
            previous,
            current: UsageCursor {
                source_id,
                provider_session_id: provider_session_id.to_string(),
                epoch,
                dimensions: current_dimensions,
                ordering_key,
                source_revision: Some(revision.to_string()),
                revision,
                updated_at: observed_at.clone(),
            },
            observation,
        })
        .map_err(runtime_error)?;
    if !delta.is_zero() {
        accounting
            .finalize_model_invocation(&invocation_id, UsageStatus::Succeeded, &observed_at)
            .map_err(runtime_error)?;
        return Ok(true);
    }
    Ok(false)
}

fn dimensions_decreased(previous: TokenDimensions, current: TokenDimensions) -> bool {
    current.input < previous.input
        || current.output < previous.output
        || current.cached_input < previous.cached_input
        || current.cache_write_input < previous.cache_write_input
        || current.reasoning_output < previous.reasoning_output
        || matches!((previous.provider_total, current.provider_total), (Some(a), Some(b)) if b < a)
}

fn subtract_dimensions(current: TokenDimensions, previous: TokenDimensions) -> TokenDimensions {
    TokenDimensions {
        input: current.input - previous.input,
        output: current.output - previous.output,
        cached_input: current.cached_input - previous.cached_input,
        cache_write_input: current.cache_write_input - previous.cache_write_input,
        reasoning_output: current.reasoning_output - previous.reasoning_output,
        provider_total: match (current.provider_total, previous.provider_total) {
            (Some(current), Some(previous)) => Some(current - previous),
            _ => None,
        },
    }
}

#[derive(Debug, Deserialize)]
struct ClaudeEvent {
    #[serde(rename = "type")]
    event_type: String,
    uuid: Option<String>,
    timestamp: Option<String>,
    message: Option<ClaudeMessage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeMessage {
    id: Option<String>,
    usage: Option<ClaudeUsage>,
}

#[derive(Debug, Deserialize)]
struct ClaudeUsage {
    input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    cache_read_input_tokens: Option<i64>,
    cache_creation_input_tokens: Option<i64>,
}

fn read_claude_events(
    path: &Path,
    file: fs::File,
    fallback_time: &str,
) -> Result<Vec<TerminalUsageEvent>, AgentRuntimeApplicationError> {
    let mut events = Vec::new();
    let mut revisions: HashMap<String, String> = HashMap::new();
    let mut starts: HashMap<String, String> = HashMap::new();
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| file_read_error("Claude", path, error))?;
        let Ok(event) = serde_json::from_str::<ClaudeEvent>(&line) else {
            continue;
        };
        if event.event_type != "assistant" {
            continue;
        }
        let Some(message) = event.message else {
            continue;
        };
        let Some(identity) = message.id.or(event.uuid).and_then(safe_identity) else {
            continue;
        };
        let Some(usage) = message.usage else {
            continue;
        };
        let revision = index.to_string();
        let event_at = event.timestamp;
        let started_at = starts
            .entry(identity.clone())
            .or_insert_with(|| {
                event_at
                    .clone()
                    .unwrap_or_else(|| fallback_time.to_string())
            })
            .clone();
        events.push(TerminalUsageEvent {
            supersedes_revision: revisions.insert(identity.clone(), revision.clone()),
            identity,
            revision,
            invocation_started_at: started_at,
            event_at,
            totals: TerminalUsageTotals {
                input: non_negative(usage.input_tokens),
                output: non_negative(usage.output_tokens),
                cached_input: non_negative(usage.cache_read_input_tokens),
                cache_write_input: non_negative(usage.cache_creation_input_tokens),
                ..TerminalUsageTotals::default()
            },
            cache_overlap: TokenOverlap::Exclusive,
            reasoning_overlap: TokenOverlap::Subset,
            normalization_version: "claude-terminal-message-v1",
        });
    }
    Ok(events)
}

#[derive(Debug, Deserialize)]
struct GeminiRecord {
    id: Option<String>,
    timestamp: Option<String>,
    #[serde(rename = "type")]
    message_type: Option<String>,
    tokens: Option<GeminiTokens>,
    #[serde(rename = "$set")]
    set: Option<GeminiSet>,
}

#[derive(Debug, Deserialize)]
struct GeminiSet {
    messages: Option<Vec<GeminiRecord>>,
}

#[derive(Debug, Deserialize)]
struct GeminiTokens {
    input: Option<i64>,
    output: Option<i64>,
    cached: Option<i64>,
    thoughts: Option<i64>,
    total: Option<i64>,
}

fn read_gemini_events(
    path: &Path,
    file: fs::File,
    fallback_time: &str,
) -> Result<Vec<TerminalUsageEvent>, AgentRuntimeApplicationError> {
    let mut events = Vec::new();
    let mut revisions = HashMap::new();
    let mut starts = HashMap::new();
    for (index, line) in std::io::BufReader::new(file).lines().enumerate() {
        let line = line.map_err(|error| file_read_error("Gemini", path, error))?;
        let Ok(record) = serde_json::from_str::<GeminiRecord>(&line) else {
            continue;
        };
        collect_gemini_record(
            record,
            index,
            fallback_time,
            &mut revisions,
            &mut starts,
            &mut events,
        );
    }
    Ok(events)
}

fn collect_gemini_record(
    record: GeminiRecord,
    index: usize,
    fallback_time: &str,
    revisions: &mut HashMap<String, String>,
    starts: &mut HashMap<String, String>,
    events: &mut Vec<TerminalUsageEvent>,
) {
    if let Some(messages) = record.set.and_then(|set| set.messages) {
        for message in messages {
            collect_gemini_record(message, index, fallback_time, revisions, starts, events);
        }
        return;
    }
    if record.message_type.as_deref() != Some("gemini") {
        return;
    }
    let Some(identity) = record.id.and_then(safe_identity) else {
        return;
    };
    let Some(tokens) = record.tokens else {
        return;
    };
    let revision = index.to_string();
    let event_at = record.timestamp;
    let started_at = starts
        .entry(identity.clone())
        .or_insert_with(|| {
            event_at
                .clone()
                .unwrap_or_else(|| fallback_time.to_string())
        })
        .clone();
    events.push(TerminalUsageEvent {
        supersedes_revision: revisions.insert(identity.clone(), revision.clone()),
        identity,
        revision,
        invocation_started_at: started_at,
        event_at,
        totals: TerminalUsageTotals {
            input: non_negative(tokens.input),
            output: non_negative(tokens.output),
            cached_input: non_negative(tokens.cached),
            reasoning_output: non_negative(tokens.thoughts),
            provider_total: tokens.total.map(|value| value.max(0)),
            ..TerminalUsageTotals::default()
        },
        cache_overlap: TokenOverlap::Subset,
        reasoning_overlap: TokenOverlap::Exclusive,
        normalization_version: "gemini-terminal-message-v1",
    });
}

#[derive(Debug, Deserialize)]
struct CodexLine {
    #[serde(rename = "type")]
    line_type: String,
    payload: Option<CodexPayload>,
}

#[derive(Debug, Deserialize)]
struct CodexPayload {
    #[serde(rename = "type")]
    payload_type: Option<String>,
    info: Option<CodexInfo>,
}

#[derive(Debug, Deserialize)]
struct CodexInfo {
    total_token_usage: Option<CodexUsage>,
}

#[derive(Debug, Deserialize)]
struct CodexUsage {
    input_tokens: Option<i64>,
    cached_input_tokens: Option<i64>,
    cache_write_input_tokens: Option<i64>,
    output_tokens: Option<i64>,
    reasoning_output_tokens: Option<i64>,
    total_tokens: Option<i64>,
}

fn read_codex_cumulative(
    path: &Path,
    file: fs::File,
) -> Result<TerminalUsageTotals, AgentRuntimeApplicationError> {
    let mut latest = TerminalUsageTotals::default();
    for line in std::io::BufReader::new(file).lines() {
        let line = line.map_err(|error| file_read_error("Codex", path, error))?;
        let Ok(event) = serde_json::from_str::<CodexLine>(&line) else {
            continue;
        };
        let Some(payload) = event.payload.filter(|payload| {
            event.line_type == "event_msg" && payload.payload_type.as_deref() == Some("token_count")
        }) else {
            continue;
        };
        let Some(usage) = payload.info.and_then(|info| info.total_token_usage) else {
            continue;
        };
        latest = TerminalUsageTotals {
            input: non_negative(usage.input_tokens),
            output: non_negative(usage.output_tokens),
            cached_input: non_negative(usage.cached_input_tokens),
            cache_write_input: non_negative(usage.cache_write_input_tokens),
            reasoning_output: non_negative(usage.reasoning_output_tokens),
            provider_total: usage.total_tokens.map(|value| value.max(0)),
        };
    }
    Ok(latest)
}

fn read_opencode_session_totals(
    path: &Path,
    session_id: &str,
) -> Result<Option<TerminalUsageTotals>, AgentRuntimeApplicationError> {
    let connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(runtime_error)?;
    connection
        .query_row(
            "SELECT tokens_input, tokens_output, tokens_reasoning, tokens_cache_read, tokens_cache_write FROM session WHERE id = ?1",
            [session_id],
            |row| {
                Ok(TerminalUsageTotals {
                    input: row.get::<_, i64>(0)?.max(0),
                    output: row.get::<_, i64>(1)?.max(0),
                    reasoning_output: row.get::<_, i64>(2)?.max(0),
                    cached_input: row.get::<_, i64>(3)?.max(0),
                    cache_write_input: row.get::<_, i64>(4)?.max(0),
                    provider_total: None,
                })
            },
        )
        .optional()
        .map_err(runtime_error)
}

fn claude_project_dir_name(cwd: &Path) -> String {
    let raw = cwd.to_string_lossy();
    raw.strip_prefix(r"\\?\")
        .unwrap_or(&raw)
        .replace([':', '\\'], "-")
}

fn claude_session_jsonl_path(
    session_folder: Option<&str>,
    runtime_session_id: &str,
) -> Option<PathBuf> {
    let cwd = session_folder
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)?;
    Some(
        dirs::home_dir()?
            .join(".claude")
            .join("projects")
            .join(claude_project_dir_name(&cwd))
            .join(format!("{runtime_session_id}.jsonl")),
    )
}

fn source_ordering_key(path: &Path) -> String {
    let metadata = fs::metadata(path).ok();
    let modified = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |duration| duration.as_millis());
    let length = metadata.map_or(0, |metadata| metadata.len());
    format!("{modified:020}:{length:020}")
}

fn safe_identity(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()
        && value.len() <= 128
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_:/".contains(character)))
    .then(|| value.to_string())
}

fn stable_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    digest[..8]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn non_negative(value: Option<i64>) -> i64 {
    value.unwrap_or(0).max(0)
}

fn runtime_error(error: impl std::fmt::Display) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Process(error.to_string())
}

fn file_read_error(
    provider: &str,
    path: &Path,
    error: std::io::Error,
) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Process(format!(
        "Failed to read {provider} usage source {}: {error}",
        stable_hash(&path.to_string_lossy())
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn claude_revisions_keep_stable_identity_and_supersession() {
        let mut file = tempfile::NamedTempFile::new().expect("fixture");
        writeln!(file, r#"{{"type":"assistant","uuid":"rev-a","timestamp":"2026-01-01T00:00:00Z","message":{{"id":"msg-a","usage":{{"input_tokens":5,"output_tokens":2}}}}}}"#).expect("write");
        writeln!(file, r#"{{"type":"assistant","uuid":"rev-b","timestamp":"2026-01-01T00:00:01Z","message":{{"id":"msg-a","usage":{{"input_tokens":6,"output_tokens":3}}}}}}"#).expect("write");

        let events = read_claude_events(
            file.path(),
            fs::File::open(file.path()).expect("open"),
            "fallback",
        )
        .expect("events");

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].identity, events[1].identity);
        assert_eq!(events[1].supersedes_revision.as_deref(), Some("0"));
        assert_eq!(events[1].totals.input, 6);
    }

    #[test]
    fn gemini_snapshots_preserve_reasoning_as_a_dimension() {
        let mut file = tempfile::NamedTempFile::new().expect("fixture");
        writeln!(file, r#"{{"$set":{{"messages":[{{"id":"msg-g","type":"gemini","timestamp":"2026-01-01T00:00:00Z","tokens":{{"input":8,"output":3,"cached":2,"thoughts":4,"total":15}}}}]}}}}"#).expect("write");

        let events = read_gemini_events(
            file.path(),
            fs::File::open(file.path()).expect("open"),
            "fallback",
        )
        .expect("events");

        assert_eq!(events.len(), 1);
        assert_eq!(events[0].totals.output, 3);
        assert_eq!(events[0].totals.reasoning_output, 4);
        assert_eq!(events[0].totals.provider_total, Some(15));
    }

    #[test]
    fn cumulative_reset_opens_a_non_negative_epoch_delta() {
        let previous = TokenDimensions {
            input: 100,
            output: 50,
            ..TokenDimensions::default()
        };
        let current = TokenDimensions {
            input: 4,
            output: 2,
            ..TokenDimensions::default()
        };

        assert!(dimensions_decreased(previous, current));
        assert_eq!(current.input, 4);
    }

    #[test]
    fn source_errors_hash_private_paths() {
        let error = file_read_error(
            "fixture",
            Path::new(r"C:\private-user\secret\raw.jsonl"),
            std::io::Error::other("broken"),
        );
        let message = error.to_string();
        assert!(!message.contains("private-user"));
        assert!(!message.contains("raw.jsonl"));
    }
}
