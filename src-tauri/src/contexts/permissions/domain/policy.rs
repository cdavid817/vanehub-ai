//! A single principal/action/resource rule and how a set of them resolves to one `Effect`.

use super::action::Action;
use super::effect::{resolve, Effect};
use super::resource::Resource;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResourcePattern {
    /// Matches any resource for the given action — how a policy template's rules are expressed
    /// (Phase 1 templates are action-scoped, not resource-scoped).
    Any,
    /// Matches only the exact resource — how a remembered grant is expressed. Not constructed by
    /// any current policy rule (Phase 1 templates are action-scoped only); exercised by this
    /// module's own tests ahead of a resource-scoped grant feature.
    #[allow(dead_code)]
    Exact(Resource),
}

impl ResourcePattern {
    pub(crate) fn matches(&self, resource: &Resource) -> bool {
        match self {
            ResourcePattern::Any => true,
            ResourcePattern::Exact(expected) => expected == resource,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Policy {
    pub(crate) action: Action,
    pub(crate) resource: ResourcePattern,
    pub(crate) effect: Effect,
    /// Reserved for future, genuinely priority-ordered rule sources (design.md Roadmap Phase 3's
    /// compiled CLI dialects); Phase 1's `resolve_for` never consults it — precedence among
    /// Phase-1 candidates is always explicit-Deny-first (`effect::resolve`), independent of this
    /// value.
    pub(crate) priority: i32,
}

impl Policy {
    pub(crate) fn new(action: Action, resource: ResourcePattern, effect: Effect) -> Self {
        Self {
            action,
            resource,
            effect,
            priority: 0,
        }
    }
}

/// Resolves the effect for `(action, resource)` from every policy that matches it, using
/// explicit-Deny-first precedence (design.md D4). A policy whose action doesn't match is ignored
/// entirely — it never contributes an implicit `Ask`.
pub(crate) fn resolve_for(policies: &[Policy], action: &Action, resource: &Resource) -> Effect {
    let candidates: Vec<Effect> = policies
        .iter()
        .filter(|policy| &policy.action == action && policy.resource.matches(resource))
        .map(|policy| policy.effect)
        .collect();
    resolve(&candidates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_matching_action_is_ignored() {
        let policies = vec![Policy::new(
            Action::file_write(),
            ResourcePattern::Any,
            Effect::Allow,
        )];
        assert_eq!(
            resolve_for(&policies, &Action::shell_exec(), &Resource::workspace()),
            Effect::Ask
        );
    }

    #[test]
    fn exact_resource_pattern_only_matches_that_resource() {
        let policies = vec![Policy::new(
            Action::file_write(),
            ResourcePattern::Exact(Resource::file_path("a.txt")),
            Effect::Allow,
        )];
        assert_eq!(
            resolve_for(
                &policies,
                &Action::file_write(),
                &Resource::file_path("a.txt")
            ),
            Effect::Allow
        );
        assert_eq!(
            resolve_for(
                &policies,
                &Action::file_write(),
                &Resource::file_path("b.txt")
            ),
            Effect::Ask
        );
    }

    #[test]
    fn conflicting_matches_resolve_deny_first() {
        let policies = vec![
            Policy::new(Action::shell_exec(), ResourcePattern::Any, Effect::Allow),
            Policy::new(
                Action::shell_exec(),
                ResourcePattern::Exact(Resource::workspace()),
                Effect::Deny,
            ),
        ];
        assert_eq!(
            resolve_for(&policies, &Action::shell_exec(), &Resource::workspace()),
            Effect::Deny
        );
    }
}
