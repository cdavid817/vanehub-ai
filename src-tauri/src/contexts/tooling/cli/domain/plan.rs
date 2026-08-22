//! A reviewable, expiring, single-use proposal to change the machine.
//!
//! Every mutation goes through one. The request that starts it carries the source, channel, and
//! target the user chose; the plan echoes those exact values back for review; and execution submits
//! only a plan id and a revision. There is no path where a command is rebuilt from UI fields at
//! execution time, which is what made the selected version droppable in the first place.
//!
//! A plan is admitted once. `admit_execution` is the gate, and it refuses an expired, consumed,
//! superseded, or stale plan *before* any external effect begins.

use chrono::{DateTime, Duration, Utc};

use super::action::CliActionKind;
use super::ids::{CliActionPlanId, CliInstallationId, CliSourceId, CliToolId};

/// Ten minutes. Long enough to read a review dialog, short enough that the environment the plan was
/// built against is probably still the environment it runs against.
pub(crate) const DEFAULT_PLAN_EXPIRY_MINUTES: i64 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliActionPlanState {
    Draft,
    Executing,
    Completed,
    Failed,
    Cancelled,
    Expired,
}

impl CliActionPlanState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Executing => "executing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
            Self::Expired => "expired",
        }
    }

    pub(crate) fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Expired
        )
    }
}

/// Whether a failing source may be swapped for another one.
///
/// There is exactly one policy in this change, and it is a field rather than an assumption so the
/// review dialog can state it and the wire contract can carry it. The vendor-installer-falls-back-
/// to-npm path this replaces was invisible precisely because nothing named it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliFallbackPolicy {
    /// Execute the disclosed source or fail. Never start a different one.
    None,
}

impl CliFallbackPolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
        }
    }
}

/// The command, structured. Never a shell string, never an installer body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliCommandPreview {
    /// Executable identity such as `npm` or `winget` -- not a resolved absolute path, which would
    /// put a home directory in a persisted plan and on screen.
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
}

impl CliCommandPreview {
    pub(crate) fn new(program: impl Into<String>, args: Vec<String>) -> Self {
        Self {
            program: program.into(),
            args,
        }
    }

    /// Whether any part of this preview could be interpreted by a shell. Nothing here is ever
    /// passed to one, but a preview that *looks* like a pipeline means an adapter built a string
    /// where it should have built argv.
    pub(crate) fn is_shell_free(&self) -> bool {
        let suspicious = ['|', ';', '&', '>', '<', '`', '$', '\n'];
        !self.program.contains(suspicious) && !self.args.iter().any(|arg| arg.contains(suspicious))
    }
}

/// Something that must hold before the plan can run. Checked during preparation and re-checked at
/// admission where the answer can change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CliPrecondition {
    SourceExecutableAvailable { source: String },
    NetworkReachable { host: String },
    ElevatedPrivileges,
}

/// Something true about the plan that the user should see before confirming.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliPlanWarning {
    /// The source cannot aim at a version; it installs whatever it considers latest. The result
    /// must not be labelled with a requested version.
    TargetIsLatestOnly,
    /// No published digest for the installer being downloaded.
    InstallerIntegrityUnverified,
    /// The source supports exact versions in general, but preflight could not confirm it here.
    ExactVersionNotConfirmed,
    /// Another installation earlier in PATH will keep winning after this runs.
    ActiveInstallationShadowed,
    DowngradeMayLoseState,
}

impl CliPlanWarning {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::TargetIsLatestOnly => "target-is-latest-only",
            Self::InstallerIntegrityUnverified => "installer-integrity-unverified",
            Self::ExactVersionNotConfirmed => "exact-version-not-confirmed",
            Self::ActiveInstallationShadowed => "active-installation-shadowed",
            Self::DowngradeMayLoseState => "downgrade-may-lose-state",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliActionPlan {
    pub(crate) id: CliActionPlanId,
    pub(crate) revision: u32,
    pub(crate) agent_id: CliToolId,
    pub(crate) action: CliActionKind,
    /// The one source this plan runs. Recorded exactly, never re-resolved at execution.
    pub(crate) source_id: CliSourceId,
    pub(crate) installation_id: Option<CliInstallationId>,
    pub(crate) current_version: Option<String>,
    /// The target the user chose, echoed verbatim. `None` only for actions that carry no version.
    pub(crate) target_version: Option<String>,
    pub(crate) channel: Option<String>,
    pub(crate) command_preview: CliCommandPreview,
    pub(crate) preconditions: Vec<CliPrecondition>,
    pub(crate) warnings: Vec<CliPlanWarning>,
    pub(crate) requires_elevation: bool,
    pub(crate) requires_network: bool,
    pub(crate) fallback_policy: CliFallbackPolicy,
    /// Binds the plan to the environment it was built against. A change here invalidates it.
    pub(crate) environment_fingerprint: String,
    pub(crate) state: CliActionPlanState,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) expires_at: DateTime<Utc>,
}

