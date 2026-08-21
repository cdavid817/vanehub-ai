use crate::contexts::agent_runtime::application::{GenerationProcessFailure, ProviderOutputFormat};
use crate::contexts::execution_observability::api::ExecutionFidelity;
use serde_json::Value;
use std::collections::VecDeque;
use std::io::Read;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderOutputStream {
    Stdout,
    #[cfg(test)]
    Stderr,
}

#[derive(Debug)]
pub(crate) struct ProviderOutputFramer {
    stdout: Vec<u8>,
    #[cfg(test)]
    stderr: Vec<u8>,
    max_buffer_bytes: usize,
}

impl ProviderOutputFramer {
    pub(crate) fn new(max_buffer_bytes: usize) -> Self {
        Self {
            stdout: Vec::new(),
            #[cfg(test)]
            stderr: Vec::new(),
            max_buffer_bytes,
        }
    }

    pub(crate) fn push(
        &mut self,
        stream: ProviderOutputStream,
        chunk: &[u8],
    ) -> Result<Vec<String>, &'static str> {
        let buffer = match stream {
            ProviderOutputStream::Stdout => &mut self.stdout,
            #[cfg(test)]
            ProviderOutputStream::Stderr => &mut self.stderr,
        };
        if buffer.len().saturating_add(chunk.len()) > self.max_buffer_bytes {
            buffer.clear();
            return Err("provider output record exceeds the bounded parser limit");
        }
        buffer.extend_from_slice(chunk);
        let mut lines = Vec::new();
        while let Some(index) = buffer.iter().position(|byte| *byte == b'\n') {
            let mut record = buffer.drain(..=index).collect::<Vec<_>>();
            record.pop();
            if record.last() == Some(&b'\r') {
                record.pop();
            }
            lines.push(
                String::from_utf8(record).map_err(|_| "provider output contains invalid UTF-8")?,
            );
        }
        Ok(lines)
    }

    pub(crate) fn finish(
        &mut self,
        stream: ProviderOutputStream,
    ) -> Result<Option<String>, &'static str> {
        let buffer = match stream {
            ProviderOutputStream::Stdout => &mut self.stdout,
            #[cfg(test)]
            ProviderOutputStream::Stderr => &mut self.stderr,
        };
        if buffer.is_empty() {
            return Ok(None);
        }
        String::from_utf8(std::mem::take(buffer))
            .map(Some)
            .map_err(|_| "provider output ends with invalid UTF-8")
    }
}

pub(crate) struct BoundedProviderLines<R> {
    reader: R,
    framer: ProviderOutputFramer,
    pending: VecDeque<String>,
    finished: bool,
}

impl<R: Read> BoundedProviderLines<R> {
    pub(crate) fn new(reader: R, max_record_bytes: usize) -> Self {
        Self {
            reader,
            framer: ProviderOutputFramer::new(max_record_bytes),
            pending: VecDeque::new(),
            finished: false,
        }
    }
}

impl<R: Read> Iterator for BoundedProviderLines<R> {
    type Item = Result<String, &'static str>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(line) = self.pending.pop_front() {
                return Some(Ok(line));
            }
            if self.finished {
                return None;
            }
            let mut chunk = [0_u8; 8 * 1024];
            match self.reader.read(&mut chunk) {
                Ok(0) => {
                    self.finished = true;
                    return self.framer.finish(ProviderOutputStream::Stdout).transpose();
                }
                Ok(length) => match self
                    .framer
                    .push(ProviderOutputStream::Stdout, &chunk[..length])
                {
                    Ok(lines) => self.pending.extend(lines),
                    Err(error) => {
                        self.finished = true;
                        return Some(Err(error));
                    }
                },
                Err(_) => {
                    self.finished = true;
                    return Some(Err("failed to read provider output"));
                }
            }
        }
    }
}

#[cfg(test)]
mod framer_tests {
    use super::*;

    #[test]
    fn chunk_partitions_preserve_utf8_and_stream_isolation() {
        let source = "alpha\n中文🙂\nomega".as_bytes();
        for width in 1..=source.len() {
            let mut framer = ProviderOutputFramer::new(1024);
            let mut lines = Vec::new();
            for chunk in source.chunks(width) {
                lines.extend(
                    framer
                        .push(ProviderOutputStream::Stdout, chunk)
                        .expect("valid chunk"),
                );
            }
            lines.extend(
                framer
                    .finish(ProviderOutputStream::Stdout)
                    .expect("valid tail"),
            );
            assert_eq!(lines, ["alpha", "中文🙂", "omega"]);
        }

        let mut framer = ProviderOutputFramer::new(1024);
        assert!(framer
            .push(ProviderOutputStream::Stdout, b"out")
            .expect("stdout")
            .is_empty());
        assert_eq!(
            framer
                .push(ProviderOutputStream::Stderr, b"err\n")
                .expect("stderr"),
            ["err"]
        );
        assert_eq!(
            framer.finish(ProviderOutputStream::Stdout).expect("tail"),
            Some("out".to_string())
        );
    }

