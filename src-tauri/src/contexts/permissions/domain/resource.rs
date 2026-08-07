//! The target an `Action` is performed against. Phase 1 only needs exact-match resource
//! identifiers (design.md Non-Goals) — glob/precedence dialects are Phase 3's problem, driven by
//! real CLI config syntaxes this phase has no reason to guess at.

use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Resource(String);

impl Resource {
    pub(crate) fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// `shell.exec`'s resource: the session's workspace folder is the only meaningful, bounded
    /// resource for Phase 1 — command-content matching is out of scope, matching today's
    /// behavior where every shell call is treated identically.
    pub(crate) fn workspace() -> Self {
        Self::new("workspace")
    }

    /// `memory.write`'s resource: a fixed singleton, since the remember tool never targets a
    /// variable resource.
    pub(crate) fn memory() -> Self {
        Self::new("memory")
    }

    pub(crate) fn file_path(path: &str) -> Self {
        Self::new(path)
    }

    pub(crate) fn mcp_tool(server_id: &str, tool_name: &str) -> Self {
        Self::new(format!("{server_id}/{tool_name}"))
    }
}

impl fmt::Display for Resource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mcp_tool_resource_combines_server_and_tool_name() {
        assert_eq!(
            Resource::mcp_tool("filesystem-tools", "search").as_str(),
            "filesystem-tools/search"
        );
    }

    #[test]
    fn file_path_resource_preserves_the_path_verbatim() {
        assert_eq!(Resource::file_path("src/lib.rs").as_str(), "src/lib.rs");
    }
}
