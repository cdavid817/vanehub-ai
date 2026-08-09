//! The native tool-use loop's fixed, provider-agnostic tool catalog.
//! Pure data and pure functions — no I/O, unit-testable without a live provider or process.
//!
//! Risk/approval classification used to live here too (`risk_tier_for`/`requires_approval`,
//! `ToolRiskTier`) but is fully retired as of `add-permissions-core`: the approval-gate call site
//! now calls `permissions::api::evaluate` instead (via `AgentPermissionPort`,
//! `api_process_adapter.rs`'s `permission_action_and_resource` does the equivalent name/operation
//! classification `risk_tier_for` used to). Nothing in production code called either function
//! anymore once that cutover landed, so they were deleted rather than left as dead code.

use super::ToolDefinition;
use serde_json::json;

pub(crate) const SHELL_TOOL_NAME: &str = "shell";
pub(crate) const FILE_TOOL_NAME: &str = "file";
pub(crate) const REMEMBER_TOOL_NAME: &str = "remember";
pub(crate) const RECALL_TOOL_NAME: &str = "recall";
pub(crate) const SEARCH_CODE_TOOL_NAME: &str = "search_code";
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
                        "description": "Path relative to the workspace root. Hidden files and directories (any path component starting with \".\") are not accessible."
                    },
                    "content": {
                        "type": "string",
                        "description": "Content to write. Required when operation is \"write\", ignored otherwise."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "0-based index of the first line to return; 0 is the file's first line. Ignored when writing. Line numbers shown in this tool's own output and in grep results are 1-based, so to jump to displayed line N, pass offset: N-1."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return. Capped at 2000; larger values are clamped. Must be at least 1 if provided. Ignored when writing."
                    }
                },
                "required": ["operation", "path"]
            }),
        },
        grep_tool_definition(),
        glob_tool_definition(),
        edit_tool_definition(),
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
                        "description": "Path relative to the workspace root. Hidden files and directories (any path component starting with \".\") are not accessible."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "0-based index of the first line to return; 0 is the file's first line. Line numbers shown in this tool's own output and in grep results are 1-based, so to jump to displayed line N, pass offset: N-1."
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum number of lines to return. Capped at 2000; larger values are clamped. Must be at least 1 if provided."
                    }
                },
                "required": ["operation", "path"]
            }),
        },
        grep_tool_definition(),
        glob_tool_definition(),
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

pub(crate) fn search_code_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: SEARCH_CODE_TOOL_NAME.to_string(),
        description: "Search indexed code in the current session workspace and return precise file and line locations.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "The code behavior, symbol, or implementation to find."
                },
                "limit": {
                    "type": "integer",
                    "description": "How many code locations to return. Defaults to 5, capped at 20."
                }
            },
            "required": ["query"],
            "additionalProperties": false
        }),
    }
}

/// `grep` and `glob` are each offered from both `tool_catalog()` and `plan_mode_tool_catalog()`
/// (`edit` only from the former, but factored the same way for consistency) -- extracted so the
/// two catalogs share one schema each instead of maintaining duplicate copies that could drift
/// apart the first time either one is edited, following the `remember_tool_definition()`
/// precedent above.
fn grep_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: GREP_TOOL_NAME.to_string(),
        description: "Search file contents in the session's workspace folder with a regular expression. Respects .gitignore, skips hidden files and directories (any path component starting with \".\"), and skips binary files. Prefer this over running grep through the shell.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression to search for."
                },
                "glob": {
                    "type": "string",
                    "description": "Optional glob limiting which files are searched, e.g. \"**/*.rs\". Matched against paths relative to path when path is given, otherwise relative to the workspace root; matched files are still reported relative to the workspace root."
                },
                "path": {
                    "type": "string",
                    "description": "Optional subdirectory (relative to the workspace root) to search within."
                },
                "output_mode": {
                    "type": "string",
                    "enum": ["files_with_matches", "content", "count"],
                    "description": "\"files_with_matches\" (default) lists matching file paths; \"content\" returns matching lines with line numbers; \"count\" returns per-file match counts."
                },
                "context": {
                    "type": "integer",
                    "description": "Lines of context around each match. Only used when output_mode is \"content\". Capped at 20; larger values are clamped."
                },
                "case_insensitive": {
                    "type": "boolean",
                    "description": "Match case-insensitively. Defaults to false."
                },
                "head_limit": {
                    "type": "integer",
                    "description": "Maximum number of result lines to return. Capped at 200; larger values are clamped. Must be at least 1 if provided."
                }
            },
            "required": ["pattern"]
        }),
    }
}

