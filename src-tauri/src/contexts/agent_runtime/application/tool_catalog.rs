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
pub(crate) const SHELL_OUTPUT_TOOL_NAME: &str = "shell_output";
pub(crate) const SHELL_KILL_TOOL_NAME: &str = "shell_kill";
pub(crate) const FILE_TOOL_NAME: &str = "file";
pub(crate) const REMEMBER_TOOL_NAME: &str = "remember";
pub(crate) const RECALL_TOOL_NAME: &str = "recall";
pub(crate) const SEARCH_CODE_TOOL_NAME: &str = "search_code";
pub(crate) const GREP_TOOL_NAME: &str = "grep";
pub(crate) const GLOB_TOOL_NAME: &str = "glob";
pub(crate) const EDIT_TOOL_NAME: &str = "edit";
pub(crate) const LIST_SKILLS_TOOL_NAME: &str = "list_skills";
pub(crate) const LOAD_SKILL_TOOL_NAME: &str = "load_skill";
pub(crate) const READ_SKILL_RESOURCE_TOOL_NAME: &str = "read_skill_resource";
pub(crate) const DELEGATE_UTILITY_SKILL_TOOL_NAME: &str = "delegate_utility_skill";
pub(crate) const FIND_DEFINITION_TOOL_NAME: &str = "find_definition";
pub(crate) const FIND_REFERENCES_TOOL_NAME: &str = "find_references";
pub(crate) const GET_HOVER_TOOL_NAME: &str = "get_hover";
pub(crate) const GET_DIAGNOSTICS_TOOL_NAME: &str = "get_diagnostics";
/// Prefixes every MCP-sourced tool's catalog name (`mcp__<server-name>__<tool-name>`,
/// `add-agent-mcp-tools`) — never collides with the fixed names above since MCP tool names are
/// always prefixed before entering the catalog.
pub(crate) const MCP_TOOL_NAME_PREFIX: &str = "mcp__";

pub(crate) fn tool_catalog() -> Vec<ToolDefinition> {
    vec![
        shell_tool_definition(),
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
        list_skills_tool_definition(),
        load_skill_tool_definition(),
        read_skill_resource_tool_definition(),
        shell_output_tool_definition(),
        shell_kill_tool_definition(),
    ]
}

fn shell_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: SHELL_TOOL_NAME.to_string(),
        description:
            "Execute a shell command in the session's workspace folder. By default the call blocks \
             and returns the command's output. Set run_in_background for work that outlives one \
             tool call -- a build, a test suite, a dev server -- and poll it with shell_output."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute."
                },
                "run_in_background": {
                    "type": "boolean",
                    "description": "Run the command in the background and return a handle immediately instead of waiting for it to finish. Poll the handle with shell_output and stop it with shell_kill. Defaults to false."
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "How long to wait for a foreground command, in milliseconds. Defaults to 60000 and is capped at 600000; a larger value is clamped rather than rejected. Ignored when run_in_background is true, which is bounded by a maximum background lifetime instead."
                }
            },
            "required": ["command"]
        }),
    }
}

fn shell_output_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: SHELL_OUTPUT_TOOL_NAME.to_string(),
        description: "Read the output a background command has produced since the last time you \
                      read it, along with its current status and, once it has finished, its exit \
                      code. Each call returns only new output."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "shell_id": {
                    "type": "string",
                    "description": "The handle returned when the background command was started."
                }
            },
            "required": ["shell_id"],
            "additionalProperties": false
        }),
    }
}

fn shell_kill_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: SHELL_KILL_TOOL_NAME.to_string(),
        description:
            "Terminate a running background command and every process it started. Reports the \
             command's existing outcome instead if it has already finished."
                .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "shell_id": {
                    "type": "string",
                    "description": "The handle returned when the background command was started."
                }
            },
            "required": ["shell_id"],
            "additionalProperties": false
        }),
    }
}

pub(crate) fn delegate_utility_skill_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: DELEGATE_UTILITY_SKILL_TOOL_NAME.to_string(),
        description: "Delegate one bounded specialist task to an available Utility Skill. VaneHub selects the workspace, provider, credentials, and executor.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "skill_id": { "type": "string", "description": "Canonical Utility Skill id or unambiguous alias." },
                "task": { "type": "string", "maxLength": 4000, "description": "The bounded specialist task." },
                "duration_ms": { "type": "integer", "minimum": 1, "maximum": 300000 },
                "tool_calls": { "type": "integer", "minimum": 0, "maximum": 32 },
                "approvals": { "type": "integer", "minimum": 0, "maximum": 8 },
                "result_chars": { "type": "integer", "minimum": 1, "maximum": 4000 }
            },
            "required": ["skill_id", "task"],
            "additionalProperties": false
        }),
    }
}

