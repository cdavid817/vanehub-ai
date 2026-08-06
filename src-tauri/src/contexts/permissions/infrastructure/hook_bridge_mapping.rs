//! `PreToolUse` tool name -> `Action`/`Resource` mapping (`claude-code-permission-hook`'s
//! "Tool-to-Action mapping is explicit and partial"). The table is intentionally partial: a tool
//! without a mapping here resolves to `None`, and the caller treats that as a fail-closed deny —
//! not a pass-through. Pass-through for genuinely unmapped tools (e.g. `WebFetch`) happens one
//! layer up, at the hook's own `matcher` config in `settings.json`; this function only ever sees
//! tool names that matcher already selected, so `None` here is defense against a future mismatch
//! between that config and this table, not the primary mechanism.

use crate::contexts::permissions::domain::{Action, Resource};
use serde_json::Value;

pub(crate) fn map_tool_to_action(
    tool_name: &str,
    tool_input: &Value,
) -> Option<(Action, Resource)> {
    match tool_name {
        "Bash" => Some((Action::shell_exec(), Resource::workspace())),
        "Edit" | "Write" => Some((Action::file_write(), file_path_resource(tool_input)?)),
        "Read" | "Glob" | "Grep" => Some((
            Action::file_read(),
            file_path_resource(tool_input).unwrap_or_else(|| Resource::file_path("unspecified")),
        )),
        _ => mcp_tool_action(tool_name),
    }
}

/// Reads must always resolve to *some* resource (`file.read` is unconditionally `Allow` in
/// Phase 1 regardless of its value, so a placeholder is harmless); writes fail closed instead of
/// guessing, since a write we can't identify the target of is exactly the case that should deny.
fn file_path_resource(tool_input: &Value) -> Option<Resource> {
    for field in ["file_path", "path", "pattern"] {
        if let Some(value) = tool_input.get(field).and_then(Value::as_str) {
            if !value.is_empty() {
                return Some(Resource::file_path(value));
            }
        }
    }
    None
}

/// Claude Code names MCP tool calls `mcp__<server>__<tool>` (confirmed against official docs
/// this session). Splits on the *first* `__` after the prefix, since tool names may themselves
/// contain further `__` while server ids are simple slugs in practice; falls back to `None`
/// (fail closed) rather than guessing if the shape doesn't match.
fn mcp_tool_action(tool_name: &str) -> Option<(Action, Resource)> {
    let rest = tool_name.strip_prefix("mcp__")?;
    let (server_id, tool) = rest.split_once("__")?;
    if server_id.is_empty() || tool.is_empty() {
        return None;
    }
    Some((Action::mcp_tool(), Resource::mcp_tool(server_id, tool)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn bash_maps_to_shell_exec_on_the_workspace() {
        let (action, resource) = map_tool_to_action("Bash", &json!({"command": "ls"})).unwrap();
        assert_eq!(action, Action::shell_exec());
        assert_eq!(resource, Resource::workspace());
    }

    #[test]
    fn edit_and_write_map_to_file_write_with_the_target_path() {
        for tool in ["Edit", "Write"] {
            let (action, resource) =
                map_tool_to_action(tool, &json!({"file_path": "src/lib.rs"})).unwrap();
            assert_eq!(action, Action::file_write());
            assert_eq!(resource, Resource::file_path("src/lib.rs"));
        }
    }

    #[test]
    fn write_without_a_file_path_fails_closed() {
        assert!(map_tool_to_action("Write", &json!({})).is_none());
    }

    #[test]
    fn read_glob_grep_map_to_file_read() {
        let (action, _) = map_tool_to_action("Read", &json!({"file_path": "a.txt"})).unwrap();
        assert_eq!(action, Action::file_read());
        let (action, _) = map_tool_to_action("Glob", &json!({"pattern": "**/*.rs"})).unwrap();
        assert_eq!(action, Action::file_read());
        let (action, _) = map_tool_to_action("Grep", &json!({"pattern": "foo"})).unwrap();
        assert_eq!(action, Action::file_read());
    }

    #[test]
    fn read_without_any_path_field_still_resolves_with_a_placeholder() {
        let (action, resource) = map_tool_to_action("Read", &json!({})).unwrap();
        assert_eq!(action, Action::file_read());
        assert_eq!(resource, Resource::file_path("unspecified"));
    }

    #[test]
    fn mcp_tool_calls_split_server_and_tool_name() {
        let (action, resource) =
            map_tool_to_action("mcp__filesystem-tools__search", &json!({})).unwrap();
        assert_eq!(action, Action::mcp_tool());
        assert_eq!(resource, Resource::mcp_tool("filesystem-tools", "search"));
    }

    #[test]
    fn mcp_tool_name_containing_extra_double_underscores_keeps_them_in_the_tool_part() {
        let (_, resource) = map_tool_to_action("mcp__server__nested__tool", &json!({})).unwrap();
        assert_eq!(resource, Resource::mcp_tool("server", "nested__tool"));
    }

    #[test]
    fn malformed_mcp_prefix_fails_closed() {
        assert!(map_tool_to_action("mcp__onlyserver", &json!({})).is_none());
        assert!(map_tool_to_action("mcp__", &json!({})).is_none());
    }

    #[test]
    fn unmapped_tools_fail_closed() {
        assert!(map_tool_to_action("WebFetch", &json!({})).is_none());
        assert!(map_tool_to_action("SomeFutureTool", &json!({})).is_none());
    }
}
