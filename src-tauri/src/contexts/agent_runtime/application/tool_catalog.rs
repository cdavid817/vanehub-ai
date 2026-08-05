//! The native tool-use loop's fixed, provider-agnostic tool catalog and risk classification.
//! Pure data and pure functions — no I/O, unit-testable without a live provider or process.

use super::{ToolDefinition, ToolRiskTier};
use serde_json::{json, Value};

pub(crate) const SHELL_TOOL_NAME: &str = "shell";
pub(crate) const FILE_TOOL_NAME: &str = "file";
pub(crate) const REMEMBER_TOOL_NAME: &str = "remember";
pub(crate) const GREP_TOOL_NAME: &str = "grep";
pub(crate) const GLOB_TOOL_NAME: &str = "glob";
pub(crate) const EDIT_TOOL_NAME: &str = "edit";
/// Prefixes every MCP-sourced tool's catalog name (`mcp__<server-name>__<tool-name>`,
/// `add-agent-mcp-tools`) — never collides with the fixed names above since MCP tool names are
/// always prefixed before entering the catalog.
pub(crate) const MCP_TOOL_NAME_PREFIX: &str = "mcp__";

pub(crate) fn tool_catalog() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: SHELL_TOOL_NAME.to_string(),
            description:
                "Execute a shell command in the session's workspace folder and return its output."
                    .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "command": {
                        "type": "string",
                        "description": "The shell command to execute."
                    }
                },
                "required": ["command"]
            }),
        },
        ToolDefinition {
            name: FILE_TOOL_NAME.to_string(),
            description: "Read or write a file relative to the session's workspace folder."
                .to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read", "write"],
                        "description": "Whether to read an existing file or write (create/overwrite) one."
                    },
                    "path": {
                        "type": "string",
                        "description": "Path relative to the workspace root."
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write. Required when operation is \"write\", ignored otherwise."
                    }
                },
                "required": ["operation", "path"]
            }),
        },
        remember_tool_definition(),
    ]
}

/// The catalog offered when the session's permission mode is plan mode
/// (`add-agent-chat-configuration`): read-only exploration only. Excludes `shell` entirely;
/// narrows `file` to its `read` operation; keeps `remember` (VaneHub-internal storage only, not a
/// "real" side effect this mode cares about, already auto-approved everywhere else). This shapes
/// what the model is *told* it can do — `execute_tool_call`'s own plan-mode checks are the actual
/// enforcement boundary, since nothing stops a model from requesting a tool/operation it was
/// never offered.
pub(crate) fn plan_mode_tool_catalog() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            name: FILE_TOOL_NAME.to_string(),
            description: "Read a file relative to the session's workspace folder. Plan mode is active: writing files is not available.".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "operation": {
                        "type": "string",
                        "enum": ["read"],
                        "description": "Only reading is available in plan mode."
                    },
                    "path": {
                        "type": "string",
                        "description": "Path relative to the workspace root."
                    }
                },
                "required": ["operation", "path"]
            }),
        },
        remember_tool_definition(),
    ]
}

fn remember_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: REMEMBER_TOOL_NAME.to_string(),
        description: "Save a fact, decision, or preference so it's available in future, separate sessions with this same project. Use for information worth remembering long-term, not routine conversation.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "content": {
                    "type": "string",
                    "description": "The fact, decision, or preference to remember."
                }
            },
            "required": ["content"]
        }),
    }
}