/// The catalog offered when the session's permission mode is plan mode
/// (`add-agent-chat-configuration`): read-only exploration only. Excludes `shell` entirely;
/// narrows `file` to its `read` operation; keeps `remember` (VaneHub-internal storage only, not a
/// "real" side effect this mode cares about, already auto-approved everywhere else). This shapes
/// what the model is *told* it can do — `execute_tool_call`'s own plan-mode checks are the actual
/// enforcement boundary, since nothing stops a model from requesting a tool/operation it was
/// never offered.
///
/// `shell_output` is here but `shell_kill` is not (`add-background-shell-execution`): reading a
/// background command's output observes work that was already approved before plan mode was
/// entered and cannot start, change, or stop anything, so a model that switches into plan mode
/// mid-task can still read the build it started. `shell_kill` acts on a process, which is exactly
/// what this mode withholds.
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
        list_skills_tool_definition(),
        load_skill_tool_definition(),
        read_skill_resource_tool_definition(),
        shell_output_tool_definition(),
    ]
}

fn list_skills_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: LIST_SKILLS_TOOL_NAME.to_string(),
        description: "List bounded metadata for effective Skills available to this session. Instruction bodies are never returned by this tool.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "maxLength": 80,
                    "description": "Optional text matched against Skill id, name, description, and aliases."
                },
                "type": {
                    "type": "string",
                    "enum": ["role", "utility"],
                    "description": "Optional Skill type filter."
                },
                "delivery": {
                    "type": "string",
                    "enum": ["eager", "on-demand"],
                    "description": "Optional delivery filter."
                },
                "availability": {
                    "type": "string",
                    "enum": ["available", "disabled", "invalid", "conflicting", "unsupported"],
                    "description": "Optional availability filter."
                },
                "limit": {
                    "type": "integer",
                    "minimum": 1,
                    "maximum": 100,
                    "description": "Maximum number of results. Defaults to 50."
                }
            },
            "additionalProperties": false
        }),
    }
}

fn load_skill_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: LOAD_SKILL_TOOL_NAME.to_string(),
        description: "Load bounded instructions and a logical resource index for one effective Role Skill by canonical id or alias.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "id": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 64,
                    "description": "Canonical Skill id or unambiguous alias."
                }
            },
            "required": ["id"],
            "additionalProperties": false
        }),
    }
}

fn read_skill_resource_tool_definition() -> ToolDefinition {
    ToolDefinition {
        name: READ_SKILL_RESOURCE_TOOL_NAME.to_string(),
        description: "Read one bounded text resource using a logical URI and revision returned by load_skill. This tool never accepts host filesystem paths.".to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "uri": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 512,
                    "pattern": "^skill://",
                    "description": "Logical skill:// resource URI returned by load_skill."
                },
                "revision": {
                    "type": "string",
                    "minLength": 1,
                    "maxLength": 128,
                    "description": "Effective package revision returned by load_skill."
                }
            },
            "required": ["uri", "revision"],
            "additionalProperties": false
        }),
    }
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

pub(crate) fn code_intelligence_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        positioned_code_intelligence_tool(
            FIND_DEFINITION_TOOL_NAME,
            "Find definitions for the symbol at a position in the current workspace.",
        ),
        positioned_code_intelligence_tool(
            FIND_REFERENCES_TOOL_NAME,
            "Find references to the symbol at a position in the current workspace.",
        ),
        positioned_code_intelligence_tool(
            GET_HOVER_TOOL_NAME,
            "Get bounded type and documentation information at a position in the current workspace.",
        ),
        ToolDefinition {
            name: GET_DIAGNOSTICS_TOOL_NAME.to_owned(),
            description: "Get the current bounded diagnostics snapshot for a file in the current workspace."
                .to_owned(),
            input_schema: document_schema(false),
        },
    ]
}

fn positioned_code_intelligence_tool(name: &str, description: &str) -> ToolDefinition {
    ToolDefinition {
        name: name.to_owned(),
        description: description.to_owned(),
        input_schema: document_schema(true),
    }
}

