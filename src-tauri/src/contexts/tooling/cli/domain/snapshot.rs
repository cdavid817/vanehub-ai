//! The normalized read model for one CLI in one environment scope.
//!
//! A snapshot is what the page renders and what the next plan is validated against. Its derived
//! fields are recomputed from its raw ones rather than assigned, so `overall_state` cannot drift
//! away from the axes it summarises.
//!
//! The rule that shapes `last_mutation`: a package manager is an external effect and cannot be
//! rolled back by writing an older row. After a mutation the snapshot reflects what was *observed*
//! on the machine, with the outcome attached -- never the pre-operation state re-saved as though
//! nothing happened.

use chrono::{DateTime, Utc};

use super::action::CliAllowedAction;
use super::ids::{CliInstallationId, CliSourceId, CliToolId};
use super::installation::{conflicts_block_mutation, select_active, CliConflict, CliInstallation};
use super::source::CliSourceSummary;
use super::status::{
    derive_overall_state, derive_readiness, CliAuthenticationStatus, CliCompatibilityStatus,
    CliDiscoveryStatus, CliExecutableStatus, CliFreshness, CliOverallState, CliReadinessStatus,
    CliStatusAxes, CliUpdateStatus,
};

/// Bumped when the persisted JSON shape changes. Decoding is fallible and an unknown version is a
/// stale-unknown result, never a panic.
pub(crate) const SNAPSHOT_SCHEMA_VERSION: u16 = 1;

/// The only scope in this change. WSL, SSH, and container scopes are deliberately out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliEnvironmentScope {
    LocalDesktop,
}

impl CliEnvironmentScope {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::LocalDesktop => "local-desktop",
        }
    }
}

/// How a completed mutation actually ended.
///
/// The distinction the old model could not express: a package command that succeeded while
/// verification failed is *not* a failure that can be undone by restoring the previous snapshot.
/// The machine changed. Saying otherwise is the defect these variants remove.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliMutationOutcome {
    /// The command succeeded and the resulting executable was verified.
    Verified,
    /// The command succeeded but verification did not confirm it. The change is presumed applied.
    AppliedUnverified,
    /// The command failed or was cancelled, yet post-detection shows the machine changed anyway.
    ChangedButFailed,
    /// The command failed and nothing on the machine changed.
    NoChangeFailed,
    /// Cancelled before any external effect was observed.
    Cancelled,
}

