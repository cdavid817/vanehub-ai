//! Open-ended action identifiers evaluated by the permission decision point.
//!
//! `Action` is a `String` newtype rather than a closed enum (design.md D1): the CLI agents later
//! phases integrate each have native concepts this codebase cannot enumerate today (Codex sandbox
//! escalation, OpenCode `external_directory`/`doom_loop`, Gemini's tool-level model), and a closed
//! enum would force a breaking change the moment one of them needs a new variant.

use std::fmt;

pub(crate) const SHELL_EXEC: &str = "shell.exec";
pub(crate) const FILE_READ: &str = "file.read";
pub(crate) const FILE_WRITE: &str = "file.write";
pub(crate) const MCP_TOOL: &str = "mcp.tool";
pub(crate) const MEMORY_WRITE: &str = "memory.write";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Action(String);

impl Action {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn shell_exec() -> Self {
        Self::new(SHELL_EXEC)
    }

    pub(crate) fn file_read() -> Self {
        Self::new(FILE_READ)
    }

    pub(crate) fn file_write() -> Self {
        Self::new(FILE_WRITE)
    }

    pub(crate) fn mcp_tool() -> Self {
        Self::new(MCP_TOOL)
    }

    pub(crate) fn memory_write() -> Self {
        Self::new(MEMORY_WRITE)
    }

    pub(crate) fn is_mcp_tool(&self) -> bool {
        self.0 == MCP_TOOL
    }
}

impl fmt::Display for Action {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<&str> for Action {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_constructors_round_trip_as_str() {
        assert_eq!(Action::shell_exec().as_str(), SHELL_EXEC);
        assert_eq!(Action::file_read().as_str(), FILE_READ);
        assert_eq!(Action::file_write().as_str(), FILE_WRITE);
        assert_eq!(Action::mcp_tool().as_str(), MCP_TOOL);
        assert_eq!(Action::memory_write().as_str(), MEMORY_WRITE);
    }

    #[test]
    fn arbitrary_action_names_are_preserved() {
        let custom = Action::new("codex.sandbox_escalation");
        assert_eq!(custom.as_str(), "codex.sandbox_escalation");
    }

    #[test]
    fn only_the_mcp_constant_is_recognized_as_mcp() {
        assert!(Action::mcp_tool().is_mcp_tool());
        assert!(!Action::shell_exec().is_mcp_tool());
        assert!(!Action::new("mcp.tool.extra").is_mcp_tool());
    }
}
