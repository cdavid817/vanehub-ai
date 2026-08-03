use super::{McpLimits, McpRuntimeError};
use crate::contexts::tooling::mcp::domain::{ConnectionOutcome, McpFailureCode, ToolDescriptor};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SerializedTool<'a> {
    name: &'a str,
    description: Option<&'a str>,
    input_schema: Option<&'a serde_json::Value>,
}

pub(super) fn enforce_outcome(outcome: ConnectionOutcome) -> ConnectionOutcome {
    let ConnectionOutcome::Connected { tools, duration_ms } = outcome else {
        return outcome;
    };
    match validate_catalog(&tools) {
        Ok(()) => ConnectionOutcome::connected(tools, duration_ms),
        Err(error) => {
            ConnectionOutcome::failed_with_code(error.to_string(), error.code(), duration_ms)
        }
    }
}

pub(super) fn validate_catalog(tools: &[ToolDescriptor]) -> Result<(), McpRuntimeError> {
    let limits = McpLimits::DEFAULT;
    limits.validate_count("MCP tool count", tools.len(), limits.tools_per_server)?;
    for tool in tools {
        if tool.name.trim().is_empty() {
            return Err(McpRuntimeError::with_diagnostic(
                McpFailureCode::Validation,
                "MCP tool name must not be empty",
            ));
        }
        limits.validate_bytes("MCP tool name", tool.name.len(), limits.tool_name_bytes)?;
        if let Some(description) = &tool.description {
            limits.validate_bytes(
                "MCP tool description",
                description.len(),
                limits.tool_description_bytes,
            )?;
        }
        if let Some(schema) = &tool.input_schema {
            if !schema.is_object() {
                return Err(McpRuntimeError::with_diagnostic(
                    McpFailureCode::Validation,
                    "MCP tool input schema must be an object",
                ));
            }
            limits.validate_json(
                "MCP tool input schema",
                schema,
                limits.schema_bytes,
                limits.json_depth,
            )?;
        }
    }
    limits.validate_bytes(
        "MCP tool catalog",
        serialized_size(tools)?,
        limits.catalog_serialized_bytes,
    )?;
    Ok(())
}