fn glob_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: GLOB_TOOL_NAME.to_string(),
        description: "Find files by name pattern in the session's workspace folder. Respects .gitignore and skips hidden files and directories (any path component starting with \".\"). Prefer this over listing files through the shell.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern, e.g. \"**/*.test.ts\". Matched against paths relative to path when path is given, otherwise relative to the workspace root; results are always reported relative to the workspace root."
                },
                "path": {
                    "type": "string",
                    "description": "Optional subdirectory (relative to the workspace root) to search within."
                }
            },
            "required": ["pattern"]
        }),
    }
}

fn edit_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: EDIT_TOOL_NAME.to_string(),
        description: "Replace an exact string in a file relative to the session's workspace folder. old_string must match exactly once unless replace_all is true. Prefer this over rewriting a whole file with the file tool.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "path": {
                    "type": "string",
                    "description": "Path relative to the workspace root. Hidden files and directories (any path component starting with \".\") are not accessible."
                },
                "old_string": {
                    "type": "string",
                    "description": "Exact text to replace. Include enough surrounding context to match exactly once."
                },
                "new_string": {
                    "type": "string",
                    "description": "Replacement text."
                },
                "replace_all": {
                    "type": "boolean",
                    "description": "Replace every occurrence instead of requiring a unique match. Defaults to false."
                }
            },
            "required": ["path", "old_string", "new_string"]
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_declares_the_six_native_tools_in_a_stable_order() {
        let catalog = tool_catalog();
        let names: Vec<&str> = catalog.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                SHELL_TOOL_NAME,
                FILE_TOOL_NAME,
                GREP_TOOL_NAME,
                GLOB_TOOL_NAME,
                EDIT_TOOL_NAME,
                REMEMBER_TOOL_NAME,
            ]
        );
    }

    #[test]
    fn plan_mode_catalog_offers_only_read_only_tools() {
        let catalog = plan_mode_tool_catalog();
        let names: Vec<&str> = catalog.iter().map(|tool| tool.name.as_str()).collect();
        assert_eq!(
            names,
            vec![
                FILE_TOOL_NAME,
                GREP_TOOL_NAME,
                GLOB_TOOL_NAME,
                REMEMBER_TOOL_NAME,
            ]
        );
        assert_eq!(
            catalog[0].input_schema["properties"]["operation"]["enum"],
            json!(["read"])
        );
    }

    #[test]
    fn plan_mode_catalog_never_offers_shell_or_edit() {
        let names: Vec<String> = plan_mode_tool_catalog()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert!(!names.contains(&SHELL_TOOL_NAME.to_string()));
        assert!(!names.contains(&EDIT_TOOL_NAME.to_string()));
    }

    /// Pins the exact argument surface the model is told about for `grep`, cross-checked against
    /// `execute_tool_call`'s parsing in `api_process_adapter.rs`. A schema missing one of these
    /// properties would leave that argument live in the parser but unreachable from the model --
    /// exactly the failure mode `offset`/`limit` had on `file` before this task.
    #[test]
    fn grep_tool_schema_declares_its_full_argument_surface() {
        let tool = tool_catalog()
            .into_iter()
            .find(|tool| tool.name == GREP_TOOL_NAME)
            .expect("grep tool present in catalog");
        assert_eq!(tool.input_schema["required"], json!(["pattern"]));
        let properties = tool.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in [
            "pattern",
            "glob",
            "path",
            "output_mode",
            "context",
            "case_insensitive",
            "head_limit",
        ] {
            assert!(properties.contains_key(key), "missing property {key}");
        }
        assert_eq!(
            tool.input_schema["properties"]["output_mode"]["enum"],
            json!(["files_with_matches", "content", "count"])
        );
    }

    #[test]
    fn glob_tool_schema_declares_its_full_argument_surface() {
        let tool = tool_catalog()
            .into_iter()
            .find(|tool| tool.name == GLOB_TOOL_NAME)
            .expect("glob tool present in catalog");
        assert_eq!(tool.input_schema["required"], json!(["pattern"]));
        let properties = tool.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in ["pattern", "path"] {
            assert!(properties.contains_key(key), "missing property {key}");
        }
    }

    #[test]
    fn edit_tool_schema_declares_its_full_argument_surface() {
        let tool = tool_catalog()
            .into_iter()
            .find(|tool| tool.name == EDIT_TOOL_NAME)
            .expect("edit tool present in catalog");
        assert_eq!(
            tool.input_schema["required"],
            json!(["path", "old_string", "new_string"])
        );
        let properties = tool.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in ["path", "old_string", "new_string", "replace_all"] {
            assert!(properties.contains_key(key), "missing property {key}");
        }
    }

    /// Full-surface pin for `file`'s schema in both catalogs -- not just `offset`/`limit`
    /// presence, per a code-review mutation test that found deleting `content` from the full
    /// catalog's `file` schema and weakening its `required` from `["operation", "path"]` to
    /// `["operation"]` left every other test in this module (and the crate: `tool_catalog.rs` is
    /// the only file that asserts on `input_schema["properties"]` at all) green. `file` has no
    /// shared constructor like `grep`/`glob` -- its full and plan-mode schemas are two
    /// hand-maintained `ToolDefinition` literals -- so both need checking independently,
    /// including the plan-mode copy's narrower `operation` enum and its deliberately absent
    /// `content` property (plan mode cannot write).
    #[test]
    fn file_tool_schema_declares_its_full_argument_surface_in_both_catalogs() {
        let full_file = tool_catalog()
            .into_iter()
            .find(|tool| tool.name == FILE_TOOL_NAME)
            .expect("file tool present in full catalog");
        assert_eq!(
            full_file.input_schema["required"],
            json!(["operation", "path"])
        );
        assert_eq!(
            full_file.input_schema["properties"]["operation"]["enum"],
            json!(["read", "write"])
        );
        let full_properties = full_file.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in ["operation", "path", "content", "offset", "limit"] {
            assert!(full_properties.contains_key(key), "missing property {key}");
        }

        let plan_mode_file = plan_mode_tool_catalog()
            .into_iter()
            .find(|tool| tool.name == FILE_TOOL_NAME)
            .expect("file tool present in plan mode catalog");
        assert_eq!(
            plan_mode_file.input_schema["required"],
            json!(["operation", "path"])
        );
        assert_eq!(
            plan_mode_file.input_schema["properties"]["operation"]["enum"],
            json!(["read"])
        );
        let plan_mode_properties = plan_mode_file.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in ["operation", "path", "offset", "limit"] {
            assert!(
                plan_mode_properties.contains_key(key),
                "missing property {key}"
            );
        }
        // Plan mode's tool description and `operation` enum already say writing is unavailable --
        // a stray `content` property here would be the schema quietly contradicting that (see
        // finding 2: plan mode's `offset`/`limit` used to carry a leftover "Ignored when writing"
        // clause for the same reason).
        assert!(!plan_mode_properties.contains_key("content"));
    }

    #[test]
    fn the_recall_tool_never_exposes_scope_to_the_model() {
        // 这条断言从前的理由是安全边界（防模型自行放宽 scope）。`agent-memory-shared-pool`
        // 之后理由变了：记忆是一个主机级共享池，所有 Agent 本来就都看得到，压根没有"别的
        // agent 的记忆"这种切片可指定。schema 里多出任何 scope 参数，都是在向模型承诺一个
        // 检索侧根本不会执行的过滤。
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
    fn the_search_code_tool_exposes_only_query_and_limit() {
        let definition = search_code_tool_definition();
        let properties = definition.input_schema["properties"]
            .as_object()
            .expect("properties");
        assert_eq!(
            properties.keys().cloned().collect::<Vec<_>>(),
            vec!["limit".to_string(), "query".to_string()]
        );
        assert_eq!(definition.input_schema["required"], json!(["query"]));
        assert_eq!(definition.input_schema["additionalProperties"], false);
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
        assert!(tool_catalog()
            .iter()
            .all(|tool| tool.name != SEARCH_CODE_TOOL_NAME));
    }
}