impl CliMutationOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::AppliedUnverified => "applied-unverified",
            Self::ChangedButFailed => "changed-but-failed",
            Self::NoChangeFailed => "no-change-failed",
            Self::Cancelled => "cancelled",
        }
    }

    /// Whether the machine may have changed. Anything but `NoChangeFailed` and `Cancelled` means
    /// the previous snapshot is no longer a truthful description of the host.
    pub(crate) fn may_have_changed_the_machine(self) -> bool {
        matches!(
            self,
            Self::Verified | Self::AppliedUnverified | Self::ChangedButFailed
        )
    }

    /// Whether the user should be warned. `Verified` is the only quiet success.
    pub(crate) fn warrants_warning(self) -> bool {
        !matches!(self, Self::Verified)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliMutationSummary {
    pub(crate) outcome: CliMutationOutcome,
    pub(crate) source_id: CliSourceId,
    pub(crate) action: String,
    pub(crate) target_version: Option<String>,
    pub(crate) operation_id: String,
    pub(crate) completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliEnvironmentSnapshot {
    pub(crate) schema_version: u16,
    pub(crate) agent_id: CliToolId,
    pub(crate) scope: CliEnvironmentScope,
    pub(crate) overall_state: CliOverallState,
    pub(crate) freshness: CliFreshness,
    pub(crate) environment_fingerprint: String,
    pub(crate) installations: Vec<CliInstallation>,
    /// What this process's PATH would actually run. `None` when nothing is on PATH -- a real
    /// answer, not a missing one.
    pub(crate) path_selected_installation_id: Option<CliInstallationId>,
    /// What the backend recommends after probing. Differs from the PATH-selected one exactly when
    /// there is something wrong worth showing.
    pub(crate) recommended_installation_id: Option<CliInstallationId>,
    pub(crate) discovery: CliDiscoveryStatus,
    pub(crate) executable: CliExecutableStatus,
    pub(crate) authentication: CliAuthenticationStatus,
    pub(crate) readiness: CliReadinessStatus,
    pub(crate) compatibility: CliCompatibilityStatus,
    pub(crate) update: CliUpdateStatus,
    pub(crate) conflicts: Vec<CliConflict>,
    pub(crate) sources: Vec<CliSourceSummary>,
    pub(crate) allowed_actions: Vec<CliAllowedAction>,
    pub(crate) last_mutation: Option<CliMutationSummary>,
    pub(crate) last_operation_id: Option<String>,
    pub(crate) checked_at: Option<DateTime<Utc>>,
}

impl CliEnvironmentSnapshot {
    /// A snapshot for a tool nothing is known about yet. Every axis is `unknown` or
    /// `not-applicable` -- never a claim that the tool is missing, which has not been established.
    pub(crate) fn never_scanned(agent_id: CliToolId, fingerprint: String) -> Self {
        Self {
            schema_version: SNAPSHOT_SCHEMA_VERSION,
            agent_id,
            scope: CliEnvironmentScope::LocalDesktop,
            overall_state: CliOverallState::Unknown,
            freshness: CliFreshness::Never,
            environment_fingerprint: fingerprint,
            installations: Vec::new(),
            path_selected_installation_id: None,
            recommended_installation_id: None,
            discovery: CliDiscoveryStatus::NotScanned,
            executable: CliExecutableStatus::Unknown,
            authentication: CliAuthenticationStatus::Unknown,
            readiness: CliReadinessStatus::Unknown,
            compatibility: CliCompatibilityStatus::Unknown,
            update: CliUpdateStatus::Unknown,
            conflicts: Vec::new(),
            sources: Vec::new(),
            allowed_actions: Vec::new(),
            last_mutation: None,
            last_operation_id: None,
            checked_at: None,
        }
    }

    pub(crate) fn axes(&self) -> CliStatusAxes {
        CliStatusAxes {
            discovery: self.discovery,
            executable: self.executable,
            authentication: self.authentication,
            compatibility: self.compatibility,
            update: self.update,
            has_conflict: !self.conflicts.is_empty(),
        }
    }

    /// Recomputes both installation identities, `executable`, `readiness`, and `overall_state`
    /// from the installations and probe results actually held.
    ///
    /// Derived fields are never assigned directly, so a caller cannot leave `overall_state` saying
    /// Ready while the executable axis says Broken.
    ///
    /// `discovery` is deliberately *not* derived. An empty installation list means "scanned and
    /// found nothing" after a refresh and "never looked" before one, and only the caller that ran
    /// the scan knows which. Deriving it would turn a tool nobody has checked into one reported as
    /// missing.
    pub(crate) fn recompute_derived(
        mut self,
        missing_dependency: bool,
        doctor_reported_problem: bool,
    ) -> Self {
        let selection = select_active(&self.installations);
        self.path_selected_installation_id = selection
            .path_selected
            .map(|index| self.installations[index].id.clone());
        self.recommended_installation_id = selection
            .recommended
            .map(|index| self.installations[index].id.clone());
        // The executable axis describes what the host would run, so it follows the PATH-selected
        // launcher. Following the recommended one instead would report Healthy for a machine whose
        // terminal runs a broken binary.
        self.executable = match selection.path_selected.or(selection.recommended) {
            Some(index) => self.installations[index].executable_status,
            None if self.discovery == CliDiscoveryStatus::NotScanned => {
                CliExecutableStatus::Unknown
            }
            // Nothing installed: there is no executable to judge, which is not a fault.
            None => CliExecutableStatus::NotApplicable,
        };

        let axes = self.axes();
        self.readiness = derive_readiness(axes, missing_dependency, doctor_reported_problem);
        self.overall_state = derive_overall_state(axes);
        self
    }

    /// Marks cached data as stale without discarding it. The page keeps showing the last known
    /// values with a badge rather than blanking out.
    pub(crate) fn mark_stale(&mut self) {
        self.freshness = CliFreshness::Stale;
    }

    pub(crate) fn mark_refreshing(&mut self) {
        self.freshness = CliFreshness::Refreshing;
    }

    /// Records how a mutation ended.
    ///
    /// Deliberately takes the *observed* snapshot as `self`: there is no method that restores a
    /// pre-operation snapshot, because a package manager cannot be rolled back by a database
    /// write and claiming otherwise is the regression this replaces.
    pub(crate) fn record_mutation(&mut self, summary: CliMutationSummary) {
        self.last_operation_id = Some(summary.operation_id.clone());
        // Verification did not confirm the machine, so what is held is last-known, not current.
        if summary.outcome != CliMutationOutcome::Verified
            && summary.outcome.may_have_changed_the_machine()
        {
            self.freshness = CliFreshness::Stale;
        }
        self.last_mutation = Some(summary);
    }

    /// The installation a mutation would target.
    ///
    /// Deliberately the *recommended* one: acting on a broken launcher that PATH happens to reach
    /// first would install over the copy the user is not using. The conflict list is what tells
    /// them the two differ, and `blocks_mutation` is what stops the action entirely when the
    /// difference makes the target ambiguous.
    pub(crate) fn recommended_installation(&self) -> Option<&CliInstallation> {
        let id = self.recommended_installation_id.as_ref()?;
        self.installations
            .iter()
            .find(|installation| &installation.id == id)
    }

    /// The installation the host would actually run.
    pub(crate) fn path_selected_installation(&self) -> Option<&CliInstallation> {
        let id = self.path_selected_installation_id.as_ref()?;
        self.installations
            .iter()
            .find(|installation| &installation.id == id)
    }

    /// Whether any conflict makes a machine change unsafe.
    pub(crate) fn blocks_mutation(&self) -> bool {
        conflicts_block_mutation(&self.conflicts)
    }

    pub(crate) fn violations(&self) -> Vec<CliSnapshotViolation> {
        let mut violations = Vec::new();
        if self.schema_version != SNAPSHOT_SCHEMA_VERSION {
            violations.push(CliSnapshotViolation::UnknownSchemaVersion);
        }
        let dangling = (self.recommended_installation_id.is_some()
            && self.recommended_installation().is_none())
            || (self.path_selected_installation_id.is_some()
                && self.path_selected_installation().is_none());
        if dangling {
            violations.push(CliSnapshotViolation::DanglingActiveInstallation);
        }
        // Installations exist but none is recommended: the page would show a version it cannot
        // attribute to anything. A missing *PATH* selection is legitimate -- it means nothing is
        // on PATH.
        if !self.installations.is_empty() && self.recommended_installation_id.is_none() {
            violations.push(CliSnapshotViolation::InstallationsWithoutActive);
        }
        if self.overall_state != derive_overall_state(self.axes()) {
            violations.push(CliSnapshotViolation::OverallStateDisagreesWithAxes);
        }
        violations
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliSnapshotViolation {
    UnknownSchemaVersion,
    /// `active_installation_id` names an installation the snapshot does not hold.
    DanglingActiveInstallation,
    /// Installations exist but none is active, which would leave the page with a version it
    /// cannot attribute to anything.
    InstallationsWithoutActive,
    OverallStateDisagreesWithAxes,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::cli::domain::installation::CliEnvironmentOrigin;
    use crate::contexts::tooling::cli::domain::source::{CliSourceConfidence, CliSourceKind};
    use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

    fn timestamp(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("timestamp")
    }

    fn tool() -> CliToolId {
        CliToolId::new("claude-code").expect("tool id")
    }

    fn installation(id: &str, version: &str, status: CliExecutableStatus) -> CliInstallation {
        CliInstallation {
            id: CliInstallationId::new(id).expect("installation id"),
            executable_path: format!("/path/{id}"),
            canonical_path: None,
            alias_paths: Vec::new(),
            target_missing: false,
            reported_version: Some(NormalizedCliVersion::parse(version)),
            source_id: Some(CliSourceId::new("npm").expect("source id")),
            source_kind: CliSourceKind::Npm,
            source_confidence: CliSourceConfidence::Inferred,
            path_priority: Some(0),
            environment_origin: CliEnvironmentOrigin::Path,
            executable_status: status,
        }
    }

    fn healthy_snapshot() -> CliEnvironmentSnapshot {
        let mut snapshot =
            CliEnvironmentSnapshot::never_scanned(tool(), "fingerprint-a".to_string());
        snapshot.installations = vec![installation("a", "1.2.0", CliExecutableStatus::Healthy)];
        // Set by whoever ran the scan; `recompute_derived` does not infer it.
        snapshot.discovery = CliDiscoveryStatus::FoundOne;
        snapshot.authentication = CliAuthenticationStatus::Authenticated;
        snapshot.compatibility = CliCompatibilityStatus::Supported;
        snapshot.update = CliUpdateStatus::UpToDate;
        snapshot.checked_at = Some(timestamp(1_000));
        snapshot.freshness = CliFreshness::Fresh;
        snapshot.recompute_derived(false, false)
    }

    #[test]
    fn a_never_scanned_snapshot_claims_nothing() {
        let snapshot = CliEnvironmentSnapshot::never_scanned(tool(), "fingerprint-a".to_string());

        // Crucially not `Missing`: nothing has been ruled out yet.
        assert_eq!(snapshot.overall_state, CliOverallState::Unknown);
        assert_eq!(snapshot.discovery, CliDiscoveryStatus::NotScanned);
        assert_eq!(snapshot.freshness, CliFreshness::Never);
        assert_eq!(snapshot.checked_at, None);
        assert_eq!(snapshot.scope, CliEnvironmentScope::LocalDesktop);
        assert_eq!(snapshot.scope.as_str(), "local-desktop");
        assert!(snapshot.violations().is_empty());
    }

    #[test]
    fn derived_fields_are_recomputed_rather_than_trusted() {
        let mut snapshot = healthy_snapshot();
        // A caller assigns something inconsistent.
        snapshot.overall_state = CliOverallState::Broken;
        snapshot.recommended_installation_id = None;
        assert!(snapshot
            .violations()
            .contains(&CliSnapshotViolation::OverallStateDisagreesWithAxes));

        let snapshot = snapshot.recompute_derived(false, false);
        assert_eq!(snapshot.overall_state, CliOverallState::Ready);
        assert_eq!(
            snapshot
                .recommended_installation_id
                .as_ref()
                .map(CliInstallationId::as_str),
            Some("a")
        );
        assert!(snapshot.violations().is_empty());
    }

    #[test]
    fn the_executable_axis_follows_the_path_selected_launcher_not_the_recommended_one() {
        let mut snapshot = healthy_snapshot();
        snapshot.installations = vec![
            installation("broken", "1.0.0", CliExecutableStatus::Broken),
            installation("working", "1.2.0", CliExecutableStatus::Healthy),
        ];
        snapshot.discovery = CliDiscoveryStatus::from_count(snapshot.installations.len());

        let snapshot = snapshot.recompute_derived(false, false);

        // Both are on PATH. The broken one is first, so that is what the user's terminal runs and
        // that is what the executable axis reports. Reporting Healthy here -- because a working
        // copy exists further down -- would describe a machine the user does not have.
        assert_eq!(snapshot.executable, CliExecutableStatus::Broken);
        assert_eq!(
            snapshot.path_selected_installation().map(|i| i.id.as_str()),
            Some("broken")
        );
        // The recommendation still points at the copy that works, so an action has a sane target.
        assert_eq!(
            snapshot.recommended_installation().map(|i| i.id.as_str()),
            Some("working")
        );
        assert_eq!(snapshot.discovery, CliDiscoveryStatus::FoundMultiple);
    }

    #[test]
    fn a_healthy_first_launcher_makes_both_identities_the_same() {
        let mut snapshot = healthy_snapshot();
        snapshot.installations = vec![
            installation("first", "1.2.0", CliExecutableStatus::Healthy),
            installation("second", "1.2.0", CliExecutableStatus::Healthy),
        ];
        snapshot.discovery = CliDiscoveryStatus::from_count(snapshot.installations.len());

        let snapshot = snapshot.recompute_derived(false, false);

        assert_eq!(snapshot.executable, CliExecutableStatus::Healthy);
        assert_eq!(
            snapshot.path_selected_installation_id,
            snapshot.recommended_installation_id
        );
    }

    #[test]
    fn nothing_installed_makes_the_executable_axis_not_applicable_not_broken() {
        let mut snapshot = healthy_snapshot();
        snapshot.installations = Vec::new();
        snapshot.discovery = CliDiscoveryStatus::NotFound;
        let snapshot = snapshot.recompute_derived(false, false);

        assert_eq!(snapshot.executable, CliExecutableStatus::NotApplicable);
        assert_eq!(snapshot.overall_state, CliOverallState::Missing);
        assert_eq!(snapshot.recommended_installation_id, None);
        assert!(snapshot.violations().is_empty());
    }

    #[test]
    fn stale_and_refreshing_preserve_everything_the_snapshot_knows() {
        let mut snapshot = healthy_snapshot();
        let installations = snapshot.installations.clone();

        snapshot.mark_refreshing();
        assert_eq!(snapshot.freshness, CliFreshness::Refreshing);
        assert_eq!(snapshot.installations, installations);
        assert_eq!(snapshot.overall_state, CliOverallState::Ready);

        snapshot.mark_stale();
        assert_eq!(snapshot.freshness, CliFreshness::Stale);
        // The cached values remain visible; only the label changed.
        assert_eq!(snapshot.installations, installations);
        assert_eq!(snapshot.checked_at, Some(timestamp(1_000)));
    }

    #[test]
    fn a_verified_mutation_leaves_the_snapshot_fresh() {
        let mut snapshot = healthy_snapshot();
        snapshot.record_mutation(CliMutationSummary {
            outcome: CliMutationOutcome::Verified,
            source_id: CliSourceId::new("npm").expect("source id"),
            action: "upgrade".to_string(),
            target_version: Some("1.3.0".to_string()),
            operation_id: "op-1".to_string(),
            completed_at: timestamp(2_000),
        });

        assert_eq!(snapshot.freshness, CliFreshness::Fresh);
        assert_eq!(snapshot.last_operation_id.as_deref(), Some("op-1"));
        assert_eq!(
            snapshot.last_mutation.as_ref().map(|m| m.outcome),
            Some(CliMutationOutcome::Verified)
        );
    }

    #[test]
    fn an_applied_but_unverified_mutation_marks_what_is_held_as_last_known() {
        // The command completed. Verification did not confirm it. The held values describe the
        // machine before the change, so they are stale -- not restored, not presented as current.
        let mut snapshot = healthy_snapshot();
        snapshot.record_mutation(CliMutationSummary {
            outcome: CliMutationOutcome::AppliedUnverified,
            source_id: CliSourceId::new("npm").expect("source id"),
            action: "upgrade".to_string(),
            target_version: Some("1.3.0".to_string()),
            operation_id: "op-2".to_string(),
            completed_at: timestamp(2_000),
        });

        assert_eq!(snapshot.freshness, CliFreshness::Stale);
        assert!(snapshot
            .last_mutation
            .as_ref()
            .is_some_and(|m| m.outcome.warrants_warning()));
    }

    #[test]
    fn a_failed_command_that_changed_the_machine_is_also_stale() {
        let mut snapshot = healthy_snapshot();
        snapshot.record_mutation(CliMutationSummary {
            outcome: CliMutationOutcome::ChangedButFailed,
            source_id: CliSourceId::new("npm").expect("source id"),
            action: "upgrade".to_string(),
            target_version: Some("1.3.0".to_string()),
            operation_id: "op-3".to_string(),
            completed_at: timestamp(2_000),
        });
        assert_eq!(snapshot.freshness, CliFreshness::Stale);
    }

    #[test]
    fn a_failure_that_changed_nothing_leaves_the_snapshot_current() {
        let mut snapshot = healthy_snapshot();
        for outcome in [
            CliMutationOutcome::NoChangeFailed,
            CliMutationOutcome::Cancelled,
        ] {
            snapshot.freshness = CliFreshness::Fresh;
            snapshot.record_mutation(CliMutationSummary {
                outcome,
                source_id: CliSourceId::new("npm").expect("source id"),
                action: "upgrade".to_string(),
                target_version: Some("1.3.0".to_string()),
                operation_id: "op-4".to_string(),
                completed_at: timestamp(2_000),
            });
            // Nothing on the machine moved, so the cached description is still accurate.
            assert_eq!(
                snapshot.freshness,
                CliFreshness::Fresh,
                "{}",
                outcome.as_str()
            );
            assert!(!outcome.may_have_changed_the_machine());
        }
    }

    #[test]
    fn outcome_classification_separates_machine_change_from_success() {
        assert!(CliMutationOutcome::Verified.may_have_changed_the_machine());
        assert!(CliMutationOutcome::AppliedUnverified.may_have_changed_the_machine());
        assert!(CliMutationOutcome::ChangedButFailed.may_have_changed_the_machine());
        assert!(!CliMutationOutcome::NoChangeFailed.may_have_changed_the_machine());
        assert!(!CliMutationOutcome::Cancelled.may_have_changed_the_machine());

        // Only a verified success is quiet.
        assert!(!CliMutationOutcome::Verified.warrants_warning());
        for outcome in [
            CliMutationOutcome::AppliedUnverified,
            CliMutationOutcome::ChangedButFailed,
            CliMutationOutcome::NoChangeFailed,
            CliMutationOutcome::Cancelled,
        ] {
            assert!(outcome.warrants_warning(), "{}", outcome.as_str());
        }
    }

    #[test]
    fn every_outcome_has_a_stable_wire_string() {
        assert_eq!(CliMutationOutcome::Verified.as_str(), "verified");
        assert_eq!(
            CliMutationOutcome::AppliedUnverified.as_str(),
            "applied-unverified"
        );
        assert_eq!(
            CliMutationOutcome::ChangedButFailed.as_str(),
            "changed-but-failed"
        );
        assert_eq!(
            CliMutationOutcome::NoChangeFailed.as_str(),
            "no-change-failed"
        );
        assert_eq!(CliMutationOutcome::Cancelled.as_str(), "cancelled");
    }

    #[test]
    fn a_dangling_active_installation_is_a_violation() {
        let mut snapshot = healthy_snapshot();
        snapshot.recommended_installation_id =
            Some(CliInstallationId::new("does-not-exist").expect("id"));
        assert!(snapshot
            .violations()
            .contains(&CliSnapshotViolation::DanglingActiveInstallation));
        assert_eq!(snapshot.recommended_installation(), None);

        snapshot.recommended_installation_id = None;
        assert!(snapshot
            .violations()
            .contains(&CliSnapshotViolation::InstallationsWithoutActive));
    }

    #[test]
    fn an_unknown_schema_version_is_a_violation_rather_than_a_panic() {
        let mut snapshot = healthy_snapshot();
        snapshot.schema_version = SNAPSHOT_SCHEMA_VERSION + 7;
        assert!(snapshot
            .violations()
            .contains(&CliSnapshotViolation::UnknownSchemaVersion));
        assert_eq!(healthy_snapshot().schema_version, SNAPSHOT_SCHEMA_VERSION);
    }
}
