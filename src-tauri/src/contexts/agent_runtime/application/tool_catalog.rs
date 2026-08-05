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
                    },
                    "offset": {
                        "type": "integer",
                        "description": "0-based index of the first line to return; 0 is the file's first line. Ignored when writing."
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
                        "description": "Path relative to the workspace root."
                    },
                    "offset": {
                        "type": "integer",
                        "description": "0-based index of the first line to return; 0 is the file's first line."
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

/// `grep` and `glob` are each offered from both `tool_catalog()` and `plan_mode_tool_catalog()`
/// (`edit` only from the former, but factored the same way for consistency) -- extracted so the
/// two catalogs share one schema each instead of maintaining duplicate copies that could drift
/// apart the first time either one is edited, following the `remember_tool_definition()`
/// precedent above.
fn grep_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: GREP_TOOL_NAME.to_string(),
        description: "Search file contents in the session's workspace folder with a regular expression. Respects .gitignore and skips binary files. Prefer this over running grep through the shell.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Regular expression to search for."
                },
                "glob": {
                    "type": "string",
                    "description": "Optional glob limiting which files are searched, e.g. \"**/*.rs\"."
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
        description: "Find files by name pattern in the session's workspace folder. Respects .gitignore. Prefer this over listing files through the shell.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "pattern": {
                    "type": "string",
                    "description": "Glob pattern matched against workspace-relative paths, e.g. \"**/*.test.ts\"."
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
                    "description": "Path relative to the workspace root."
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
        // `grep`/`glob` are AutoApprove unconditionally and are not part of `requires_approval`'s
        // trust allowlist at all -- unlike `shell`/`file`/`edit`, nothing here should depend on
        // the trust flag either way. Exercises `requires_approval` itself (the actual production
        // entry point composing `risk_tier_for` with the trust flag), not just `risk_tier_for` in
        // isolation, with the flag both on and off: a future change that narrows
        // `requires_approval`'s carve-out logic and accidentally starts gating search tools on
        // trust would fail here even though `search_tools_do_not_require_approval` alone
        // (asserting on `risk_tier_for`) would still pass.
        for trusted in [false, true] {
            assert!(!requires_approval(
                GREP_TOOL_NAME,
                &json!({"pattern": "needle"}),
                trusted
            ));
            assert!(!requires_approval(
                GLOB_TOOL_NAME,
                &json!({"pattern": "**/*.rs"}),
                trusted
            ));
        }
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
}
