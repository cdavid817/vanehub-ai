//! Pure translation between the OpenAI Chat Completions wire format (spoken by a large class of
//! third-party and relay LLM providers, not just OpenAI's own hosted API) and the
//! application-layer `GenerationProcessEvent` vocabulary. Mirrors `anthropic_provider.rs`'s
//! function shapes so `RuntimeAgentApiAdapter::execute` can pick either module by
//! `interface_format` without any other branching. No I/O lives here so this module is
//! unit-testable without a live network connection.

use super::api_process_adapter::GenerationOptions;
use super::tool_call_accumulator::ToolCallAccumulator;
use crate::contexts::agent_runtime::application::{
    AgentMessage, GenerationProcessEvent, GenerationProcessFailure, ToolDefinition, ToolUseBlock,
};
use serde_json::{json, Value};

/// A tool call's block, its output text, and whether execution failed — the shape both wire
/// formats need to build a reply turn from.
type ExecutedToolCall = (ToolUseBlock, String, bool);

/// The literal SSE payload OpenAI-compatible endpoints send to terminate a stream — not a JSON
/// object, so it must be checked before attempting to parse `data` as JSON.
const DONE_SENTINEL: &str = "[DONE]";

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

/// Builds the streaming Chat Completions request body from already-converted turns and, when
/// `tools` is non-empty, a `tools` declaration in OpenAI's
/// `{type: "function", function: {name, description, parameters}}` shape. `system`, when
/// present, is prepended as a `role: "system"` message *at request-build time* — this wire
/// format has no separate system field the way Anthropic's does, but the caller's `messages`
/// slice is never mutated to include it, so bound-Skill content (`add-agent-skill-support`) is
/// never mistaken for a turn a caller (e.g. context compaction) might rewrite or summarize away.
/// `options.reasoning_depth`, when present, becomes a `reasoning_effort` field
/// (`add-agent-chat-configuration`) — the standard-ish request parameter reasoning-capable
/// OpenAI-compatible models use. `options.thinking` has no effect here: there is no universal
/// "enable reasoning" request flag across OpenAI-compatible vendors (many reasoning models
/// reason automatically based on the chosen model id, not a request-level toggle).
pub(crate) fn build_request_body(
    model: &str,
    messages: &[Value],
    tools: &[ToolDefinition],
    system: Option<&str>,
    options: &GenerationOptions,
) -> Value {
    let mut all_messages = Vec::with_capacity(messages.len() + 1);
    if let Some(system) = system {
        all_messages.push(json!({ "role": "system", "content": system }));
    }
    all_messages.extend_from_slice(messages);
    let mut body = json!({
        "model": model,
        "stream": true,
        "messages": all_messages,
    });
    if !tools.is_empty() {
        body["tools"] = Value::Array(tools.iter().map(tool_definition_to_json).collect());
    }
    if let Some(depth) = options.reasoning_depth {
        body["reasoning_effort"] = json!(reasoning_effort(depth));
    }
    body
}

/// Folds VaneHub's `"max"` reasoning-depth tier down to `"high"` — the same kind of fold-down
/// `providers/invocation.rs` already applies for codex-cli's own reasoning-effort vocabulary
/// (`"max"` → `"xhigh"`), since `"max"` is a VaneHub-only tier with no equivalent in the standard
/// `reasoning_effort` enum.
fn reasoning_effort(depth: &str) -> &str {
    if depth == "max" {
        "high"
    } else {
        depth
    }
}

fn tool_definition_to_json(tool: &ToolDefinition) -> Value {
    json!({
        "type": "function",
        "function": {
            "name": tool.name,
            "description": tool.description,
            "parameters": tool.input_schema,
        },
    })
}

/// Builds the assistant turn (with any tool calls it requested) followed by one `role: "tool"`
/// message per executed call, ready to append to the next request's `messages` array.
pub(crate) fn build_reply_turns(assistant_text: &str, executed: &[ExecutedToolCall]) -> Vec<Value> {
    let tool_calls: Vec<Value> = executed
        .iter()
        .map(|(tool_use, _, _)| {
            json!({
                "id": tool_use.id,
                "type": "function",
                "function": {
                    "name": tool_use.name,
                    "arguments": tool_use.input.clone().unwrap_or(Value::Null).to_string(),
                },
            })
        })
        .collect();
    let assistant_content = if assistant_text.is_empty() {
        Value::Null
    } else {
        Value::String(assistant_text.to_string())
    };
    let mut turns = vec![json!({
        "role": "assistant",
        "content": assistant_content,
        "tool_calls": tool_calls,
    })];
    for (tool_use, output, _is_error) in executed {
        turns.push(json!({
            "role": "tool",
            "tool_call_id": tool_use.id,
            "content": output,
        }));
    }
    turns
}