    #[test]
    fn malformed_and_oversized_records_fail_closed() {
        let mut framer = ProviderOutputFramer::new(4);
        assert!(framer.push(ProviderOutputStream::Stdout, b"12345").is_err());
        let mut framer = ProviderOutputFramer::new(1024);
        assert!(framer
            .push(ProviderOutputStream::Stdout, &[0xff, b'\n'])
            .is_err());
    }

    #[test]
    fn bounded_line_iterator_preserves_tail_and_classifies_failures() {
        let lines = BoundedProviderLines::new(&b"one\ntwo"[..], 16)
            .collect::<Result<Vec<_>, _>>()
            .expect("bounded lines");
        assert_eq!(lines, ["one", "two"]);

        let error = BoundedProviderLines::new(&b"oversized"[..], 4)
            .collect::<Result<Vec<_>, _>>()
            .expect_err("oversized record");
        assert_eq!(
            error,
            "provider output record exceeds the bounded parser limit"
        );
    }

    #[test]
    fn provider_parser_fixed_fixture_benchmark() {
        let record = b"{\"type\":\"item.completed\",\"item\":{\"type\":\"agent_message\",\"text\":\"ok\"}}\n";
        let fixture = record.repeat(10_000);
        let started = std::time::Instant::now();
        let lines = BoundedProviderLines::new(fixture.as_slice(), 262_144)
            .collect::<Result<Vec<_>, _>>()
            .expect("fixed parser fixture");
        let elapsed = started.elapsed();
        eprintln!(
            "provider_sdk parser_fixture_bytes={} records={} elapsed={elapsed:?} target={} arch={}",
            fixture.len(),
            lines.len(),
            std::env::consts::OS,
            std::env::consts::ARCH
        );
        assert_eq!(lines.len(), 10_000);
        assert_eq!(lines[0].as_bytes(), &record[..record.len() - 1]);
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProviderOutputEvent {
    Token(String),
    Thinking(String),
    ToolLifecycle(Box<ProviderToolEvent>),
    RichBlock(Value),
    SessionId(String),
    Completed(Option<ProviderReportedUsage>),
    Failed(GenerationProcessFailure),
    Empty,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum ProviderUsageOverlap {
    Subset,
    Exclusive,
    #[default]
    Unknown,
}

/// Provider-native completion usage with explicit, versioned dimension semantics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ProviderReportedUsage {
    pub(crate) input_tokens: i64,
    pub(crate) output_tokens: i64,
    pub(crate) cache_read_tokens: i64,
    pub(crate) cache_creation_tokens: i64,
    pub(crate) reasoning_output_tokens: i64,
    pub(crate) provider_total_tokens: Option<i64>,
    pub(crate) cache_overlap: ProviderUsageOverlap,
    pub(crate) reasoning_overlap: ProviderUsageOverlap,
    pub(crate) normalization_version: &'static str,
    pub(crate) model_id: Option<String>,
    pub(crate) source_identity: Option<String>,
    pub(crate) source_revision: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderToolPhase {
    Started,
    Updated,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ProviderToolEvent {
    pub(crate) call_id: Option<String>,
    pub(crate) name: Option<String>,
    pub(crate) input: Option<Value>,
    pub(crate) output: Option<Value>,
    pub(crate) phase: ProviderToolPhase,
    pub(crate) provider_timestamp: Option<String>,
    pub(crate) status: String,
    pub(crate) fidelity: ExecutionFidelity,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) parent_trace_id: Option<String>,
    pub(crate) parent_span_id: Option<String>,
    pub(crate) delegation_id: Option<String>,
    pub(crate) attempt: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserKind {
    Claude,
    StructuredJson,
    Antigravity,
    #[cfg(test)]
    GenericLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderOutputParser {
    kind: ParserKind,
}

#[cfg(test)]
pub(crate) fn output_parser_for(agent_id: &str) -> ProviderOutputParser {
    match agent_id {
        "claude-code" => output_parser_for_format(ProviderOutputFormat::ClaudeStreamJson),
        "codex-cli" | "gemini-cli" | "opencode" => {
            output_parser_for_format(ProviderOutputFormat::StructuredJsonLines)
        }
        "antigravity-cli" => output_parser_for_format(ProviderOutputFormat::AntigravityStreamJson),
        _ => ProviderOutputParser {
            kind: ParserKind::GenericLine,
        },
    }
}

pub(crate) fn output_parser_for_format(format: ProviderOutputFormat) -> ProviderOutputParser {
    let kind = match format {
        ProviderOutputFormat::ClaudeStreamJson => ParserKind::Claude,
        ProviderOutputFormat::StructuredJsonLines => ParserKind::StructuredJson,
        ProviderOutputFormat::AntigravityStreamJson => ParserKind::Antigravity,
    };
    ProviderOutputParser { kind }
}

impl ProviderOutputParser {
    pub(crate) fn parse_line(&self, line: &str) -> ProviderOutputEvent {
        match self.kind {
            ParserKind::Claude => parse_claude_line(line),
            ParserKind::StructuredJson => parse_structured_json_line(line),
            ParserKind::Antigravity => parse_antigravity_line(line),
            #[cfg(test)]
            ParserKind::GenericLine => parse_generic_line(line),
        }
    }
}

/// Test-only since the claude parser stopped falling back to it for unrecognised structured
/// events; `ParserKind::GenericLine`, its one remaining caller, is itself `#[cfg(test)]`.
#[cfg(test)]
fn parse_generic_line(line: &str) -> ProviderOutputEvent {
    if line.trim().is_empty() {
        ProviderOutputEvent::Empty
    } else {
        ProviderOutputEvent::Token(line.to_string())
    }
}

/// Antigravity's `stream-json` wraps each event as `{"event":"<kind>","<kind>":{...}}` — verified
/// against a real run, not inferred from the flat `{"type":...}` shape the other CLIs use.
///
/// `result` is mapped in full because its payload is captured verbatim. `init` is read only for
/// `conversation_id`, and `step_update` is deliberately consumed without emitting increments: its
/// payload shape has not been observed on a live authenticated run, and inventing field names
/// would produce a parser that silently drops real output. The completed turn still carries the
/// whole reply through `result.response`, so a run is usable — it just is not token-by-token yet.
fn parse_antigravity_line(line: &str) -> ProviderOutputEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ProviderOutputEvent::Empty;
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return ProviderOutputEvent::Empty;
    };
    let Some(kind) = value.get("event").and_then(Value::as_str) else {
        return ProviderOutputEvent::Empty;
    };
    let payload = value.get(kind);

    match kind {
        "init" => payload
            .and_then(|payload| payload.get("conversation_id"))
            .and_then(Value::as_str)
            .filter(|id| !id.trim().is_empty())
            .map_or(ProviderOutputEvent::Empty, |id| {
                ProviderOutputEvent::SessionId(id.to_string())
            }),
        "result" => antigravity_result_event(payload),
        // Unknown kinds — including `step_update` until its payload is captured — must not fail the
        // run. A stricter parser would turn every future Antigravity event into a hard error.
        _ => ProviderOutputEvent::Empty,
    }
}

fn antigravity_result_event(payload: Option<&Value>) -> ProviderOutputEvent {
    let Some(payload) = payload else {
        return ProviderOutputEvent::Empty;
    };
    let status = payload
        .get("status")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let diagnostic = payload
        .get("error")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();

    match status {
        "SUCCESS" => ProviderOutputEvent::Completed(antigravity_usage(payload)),
        // The parser vocabulary has no cancelled variant — cancellation is driven by an explicit
        // stop at the chat layer, not inferred from provider output — so a self-reported cancel
        // surfaces as a non-retryable failure carrying the CLI's own wording. Retrying it would
        // not help either way.
        "CANCELED" | "INTERRUPTED" => ProviderOutputEvent::Failed(
            GenerationProcessFailure::non_retryable(fallback_diagnostic(&diagnostic, status)),
        ),
        // Verified from a real unauthenticated run: the process exits 1 and still emits a
        // well-formed terminal event whose error names the authentication failure.
        "ERROR" | "INVALID" => ProviderOutputEvent::Failed(
            GenerationProcessFailure::non_retryable(fallback_diagnostic(&diagnostic, status)),
        ),
        // `WAITING` and `RUNNING` are non-terminal; seeing one on a terminal event means the
        // contract changed, which is worth failing loudly rather than reporting a silent success.
        other => ProviderOutputEvent::Failed(GenerationProcessFailure::non_retryable(format!(
            "Antigravity reported a non-terminal result status: {other}"
        ))),
    }
}

fn fallback_diagnostic(diagnostic: &str, status: &str) -> String {
    if diagnostic.trim().is_empty() {
        format!("Antigravity ended the turn with status {status}")
    } else {
        diagnostic.to_string()
    }
}

fn antigravity_usage(payload: &Value) -> Option<ProviderReportedUsage> {
    let usage = payload.get("usage")?;
    // The verified v1.1.11 result total equals fresh input + output + thinking + cache read, so
    // these fields are exclusive rather than subsets or values that should be folded together.
    non_degenerate(ProviderReportedUsage {
        input_tokens: non_negative_i64(usage, "input_tokens"),
        output_tokens: non_negative_i64(usage, "output_tokens"),
        cache_read_tokens: non_negative_i64(usage, "cache_read_tokens"),
        cache_creation_tokens: 0,
        reasoning_output_tokens: non_negative_i64(usage, "thinking_tokens"),
        provider_total_tokens: optional_non_negative_i64(usage, "total_tokens"),
        cache_overlap: ProviderUsageOverlap::Exclusive,
        reasoning_overlap: ProviderUsageOverlap::Exclusive,
        normalization_version: "antigravity-result-usage-v2",
        source_identity: bounded_safe_identity(payload.get("conversation_id")),
        ..ProviderReportedUsage::default()
    })
}

fn parse_claude_line(line: &str) -> ProviderOutputEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ProviderOutputEvent::Empty;
    }

    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return ProviderOutputEvent::Token(line.to_string());
    };
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();