/// Classifies a tool call's risk tier by tool name and, for the file tool, its `operation`
/// field — a structural distinction (which operation was requested), not a content-safety
/// judgment about a specific command or path. Unknown tool names and file calls with a missing
/// or unrecognized `operation` fail closed to `RequiresApproval`.
pub(crate) fn risk_tier_for(tool_name: &str, input: &Value) -> ToolRiskTier {
    match tool_name {
        FILE_TOOL_NAME => match input.get("operation").and_then(Value::as_str) {
            Some("read") => ToolRiskTier::AutoApprove,
            _ => ToolRiskTier::RequiresApproval,
        },
        // Only ever writes to this app's own internal storage — never the user's filesystem,
        // shell, or anything else external — so a wrong or low-value memory is no worse than a
        // mistake the user can delete via the memory management view (`add-agent-cross-session-memory`).
        REMEMBER_TOOL_NAME => ToolRiskTier::AutoApprove,
        // Both are read-only and bounded by the workspace boundary and .gitignore. Making them
        // auto-approve is the entire point of this capability — prompting on every search would
        // push the model back toward guessing via shell instead, which is more dangerous, not less.
        GREP_TOOL_NAME | GLOB_TOOL_NAME => ToolRiskTier::AutoApprove,
        // Falls into the default `_` arm below anyway, but listed explicitly so this contract is
        // locked by its own test (`edit_always_requires_approval`) rather than being an accident
        // of the default branch that a future change to that branch could silently widen.
        EDIT_TOOL_NAME => ToolRiskTier::RequiresApproval,
        _ => ToolRiskTier::RequiresApproval,
    }
}

