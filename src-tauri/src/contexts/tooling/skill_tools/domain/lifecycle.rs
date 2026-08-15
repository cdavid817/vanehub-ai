/// Consecutive deterministic failures that quarantine one tool revision.
///
/// Counted per immutable revision, so a fix ships as new content with a fresh counter rather than
/// needing the old evidence cleared.
pub(crate) const QUARANTINE_FAILURE_THRESHOLD: u32 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolValidationState {
    /// Discovered but not yet validated — the state a read-only discovery pass leaves behind.
    Pending,
    Valid,
    Invalid,
}

impl SkillToolValidationState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Valid => "valid",
            Self::Invalid => "invalid",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "pending" => Some(Self::Pending),
            "valid" => Some(Self::Valid),
            "invalid" => Some(Self::Invalid),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolQuarantine {
    Clear,
    Quarantined { reason: String },
}

impl SkillToolQuarantine {
    pub(crate) fn is_quarantined(&self) -> bool {
        matches!(self, Self::Quarantined { .. })
    }

    pub(crate) fn reason(&self) -> Option<&str> {
        match self {
            Self::Clear => None,
            Self::Quarantined { reason } => Some(reason.as_str()),
        }
    }
}

/// Why a discovered tool revision is not eligible to run.
///
/// Ordered by precedence in `SkillToolLifecycle::availability`: the first blocking reason is the
/// one an operator must act on, and reporting "untrusted" for a tool that also failed validation
/// would send them to the wrong action.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolIneligibility {
    Shadowed,
    Invalid,
    Quarantined,
    Untrusted,
    Disabled,
    ModuleRuntimeUnavailable,
}

impl SkillToolIneligibility {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Shadowed => "shadowed",
            Self::Invalid => "invalid",
            Self::Quarantined => "quarantined",
            Self::Untrusted => "untrusted",
            Self::Disabled => "disabled",
            Self::ModuleRuntimeUnavailable => "module-runtime-unavailable",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolAvailability {
    Available,
    Unavailable(SkillToolIneligibility),
}

impl SkillToolAvailability {
    pub(crate) fn is_available(self) -> bool {
        matches!(self, Self::Available)
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable(reason) => reason.as_str(),
        }
    }
}

/// Per-revision governance state. Enablement is separate from trust throughout: trusting content
/// and choosing to run it are different operator decisions (`design.md` decision 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolLifecycle {
    pub(crate) validation: SkillToolValidationState,
    pub(crate) trusted: bool,
    pub(crate) enabled: bool,
    pub(crate) quarantine: SkillToolQuarantine,
    pub(crate) consecutive_failures: u32,
}

impl Default for SkillToolLifecycle {
    /// A newly discovered revision starts unvalidated, untrusted, and disabled: nothing about
    /// having been found on disk is evidence that it should run.
    fn default() -> Self {
        Self {
            validation: SkillToolValidationState::Pending,
            trusted: false,
            enabled: false,
            quarantine: SkillToolQuarantine::Clear,
            consecutive_failures: 0,
        }
    }
}

impl SkillToolLifecycle {
    pub(crate) fn availability(
        &self,
        shadowed: bool,
        module_runtime_available: bool,
        requires_module_runtime: bool,
    ) -> SkillToolAvailability {
        if shadowed {
            return SkillToolAvailability::Unavailable(SkillToolIneligibility::Shadowed);
        }
        if self.validation != SkillToolValidationState::Valid {
            return SkillToolAvailability::Unavailable(SkillToolIneligibility::Invalid);
        }
        if self.quarantine.is_quarantined() {
            return SkillToolAvailability::Unavailable(SkillToolIneligibility::Quarantined);
        }
        if !self.trusted {
            return SkillToolAvailability::Unavailable(SkillToolIneligibility::Untrusted);
        }
        if !self.enabled {
            return SkillToolAvailability::Unavailable(SkillToolIneligibility::Disabled);
        }
        if requires_module_runtime && !module_runtime_available {
            return SkillToolAvailability::Unavailable(
                SkillToolIneligibility::ModuleRuntimeUnavailable,
            );
        }
        SkillToolAvailability::Available
    }

    /// Records one deterministic failure, quarantining at the threshold.
    pub(crate) fn record_failure(&mut self, reason: &str) -> bool {
        self.consecutive_failures = self.consecutive_failures.saturating_add(1);
        if self.consecutive_failures >= QUARANTINE_FAILURE_THRESHOLD
            && !self.quarantine.is_quarantined()
        {
            self.quarantine = SkillToolQuarantine::Quarantined {
                reason: reason.chars().take(256).collect(),
            };
            return true;
        }
        false
    }

