use crate::contexts::execution_observability::api::ExecutionFidelity;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProviderOutputEvent {
    Token(String),
    Thinking(String),
    ToolLifecycle(Box<ProviderToolEvent>),
    RichBlock(Value),
    SessionId(String),
    Completed,
    Failed(String),
    Empty,
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
    GenericLine,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProviderOutputParser {
    kind: ParserKind,
}

pub(crate) fn output_parser_for(agent_id: &str) -> ProviderOutputParser {
    let kind = match agent_id {
        "claude-code" => ParserKind::Claude,
        "codex-cli" | "gemini-cli" | "opencode" => ParserKind::StructuredJson,
        _ => ParserKind::GenericLine,
    };
    ProviderOutputParser { kind }
}

impl ProviderOutputParser {
    pub(crate) fn parse_line(&self, line: &str) -> ProviderOutputEvent {
        match self.kind {
            ParserKind::Claude => parse_claude_line(line),
            ParserKind::StructuredJson => parse_structured_json_line(line),
            ParserKind::GenericLine => parse_generic_line(line),
        }
    }
}

fn parse_generic_line(line: &str) -> ProviderOutputEvent {
    if line.trim().is_empty() {
        ProviderOutputEvent::Empty
    } else {
        ProviderOutputEvent::Token(line.to_string())
    }
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
        "result" | "complete" | "completed" => ProviderOutputEvent::Completed,
        "error" | "failed" => ProviderOutputEvent::Failed(
            value
                .get("message")
                .or_else(|| value.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("Agent output reported an error.")
                .to_string(),
        ),
        _ => parse_generic_line(line),
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
        return ProviderOutputEvent::Failed(
            error_value(&value).unwrap_or_else(|| "Agent CLI reported an error.".to_string()),
        );
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
        return ProviderOutputEvent::Completed;
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
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/error/message").and_then(Value::as_str))
        .map(str::to_string)
        .filter(|message| !message.is_empty())
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
