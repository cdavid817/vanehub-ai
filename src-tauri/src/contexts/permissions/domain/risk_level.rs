//! Fixed risk classification, one level per `Action` (design.md D9) — a pure function, not a
//! per-policy configurable field. A template can change what happens at a given risk level (via
//! its `Effect` for that action), but it cannot change which level an action *is*; keeping the
//! mapping fixed avoids a second, independent axis of misconfiguration (e.g. a `readonly`
//! template someone miscategorizes `shell.exec` as `L0` under would silently defeat the
//! template's own intent).

use super::action::{Action, FILE_READ, FILE_WRITE, MCP_TOOL, MEMORY_WRITE, SHELL_EXEC};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum RiskLevel {
    L0,
    L1,
    L2,
    /// Declared but never produced in Phase 1 — reserved for a future network/external-effect
    /// category (design.md Roadmap, Phase 3+).
    #[allow(dead_code)]
    L3,
}

/// `mcp.tool` maps to `L2` for audit/display consistency only: its floor is `Ask` regardless of
/// risk level (design.md D3), so this value never changes what it resolves to. Any action name
/// this function doesn't explicitly recognize fails closed to `L2`, matching `risk_tier_for`'s
/// existing catch-all-to-`RequiresApproval` philosophy.
pub(crate) fn risk_level_for(action: &Action) -> RiskLevel {
    match action.as_str() {
        FILE_READ | MEMORY_WRITE => RiskLevel::L0,
        FILE_WRITE => RiskLevel::L1,
        SHELL_EXEC | MCP_TOOL => RiskLevel::L2,
        _ => RiskLevel::L2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_actions_are_l0() {
        assert_eq!(risk_level_for(&Action::file_read()), RiskLevel::L0);
        assert_eq!(risk_level_for(&Action::memory_write()), RiskLevel::L0);
    }

    #[test]
    fn file_write_is_l1() {
        assert_eq!(risk_level_for(&Action::file_write()), RiskLevel::L1);
    }

    #[test]
    fn shell_and_mcp_are_l2() {
        assert_eq!(risk_level_for(&Action::shell_exec()), RiskLevel::L2);
        assert_eq!(risk_level_for(&Action::mcp_tool()), RiskLevel::L2);
    }

    #[test]
    fn unknown_action_fails_closed_to_l2() {
        assert_eq!(
            risk_level_for(&Action::new("codex.sandbox_escalation")),
            RiskLevel::L2
        );
    }
}