    match event_type {
        "rich_block" => value
            .get("block")
            .filter(|block| valid_rich_block(block))
            .cloned()
            .map(ProviderOutputEvent::RichBlock)
            .unwrap_or(ProviderOutputEvent::Empty),
        "system" | "session_init" => value
            .get("session_id")
            .or_else(|| value.get("sessionId"))
            .and_then(Value::as_str)
            .map(|session_id| ProviderOutputEvent::SessionId(session_id.to_string()))
            .unwrap_or(ProviderOutputEvent::Empty),
        "assistant" | "assistant_delta" | "content_block_delta" => {
            let text = value
                .pointer("/message/content/0/text")
                .or_else(|| value.pointer("/delta/text"))
                .or_else(|| value.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.is_empty() {
                ProviderOutputEvent::Empty
            } else {
                ProviderOutputEvent::Token(text.to_string())
            }
        }
        "thinking" | "thinking_delta" => {
            let text = value
                .pointer("/delta/thinking")
                .or_else(|| value.get("thinking"))
                .or_else(|| value.get("text"))
                .and_then(Value::as_str)
                .unwrap_or_default();
            if text.is_empty() {
                ProviderOutputEvent::Empty
            } else {
                ProviderOutputEvent::Thinking(text.to_string())
            }
        }
        "tool_use" | "tool_result" | "tool_error" | "tool_failure" => {
            ProviderOutputEvent::ToolLifecycle(Box::new(parse_tool_event(&value, event_type)))
        }
        "result" | "complete" | "completed" => {
            // claude-code reports failures through `result` with `is_error`, never through the
            // `error` arm below, and leaves `subtype` as "success" — so this flag is the only
            // trustworthy signal that the run failed.
            if value.get("is_error").and_then(Value::as_bool) == Some(true) {
                ProviderOutputEvent::Failed(provider_failure(
                    &value,
                    "Agent CLI reported a failed result.",
                ))
            } else {
                ProviderOutputEvent::Completed(claude_usage(&value))
            }
        }
        "error" | "failed" => {
            ProviderOutputEvent::Failed(provider_failure(&value, "Agent output reported an error."))
        }
        // A line that parsed as JSON is a structured event whether or not this parser has an arm
        // for it, so falling back to `parse_generic_line` here published the envelope itself as
        // the Agent's words. That is not hypothetical: `--include-partial-messages` is in
        // VaneHub's own argv, and one turn emits eight `stream_event` wrappers plus a
        // `rate_limit_event` around the single `assistant` event that carries the reply. The
        // raw-text fallback above still covers output that is not JSON at all, which is the case
        // it was written for.
        _ => ProviderOutputEvent::Empty,
    }
}

fn parse_structured_json_line(line: &str) -> ProviderOutputEvent {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return ProviderOutputEvent::Empty;
    }
    let Ok(value) = serde_json::from_str::<Value>(trimmed) else {
        return ProviderOutputEvent::Token(line.to_string());
    };
    let event_type = value
        .get("type")
        .or_else(|| value.get("event"))
        .and_then(Value::as_str)
        .unwrap_or_default();