/// Whether a tool call must pause for human approval, composing `risk_tier_for`'s static
/// classification with a per-agent trust grant (`add-agent-tool-trust`). `risk_tier_for` itself
/// stays agent-trust-unaware — this function is the only place the two are combined, kept
/// separate so `risk_tier_for`'s existing pure "classify by name+input" contract and test suite
/// need no changes. `auto_approve_tools` can only ever skip approval for `shell`, `file`, and
/// `edit` calls — MCP-sourced calls always fall through to `risk_tier_for`'s own unconditional
/// `RequiresApproval` for any name it doesn't explicitly recognize, with no carve-out here.
pub(crate) fn requires_approval(tool_name: &str, input: &Value, auto_approve_tools: bool) -> bool {
    if auto_approve_tools
        && (tool_name == SHELL_TOOL_NAME
            || tool_name == FILE_TOOL_NAME
            || tool_name == EDIT_TOOL_NAME)
    {
        return false;
    }
    risk_tier_for(tool_name, input) == ToolRiskTier::RequiresApproval
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_declares_exactly_shell_file_and_remember_tools() {
        let catalog = tool_catalog();
        assert_eq!(catalog.len(), 3);
        assert_eq!(catalog[0].name, SHELL_TOOL_NAME);
        assert_eq!(catalog[1].name, FILE_TOOL_NAME);
        assert_eq!(catalog[2].name, REMEMBER_TOOL_NAME);
    }

    #[test]
    fn plan_mode_catalog_offers_only_read_only_file_and_remember() {
        let catalog = plan_mode_tool_catalog();
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog[0].name, FILE_TOOL_NAME);
        assert_eq!(
            catalog[0].input_schema["properties"]["operation"]["enum"],
            json!(["read"])
        );
        assert_eq!(catalog[1].name, REMEMBER_TOOL_NAME);
    }

    #[test]
    fn shell_always_requires_approval() {
        assert_eq!(
            risk_tier_for(SHELL_TOOL_NAME, &json!({"command": "ls"})),
            ToolRiskTier::RequiresApproval
        );
        assert_eq!(
            risk_tier_for(SHELL_TOOL_NAME, &json!({"command": "rm -rf /"})),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn file_read_auto_approves() {
        assert_eq!(
            risk_tier_for(
                FILE_TOOL_NAME,
                &json!({"operation": "read", "path": "a.txt"})
            ),
            ToolRiskTier::AutoApprove
        );
    }

    #[test]
    fn file_write_requires_approval() {
        assert_eq!(
            risk_tier_for(
                FILE_TOOL_NAME,
                &json!({"operation": "write", "path": "a.txt", "content": "x"})
            ),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn file_call_with_missing_or_unknown_operation_fails_closed() {
        assert_eq!(
            risk_tier_for(FILE_TOOL_NAME, &json!({"path": "a.txt"})),
            ToolRiskTier::RequiresApproval
        );
        assert_eq!(
            risk_tier_for(
                FILE_TOOL_NAME,
                &json!({"operation": "delete", "path": "a.txt"})
            ),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn unknown_tool_name_fails_closed() {
        assert_eq!(
            risk_tier_for("unknown", &json!({})),
            ToolRiskTier::RequiresApproval
        );
    }

    /// Locks in design.md Decision 7 (`add-agent-mcp-tools`): MCP tool names always fall through
    /// to the existing catch-all above since they never literally equal `FILE_TOOL_NAME` or
    /// `REMEMBER_TOOL_NAME` (guaranteed by the mandatory `mcp__` prefix) — no production code
    /// change is needed for MCP calls to require approval unconditionally, but that behavior
    /// deserves its own test rather than being an accident of the existing match arms.
    #[test]
    fn mcp_sourced_tool_names_always_require_approval() {
        assert_eq!(
            risk_tier_for("mcp__filesystem-tools__search", &json!({"query": "x"})),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn remember_always_auto_approves() {
        assert_eq!(
            risk_tier_for(REMEMBER_TOOL_NAME, &json!({"content": "Uses pnpm."})),
            ToolRiskTier::AutoApprove
        );
    }

    #[test]
    fn trusted_agent_skips_approval_for_shell() {
        assert!(!requires_approval(
            SHELL_TOOL_NAME,
            &json!({"command": "rm -rf /"}),
            true
        ));
    }

    #[test]
    fn trusted_agent_skips_approval_for_file_write() {
        assert!(!requires_approval(
            FILE_TOOL_NAME,
            &json!({"operation": "write", "path": "a.txt", "content": "x"}),
            true
        ));
    }

    #[test]
    fn untrusted_agent_still_requires_approval_for_shell_and_file_write() {
        assert!(requires_approval(
            SHELL_TOOL_NAME,
            &json!({"command": "ls"}),
            false
        ));
        assert!(requires_approval(
            FILE_TOOL_NAME,
            &json!({"operation": "write", "path": "a.txt", "content": "x"}),
            false
        ));
    }

    #[test]
    fn trusted_agent_still_requires_approval_for_mcp_calls() {
        assert!(requires_approval(
            "mcp__filesystem-tools__search",
            &json!({"query": "x"}),
            true
        ));
    }

    #[test]
    fn trust_flag_never_affects_already_auto_approved_tools() {
        assert!(!requires_approval(
            REMEMBER_TOOL_NAME,
            &json!({"content": "Uses pnpm."}),
            false
        ));
        assert!(!requires_approval(
            FILE_TOOL_NAME,
            &json!({"operation": "read", "path": "a.txt"}),
            false
        ));
    }

    #[test]
    fn search_tools_do_not_require_approval() {
        assert_eq!(
            risk_tier_for(GREP_TOOL_NAME, &json!({"pattern": "needle"})),
            ToolRiskTier::AutoApprove
        );
        assert_eq!(
            risk_tier_for(GLOB_TOOL_NAME, &json!({"pattern": "**/*.rs"})),
            ToolRiskTier::AutoApprove
        );
    }

    #[test]
    fn edit_always_requires_approval() {
        assert_eq!(
            risk_tier_for(
                EDIT_TOOL_NAME,
                &json!({"path": "a.rs", "old_string": "a", "new_string": "b"})
            ),
            ToolRiskTier::RequiresApproval
        );
    }

    #[test]
    fn a_trusted_agent_skips_approval_for_edit() {
        let input = json!({"path": "a.rs", "old_string": "a", "new_string": "b"});
        assert!(requires_approval(EDIT_TOOL_NAME, &input, false));
        assert!(!requires_approval(EDIT_TOOL_NAME, &input, true));
    }

    #[test]
    fn trust_does_not_widen_to_mcp_tools() {
        let input = json!({});
        assert!(requires_approval("mcp__server__tool", &input, true));
    }
}
