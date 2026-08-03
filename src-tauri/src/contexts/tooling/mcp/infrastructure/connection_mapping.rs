use crate::contexts::tooling::mcp::application::{McpLimits, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::{McpFailureCode, ToolCallOutcome, ToolDescriptor};
use rmcp::model::{CallToolRequestParams, CallToolResult, ContentBlock, Tool};

pub(super) fn validate_call_tool(
    tool_name: &str,
    arguments: serde_json::Value,
) -> Result<CallToolRequestParams, McpRuntimeError> {
    if tool_name.trim().is_empty() {
        return Err(McpRuntimeError::with_diagnostic(
            McpFailureCode::Validation,
            "tool name must not be empty",
        ));
    }
    let limits = McpLimits::DEFAULT;
    limits.validate_bytes("tool name", tool_name.len(), limits.tool_name_bytes)?;
    let arguments = match arguments {
        serde_json::Value::Object(map) => serde_json::Value::Object(map),
        serde_json::Value::Null => serde_json::Value::Null,
        _ => {
            return Err(McpRuntimeError::with_diagnostic(
                McpFailureCode::Validation,
                "tool call arguments must be a JSON object or null",
            ));
        }
    };
    limits.validate_json(
        "tool arguments",
        &arguments,
        limits.tool_arguments_bytes,
        limits.json_depth,
    )?;

    let mut params = CallToolRequestParams::new(tool_name.to_string());
    if let serde_json::Value::Object(arguments) = arguments {
        params = params.with_arguments(arguments);
    }
    Ok(params)
}

pub(super) fn map_call_result(result: CallToolResult) -> ToolCallOutcome {
    let content = match render_content(&result.content) {
        Ok(content) => content,
        Err(error) => {
            return ToolCallOutcome::failed_with_code(error.to_string(), error.code());
        }
    };
    if result.is_error.unwrap_or(false) {
        ToolCallOutcome::failed(content)
    } else {
        ToolCallOutcome::success(content)
    }
}

fn render_content(blocks: &[ContentBlock]) -> Result<String, McpRuntimeError> {
    let limits = McpLimits::DEFAULT;
    let mut rendered = String::new();
    for (index, block) in blocks.iter().enumerate() {
        let content = match block {
            ContentBlock::Text(text) => text.text.as_str(),
            ContentBlock::Image(_) => "[image content omitted]",
            ContentBlock::Audio(_) => "[audio content omitted]",
            ContentBlock::Resource(_) | ContentBlock::ResourceLink(_) => {
                "[resource content omitted]"
            }
            _ => "[content omitted]",
        };
        let separator_bytes = usize::from(index > 0);
        let next_size = rendered
            .len()
            .checked_add(separator_bytes)
            .and_then(|size| size.checked_add(content.len()))
            .unwrap_or(usize::MAX);
        limits.validate_bytes("rendered tool result", next_size, limits.tool_result_bytes)?;
        if separator_bytes > 0 {
            rendered.push('\n');
        }
        rendered.push_str(content);
    }
    Ok(rendered)
}

pub(super) fn map_tools(tools: Vec<Tool>) -> Vec<ToolDescriptor> {
    tools
        .into_iter()
        .map(|tool| ToolDescriptor {
            name: tool.name.into_owned(),
            description: tool.description.map(|description| description.into_owned()),
            input_schema: Some(serde_json::Value::Object(
                tool.input_schema.as_ref().clone(),
            )),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn call_validation_accepts_boundary_and_rejects_invalid_shape_size_and_depth() {
        let limits = McpLimits::DEFAULT;
        let exact_name = "x".repeat(limits.tool_name_bytes);
        assert!(validate_call_tool(&exact_name, serde_json::json!({})).is_ok());
        assert_eq!(
            validate_call_tool(
                &"x".repeat(limits.tool_name_bytes + 1),
                serde_json::json!({})
            )
            .expect_err("name limit")
            .code(),
            McpFailureCode::LimitExceeded
        );
        assert_eq!(
            validate_call_tool("tool", serde_json::json!([]))
                .expect_err("shape")
                .code(),
            McpFailureCode::Validation
        );

        let exact = serde_json::json!({
            "value": "x".repeat(limits.tool_arguments_bytes - 12)
        });
        assert_eq!(
            serde_json::to_vec(&exact).expect("serialize").len(),
            limits.tool_arguments_bytes
        );
        assert!(validate_call_tool("tool", exact).is_ok());
        let oversized = serde_json::json!({
            "value": "x".repeat(limits.tool_arguments_bytes - 11)
        });
        assert_eq!(
            validate_call_tool("tool", oversized)
                .expect_err("argument bytes")
                .code(),
            McpFailureCode::LimitExceeded
        );

        let mut exact_depth = serde_json::Value::Null;
        for _ in 1..limits.json_depth {
            exact_depth = serde_json::json!({ "nested": exact_depth });
        }
        assert!(validate_call_tool("tool", exact_depth.clone()).is_ok());
        let too_deep = serde_json::json!({ "nested": exact_depth });
        assert_eq!(
            validate_call_tool("tool", too_deep)
                .expect_err("argument depth")
                .code(),
            McpFailureCode::LimitExceeded
        );
    }

    #[test]
    fn rendered_result_accepts_exact_boundary_and_rejects_limit_plus_one() {
        let maximum = McpLimits::DEFAULT.tool_result_bytes;
        let exact = map_call_result(CallToolResult::success(vec![ContentBlock::text(
            "x".repeat(maximum),
        )]));
        assert!(!exact.is_error);
        assert_eq!(exact.content.len(), maximum);

        let oversized = map_call_result(CallToolResult::success(vec![ContentBlock::text(
            "private-result".to_string() + &"x".repeat(maximum),
        )]));
        assert!(oversized.is_error);
        assert_eq!(oversized.error_code, Some(McpFailureCode::LimitExceeded));
        assert_eq!(
            oversized.content,
            McpFailureCode::LimitExceeded.safe_message()
        );
        assert!(!oversized.content.contains("private-result"));
    }

    #[test]
    fn non_text_result_blocks_render_as_explicit_placeholders() {
        let outcome = map_call_result(CallToolResult::success(vec![
            ContentBlock::image("private-image-data", "image/png"),
            ContentBlock::audio("private-audio-data", "audio/wav"),
            ContentBlock::embedded_text("file:///private.txt", "private-resource-data"),
        ]));

        assert!(!outcome.is_error);
        assert_eq!(
            outcome.content,
            "[image content omitted]\n[audio content omitted]\n[resource content omitted]"
        );
        assert!(!outcome.content.contains("private-"));
    }
}