    if matches!(
        event_type,
        "error" | "failed" | "failure" | "turn.failed" | "run_error"
    ) {
        return ProviderOutputEvent::Failed(provider_failure(
            &value,
            "Agent CLI reported an error.",
        ));
    }
    if matches!(
        event_type,
        "result"
            | "done"
            | "complete"
            | "completed"
            | "turn.completed"
            | "step_finish"
            | "step-finish"
    ) {
        // `result` is gemini-cli's completion shape here (claude-code's own `result`
        // line never reaches this function — it goes through `parse_claude_line`
        // instead); `done`/`complete`/`completed` are untyped generic terminal
        // markers with no known usage shape to parse.
        let usage = match event_type {
            "turn.completed" => codex_usage(&value),
            "result" => gemini_usage(&value),
            "step_finish" | "step-finish" => opencode_usage(&value),
            _ => None,
        };
        return ProviderOutputEvent::Completed(usage);
    }
    if let Some(session_id) = session_id(&value) {
        if matches!(
            event_type,
            "session"
                | "session_init"
                | "session_configured"
                | "start"
                | "started"
                | "thread.started"
                | "conversation.started"
                | "step_start"
                | "step-start"
        ) {
            return ProviderOutputEvent::SessionId(session_id);
        }
    }
    if matches!(
        event_type,
        "thinking" | "thinking_delta" | "reasoning" | "reasoning_delta"
    ) {
        return thinking_value(&value)
            .map(ProviderOutputEvent::Thinking)
            .unwrap_or(ProviderOutputEvent::Empty);
    }
    if event_type == "rich_block" {
        return value
            .get("block")
            .filter(|block| valid_rich_block(block))
            .cloned()
            .map(ProviderOutputEvent::RichBlock)
            .unwrap_or(ProviderOutputEvent::Empty);
    }
    if is_tool_event(&value, event_type) {
        return ProviderOutputEvent::ToolLifecycle(Box::new(parse_tool_event(&value, event_type)));
    }
    text_value(&value)
        .map(ProviderOutputEvent::Token)
        .unwrap_or(ProviderOutputEvent::Empty)
}

