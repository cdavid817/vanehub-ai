use crate::contexts::agent_runtime::application::ToolUseBlock;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ProviderOutputEvent {
    Token(String),
    Thinking(String),
    ToolUse(ToolUseBlock),
    RichBlock(Value),
    SessionId(String),
    Completed,
    Failed(String),
    Empty,
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
        "tool_use" => ProviderOutputEvent::ToolUse(ToolUseBlock {
            id: string_field(&value, "id", "tool"),
            name: string_field(&value, "name", "tool"),
            input: value.get("input").cloned(),
            output: value.get("output").cloned(),
            status: string_field(&value, "status", "running"),
        }),
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
        "result" | "done" | "complete" | "completed" | "turn.completed"
    ) {
        return ProviderOutputEvent::Completed;
    }
    if let Some(session_id) = session_id(&value) {
        if matches!(
            event_type,
            "session" | "session_init" | "session_configured" | "start" | "started"
        ) {
            return ProviderOutputEvent::SessionId(session_id);
        }
    }
    if matches!(event_type, "thinking" | "thinking_delta" | "reasoning") {
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
    if matches!(event_type, "tool_use" | "tool" | "tool_call" | "tool.start") {
        return ProviderOutputEvent::ToolUse(ToolUseBlock {
            id: nested_string_field(&value, "id", "/tool/id", "tool"),
            name: nested_string_field(&value, "name", "/tool/name", "tool"),
            input: value
                .get("input")
                .or_else(|| value.pointer("/tool/input"))
                .cloned(),
            output: value
                .get("output")
                .or_else(|| value.pointer("/tool/output"))
                .cloned(),
            status: string_field(&value, "status", "running"),
        });
    }
    text_value(&value)
        .map(ProviderOutputEvent::Token)
        .unwrap_or(ProviderOutputEvent::Empty)
}

fn text_value(value: &Value) -> Option<String> {
    [
        "/delta/text",
        "/message/content/0/text",
        "/content/0/text",
        "/content/text",
        "/data/text",
    ]
    .iter()
    .find_map(|pointer| value.pointer(pointer).and_then(Value::as_str))
    .or_else(|| value.get("text").and_then(Value::as_str))
    .or_else(|| value.get("content").and_then(Value::as_str))
    .map(str::to_string)
    .filter(|text| !text.is_empty())
}

fn thinking_value(value: &Value) -> Option<String> {
    [
        "/delta/thinking",
        "/thinking",
        "/reasoning",
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
        "/metadata/session_id",
        "/metadata/sessionId",
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

fn string_field(value: &Value, field: &str, fallback: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}

fn nested_string_field(value: &Value, field: &str, pointer: &str, fallback: &str) -> String {
    value
        .get(field)
        .or_else(|| value.pointer(pointer))
        .and_then(Value::as_str)
        .unwrap_or(fallback)
        .to_string()
}