fn document_schema(with_position: bool) -> serde_json::Value {
    let mut properties = serde_json::Map::from_iter([(
        "path".to_owned(),
        json!({
            "type": "string",
            "description": "A normalized path relative to the current session workspace."
        }),
    )]);
    let mut required = vec!["path"];
    if with_position {
        properties.insert(
            "line".to_owned(),
            json!({ "type": "integer", "minimum": 1, "description": "1-based line." }),
        );
        properties.insert(
            "column".to_owned(),
            json!({ "type": "integer", "minimum": 1, "description": "1-based Unicode scalar column." }),
        );
        required.extend(["line", "column"]);
    }
    json!({
        "type": "object",
        "properties": properties,
        "required": required,
        "additionalProperties": false
    })
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
    fn catalog_declares_native_and_fixed_skill_tools_in_a_stable_order() {
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
                LIST_SKILLS_TOOL_NAME,
                LOAD_SKILL_TOOL_NAME,
                READ_SKILL_RESOURCE_TOOL_NAME,
                SHELL_OUTPUT_TOOL_NAME,
                SHELL_KILL_TOOL_NAME,
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
                LIST_SKILLS_TOOL_NAME,
                LOAD_SKILL_TOOL_NAME,
                READ_SKILL_RESOURCE_TOOL_NAME,
                SHELL_OUTPUT_TOOL_NAME,
            ]
        );
        assert_eq!(
            catalog[0].input_schema["properties"]["operation"]["enum"],
            json!(["read"])
        );
    }

    #[test]
    fn plan_mode_catalog_never_offers_shell_edit_or_process_termination() {
        let names: Vec<String> = plan_mode_tool_catalog()
            .into_iter()
            .map(|tool| tool.name)
            .collect();
        assert!(!names.contains(&SHELL_TOOL_NAME.to_string()));
        assert!(!names.contains(&EDIT_TOOL_NAME.to_string()));
        // `shell_kill` acts on a process; plan mode withholds every tool that does.
        assert!(!names.contains(&SHELL_KILL_TOOL_NAME.to_string()));
    }

    /// The two background tools are appended after the pre-existing nine rather than inserted
    /// among them. Tool order is part of the prompt-cache prefix, so inserting in the middle would
    /// invalidate the cached prefix of every native generation for no functional gain.
    #[test]
    fn background_tools_extend_the_catalog_without_reordering_the_existing_prefix() {
        let names: Vec<String> = tool_catalog().into_iter().map(|tool| tool.name).collect();
        assert_eq!(
            names[..9],
            [
                SHELL_TOOL_NAME,
                FILE_TOOL_NAME,
                GREP_TOOL_NAME,
                GLOB_TOOL_NAME,
                EDIT_TOOL_NAME,
                REMEMBER_TOOL_NAME,
                LIST_SKILLS_TOOL_NAME,
                LOAD_SKILL_TOOL_NAME,
                READ_SKILL_RESOURCE_TOOL_NAME,
            ]
            .map(str::to_string)
        );
    }

    #[test]
    fn shell_schema_declares_background_and_timeout_without_making_them_required() {
        let shell = tool_catalog()
            .into_iter()
            .find(|tool| tool.name == SHELL_TOOL_NAME)
            .expect("shell tool present in catalog");
        assert_eq!(shell.input_schema["required"], json!(["command"]));
        let properties = shell.input_schema["properties"]
            .as_object()
            .expect("properties object");
        for key in ["command", "run_in_background", "timeout_ms"] {
            assert!(properties.contains_key(key), "missing property {key}");
        }
        assert_eq!(properties["run_in_background"]["type"], "boolean");
        assert_eq!(properties["timeout_ms"]["type"], "integer");
    }

    #[test]
    fn background_handle_tools_accept_only_a_handle() {
        for definition in [shell_output_tool_definition(), shell_kill_tool_definition()] {
            assert_eq!(definition.input_schema["required"], json!(["shell_id"]));
            assert_eq!(definition.input_schema["additionalProperties"], false);
            let properties = definition.input_schema["properties"]
                .as_object()
                .expect("properties object");
            assert_eq!(
                properties.len(),
                1,
                "{} exposes extra inputs",
                definition.name
            );
            // A session-supplied session id would let one session read another's output; scope is
            // injected by the runtime, exactly as it is for `recall`.
            for forbidden in ["session_id", "sessionId", "workspace", "path"] {
                assert!(!properties.contains_key(forbidden));
            }
        }
    }

    #[test]
    fn fixed_skill_schemas_are_closed_bounded_and_identical_in_plan_mode() {
        let full = tool_catalog();
        let plan = plan_mode_tool_catalog();
        for name in [
            LIST_SKILLS_TOOL_NAME,
            LOAD_SKILL_TOOL_NAME,
            READ_SKILL_RESOURCE_TOOL_NAME,
        ] {
            let full_tool = full
                .iter()
                .find(|tool| tool.name == name)
                .expect("full tool");
            let plan_tool = plan
                .iter()
                .find(|tool| tool.name == name)
                .expect("plan tool");
            assert_eq!(full_tool, plan_tool);
            assert_eq!(full_tool.input_schema["additionalProperties"], false);
        }
        let list = full
            .iter()
            .find(|tool| tool.name == LIST_SKILLS_TOOL_NAME)
            .expect("list_skills");
        assert_eq!(list.input_schema["properties"]["limit"]["maximum"], 100);
        let read = full
            .iter()
            .find(|tool| tool.name == READ_SKILL_RESOURCE_TOOL_NAME)
            .expect("read_skill_resource");
        assert_eq!(read.input_schema["required"], json!(["uri", "revision"]));
        assert!(read.input_schema["properties"].get("path").is_none());
    }

    #[test]
    fn fixed_skill_schemas_do_not_depend_on_inventory_contents() {
        let schemas = || {
            tool_catalog()
                .into_iter()
                .filter(|tool| {
                    matches!(
                        tool.name.as_str(),
                        LIST_SKILLS_TOOL_NAME
                            | LOAD_SKILL_TOOL_NAME
                            | READ_SKILL_RESOURCE_TOOL_NAME
                    )
                })
                .collect::<Vec<_>>()
        };
        let baseline = schemas();
        for simulated_inventory in [
            Vec::<&str>::new(),
            vec!["code-review"],
            vec!["code-review", "shadowed-review", "disabled-skill"],
        ] {
            assert_eq!(schemas(), baseline, "inventory was {simulated_inventory:?}");
        }
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
    fn code_intelligence_tools_have_provider_neutral_workspace_implicit_schemas() {
        let definitions = code_intelligence_tool_definitions();
        assert_eq!(
            definitions
                .iter()
                .map(|definition| definition.name.as_str())
                .collect::<Vec<_>>(),
            vec![
                FIND_DEFINITION_TOOL_NAME,
                FIND_REFERENCES_TOOL_NAME,
                GET_HOVER_TOOL_NAME,
                GET_DIAGNOSTICS_TOOL_NAME,
            ]
        );

        for definition in &definitions {
            let properties = definition.input_schema["properties"]
                .as_object()
                .expect("properties");
            assert!(properties.contains_key("path"), "{}", definition.name);
            assert_eq!(definition.input_schema["additionalProperties"], false);
            for forbidden in [
                "workspace",
                "workspace_id",
                "root",
                "server",
                "server_path",
                "uri",
            ] {
                assert!(
                    !properties.contains_key(forbidden),
                    "{} must not expose {forbidden}",
                    definition.name
                );
            }
        }

        for definition in &definitions[..3] {
            assert_eq!(
                definition.input_schema["required"],
                json!(["path", "line", "column"])
            );
            assert_eq!(definition.input_schema["properties"]["line"]["minimum"], 1);
            assert_eq!(
                definition.input_schema["properties"]["column"]["minimum"],
                1
            );
        }
        assert_eq!(definitions[3].input_schema["required"], json!(["path"]));
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

    #[test]
    fn utility_delegation_schema_exposes_only_bounded_inputs() {
        let definition = delegate_utility_skill_tool_definition();
        assert_eq!(definition.name, DELEGATE_UTILITY_SKILL_TOOL_NAME);
        assert_eq!(
            definition.input_schema["required"],
            json!(["skill_id", "task"])
        );
        assert_eq!(definition.input_schema["additionalProperties"], false);
        let properties = definition.input_schema["properties"]
            .as_object()
            .expect("properties");
        for forbidden in ["workspace", "path", "environment", "credential", "provider"] {
            assert!(!properties.contains_key(forbidden));
        }
    }
}