/// A usage payload that is present but all-zero (e.g. a CLI error response that still
/// emits a zero-filled usage block) is treated as absent rather than a valid reported
/// zero, so it falls back to the estimated/character-count path instead of persisting
/// a permanently-stuck zero. See `add-reported-usage-ingestion` design.md Decision 4.
fn non_degenerate(usage: ProviderReportedUsage) -> Option<ProviderReportedUsage> {
    if usage.input_tokens == 0
        && usage.output_tokens == 0
        && usage.cache_read_tokens == 0
        && usage.cache_creation_tokens == 0
        && usage.reasoning_output_tokens == 0
        && usage.provider_total_tokens.unwrap_or_default() == 0
    {
        None
    } else {
        Some(usage)
    }
}

fn non_negative_i64(value: &Value, key: &str) -> i64 {
    value.get(key).and_then(Value::as_i64).unwrap_or(0).max(0)
}

fn optional_non_negative_i64(value: &Value, key: &str) -> Option<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .map(|count| count.max(0))
}

fn gemini_model_stat(stats: &Value, key: &str) -> Option<i64> {
    let models = stats.get("models")?.as_object()?;
    let mut found = false;
    let total = models.values().fold(0_i64, |total, model| {
        model
            .get(key)
            .and_then(Value::as_i64)
            .map_or(total, |count| {
                found = true;
                total.saturating_add(count.max(0))
            })
    });
    found.then_some(total)
}

fn gemini_stat(stats: &Value, key: &str) -> i64 {
    stats
        .get(key)
        .and_then(Value::as_i64)
        .map(|count| count.max(0))
        .or_else(|| gemini_model_stat(stats, key))
        .unwrap_or_default()
}

fn gemini_model_id(stats: &Value) -> Option<String> {
    let models = stats.get("models")?.as_object()?;
    let mut names = models.keys();
    let name = names.next()?;
    if names.next().is_some()
        || name.is_empty()
        || name.len() > 128
        || !name
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_/:".contains(character))
    {
        return None;
    }
    Some(name.clone())
}