fn serialized_size(tools: &[ToolDescriptor]) -> Result<usize, McpRuntimeError> {
    let serialized = tools
        .iter()
        .map(|tool| SerializedTool {
            name: &tool.name,
            description: tool.description.as_deref(),
            input_schema: tool.input_schema.as_ref(),
        })
        .collect::<Vec<_>>();
    McpLimits::DEFAULT.validate_serialized("MCP tool catalog", &serialized, usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn tool(name: String) -> ToolDescriptor {
        ToolDescriptor {
            name,
            description: None,
            input_schema: None,
        }
    }

    fn nested_object(depth: usize) -> Value {
        (1..depth).fold(Value::Null, |value, _| {
            Value::Object(serde_json::Map::from_iter([("nested".to_string(), value)]))
        })
    }

    fn object_with_size(target: usize) -> Value {
        let mut value = serde_json::json!({ "value": "" });
        let base = serde_json::to_vec(&value).expect("base schema").len();
        let Value::String(contents) = value
            .as_object_mut()
            .expect("object")
            .get_mut("value")
            .expect("value")
        else {
            panic!("string value");
        };
        contents.push_str(&"x".repeat(target - base));
        assert_eq!(serde_json::to_vec(&value).expect("schema").len(), target);
        value
    }

    fn catalog_with_size(target: usize) -> Vec<ToolDescriptor> {
        let limits = McpLimits::DEFAULT;
        let mut tools = (0..15)
            .map(|index| ToolDescriptor {
                name: format!("tool-{index}"),
                description: None,
                input_schema: Some(object_with_size(limits.schema_bytes)),
            })
            .collect::<Vec<_>>();
        tools.push(ToolDescriptor {
            name: "tool-last".to_string(),
            description: None,
            input_schema: Some(serde_json::json!({ "value": "" })),
        });
        let base = serialized_size(&tools).expect("base size");
        assert!(base <= target);
        let Value::String(value) = tools
            .last_mut()
            .expect("last tool")
            .input_schema
            .as_mut()
            .expect("schema")
            .as_object_mut()
            .expect("object")
            .get_mut("value")
            .expect("value")
        else {
            panic!("string schema");
        };
        value.push_str(&"x".repeat(target - base));
        assert_eq!(serialized_size(&tools).expect("target size"), target);
        tools
    }

    #[test]
    fn tool_count_name_and_description_accept_boundary_and_reject_limit_plus_one() {
        let limits = McpLimits::DEFAULT;
        let tools = (0..limits.tools_per_server)
            .map(|index| tool(format!("tool-{index}")))
            .collect::<Vec<_>>();
        assert!(validate_catalog(&tools).is_ok());
        let mut too_many = tools;
        too_many.push(tool("extra".to_string()));
        assert_eq!(
            validate_catalog(&too_many).expect_err("tool count").code(),
            McpFailureCode::LimitExceeded
        );

        assert!(validate_catalog(&[tool("x".repeat(limits.tool_name_bytes))]).is_ok());
        assert!(validate_catalog(&[tool("x".repeat(limits.tool_name_bytes + 1))]).is_err());

        let mut described = tool("described".to_string());
        described.description = Some("x".repeat(limits.tool_description_bytes));
        assert!(validate_catalog(&[described.clone()]).is_ok());
        described.description = Some("x".repeat(limits.tool_description_bytes + 1));
        assert!(validate_catalog(&[described]).is_err());
    }

    #[test]
    fn schema_size_and_depth_accept_boundary_and_reject_limit_plus_one() {
        let limits = McpLimits::DEFAULT;
        let mut schema_tool = tool("schema".to_string());
        schema_tool.input_schema = Some(object_with_size(limits.schema_bytes));
        assert!(validate_catalog(&[schema_tool.clone()]).is_ok());
        schema_tool.input_schema = Some(object_with_size(limits.schema_bytes + 1));
        assert!(validate_catalog(&[schema_tool]).is_err());

        let mut depth_tool = tool("depth".to_string());
        depth_tool.input_schema = Some(nested_object(limits.json_depth));
        assert!(validate_catalog(&[depth_tool.clone()]).is_ok());
        depth_tool.input_schema = Some(nested_object(limits.json_depth + 1));
        assert!(validate_catalog(&[depth_tool]).is_err());
    }

    #[test]
    fn malformed_descriptors_fail_validation() {
        assert_eq!(
            validate_catalog(&[tool("  ".to_string())])
                .expect_err("empty name")
                .code(),
            McpFailureCode::Validation
        );
        let mut invalid_schema = tool("invalid-schema".to_string());
        invalid_schema.input_schema = Some(Value::Array(Vec::new()));
        assert_eq!(
            validate_catalog(&[invalid_schema])
                .expect_err("schema shape")
                .code(),
            McpFailureCode::Validation
        );
    }

    #[test]
    fn serialized_catalog_accepts_boundary_and_rejects_limit_plus_one() {
        let maximum = McpLimits::DEFAULT.catalog_serialized_bytes;
        assert!(validate_catalog(&catalog_with_size(maximum)).is_ok());
        assert_eq!(
            validate_catalog(&catalog_with_size(maximum + 1))
                .expect_err("catalog limit")
                .code(),
            McpFailureCode::LimitExceeded
        );
    }

    #[test]
    fn oversized_connected_outcome_becomes_safe_limit_failure() {
        let outcome = enforce_outcome(ConnectionOutcome::connected(
            vec![tool("x".repeat(McpLimits::DEFAULT.tool_name_bytes + 1))],
            17,
        ));
        assert_eq!(outcome.error_code(), Some(McpFailureCode::LimitExceeded));
        assert_eq!(
            outcome.error(),
            Some(McpFailureCode::LimitExceeded.safe_message())
        );
        assert_eq!(outcome.duration_ms(), 17);
    }
}
