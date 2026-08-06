//! Sandboxed execution for five of the native tool-use loop's six tools (`remember` is the sixth
//! -- it never touches the filesystem or a process, so it executes directly in
//! `api_process_adapter.rs` rather than living here). `file`, `edit`, and `shell` each wrap an
//! existing, already-hardened `platform` primitive (`platform::filesystem::BoundedFilesystem`;
//! `platform::process`) rather than reimplementing process/path safety. `grep` and `glob` instead
//! wrap `walk.rs`, a traversal primitive introduced alongside them in this same change, not a
//! pre-existing one.

mod edit_tool;
mod file_tool;
mod glob_tool;
mod grep_tool;
mod shell_tool;
mod walk;

pub(crate) use edit_tool::execute_edit;
pub(crate) use file_tool::execute_file;
pub(crate) use glob_tool::execute_glob;
pub(crate) use grep_tool::{execute_grep, GrepRequest, OUTPUT_MODE_FILES};
pub(crate) use shell_tool::execute_shell;

/// Shared cap on how many matches a search-style tool (glob, grep) returns. Bounds the reply
/// turn the model has to read, independent of how many files the workspace actually contains.
/// For grep's `content` mode this caps output *lines*, not distinct matches: a match's context
/// lines share the same budget, so e.g. `head_limit: 5` with `context: 2` can return lines
/// belonging to a single match.
pub(crate) const MAX_SEARCH_RESULTS: usize = 200;

/// Shared cap on a single tool result's byte size, applied by every tool that can produce
/// unbounded output (shell, grep, file read). One constant so the limits move together instead
/// of drifting apart the first time someone tunes one but not the others.
pub(crate) const MAX_TOOL_OUTPUT_BYTES: usize = 64 * 1024;

/// The outcome of executing a tool call, ready to translate into a `tool_result`/`tool` reply
/// turn and a `ToolLifecycleEvent::Completed`/`Failed` phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolExecutionOutcome {
    pub(crate) output: String,
    pub(crate) is_error: bool,
}