/// claude-code `result` line: `{"usage":{"input_tokens","output_tokens","cache_creation_input_tokens","cache_read_input_tokens"}}`.
fn claude_usage(value: &Value) -> Option<ProviderReportedUsage> {
    let usage = value.get("usage")?;
    non_degenerate(ProviderReportedUsage {
        input_tokens: non_negative_i64(usage, "input_tokens"),
        output_tokens: non_negative_i64(usage, "output_tokens"),
        cache_read_tokens: non_negative_i64(usage, "cache_read_input_tokens"),
        cache_creation_tokens: non_negative_i64(usage, "cache_creation_input_tokens"),
        provider_total_tokens: optional_non_negative_i64(usage, "total_tokens"),
        cache_overlap: ProviderUsageOverlap::Exclusive,
        reasoning_overlap: ProviderUsageOverlap::Subset,
        normalization_version: "claude-code-result-usage-v1",
        ..ProviderReportedUsage::default()
    })
}

/// codex-cli `turn.completed` line: `{"usage":{"input_tokens","cached_input_tokens","cache_write_input_tokens","output_tokens","reasoning_output_tokens"}}`.
fn codex_usage(value: &Value) -> Option<ProviderReportedUsage> {
    let usage = value.get("usage")?;
    non_degenerate(ProviderReportedUsage {
        input_tokens: non_negative_i64(usage, "input_tokens"),
        output_tokens: non_negative_i64(usage, "output_tokens"),
        cache_read_tokens: non_negative_i64(usage, "cached_input_tokens"),
        cache_creation_tokens: non_negative_i64(usage, "cache_write_input_tokens"),
        reasoning_output_tokens: non_negative_i64(usage, "reasoning_output_tokens"),
        provider_total_tokens: optional_non_negative_i64(usage, "total_tokens"),
        cache_overlap: ProviderUsageOverlap::Subset,
        reasoning_overlap: ProviderUsageOverlap::Subset,
        normalization_version: "codex-turn-completed-usage-v1",
        ..ProviderReportedUsage::default()
    })
}

/// gemini-cli `result` line: `{"stats":{"input_tokens","output_tokens","cached","total_tokens"}}`.
/// gemini-cli's stream-json stats have no separate cache-write figure.
fn gemini_usage(value: &Value) -> Option<ProviderReportedUsage> {
    let stats = value.get("stats")?;
    non_degenerate(ProviderReportedUsage {
        input_tokens: gemini_stat(stats, "input_tokens"),
        output_tokens: gemini_stat(stats, "output_tokens"),
        cache_read_tokens: gemini_stat(stats, "cached"),
        cache_creation_tokens: 0,
        reasoning_output_tokens: gemini_stat(stats, "thoughts"),
        provider_total_tokens: optional_non_negative_i64(stats, "total_tokens")
            .or_else(|| gemini_model_stat(stats, "total_tokens")),
        cache_overlap: ProviderUsageOverlap::Subset,
        reasoning_overlap: ProviderUsageOverlap::Exclusive,
        normalization_version: "gemini-result-stream-stats-v1",
        model_id: gemini_model_id(stats),
        ..ProviderReportedUsage::default()
    })
}

fn bounded_safe_identity(value: Option<&Value>) -> Option<String> {
    let identity = value?.as_str()?.trim();
    (!identity.is_empty()
        && identity.len() <= 128
        && identity
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || ".-_:/".contains(character)))
    .then(|| identity.to_string())
}

fn opencode_revision(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .and_then(Value::as_u64)
        .map(|timestamp| timestamp.to_string())
        .or_else(|| bounded_safe_identity(value.get("revision")))
}

/// opencode `step_finish`/`step-finish` line: `{"timestamp":...,"part":{"id":...,"tokens":{"total","input","output","reasoning","cache":{"read","write"}}}}`.
/// OpenCode normalizes these categories as exclusive dimensions before emitting the part.
fn opencode_usage(value: &Value) -> Option<ProviderReportedUsage> {
    let tokens = value.pointer("/part/tokens")?;
    non_degenerate(ProviderReportedUsage {
        input_tokens: non_negative_i64(tokens, "input"),
        output_tokens: non_negative_i64(tokens, "output"),
        cache_read_tokens: tokens
            .pointer("/cache/read")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0),
        cache_creation_tokens: tokens
            .pointer("/cache/write")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .max(0),
        reasoning_output_tokens: non_negative_i64(tokens, "reasoning"),
        provider_total_tokens: optional_non_negative_i64(tokens, "total"),
        cache_overlap: ProviderUsageOverlap::Exclusive,
        reasoning_overlap: ProviderUsageOverlap::Exclusive,
        normalization_version: "opencode-step-finish-tokens-v1",
        source_identity: bounded_safe_identity(value.pointer("/part/id")),
        source_revision: opencode_revision(value),
        ..ProviderReportedUsage::default()
    })
}

