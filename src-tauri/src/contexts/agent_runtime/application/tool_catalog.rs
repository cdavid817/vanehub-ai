//! The native tool-use loop's fixed, provider-agnostic tool catalog and risk classification.
//! Pure data and pure functions — no I/O, unit-testable without a live provider or process.

use super::{ToolDefinition, ToolRiskTier};
use serde_json::{json, Value};

pub(crate) const SHELL_TOOL_NAME: &str = "shell";
pub(crate) const FILE_TOOL_NAME: &str = "file";
pub(crate) const REMEMBER_TOOL_NAME: &str = "remember";
pub(crate) const RECALL_TOOL_NAME: &str = "recall";
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

/// scope（agent id 与工作区文件夹）**刻意不进 schema**：它由运行时从会话上下文注入，模型无法
/// 指定——否则模型可构造参数读取其他 agent 或其他项目的记忆。这是安全边界，不是省事。Not part
/// of the unconditional `tool_catalog()`/`plan_mode_tool_catalog()` — `resolve_tool_catalog`
/// injects it only when retrieval is actually configured.
pub(crate) fn recall_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: RECALL_TOOL_NAME.to_string(),
        description: "Search your saved memories for this project by meaning, not just keywords. Use when the user refers to something from an earlier session, or when you need context that isn't in the current conversation.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "What to look for, in natural language."
                },
                "limit": {
                    "type": "integer",
                    "description": "How many memories to return. Defaults to 5, capped at 20."
                }
            },
            "required": ["query"]
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
        // Only ever reads this app's own internal storage — never the user's filesystem, shell,
        // or anything else external. The one new outbound surface is the query text going to the
        // embedding provider, which is not a new exposure: the memory content it searches over
        // was already sent to that same provider at index time.
        RECALL_TOOL_NAME => ToolRiskTier::AutoApprove,
        _ => ToolRiskTier::RequiresApproval,
    }
}

/// Whether a tool call must pause for human approval, composing `risk_tier_for`'s static
/// classification with a per-agent trust grant (`add-agent-tool-trust`). `risk_tier_for` itself
/// stays agent-trust-unaware — this function is the only place the two are combined, kept
/// separate so `risk_tier_for`'s existing pure "classify by name+input" contract and test suite
/// need no changes. `auto_approve_tools` can only ever skip approval for `shell` and `file`
/// calls — MCP-sourced calls always fall through to `risk_tier_for`'s own unconditional
/// `RequiresApproval` for any name it doesn't explicitly recognize, with no carve-out here.
pub(crate) fn requires_approval(tool_name: &str, input: &Value, auto_approve_tools: bool) -> bool {
    if auto_approve_tools && (tool_name == SHELL_TOOL_NAME || tool_name == FILE_TOOL_NAME) {
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
    fn the_recall_tool_never_exposes_scope_to_the_model() {
        // scope 若进 schema，模型就能构造参数读别的 agent 或别的项目的记忆。这是安全边界。
        let definition = recall_tool_definition();
        let properties = definition.input_schema["properties"]
            .as_object()
            .expect("properties");
        assert!(properties.contains_key("query"));
        assert!(properties.contains_key("limit"));
        assert_eq!(properties.len(), 2);
        for forbidden in ["agent_id", "agentId", "folder", "scope", "project"] {
            assert!(
                !properties.contains_key(forbidden),
                "{forbidden} must not be model-supplied"
            );
        }
        assert_eq!(definition.input_schema["required"], json!(["query"]));
    }

    #[test]
    fn recall_auto_approves_for_the_same_reason_remember_does() {
        assert_eq!(
            risk_tier_for(RECALL_TOOL_NAME, &json!({"query": "npm"})),
            ToolRiskTier::AutoApprove
        );
    }

    #[test]
    fn the_fixed_catalog_stays_unconditional_and_excludes_recall() {
        // tool_catalog()/plan_mode_tool_catalog() 保持纯函数、不感知配置；
        // 条件性只存在于 resolve_tool_catalog()。
        assert!(tool_catalog()
            .iter()
            .all(|tool| tool.name != RECALL_TOOL_NAME));
        assert!(plan_mode_tool_catalog()
            .iter()
            .all(|tool| tool.name != RECALL_TOOL_NAME));
    }
}
