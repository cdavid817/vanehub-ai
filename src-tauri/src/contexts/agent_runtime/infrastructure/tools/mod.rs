//! Sandboxed execution for the native tool-use loop's six tools. Each wraps an existing,
//! already-hardened `platform` primitive rather than reimplementing process/path safety.

mod file_tool;
mod glob_tool;
mod grep_tool;
mod shell_tool;
mod walk;

pub(crate) use file_tool::execute_file;
pub(crate) use glob_tool::execute_glob;
pub(crate) use grep_tool::{execute_grep, GrepRequest};
pub(crate) use shell_tool::execute_shell;

/// Shared cap on how many matches a search-style tool (glob, grep) returns. Bounds the reply
/// turn the model has to read, independent of how many files the workspace actually contains.
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