    /// A success clears the streak but never lifts quarantine: recovery requires a clean
    /// revalidation and an explicit operator action, so one lucky run cannot re-enable a tool.
    pub(crate) fn record_success(&mut self) {
        self.consecutive_failures = 0;
    }

    pub(crate) fn recover(&mut self, revalidated: SkillToolValidationState) -> bool {
        if revalidated != SkillToolValidationState::Valid {
            return false;
        }
        self.validation = revalidated;
        self.quarantine = SkillToolQuarantine::Clear;
        self.consecutive_failures = 0;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runnable() -> SkillToolLifecycle {
        SkillToolLifecycle {
            validation: SkillToolValidationState::Valid,
            trusted: true,
            enabled: true,
            ..SkillToolLifecycle::default()
        }
    }

    #[test]
    fn a_freshly_discovered_revision_is_unvalidated_untrusted_and_disabled() {
        let lifecycle = SkillToolLifecycle::default();
        assert_eq!(lifecycle.validation, SkillToolValidationState::Pending);
        assert!(!lifecycle.trusted);
        assert!(!lifecycle.enabled);
        assert_eq!(
            lifecycle.availability(false, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Invalid)
        );
    }

    #[test]
    fn availability_reports_the_first_blocking_reason_in_precedence_order() {
        let lifecycle = runnable();
        assert!(lifecycle.availability(false, true, true).is_available());
        assert_eq!(
            lifecycle.availability(true, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Shadowed)
        );
        assert_eq!(
            lifecycle.availability(false, false, true),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::ModuleRuntimeUnavailable)
        );
        // A declarative tool stays available when the module runtime is off.
        assert!(lifecycle.availability(false, false, false).is_available());

        let untrusted = SkillToolLifecycle {
            trusted: false,
            ..runnable()
        };
        assert_eq!(
            untrusted.availability(false, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Untrusted)
        );
        let quarantined = SkillToolLifecycle {
            trusted: false,
            quarantine: SkillToolQuarantine::Quarantined {
                reason: "trap".to_string(),
            },
            ..runnable()
        };
        assert_eq!(
            quarantined.availability(false, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Quarantined)
        );
        let disabled = SkillToolLifecycle {
            enabled: false,
            ..runnable()
        };
        assert_eq!(
            disabled.availability(false, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Disabled)
        );
    }

    #[test]
    fn repeated_failures_quarantine_once_and_a_success_only_clears_the_streak() {
        let mut lifecycle = runnable();
        assert!(!lifecycle.record_failure("trap"));
        assert!(!lifecycle.record_failure("trap"));
        assert!(lifecycle.record_failure("trap"));
        assert_eq!(lifecycle.quarantine.reason(), Some("trap"));
        assert!(!lifecycle.record_failure("trap"));

        lifecycle.record_success();
        assert_eq!(lifecycle.consecutive_failures, 0);
        assert!(lifecycle.quarantine.is_quarantined());
        assert_eq!(
            lifecycle.availability(false, true, false),
            SkillToolAvailability::Unavailable(SkillToolIneligibility::Quarantined)
        );
    }

    #[test]
    fn recovery_requires_a_clean_revalidation() {
        let mut lifecycle = runnable();
        for _ in 0..QUARANTINE_FAILURE_THRESHOLD {
            lifecycle.record_failure("oversized output");
        }
        assert!(!lifecycle.recover(SkillToolValidationState::Invalid));
        assert!(lifecycle.quarantine.is_quarantined());

        assert!(lifecycle.recover(SkillToolValidationState::Valid));
        assert!(!lifecycle.quarantine.is_quarantined());
        assert_eq!(lifecycle.consecutive_failures, 0);
        assert!(lifecycle.availability(false, true, false).is_available());
    }

    #[test]
    fn quarantine_reasons_are_bounded_before_they_can_be_persisted() {
        let mut lifecycle = runnable();
        let long_reason = "x".repeat(1024);
        for _ in 0..QUARANTINE_FAILURE_THRESHOLD {
            lifecycle.record_failure(&long_reason);
        }
        assert_eq!(lifecycle.quarantine.reason().map(str::len), Some(256));
        assert_eq!(
            SkillToolValidationState::parse("valid"),
            Some(SkillToolValidationState::Valid)
        );
        assert_eq!(SkillToolValidationState::parse("bogus"), None);
    }
}
