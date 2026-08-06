//! The four named, pre-built policy sets a principal can be assigned
//! (`permissions-core`'s "Policy templates provide named, pre-built policy sets").
//!
//! Templates are data (`policies_for_template`), not code branches — a template's rules are
//! inspectable and testable as an ordinary `Vec<Policy>`, matching how `risk_tier_for` is
//! already a pure classification function today. `mcp.tool` deliberately never appears in any
//! template's rule set: its `Ask` floor is an `evaluate()` precondition checked before any
//! template lookup happens (design.md D3), so a template-level entry would be dead weight no
//! template could ever actually change.

use super::action::Action;
use super::effect::Effect;
use super::policy::{Policy, ResourcePattern};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum PolicyTemplateName {
    Readonly,
    Standard,
    Trusted,
    Yolo,
}

impl PolicyTemplateName {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            PolicyTemplateName::Readonly => "readonly",
            PolicyTemplateName::Standard => "standard",
            PolicyTemplateName::Trusted => "trusted",
            PolicyTemplateName::Yolo => "yolo",
        }
    }

    pub(crate) fn from_str(value: &str) -> Option<Self> {
        match value {
            "readonly" => Some(PolicyTemplateName::Readonly),
            "standard" => Some(PolicyTemplateName::Standard),
            "trusted" => Some(PolicyTemplateName::Trusted),
            "yolo" => Some(PolicyTemplateName::Yolo),
            _ => None,
        }
    }

    /// Whether assigning this template requires the dedicated, explicit confirmation step
    /// (`permissions-approval`'s "Increasing a principal's trust requires explicit confirmation;
    /// decreasing it does not") — `true` for the two templates that auto-allow `shell.exec`/
    /// `file.write`, `false` for the two that don't.
    pub(crate) fn requires_confirmation_to_assign(self) -> bool {
        matches!(self, PolicyTemplateName::Trusted | PolicyTemplateName::Yolo)
    }
}

/// `shell.exec`/`file.write` are the only actions a template differentiates. `file.read` and
/// `memory.write` are fixed `Allow` regardless of template — even `readonly` still allows
/// reading, that's the whole point of the name. `Trusted` and `Yolo` deliberately produce
/// identical policy rules: they differ only in their assignment-time confirmation strength
/// (`requires_confirmation_to_assign` is `true` for both; the stronger `yolo` copy/flow itself is
/// a `permissions-approval` UI concern, not a domain-level rule difference).
pub(crate) fn policies_for_template(name: PolicyTemplateName) -> Vec<Policy> {
    let mutable_effect = match name {
        PolicyTemplateName::Readonly => Effect::Deny,
        PolicyTemplateName::Standard => Effect::Ask,
        PolicyTemplateName::Trusted | PolicyTemplateName::Yolo => Effect::Allow,
    };
    vec![
        Policy::new(Action::file_read(), ResourcePattern::Any, Effect::Allow),
        Policy::new(Action::memory_write(), ResourcePattern::Any, Effect::Allow),
        Policy::new(Action::shell_exec(), ResourcePattern::Any, mutable_effect),
        Policy::new(Action::file_write(), ResourcePattern::Any, mutable_effect),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::permissions::domain::policy::resolve_for;
    use crate::contexts::permissions::domain::resource::Resource;

    fn effect_for(name: PolicyTemplateName, action: Action) -> Effect {
        resolve_for(
            &policies_for_template(name),
            &action,
            &Resource::workspace(),
        )
    }

    #[test]
    fn readonly_denies_shell_and_file_write() {
        assert_eq!(
            effect_for(PolicyTemplateName::Readonly, Action::shell_exec()),
            Effect::Deny
        );
        assert_eq!(
            effect_for(PolicyTemplateName::Readonly, Action::file_write()),
            Effect::Deny
        );
    }

    #[test]
    fn standard_asks_for_shell_and_file_write() {
        assert_eq!(
            effect_for(PolicyTemplateName::Standard, Action::shell_exec()),
            Effect::Ask
        );
        assert_eq!(
            effect_for(PolicyTemplateName::Standard, Action::file_write()),
            Effect::Ask
        );
    }

    #[test]
    fn trusted_and_yolo_both_allow_shell_and_file_write() {
        for name in [PolicyTemplateName::Trusted, PolicyTemplateName::Yolo] {
            assert_eq!(effect_for(name, Action::shell_exec()), Effect::Allow);
            assert_eq!(effect_for(name, Action::file_write()), Effect::Allow);
        }
    }

    #[test]
    fn every_template_still_allows_read_and_memory_write() {
        for name in [
            PolicyTemplateName::Readonly,
            PolicyTemplateName::Standard,
            PolicyTemplateName::Trusted,
            PolicyTemplateName::Yolo,
        ] {
            assert_eq!(effect_for(name, Action::file_read()), Effect::Allow);
            assert_eq!(effect_for(name, Action::memory_write()), Effect::Allow);
        }
    }

    #[test]
    fn no_template_declares_an_mcp_rule() {
        for name in [
            PolicyTemplateName::Readonly,
            PolicyTemplateName::Standard,
            PolicyTemplateName::Trusted,
            PolicyTemplateName::Yolo,
        ] {
            assert!(policies_for_template(name)
                .iter()
                .all(|policy| policy.action != Action::mcp_tool()));
        }
    }

    #[test]
    fn only_trusted_and_yolo_require_confirmation_to_assign() {
        assert!(!PolicyTemplateName::Readonly.requires_confirmation_to_assign());
        assert!(!PolicyTemplateName::Standard.requires_confirmation_to_assign());
        assert!(PolicyTemplateName::Trusted.requires_confirmation_to_assign());
        assert!(PolicyTemplateName::Yolo.requires_confirmation_to_assign());
    }

    #[test]
    fn as_str_and_from_str_round_trip() {
        for name in [
            PolicyTemplateName::Readonly,
            PolicyTemplateName::Standard,
            PolicyTemplateName::Trusted,
            PolicyTemplateName::Yolo,
        ] {
            assert_eq!(PolicyTemplateName::from_str(name.as_str()), Some(name));
        }
        assert_eq!(PolicyTemplateName::from_str("unknown"), None);
    }
}
