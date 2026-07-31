//! Sandboxed execution for the native tool-use loop's two tools. Each wraps an existing,
//! already-hardened `platform` primitive rather than reimplementing process/path safety.

mod file_tool;
mod shell_tool;

pub(crate) use file_tool::execute_file;
pub(crate) use shell_tool::execute_shell;

/// The outcome of executing a tool call, ready to translate into a `tool_result`/`tool` reply
/// turn and a `ToolLifecycleEvent::Completed`/`Failed` phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolExecutionOutcome {
    pub(crate) output: String,
    pub(crate) is_error: bool,
}