/// Why a plan may not run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliPlanRejection {
    /// The caller holds an older revision than the stored plan.
    RevisionMismatch {
        expected: u32,
        actual: u32,
    },
    Expired,
    /// Already admitted, or already finished. A retry needs a new plan, not this one again.
    Consumed,
    /// The environment changed after review. Executing anyway would run a command built for a
    /// machine that no longer exists.
    Stale,
}

impl CliPlanRejection {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::RevisionMismatch { .. } => "plan-revision-mismatch",
            Self::Expired => "plan-expired",
            Self::Consumed => "plan-consumed",
            Self::Stale => "plan-stale",
        }
    }
}

impl CliActionPlan {
    pub(crate) fn default_expiry(created_at: DateTime<Utc>) -> DateTime<Utc> {
        created_at + Duration::minutes(DEFAULT_PLAN_EXPIRY_MINUTES)
    }

    pub(crate) fn is_expired(&self, now: DateTime<Utc>) -> bool {
        now >= self.expires_at
    }

    /// The gate every execution passes through, in the order a caller can act on: identity, then
    /// consumption, then expiry, then environment.
    ///
    /// Success means the plan *may* transition to `Executing`; the repository still performs that
    /// transition atomically, so two concurrent callers cannot both be admitted.
    pub(crate) fn admit_execution(
        &self,
        expected_revision: u32,
        current_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<(), CliPlanRejection> {
        if self.revision != expected_revision {
            return Err(CliPlanRejection::RevisionMismatch {
                expected: expected_revision,
                actual: self.revision,
            });
        }
        if self.state != CliActionPlanState::Draft {
            return Err(CliPlanRejection::Consumed);
        }
        if self.is_expired(now) {
            return Err(CliPlanRejection::Expired);
        }
        if self.environment_fingerprint != current_fingerprint {
            return Err(CliPlanRejection::Stale);
        }
        Ok(())
    }

    /// Whether the plan's own content satisfies the invariants that make it safe to persist.
    ///
    /// Checked at construction. A plan that fails this is a programming error in an adapter, not a
    /// user-facing condition.
    pub(crate) fn violations(&self) -> Vec<CliPlanInvariantViolation> {
        let mut violations = Vec::new();
        if self.fallback_policy != CliFallbackPolicy::None {
            violations.push(CliPlanInvariantViolation::FallbackPolicyNotNone);
        }
        if self.environment_fingerprint.is_empty() {
            violations.push(CliPlanInvariantViolation::MissingFingerprint);
        }
        if self.expires_at <= self.created_at {
            violations.push(CliPlanInvariantViolation::NonPositiveLifetime);
        }
        if !self.command_preview.is_shell_free() {
            violations.push(CliPlanInvariantViolation::ShellInterpolationInPreview);
        }
        if self.command_preview.program.is_empty() {
            violations.push(CliPlanInvariantViolation::EmptyProgram);
        }
        // A version-bearing action with no target would execute at whatever the source defaults
        // to while the plan claims something specific was requested.
        let needs_target = matches!(
            self.action,
            CliActionKind::Install | CliActionKind::Upgrade | CliActionKind::Downgrade
        );
        if needs_target && self.target_version.is_none() {
            violations.push(CliPlanInvariantViolation::MissingTargetVersion);
        }
        violations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliPlanInvariantViolation {
    FallbackPolicyNotNone,
    MissingFingerprint,
    NonPositiveLifetime,
    ShellInterpolationInPreview,
    EmptyProgram,
    MissingTargetVersion,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("timestamp")
    }

    fn plan() -> CliActionPlan {
        let created_at = timestamp(1_000);
        CliActionPlan {
            id: CliActionPlanId::new("plan-1").expect("plan id"),
            revision: 1,
            agent_id: CliToolId::new("claude-code").expect("tool id"),
            action: CliActionKind::Upgrade,
            source_id: CliSourceId::new("npm").expect("source id"),
            installation_id: Some(CliInstallationId::new("install-1").expect("installation id")),
            current_version: Some("1.2.0".to_string()),
            target_version: Some("1.3.0".to_string()),
            channel: Some("stable".to_string()),
            command_preview: CliCommandPreview::new(
                "npm",
                vec![
                    "install".to_string(),
                    "--global".to_string(),
                    "@anthropic-ai/claude-code@1.3.0".to_string(),
                ],
            ),
            preconditions: vec![CliPrecondition::SourceExecutableAvailable {
                source: "npm".to_string(),
            }],
            warnings: Vec::new(),
            requires_elevation: false,
            requires_network: true,
            fallback_policy: CliFallbackPolicy::None,
            environment_fingerprint: "fingerprint-a".to_string(),
            state: CliActionPlanState::Draft,
            created_at,
            expires_at: CliActionPlan::default_expiry(created_at),
        }
    }

    #[test]
    fn a_plan_expires_ten_minutes_after_creation() {
        let plan = plan();
        assert_eq!(plan.expires_at, timestamp(1_000 + 600));
        assert!(!plan.is_expired(timestamp(1_599)));
        assert!(plan.is_expired(timestamp(1_600)));
    }

    #[test]
    fn a_valid_plan_is_admitted_once_and_only_once() {
        let plan = plan();
        assert_eq!(
            plan.admit_execution(1, "fingerprint-a", timestamp(1_100)),
            Ok(())
        );

        // Once the repository has moved it to executing, the same plan is refused. A retry has to
        // build a new plan against the current environment.
        let consumed = CliActionPlan {
            state: CliActionPlanState::Executing,
            ..plan
        };
        assert_eq!(
            consumed.admit_execution(1, "fingerprint-a", timestamp(1_100)),
            Err(CliPlanRejection::Consumed)
        );
    }

    #[test]
    fn every_terminal_state_refuses_a_second_execution() {
        for state in [
            CliActionPlanState::Completed,
            CliActionPlanState::Failed,
            CliActionPlanState::Cancelled,
            CliActionPlanState::Expired,
        ] {
            assert!(state.is_terminal(), "{}", state.as_str());
            let finished = CliActionPlan { state, ..plan() };
            assert_eq!(
                finished.admit_execution(1, "fingerprint-a", timestamp(1_100)),
                Err(CliPlanRejection::Consumed)
            );
        }
        assert!(!CliActionPlanState::Draft.is_terminal());
        assert!(!CliActionPlanState::Executing.is_terminal());
    }

    #[test]
    fn a_changed_environment_makes_the_plan_stale_instead_of_running_it() {
        // An external terminal changed PATH between review and confirm. The recorded command was
        // built for a machine that no longer exists.
        let plan = plan();
        assert_eq!(
            plan.admit_execution(1, "fingerprint-b", timestamp(1_100)),
            Err(CliPlanRejection::Stale)
        );
    }

    #[test]
    fn an_expired_plan_is_refused_before_the_fingerprint_is_even_considered() {
        let plan = plan();
        // Expiry is reported even though the fingerprint also fails, so the user is told the
        // actionable thing: prepare a new plan.
        assert_eq!(
            plan.admit_execution(1, "fingerprint-b", timestamp(2_000)),
            Err(CliPlanRejection::Expired)
        );
    }

    #[test]
    fn a_superseded_revision_is_refused_with_both_numbers() {
        let plan = CliActionPlan {
            revision: 3,
            ..plan()
        };
        assert_eq!(
            plan.admit_execution(2, "fingerprint-a", timestamp(1_100)),
            Err(CliPlanRejection::RevisionMismatch {
                expected: 2,
                actual: 3
            })
        );
    }

    #[test]
    fn rejections_carry_the_wire_codes_the_frontend_maps_to_messages() {
        assert_eq!(CliPlanRejection::Expired.as_str(), "plan-expired");
        assert_eq!(CliPlanRejection::Consumed.as_str(), "plan-consumed");
        assert_eq!(CliPlanRejection::Stale.as_str(), "plan-stale");
        assert_eq!(
            CliPlanRejection::RevisionMismatch {
                expected: 1,
                actual: 2
            }
            .as_str(),
            "plan-revision-mismatch"
        );
    }

    #[test]
    fn a_well_formed_plan_violates_nothing() {
        assert!(plan().violations().is_empty());
        assert_eq!(plan().fallback_policy, CliFallbackPolicy::None);
        assert_eq!(CliFallbackPolicy::None.as_str(), "none");
    }

    #[test]
    fn a_preview_that_looks_like_a_shell_command_is_a_violation() {
        // The vendor path this replaces built `bash -lc "tmp=$(mktemp) && wget ... | ..."`. A
        // preview carrying that shape means an adapter built a string instead of argv.
        let piped = CliActionPlan {
            command_preview: CliCommandPreview::new(
                "bash",
                vec![
                    "-lc".to_string(),
                    "curl https://example.test/install.sh | bash".to_string(),
                ],
            ),
            ..plan()
        };
        assert!(!piped.command_preview.is_shell_free());
        assert!(piped
            .violations()
            .contains(&CliPlanInvariantViolation::ShellInterpolationInPreview));

        for hostile in ["a && b", "a; b", "a > b", "a `b`", "a $b", "a\nb"] {
            let preview = CliCommandPreview::new("npm", vec![hostile.to_string()]);
            assert!(!preview.is_shell_free(), "{hostile} must be rejected");
        }

        // Ordinary arguments, including a scoped package with an @ version, stay acceptable.
        assert!(plan().command_preview.is_shell_free());
        assert!(CliCommandPreview::new(
            "winget",
            vec!["--id".to_string(), "Anthropic.ClaudeCode".to_string()]
        )
        .is_shell_free());
    }

    #[test]
    fn a_version_bearing_action_without_a_target_is_a_violation() {
        for action in [
            CliActionKind::Install,
            CliActionKind::Upgrade,
            CliActionKind::Downgrade,
        ] {
            let untargeted = CliActionPlan {
                action,
                target_version: None,
                ..plan()
            };
            assert!(
                untargeted
                    .violations()
                    .contains(&CliPlanInvariantViolation::MissingTargetVersion),
                "{} must carry a target",
                action.as_str()
            );
        }

        // Uninstall and repair carry none by nature.
        for action in [CliActionKind::Uninstall, CliActionKind::Repair] {
            let untargeted = CliActionPlan {
                action,
                target_version: None,
                ..plan()
            };
            assert!(untargeted.violations().is_empty(), "{}", action.as_str());
        }
    }

    #[test]
    fn a_plan_without_a_fingerprint_or_lifetime_is_a_violation() {
        let unbound = CliActionPlan {
            environment_fingerprint: String::new(),
            ..plan()
        };
        assert!(unbound
            .violations()
            .contains(&CliPlanInvariantViolation::MissingFingerprint));

        let instant = CliActionPlan {
            expires_at: timestamp(1_000),
            ..plan()
        };
        assert!(instant
            .violations()
            .contains(&CliPlanInvariantViolation::NonPositiveLifetime));

        let programless = CliActionPlan {
            command_preview: CliCommandPreview::new("", vec![]),
            ..plan()
        };
        assert!(programless
            .violations()
            .contains(&CliPlanInvariantViolation::EmptyProgram));
    }

    #[test]
    fn the_plan_records_the_exact_source_and_target_it_will_run() {
        // The structural fix for the dropped selection: these fields are what execution reads, and
        // execution receives nothing else.
        let plan = plan();
        assert_eq!(plan.source_id.as_str(), "npm");
        assert_eq!(plan.target_version.as_deref(), Some("1.3.0"));
        assert_eq!(plan.current_version.as_deref(), Some("1.2.0"));
        assert_eq!(plan.channel.as_deref(), Some("stable"));
        assert!(plan
            .command_preview
            .args
            .iter()
            .any(|arg| arg.ends_with("@1.3.0")));
    }

    #[test]
    fn preconditions_and_warnings_are_structured_for_the_review_dialog() {
        let plan = CliActionPlan {
            warnings: vec![
                CliPlanWarning::TargetIsLatestOnly,
                CliPlanWarning::InstallerIntegrityUnverified,
            ],
            preconditions: vec![
                CliPrecondition::SourceExecutableAvailable {
                    source: "npm".to_string(),
                },
                CliPrecondition::NetworkReachable {
                    host: "registry.npmjs.org".to_string(),
                },
                CliPrecondition::ElevatedPrivileges,
            ],
            requires_elevation: true,
            ..plan()
        };

        assert_eq!(
            plan.warnings.iter().map(|w| w.as_str()).collect::<Vec<_>>(),
            vec!["target-is-latest-only", "installer-integrity-unverified"]
        );
        assert_eq!(plan.preconditions.len(), 3);
        assert!(plan.requires_elevation);
        assert!(plan.requires_network);
        assert_eq!(
            CliPlanWarning::ExactVersionNotConfirmed.as_str(),
            "exact-version-not-confirmed"
        );
        assert_eq!(
            CliPlanWarning::ActiveInstallationShadowed.as_str(),
            "active-installation-shadowed"
        );
        assert_eq!(
            CliPlanWarning::DowngradeMayLoseState.as_str(),
            "downgrade-may-lose-state"
        );
    }

    #[test]
    fn plan_states_have_stable_wire_strings() {
        assert_eq!(CliActionPlanState::Draft.as_str(), "draft");
        assert_eq!(CliActionPlanState::Executing.as_str(), "executing");
        assert_eq!(CliActionPlanState::Completed.as_str(), "completed");
        assert_eq!(CliActionPlanState::Failed.as_str(), "failed");
        assert_eq!(CliActionPlanState::Cancelled.as_str(), "cancelled");
        assert_eq!(CliActionPlanState::Expired.as_str(), "expired");
    }
}