fn text_value(value: &Value) -> Option<String> {
    [
        "/delta/text",
        "/message/content/0/text",
        "/item/content/0/text",
        "/item/content/0/output_text",
        "/item/content/0/text/text",
        "/item/content/0/text/value",
        "/content/0/text",
        "/content/0/output_text",
        "/content/text",
        "/data/text",
        "/data/message",
        "/part/text",
        "/delta",
        "/message",
        "/output_text",
        "/response/output_text",
        "/response/output/0/content/0/text",
        "/response/output/0/content/0/output_text",
    ]
    .iter()
    .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
    .or_else(|| value.get("text").and_then(Value::as_str))
    .or_else(|| value.get("message").and_then(Value::as_str))
    .or_else(|| value.get("content").and_then(Value::as_str))
    .map(str::to_string)
    .filter(|text| !text.is_empty())
}

fn thinking_value(value: &Value) -> Option<String> {
    [
        "/delta/thinking",
        "/thinking",
        "/reasoning",
        "/item/summary/0/text",
        "/item/content/0/summary",
        "/data/thinking",
    ]
    .iter()
    .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
    .map(str::to_string)
    .filter(|text| !text.is_empty())
}

fn session_id(value: &Value) -> Option<String> {
    [
        "/session_id",
        "/sessionId",
        "/session/id",
        "/sessionID",
        "/thread_id",
        "/threadId",
        "/conversation_id",
        "/conversationId",
        "/metadata/session_id",
        "/metadata/sessionId",
        "/part/sessionID",
        "/part/session_id",
    ]
    .iter()
    .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
    .map(str::to_string)
    .filter(|session_id| !session_id.is_empty())
}

fn error_value(value: &Value) -> Option<String> {
    value
        .get("message")
        .or_else(|| value.get("error"))
        .or_else(|| value.get("result"))
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/error/message").and_then(Value::as_str))
        .map(str::to_string)
        .filter(|message| !message.is_empty())
}

fn provider_failure(value: &Value, fallback: &str) -> GenerationProcessFailure {
    let diagnostic = error_value(value).unwrap_or_else(|| fallback.to_string());
    if structured_error_codes(value)
        .into_iter()
        .any(|code| is_non_retryable_error_code(&code))
    {
        GenerationProcessFailure::non_retryable(diagnostic)
    } else {
        GenerationProcessFailure::retryable(diagnostic)
    }
}

fn structured_error_codes(value: &Value) -> Vec<String> {
    [
        "/code",
        "/status",
        "/reason",
        "/error/code",
        "/error/status",
        "/error/type",
        "/error/reason",
    ]
    .into_iter()
    .filter_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
    .map(normalize_error_code)
    .chain(http_status_error_code(value))
    .collect()
}

/// Maps a numeric HTTP status to the code vocabulary `is_non_retryable_error_code` understands.
/// Only statuses whose meaning is unambiguous are mapped: 429 and 5xx stay unmapped so they keep
/// the retryable default.
fn http_status_error_code(value: &Value) -> Option<String> {
    let status = ["/api_error_status", "/status_code", "/http_status"]
        .into_iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_i64))?;
    match status {
        400 => Some("bad_request".to_string()),
        401 => Some("unauthorized".to_string()),
        403 => Some("forbidden".to_string()),
        _ => None,
    }
}

fn normalize_error_code(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['-', '.', ' '], "_")
}

fn is_non_retryable_error_code(code: &str) -> bool {
    matches!(
        code,
        "invalid_request"
            | "invalid_argument"
            | "bad_request"
            | "permission_denied"
            | "forbidden"
            | "unauthorized"
            | "unauthenticated"
            | "authentication_error"
            | "authorization_error"
            | "policy_rejection"
            | "policy_violation"
            | "content_policy_violation"
            | "context_length_exceeded"
            | "configuration_error"
            | "unsupported"
    )
}

fn valid_rich_block(block: &Value) -> bool {
    block
        .get("id")
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
        && block
            .get("kind")
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        && block.get("v").and_then(Value::as_i64) == Some(1)
}

