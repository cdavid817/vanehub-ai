//! Pure translation between the Anthropic Messages API wire format and the application-layer
//! `GenerationProcessEvent` vocabulary. No I/O lives here so this module is unit-testable without
//! a live network connection, mirroring `providers::output`'s CLI-line-parsing tests.

use super::api_process_adapter::GenerationOptions;
use super::tool_call_accumulator::ToolCallAccumulator;
use crate::contexts::agent_runtime::application::{
    AgentMessage, GenerationProcessEvent, GenerationProcessFailure, ToolDefinition, ToolUseBlock,
};
use serde_json::{json, Value};

/// A tool call's block, its output text, and whether execution failed — the shape both wire
/// formats need to build a reply turn from.
type ExecutedToolCall = (ToolUseBlock, String, bool);

pub(crate) const ANTHROPIC_VERSION: &str = "2023-06-01";
const DEFAULT_MAX_TOKENS: u32 = 8192;

/// Filters conversation history to `user`/`assistant` turns with non-empty content and maps
/// each into the `{role, content}` shape both wire formats share for plain historical turns.
/// Called once at the start of a generation; subsequent tool-use loop iterations extend the
/// resulting `Vec` with `build_reply_turns`' wire-format-specific turns instead of re-deriving
/// from `AgentMessage` history.
pub(crate) fn history_to_turns(history: &[AgentMessage]) -> Vec<Value> {
    history
        .iter()
        .filter(|message| {
            matches!(message.role.as_str(), "user" | "assistant") && !message.content.is_empty()
        })
        .map(|message| json!({ "role": message.role, "content": message.content }))
        .collect()
}

/// Builds the streaming Messages API request body from already-converted turns and, when
/// `tools` is non-empty, a `tools` declaration in Anthropic's `{name, description, input_schema}`
/// shape. `system`, when present, becomes the request's top-level `system` field — Anthropic's
/// wire format keeps it separate from `messages` natively, so bound-Skill content
/// (`add-agent-skill-support`) never needs to be represented as a turn. `options.thinking`, when
/// `true`, enables extended thinking (`add-agent-chat-configuration`); `options.reasoning_depth`
/// has no Anthropic-side effect — adaptive thinking has no separate depth parameter to set.
pub(crate) fn build_request_body(
    model: &str,
    messages: &[Value],
    tools: &[ToolDefinition],
    system: Option<&str>,
    options: &GenerationOptions,
) -> Value {
    let mut body = json!({
        "model": model,
        "max_tokens": DEFAULT_MAX_TOKENS,
        "stream": true,
        "messages": messages,
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.iter().map(tool_definition_to_json).collect());
    }
    if let Some(system) = system {
        body["system"] = json!(system);
    }
    if options.thinking {
        body["thinking"] = json!({ "type": "adaptive" });
    }
    body
}

fn tool_definition_to_json(tool: &ToolDefinition) -> Value {
    json!({
        "name": tool.name,
        "description": tool.description,
        "input_schema": tool.input_schema,
    })
}

/// Builds the assistant turn (text + any tool calls it requested) followed by one user turn
/// carrying all of their results, ready to append to the next request's `messages` array.
pub(crate) fn build_reply_turns(assistant_text: &str, executed: &[ExecutedToolCall]) -> Vec<Value> {
    let mut assistant_content: Vec<Value> = Vec::new();
    if !assistant_text.is_empty() {
        assistant_content.push(json!({ "type": "text", "text": assistant_text }));
    }
    for (tool_use, _, _) in executed {
        assistant_content.push(json!({
            "type": "tool_use",
            "id": tool_use.id,
            "name": tool_use.name,
            "input": tool_use.input.clone().unwrap_or(Value::Null),
        }));
    }
    let tool_results: Vec<Value> = executed
        .iter()
        .map(|(tool_use, output, is_error)| {
            json!({
                "type": "tool_result",
                "tool_use_id": tool_use.id,
                "content": output,
                "is_error": is_error,
            })
        })
        .collect();
    vec![
        json!({ "role": "assistant", "content": assistant_content }),
        json!({ "role": "user", "content": tool_results }),
    ]
}

