use crate::contexts::agent_runtime::application::{
    AgentMcpToolPort, AgentRuntimeApplicationError, AgentToolCallOutcome, ToolDefinition,
    MCP_TOOL_NAME_PREFIX,
};
use crate::contexts::tooling::mcp::api::{McpApi, McpServerToolEntry};
use serde_json::{json, Value};

/// Wraps `tooling::mcp`'s public facade to satisfy `agent_runtime`'s own `AgentMcpToolPort` —
/// mirrors `RuntimeAgentSkillAdapter`'s existing pattern for depending on another context's API
/// through an `agent_runtime`-owned port rather than that context's types directly. Owns the
/// sync/async bridge (`tauri::async_runtime::block_on`) so the port trait itself can stay sync,
/// matching how it's consumed from the tool-execution loop's synchronous call chain.
#[derive(Clone)]
pub(crate) struct RuntimeAgentMcpToolAdapter {
    mcp: McpApi,
}

impl RuntimeAgentMcpToolAdapter {
    pub(crate) fn new(mcp: McpApi) -> Self {
        Self { mcp }
    }
}

impl AgentMcpToolPort for RuntimeAgentMcpToolAdapter {
    fn catalog_entries(
        &self,
        project_path: &str,
    ) -> Result<Vec<ToolDefinition>, AgentRuntimeApplicationError> {
        self.mcp
            .visible_tool_catalog(project_path)
            .map(|entries| entries.into_iter().map(to_tool_definition).collect())
            .map_err(|error| AgentRuntimeApplicationError::Mcp(error.to_string()))
    }

    fn call_tool(
        &self,
        project_path: &str,
        tool_name: &str,
        arguments: &Value,
    ) -> AgentToolCallOutcome {
        let Some((server_name, remote_tool_name)) = split_tool_name(tool_name) else {
            return AgentToolCallOutcome {
                output: format!("\"{tool_name}\" is not a valid MCP tool name."),
                is_error: true,
            };
        };
        let result = tauri::async_runtime::block_on(self.mcp.call_tool(
            project_path,
            server_name,
            remote_tool_name,
            arguments.clone(),
        ));
        match result {
            Ok(outcome) => AgentToolCallOutcome {
                output: outcome.content,
                is_error: outcome.is_error,
            },
            Err(error) => AgentToolCallOutcome {
                output: error.to_string(),
                is_error: true,
            },
        }
    }
}

fn to_tool_definition(entry: McpServerToolEntry) -> ToolDefinition {
    ToolDefinition {
        name: format!(
            "{MCP_TOOL_NAME_PREFIX}{}__{}",
            entry.server_name, entry.tool.name
        ),
        description: entry.tool.description.unwrap_or_default(),
        input_schema: entry
            .tool
            .input_schema
            .unwrap_or_else(|| json!({ "type": "object" })),
    }
}

/// Splits a catalog tool name back into `(server_name, tool_name)`. Only splits on the *first*
/// `__` after the prefix — MCP server names are validated kebab-case (no `_` at all), so the
/// first `__` is always the true boundary no matter how many further `__` sequences the remote
/// tool's own name contains.
fn split_tool_name(name: &str) -> Option<(&str, &str)> {
    name.strip_prefix(MCP_TOOL_NAME_PREFIX)?.split_once("__")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_the_server_name_from_the_tool_name_on_the_first_double_underscore() {
        assert_eq!(
            split_tool_name("mcp__filesystem-tools__search"),
            Some(("filesystem-tools", "search"))
        );
    }

    #[test]
    fn a_tool_name_containing_its_own_double_underscore_still_splits_correctly() {
        assert_eq!(
            split_tool_name("mcp__filesystem-tools__search__advanced"),
            Some(("filesystem-tools", "search__advanced"))
        );
    }

    #[test]
    fn a_name_without_the_mcp_prefix_does_not_split() {
        assert_eq!(split_tool_name("shell"), None);
    }

    #[test]
    fn to_tool_definition_prefixes_the_name_and_defaults_missing_fields() {
        let entry = McpServerToolEntry {
            server_name: "filesystem-tools".to_string(),
            tool: crate::contexts::tooling::mcp::api::ToolDescriptor {
                name: "search".to_string(),
                description: None,
                input_schema: None,
            },
        };

        let definition = to_tool_definition(entry);

        assert_eq!(definition.name, "mcp__filesystem-tools__search");
        assert_eq!(definition.description, "");
        assert_eq!(definition.input_schema, json!({ "type": "object" }));
    }

    #[test]
    fn to_tool_definition_preserves_provided_description_and_schema() {
        let entry = McpServerToolEntry {
            server_name: "filesystem-tools".to_string(),
            tool: crate::contexts::tooling::mcp::api::ToolDescriptor {
                name: "search".to_string(),
                description: Some("Search files".to_string()),
                input_schema: Some(json!({ "type": "object", "properties": {} })),
            },
        };

        let definition = to_tool_definition(entry);

        assert_eq!(definition.description, "Search files");
        assert_eq!(
            definition.input_schema,
            json!({ "type": "object", "properties": {} })
        );
    }
}
