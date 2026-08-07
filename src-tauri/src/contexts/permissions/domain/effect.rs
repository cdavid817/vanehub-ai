//! The three-valued outcome of a permission evaluation, replacing the legacy two-value
//! `ToolRiskTier` (`AutoApprove`/`RequiresApproval`).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Effect {
    Allow,
    Deny,
    Ask,
}

/// Resolves conflicting policy matches by explicit-Deny-first precedence (design.md D4):
/// explicit `Deny` beats explicit `Allow`, which beats the default `Ask`. An action with no
/// matching policy at all (empty `candidates`) defaults to `Ask`, never `Allow` —
/// `permissions-core`'s "Unmatched action defaults to Ask" requirement.
pub(crate) fn resolve(candidates: &[Effect]) -> Effect {
    if candidates.contains(&Effect::Deny) {
        Effect::Deny
    } else if candidates.contains(&Effect::Allow) {
        Effect::Allow
    } else {
        Effect::Ask
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deny_wins_over_a_conflicting_allow() {
        assert_eq!(resolve(&[Effect::Allow, Effect::Deny]), Effect::Deny);
        assert_eq!(resolve(&[Effect::Deny, Effect::Allow]), Effect::Deny);
    }

    #[test]
    fn allow_wins_when_no_deny_present() {
        assert_eq!(resolve(&[Effect::Ask, Effect::Allow]), Effect::Allow);
    }

    #[test]
    fn no_candidates_defaults_to_ask() {
        assert_eq!(resolve(&[]), Effect::Ask);
    }

    #[test]
    fn ask_alone_stays_ask() {
        assert_eq!(resolve(&[Effect::Ask]), Effect::Ask);
    }
}