/// Translates one decoded SSE `data:` payload into an application event, or `None` when the
/// event carries nothing the chat pipeline needs to see directly (e.g. `message_start`, `ping`,
/// or a tool-call fragment that only mutates `accumulator` without completing yet).
pub(crate) fn translate_sse_data(
    data: &str,
    accumulator: &mut ToolCallAccumulator,
) -> Option<GenerationProcessEvent> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return None;
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    let event_type = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match event_type {
        "content_block_start" => {
            translate_content_block_start(&value, accumulator);
            None
        }
        "content_block_delta" => translate_content_block_delta(&value, accumulator),
        "content_block_stop" => {
            if let Some(index) = block_index(&value) {
                accumulator.finish(index);
            }
            None
        }
        "message_stop" => Some(GenerationProcessEvent::Completed(None)),
        "error" => Some(GenerationProcessEvent::Failed(translate_error(&value))),
        _ => None,
    }
}

fn block_index(value: &Value) -> Option<u32> {
    value.get("index").and_then(Value::as_u64).map(|v| v as u32)
}

fn translate_content_block_start(value: &Value, accumulator: &mut ToolCallAccumulator) {
    let Some(index) = block_index(value) else {
        return;
    };
    let block = value.get("content_block");
    if block.and_then(|b| b.get("type")).and_then(Value::as_str) != Some("tool_use") {
        return;
    }
    let id = block
        .and_then(|b| b.get("id"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let name = block
        .and_then(|b| b.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    accumulator.start(index, id, name);
}

fn translate_content_block_delta(
    value: &Value,
    accumulator: &mut ToolCallAccumulator,
) -> Option<GenerationProcessEvent> {
    let delta_type = value
        .pointer("/delta/type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match delta_type {
        "text_delta" => value
            .pointer("/delta/text")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| GenerationProcessEvent::Token(text.to_string())),
        "thinking_delta" => value
            .pointer("/delta/thinking")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(|text| GenerationProcessEvent::Thinking(text.to_string())),
        "input_json_delta" => {
            if let (Some(index), Some(fragment)) = (
                block_index(value),
                value.pointer("/delta/partial_json").and_then(Value::as_str),
            ) {
                accumulator.append_json(index, fragment);
            }
            None
        }
        _ => None,
    }
}

fn translate_error(value: &Value) -> GenerationProcessFailure {
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("Anthropic API reported an error.");
    let error_type = value
        .pointer("/error/type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    classify_failure(error_type, message)
}

fn classify_failure(error_type: &str, message: &str) -> GenerationProcessFailure {
    let non_retryable = matches!(
        error_type,
        "invalid_request_error"
            | "authentication_error"
            | "permission_error"
            | "not_found_error"
            | "billing_error"
    );
    if non_retryable {
        GenerationProcessFailure::non_retryable(message.to_string())
    } else {
        GenerationProcessFailure::retryable(message.to_string())
    }
}

/// Builds a terminal failure from a non-2xx HTTP response received before any SSE data arrived.
pub(crate) fn failure_from_http_status(status: u16, body: &str) -> GenerationProcessFailure {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/message"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("Anthropic API request failed with status {status}."));
    let error_type = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/type"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let failure = if matches!(status, 400 | 401 | 403 | 404) {
        GenerationProcessFailure::non_retryable(message)
    } else {
        classify_failure(error_type, &message)
    };
    match status {
        401 | 403 => failure.with_safe_error(
            "Provider authentication failed. Check the API key in the active OnePiece configuration.",
        ),
        400 => failure.with_safe_error(
            "The provider rejected the request. Check the model and endpoint in the active OnePiece configuration.",
        ),
        404 => failure.with_safe_error(
            "The configured provider endpoint or model was not found. Check the active OnePiece configuration.",
        ),
        429 => failure.with_safe_error(
            "The provider rate limit was reached. Try again later or use another OnePiece configuration.",
        ),
        500..=599 => failure.with_safe_error(
            "The provider service is unavailable. Try again later.",
        ),
        _ => failure,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::{
        GenerationProcessFailureKind, MessageTokenUsage, ToolUseBlock,
    };

    fn translate(data: &str) -> Option<GenerationProcessEvent> {
        translate_sse_data(data, &mut ToolCallAccumulator::default())
    }

    fn message(role: &str, content: &str) -> AgentMessage {
        AgentMessage {
            id: "message-1".to_string(),
            session_id: "session-1".to_string(),
            role: role.to_string(),
            content: content.to_string(),
            status: "completed".to_string(),
            tool_use: Vec::<ToolUseBlock>::new(),
            thinking_content: None,
            rich_blocks: Vec::new(),
            token_usage: None::<MessageTokenUsage>,
            file_references: Vec::new(),
            error: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn history_to_turns_keeps_user_and_assistant_turns_and_drops_empty_placeholder() {
        let history = vec![
            message("user", "Hello"),
            message("assistant", "Hi there"),
            message("assistant", ""),
        ];
        let turns = history_to_turns(&history);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0]["role"], "user");
        assert_eq!(turns[0]["content"], "Hello");
        assert_eq!(turns[1]["role"], "assistant");
    }

    fn no_options() -> GenerationOptions<'static> {
        GenerationOptions::disabled()
    }

    #[test]
    fn request_body_embeds_model_and_turns() {
        let turns = history_to_turns(&[message("user", "Hello")]);
        let body = build_request_body("claude-opus-4-8", &turns, &[], None, &no_options());
        assert_eq!(body["model"], "claude-opus-4-8");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"].as_array().expect("array").len(), 1);
        assert!(body.get("tools").is_none());
        assert!(body.get("system").is_none());
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn request_body_declares_tools_when_provided() {
        let tools = crate::contexts::agent_runtime::application::tool_catalog();
        let body = build_request_body("claude-opus-4-8", &[], &tools, None, &no_options());
        let declared = body["tools"].as_array().expect("tools array");
        assert_eq!(declared.len(), 6);
        assert_eq!(declared[0]["name"], "shell");
        assert!(declared[0]["input_schema"]["properties"]["command"].is_object());
        assert_eq!(declared[1]["name"], "file");
        assert_eq!(declared[2]["name"], "grep");
        assert_eq!(declared[3]["name"], "glob");
        assert_eq!(declared[4]["name"], "edit");
        assert_eq!(declared[5]["name"], "remember");
    }

    #[test]
    fn request_body_sets_top_level_system_field_when_present() {
        let body = build_request_body(
            "claude-opus-4-8",
            &[],
            &[],
            Some("Be concise."),
            &no_options(),
        );
        assert_eq!(body["system"], "Be concise.");
        assert!(body["messages"].as_array().expect("array").is_empty());
    }

    #[test]
    fn request_body_enables_adaptive_thinking_when_requested() {
        let options = GenerationOptions {
            thinking: true,
            reasoning_depth: None,
        };
        let body = build_request_body("claude-opus-4-8", &[], &[], None, &options);
        assert_eq!(body["thinking"], json!({ "type": "adaptive" }));
    }

    #[test]
    fn request_body_omits_thinking_when_not_requested() {
        let body = build_request_body("claude-opus-4-8", &[], &[], None, &no_options());
        assert!(body.get("thinking").is_none());
    }

    #[test]
    fn request_body_ignores_reasoning_depth() {
        let options = GenerationOptions {
            thinking: false,
            reasoning_depth: Some("high"),
        };
        let body = build_request_body("claude-opus-4-8", &[], &[], None, &options);
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn tool_use_block_accumulates_across_start_delta_and_stop() {
        let mut accumulator = ToolCallAccumulator::default();
        let start = r#"{"type":"content_block_start","index":0,"content_block":{"type":"tool_use","id":"toolu_1","name":"shell","input":{}}}"#;
        let delta1 = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"{\"comm"}}"#;
        let delta2 = r#"{"type":"content_block_delta","index":0,"delta":{"type":"input_json_delta","partial_json":"and\":\"ls\"}"}}"#;
        let stop = r#"{"type":"content_block_stop","index":0}"#;

        assert_eq!(translate_sse_data(start, &mut accumulator), None);
        assert_eq!(translate_sse_data(delta1, &mut accumulator), None);
        assert_eq!(translate_sse_data(delta2, &mut accumulator), None);
        assert_eq!(translate_sse_data(stop, &mut accumulator), None);

        let completed = accumulator.take_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, "toolu_1");
        assert_eq!(completed[0].name, "shell");
        assert_eq!(completed[0].input, Some(json!({"command": "ls"})));
    }

    #[test]
    fn text_content_block_start_and_stop_do_not_register_a_tool_call() {
        let mut accumulator = ToolCallAccumulator::default();
        let start =
            r#"{"type":"content_block_start","index":0,"content_block":{"type":"text","text":""}}"#;
        let stop = r#"{"type":"content_block_stop","index":0}"#;
        translate_sse_data(start, &mut accumulator);
        translate_sse_data(stop, &mut accumulator);
        assert!(accumulator.take_completed().is_empty());
    }

    #[test]
    fn build_reply_turns_includes_text_tool_use_and_tool_result() {
        let tool_use = ToolUseBlock {
            id: "toolu_1".to_string(),
            name: "shell".to_string(),
            input: Some(json!({"command": "ls"})),
            output: None,
            status: "pending".to_string(),
        };
        let turns = build_reply_turns("Let me check.", &[(tool_use, "a.txt\n".to_string(), false)]);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0]["role"], "assistant");
        assert_eq!(turns[0]["content"][0]["type"], "text");
        assert_eq!(turns[0]["content"][1]["type"], "tool_use");
        assert_eq!(turns[0]["content"][1]["id"], "toolu_1");
        assert_eq!(turns[1]["role"], "user");
        assert_eq!(turns[1]["content"][0]["type"], "tool_result");
        assert_eq!(turns[1]["content"][0]["tool_use_id"], "toolu_1");
        assert_eq!(turns[1]["content"][0]["content"], "a.txt\n");
        assert_eq!(turns[1]["content"][0]["is_error"], false);
    }

    #[test]
    fn text_delta_becomes_token_event() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"Hello"}}"#;
        assert_eq!(
            translate(data),
            Some(GenerationProcessEvent::Token("Hello".to_string()))
        );
    }

    #[test]
    fn thinking_delta_becomes_thinking_event() {
        let data = r#"{"type":"content_block_delta","index":0,"delta":{"type":"thinking_delta","thinking":"reasoning..."}}"#;
        assert_eq!(
            translate(data),
            Some(GenerationProcessEvent::Thinking("reasoning...".to_string()))
        );
    }

    #[test]
    fn message_stop_becomes_completed() {
        let data = r#"{"type":"message_stop"}"#;
        assert_eq!(
            translate(data),
            Some(GenerationProcessEvent::Completed(None))
        );
    }

    #[test]
    fn irrelevant_events_are_ignored() {
        for data in [
            r#"{"type":"message_start","message":{}}"#,
            r#"{"type":"content_block_start","index":0}"#,
            r#"{"type":"content_block_stop","index":0}"#,
            r#"{"type":"message_delta","delta":{"stop_reason":"end_turn"}}"#,
            r#"{"type":"ping"}"#,
            "",
            "   ",
        ] {
            assert_eq!(translate(data), None, "data was: {data}");
        }
    }

    #[test]
    fn stream_error_event_becomes_failed() {
        let data = r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#;
        match translate(data) {
            Some(GenerationProcessEvent::Failed(failure)) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
                assert_eq!(failure.diagnostic, "Overloaded");
            }
            other => panic!("expected Failed event, got {other:?}"),
        }
    }

    #[test]
    fn authentication_error_is_non_retryable() {
        let data = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        match translate(data) {
            Some(GenerationProcessEvent::Failed(failure)) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
            }
            other => panic!("expected Failed event, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_is_ignored_rather_than_panicking() {
        assert_eq!(translate("not json"), None);
    }

    #[test]
    fn http_status_failure_reads_error_body_message() {
        let body = r#"{"type":"error","error":{"type":"authentication_error","message":"invalid x-api-key"}}"#;
        let failure = failure_from_http_status(401, body);
        assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
        assert_eq!(failure.diagnostic, "invalid x-api-key");
        assert_eq!(
            failure.safe_error.as_deref(),
            Some(
                "Provider authentication failed. Check the API key in the active OnePiece configuration."
            )
        );
    }

    #[test]
    fn http_status_failure_falls_back_when_body_is_not_json() {
        let failure = failure_from_http_status(500, "internal error");
        assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
        assert_eq!(
            failure.diagnostic,
            "Anthropic API request failed with status 500."
        );
    }
}