/// Translates one decoded SSE `data:` payload into an application event, or `None` when the
/// event carries nothing the chat pipeline needs to see directly (e.g. a delta with no
/// content/reasoning/tool-call fragment and no error).
pub(crate) fn translate_sse_data(
    data: &str,
    accumulator: &mut ToolCallAccumulator,
) -> Option<GenerationProcessEvent> {
    let trimmed = data.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == DONE_SENTINEL {
        accumulator.finish_all_pending();
        return Some(GenerationProcessEvent::Completed(None));
    }
    let value: Value = serde_json::from_str(trimmed).ok()?;
    if value.get("error").is_some() {
        return Some(GenerationProcessEvent::Failed(translate_error(&value)));
    }
    translate_delta(&value, accumulator)
}

fn translate_delta(
    value: &Value,
    accumulator: &mut ToolCallAccumulator,
) -> Option<GenerationProcessEvent> {
    let delta = value.pointer("/choices/0/delta")?;
    if let Some(tool_calls) = delta.get("tool_calls").and_then(Value::as_array) {
        for call in tool_calls {
            accumulate_tool_call_fragment(call, accumulator);
        }
        return None;
    }
    if let Some(content) = delta
        .get("content")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        return Some(GenerationProcessEvent::Token(content.to_string()));
    }
    if let Some(reasoning) = delta
        .get("reasoning_content")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        return Some(GenerationProcessEvent::Thinking(reasoning.to_string()));
    }
    None
}

fn accumulate_tool_call_fragment(call: &Value, accumulator: &mut ToolCallAccumulator) {
    let Some(index) = call.get("index").and_then(Value::as_u64).map(|v| v as u32) else {
        return;
    };
    let id = call.get("id").and_then(Value::as_str).unwrap_or_default();
    let name = call
        .pointer("/function/name")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if !id.is_empty() || !name.is_empty() {
        accumulator.start(index, id, name);
    }
    if let Some(fragment) = call.pointer("/function/arguments").and_then(Value::as_str) {
        accumulator.append_json(index, fragment);
    }
}

fn translate_error(value: &Value) -> GenerationProcessFailure {
    let message = value
        .pointer("/error/message")
        .and_then(Value::as_str)
        .unwrap_or("The OpenAI-compatible endpoint reported an error.");
    // Unlike Anthropic's fixed `error.type` enum, "OpenAI-compatible" providers do not agree on
    // an error taxonomy, so an in-stream error (arriving after the HTTP status already
    // succeeded) is treated as non-retryable by default rather than guessed at.
    GenerationProcessFailure::non_retryable(message.to_string())
}