fn first_string_field(value: &Value, pointers: &[&str]) -> Option<String> {
    pointers
        .iter()
        .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
        .map(str::to_string)
        .filter(|field| !field.trim().is_empty())
}

fn is_tool_event(value: &Value, event_type: &str) -> bool {
    if matches!(
        event_type,
        "tool_use"
            | "tool_result"
            | "tool_error"
            | "tool_failure"
            | "tool"
            | "tool_call"
            | "tool.start"
            | "tool.update"
            | "tool.completed"
            | "tool.failed"
            | "tool_call_start"
            | "tool_call_end"
            | "tool_call_error"
            | "tool-call-start"
            | "tool-call-end"
            | "tool-call-error"
    ) {
        return true;
    }
    matches!(
        event_type,
        "item.started" | "item.updated" | "item.completed"
    ) && first_string_field(value, &["/item/type", "/part/type"]).is_some_and(|kind| {
        matches!(
            kind.as_str(),
            "tool" | "tool_call" | "function_call" | "command_execution" | "mcp_tool_call"
        )
    })
}

fn parse_tool_event(value: &Value, event_type: &str) -> ProviderToolEvent {
    let reported_status = first_string_field(
        value,
        &["/status", "/tool/status", "/item/status", "/part/status"],
    );
    let phase = tool_phase(event_type, reported_status.as_deref());
    let call_id = first_string_field(
        value,
        &[
            "/id",
            "/call_id",
            "/callId",
            "/tool_use_id",
            "/tool/id",
            "/tool/call_id",
            "/item/id",
            "/item/call_id",
            "/part/id",
            "/part/callID",
        ],
    );
    ProviderToolEvent {
        fidelity: if call_id.is_some() {
            ExecutionFidelity::Inferred
        } else {
            ExecutionFidelity::Opaque
        },
        call_id,
        name: first_string_field(
            value,
            &[
                "/name",
                "/tool/name",
                "/item/name",
                "/part/tool",
                "/part/name",
            ],
        ),
        input: value
            .get("input")
            .or_else(|| value.pointer("/tool/input"))
            .or_else(|| value.pointer("/part/input"))
            .or_else(|| value.pointer("/item/input"))
            .cloned(),
        output: value
            .get("output")
            .or_else(|| value.pointer("/tool/output"))
            .or_else(|| value.pointer("/part/output"))
            .or_else(|| value.pointer("/item/output"))
            .or_else(|| value.get("content"))
            .cloned(),
        phase,
        provider_timestamp: provider_timestamp(value),
        parent_run_id: first_string_field(value, &["/parent_run_id", "/parent/run_id"]),
        parent_trace_id: first_string_field(value, &["/parent_trace_id", "/parent/trace_id"]),
        parent_span_id: first_string_field(value, &["/parent_span_id", "/parent/span_id"]),
        delegation_id: first_string_field(value, &["/delegation_id", "/delegation/id"]),
        attempt: value
            .get("attempt")
            .or_else(|| value.pointer("/delegation/attempt"))
            .and_then(Value::as_u64)
            .and_then(|attempt| u32::try_from(attempt).ok()),
        status: match phase {
            ProviderToolPhase::Started | ProviderToolPhase::Updated => "running",
            ProviderToolPhase::Completed => "completed",
            ProviderToolPhase::Failed => "failed",
        }
        .to_string(),
    }
}

fn tool_phase(event_type: &str, status: Option<&str>) -> ProviderToolPhase {
    let status = status.unwrap_or_default().to_ascii_lowercase();
    if matches!(status.as_str(), "failed" | "error" | "cancelled")
        || matches!(
            event_type,
            "tool_error" | "tool_failure" | "tool.failed" | "tool_call_error" | "tool-call-error"
        )
    {
        ProviderToolPhase::Failed
    } else if matches!(
        status.as_str(),
        "completed" | "complete" | "success" | "succeeded"
    ) || matches!(
        event_type,
        "tool_result" | "tool.completed" | "tool_call_end" | "tool-call-end" | "item.completed"
    ) {
        ProviderToolPhase::Completed
    } else if matches!(event_type, "tool.update" | "item.updated") {
        ProviderToolPhase::Updated
    } else {
        ProviderToolPhase::Started
    }
}

fn provider_timestamp(value: &Value) -> Option<String> {
    value
        .get("timestamp")
        .or_else(|| value.get("created_at"))
        .or_else(|| value.pointer("/metadata/timestamp"))
        .and_then(|timestamp| match timestamp {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            _ => None,
        })
}