/// Builds a terminal failure from a non-2xx HTTP response received before any SSE data arrived.
/// Classification uses the HTTP status rather than `error.type`, since that field's values are
/// not standardized across OpenAI-compatible vendors the way they are for Anthropic's own API.
pub(crate) fn failure_from_http_status(status: u16, body: &str) -> GenerationProcessFailure {
    let parsed: Option<Value> = serde_json::from_str(body).ok();
    let message = parsed
        .as_ref()
        .and_then(|value| value.pointer("/error/message"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| format!("OpenAI-compatible API request failed with status {status}."));
    let failure = if matches!(status, 400 | 401 | 403 | 404 | 422) {
        GenerationProcessFailure::non_retryable(message)
    } else {
        GenerationProcessFailure::retryable(message)
    };
    match status {
        401 | 403 => failure.with_safe_error(
            "Provider authentication failed. Check the API key in the active OnePiece configuration.",
        ),
        400 | 422 => failure.with_safe_error(
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
            speaker_seat_id: None,
            seat_index: None,
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
            session_sequence: 1,
            execution_run_id: None,
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
        let body = build_request_body("deepseek-chat", &turns, &[], None, &no_options());
        assert_eq!(body["model"], "deepseek-chat");
        assert_eq!(body["stream"], true);
        assert_eq!(body["messages"].as_array().expect("array").len(), 1);
        assert!(body.get("tools").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn request_body_declares_tools_when_provided() {
        let tools = crate::contexts::agent_runtime::application::tool_catalog();
        let body = build_request_body("deepseek-chat", &[], &tools, None, &no_options());
        let declared = body["tools"].as_array().expect("tools array");
        assert_eq!(declared.len(), 6);
        assert_eq!(declared[0]["type"], "function");
        assert_eq!(declared[0]["function"]["name"], "shell");
        assert_eq!(declared[1]["function"]["name"], "file");
        assert_eq!(declared[2]["function"]["name"], "grep");
        assert_eq!(declared[3]["function"]["name"], "glob");
        assert_eq!(declared[4]["function"]["name"], "edit");
        assert_eq!(declared[5]["function"]["name"], "remember");
    }

    #[test]
    fn request_body_prepends_a_system_message_without_mutating_the_caller_turns() {
        let turns = history_to_turns(&[message("user", "Hello")]);
        let body = build_request_body(
            "deepseek-chat",
            &turns,
            &[],
            Some("Be concise."),
            &no_options(),
        );
        let messages = body["messages"].as_array().expect("array");
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0]["role"], "system");
        assert_eq!(messages[0]["content"], "Be concise.");
        assert_eq!(messages[1]["role"], "user");
        assert_eq!(turns.len(), 1, "the caller's turns slice must be untouched");
    }

    #[test]
    fn request_body_passes_reasoning_depth_through_as_reasoning_effort() {
        for depth in ["low", "medium", "high"] {
            let options = GenerationOptions {
                thinking: false,
                reasoning_depth: Some(depth),
            };
            let body = build_request_body("deepseek-chat", &[], &[], None, &options);
            assert_eq!(body["reasoning_effort"], depth);
        }
    }

    #[test]
    fn request_body_folds_max_reasoning_depth_down_to_high() {
        let options = GenerationOptions {
            thinking: false,
            reasoning_depth: Some("max"),
        };
        let body = build_request_body("deepseek-chat", &[], &[], None, &options);
        assert_eq!(body["reasoning_effort"], "high");
    }

    #[test]
    fn request_body_ignores_thinking() {
        let options = GenerationOptions {
            thinking: true,
            reasoning_depth: None,
        };
        let body = build_request_body("deepseek-chat", &[], &[], None, &options);
        assert!(body.get("thinking").is_none());
        assert!(body.get("reasoning_effort").is_none());
    }

    #[test]
    fn tool_call_accumulates_across_streamed_argument_fragments_and_flushes_at_done() {
        let mut accumulator = ToolCallAccumulator::default();
        let start = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"id":"call_1","type":"function","function":{"name":"shell","arguments":""}}]},"finish_reason":null}]}"#;
        let fragment1 = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"{\"comm"}}]},"finish_reason":null}]}"#;
        let fragment2 = r#"{"choices":[{"index":0,"delta":{"tool_calls":[{"index":0,"function":{"arguments":"and\":\"ls\"}"}}]},"finish_reason":null}]}"#;
        let finish = r#"{"choices":[{"index":0,"delta":{},"finish_reason":"tool_calls"}]}"#;

        assert_eq!(translate_sse_data(start, &mut accumulator), None);
        assert_eq!(translate_sse_data(fragment1, &mut accumulator), None);
        assert_eq!(translate_sse_data(fragment2, &mut accumulator), None);
        assert_eq!(translate_sse_data(finish, &mut accumulator), None);
        assert!(
            accumulator.take_completed().is_empty(),
            "not flushed until [DONE]"
        );

        assert_eq!(
            translate_sse_data("[DONE]", &mut accumulator),
            Some(GenerationProcessEvent::Completed(None))
        );
        let completed = accumulator.take_completed();
        assert_eq!(completed.len(), 1);
        assert_eq!(completed[0].id, "call_1");
        assert_eq!(completed[0].name, "shell");
        assert_eq!(completed[0].input, Some(json!({"command": "ls"})));
    }

    #[test]
    fn parallel_tool_calls_accumulate_independently_by_index() {
        let mut accumulator = ToolCallAccumulator::default();
        let chunk = r#"{"choices":[{"index":0,"delta":{"tool_calls":[
            {"index":0,"id":"call_1","type":"function","function":{"name":"shell","arguments":"{}"}},
            {"index":1,"id":"call_2","type":"function","function":{"name":"file","arguments":"{\"operation\":\"read\",\"path\":\"a.txt\"}"}}
        ]},"finish_reason":null}]}"#;
        translate_sse_data(chunk, &mut accumulator);
        translate_sse_data("[DONE]", &mut accumulator);
        let completed = accumulator.take_completed();
        assert_eq!(completed.len(), 2);
        assert_eq!(completed[0].name, "shell");
        assert_eq!(completed[1].name, "file");
    }

    #[test]
    fn build_reply_turns_includes_tool_calls_and_one_tool_message_per_call() {
        let tool_use = ToolUseBlock {
            id: "call_1".to_string(),
            name: "shell".to_string(),
            input: Some(json!({"command": "ls"})),
            output: None,
            status: "pending".to_string(),
        };
        let turns = build_reply_turns("Let me check.", &[(tool_use, "a.txt\n".to_string(), false)]);
        assert_eq!(turns.len(), 2);
        assert_eq!(turns[0]["role"], "assistant");
        assert_eq!(turns[0]["content"], "Let me check.");
        assert_eq!(turns[0]["tool_calls"][0]["id"], "call_1");
        assert_eq!(turns[0]["tool_calls"][0]["function"]["name"], "shell");
        assert_eq!(turns[1]["role"], "tool");
        assert_eq!(turns[1]["tool_call_id"], "call_1");
        assert_eq!(turns[1]["content"], "a.txt\n");
    }

    #[test]
    fn content_delta_becomes_token_event() {
        let data = r#"{"choices":[{"index":0,"delta":{"content":"Hello"},"finish_reason":null}]}"#;
        assert_eq!(
            translate(data),
            Some(GenerationProcessEvent::Token("Hello".to_string()))
        );
    }

    #[test]
    fn reasoning_content_delta_becomes_thinking_event() {
        let data = r#"{"choices":[{"index":0,"delta":{"reasoning_content":"reasoning..."},"finish_reason":null}]}"#;
        assert_eq!(
            translate(data),
            Some(GenerationProcessEvent::Thinking("reasoning...".to_string()))
        );
    }

    #[test]
    fn done_sentinel_becomes_completed() {
        assert_eq!(
            translate("[DONE]"),
            Some(GenerationProcessEvent::Completed(None))
        );
        assert_eq!(
            translate(" [DONE] "),
            Some(GenerationProcessEvent::Completed(None))
        );
    }

    #[test]
    fn irrelevant_events_are_ignored() {
        for data in [
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{"role":"assistant"},"finish_reason":null}]}"#,
            r#"{"choices":[{"index":0,"delta":{},"finish_reason":"stop"}]}"#,
            r#"{"id":"chatcmpl-1","object":"chat.completion.chunk","choices":[]}"#,
            "",
            "   ",
        ] {
            assert_eq!(translate(data), None, "data was: {data}");
        }
    }

    #[test]
    fn in_stream_error_becomes_non_retryable_failed() {
        let data = r#"{"error":{"message":"invalid api key","type":"invalid_request_error"}}"#;
        match translate(data) {
            Some(GenerationProcessEvent::Failed(failure)) => {
                assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
                assert_eq!(failure.diagnostic, "invalid api key");
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
        let body =
            r#"{"error":{"message":"Incorrect API key provided","type":"invalid_request_error"}}"#;
        let failure = failure_from_http_status(401, body);
        assert_eq!(failure.kind, GenerationProcessFailureKind::NonRetryable);
        assert_eq!(failure.diagnostic, "Incorrect API key provided");
        assert_eq!(
            failure.safe_error.as_deref(),
            Some(
                "Provider authentication failed. Check the API key in the active OnePiece configuration."
            )
        );
    }

    #[test]
    fn http_status_failure_classifies_rate_limit_as_retryable() {
        let body = r#"{"error":{"message":"Rate limit reached","type":"rate_limit_error"}}"#;
        let failure = failure_from_http_status(429, body);
        assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
    }

    #[test]
    fn http_status_failure_falls_back_when_body_is_not_json() {
        let failure = failure_from_http_status(500, "internal error");
        assert_eq!(failure.kind, GenerationProcessFailureKind::Retryable);
        assert_eq!(
            failure.diagnostic,
            "OpenAI-compatible API request failed with status 500."
        );
    }
}
