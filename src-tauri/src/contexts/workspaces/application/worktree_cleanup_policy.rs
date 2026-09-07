//! Whether a worktree may be offered for removal, and why not.
//!
//! Pure: it takes facts and returns a decision. Every refusal is a stable reason code the
//! frontend localizes, and any fact that is missing or incomplete is a refusal — the policy
//! never infers safety from the absence of evidence.

use super::worktree_cleanup_models::{CheckCompleteness, WorktreeInspection, WorktreeProbe};
use crate::contexts::workspaces::domain::WorktreeOrigin;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorktreeCleanupPolicy {
    Keep,
    RemoveSafe,
}

impl WorktreeCleanupPolicy {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Keep => "keep",
            Self::RemoveSafe => "remove-safe",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "keep" => Some(Self::Keep),
            "remove-safe" => Some(Self::RemoveSafe),
            _ => None,
        }
    }
}

/// Stable reason codes. Strings rather than an enum so the DTO, the journal and the locale
/// tables all carry the same literal without a mapping each could get wrong.
pub(crate) mod reason {
    pub(crate) const PROVENANCE_UNVERIFIED: &str = "provenance_unverified";
    pub(crate) const ORIGIN_NOT_ORDINARY: &str = "origin_not_ordinary";
    pub(crate) const RESOURCE_STATUS: &str = "resource_status";
    pub(crate) const DIRECTORY_MISSING: &str = "directory_missing";
    pub(crate) const NOT_GIT_WORKTREE: &str = "not_git_worktree";
    pub(crate) const MAIN_OR_BARE_WORKSPACE: &str = "main_or_bare_workspace";
    pub(crate) const NOT_REGISTERED: &str = "not_registered";
    pub(crate) const IDENTITY_MISMATCH: &str = "identity_mismatch";
    pub(crate) const LOCKED: &str = "locked";
    pub(crate) const PRUNABLE: &str = "prunable";
    pub(crate) const DETACHED_HEAD: &str = "detached_head";
    pub(crate) const BRANCH_NOT_RESOLVING: &str = "branch_not_resolving";
    pub(crate) const IN_PROGRESS_OPERATION: &str = "in_progress_operation";
    pub(crate) const NESTED_LAYOUT: &str = "nested_layout";
    pub(crate) const UNSUPPORTED_LAYOUT: &str = "unsupported_layout";
    pub(crate) const TRACKED_CHANGES: &str = "tracked_changes";
    pub(crate) const STAGED_CHANGES: &str = "staged_changes";
    pub(crate) const CONFLICTS: &str = "conflicts";
    pub(crate) const UNTRACKED_FILES: &str = "untracked_files";
    pub(crate) const CHANGES_INCOMPLETE: &str = "changes_incomplete";
    pub(crate) const IGNORED_INCOMPLETE: &str = "ignored_incomplete";
    pub(crate) const REFERENCES_INCOMPLETE: &str = "references_incomplete";
    pub(crate) const EXTERNAL_REFERENCES: &str = "external_references";
    pub(crate) const GATE_HELD: &str = "gate_held";
    pub(crate) const NO_ANCHOR: &str = "no_anchor";
    pub(crate) const PROBE_FAILED: &str = "probe_failed";
    pub(crate) const GIT_UNAVAILABLE: &str = "git_unavailable";
    pub(crate) const REMOTE_WORKSPACE: &str = "remote_workspace";
    pub(crate) const NO_WORKTREE: &str = "no_worktree";
}

/// References the caller found outside the deletion selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct ReferenceSummary {
    pub(crate) external_count: usize,
    pub(crate) completeness: Option<CheckCompleteness>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CleanupEvaluation {
    pub(crate) allowed_policies: Vec<WorktreeCleanupPolicy>,
    pub(crate) blockers: Vec<&'static str>,
    pub(crate) requires_ignored_acknowledgement: bool,
    pub(crate) checks: CheckCompleteness,
}

impl CleanupEvaluation {
    pub(crate) fn allows_removal(&self) -> bool {
        self.allowed_policies
            .contains(&WorktreeCleanupPolicy::RemoveSafe)
    }

    fn keep_only(blockers: Vec<&'static str>, checks: CheckCompleteness) -> Self {
        Self {
            allowed_policies: vec![WorktreeCleanupPolicy::Keep],
            blockers,
            requires_ignored_acknowledgement: false,
            checks,
        }
    }
}

/// Decide from a full inspection. `Keep` is always allowed: nothing here ever stops a user from
/// deleting their own session record.
pub(crate) fn evaluate_cleanup(
    inspection: &WorktreeInspection,
    references: ReferenceSummary,
    gate_held_by_other: bool,
) -> CleanupEvaluation {
    let mut blockers = Vec::new();
    let record = &inspection.record;
    if record.origin != WorktreeOrigin::OrdinarySession {
        blockers.push(reason::ORIGIN_NOT_ORDINARY);
    }
    let provenance_verified = matches!(
        record.provenance,
        crate::contexts::workspaces::domain::WorktreeProvenance::Verified
            | crate::contexts::workspaces::domain::WorktreeProvenance::LegacyVerified
    ) && record.identity.is_some();
    if !provenance_verified {
        blockers.push(reason::PROVENANCE_UNVERIFIED);
    } else if !record.cleanup_eligible() {
        blockers.push(reason::RESOURCE_STATUS);
    }
    if !inspection.identity_matches {
        blockers.push(reason::IDENTITY_MISMATCH);
    }
    if gate_held_by_other {
        blockers.push(reason::GATE_HELD);
    }
    let (probe_blockers, checks, requires_acknowledgement) = evaluate_probe(&inspection.probe);
    blockers.extend(probe_blockers);
    match references.completeness {
        Some(CheckCompleteness::Complete) => {
            if references.external_count > 0 {
                blockers.push(reason::EXTERNAL_REFERENCES);
            }
        }
        _ => blockers.push(reason::REFERENCES_INCOMPLETE),
    }
    let checks = if references.completeness == Some(CheckCompleteness::Complete) {
        checks
    } else {
        CheckCompleteness::Incomplete
    };
    blockers.dedup();
    if blockers.is_empty() {
        CleanupEvaluation {
            allowed_policies: vec![
                WorktreeCleanupPolicy::Keep,
                WorktreeCleanupPolicy::RemoveSafe,
            ],
            blockers,
            requires_ignored_acknowledgement: requires_acknowledgement,
            checks,
        }
    } else {
        CleanupEvaluation::keep_only(blockers, checks)
    }
}

/// The probe's own contribution: blockers, overall completeness, and whether ignored files exist
/// (which is a separate acknowledgement, not a blocker).
fn evaluate_probe(probe: &WorktreeProbe) -> (Vec<&'static str>, CheckCompleteness, bool) {
    let mut blockers = Vec::new();
    let mut checks = CheckCompleteness::Complete;
    if let Some(failure) = probe.failure {
        blockers.push(failure);
        return (blockers, CheckCompleteness::Incomplete, false);
    }
    if !probe.root_exists {
        blockers.push(reason::DIRECTORY_MISSING);
        return (blockers, CheckCompleteness::Complete, false);
    }
    if probe.identity.is_none() {
        blockers.push(reason::NOT_GIT_WORKTREE);
        return (blockers, CheckCompleteness::Complete, false);
    }
    if !probe.is_linked {
        blockers.push(reason::MAIN_OR_BARE_WORKSPACE);
    }
    if probe.anchor.is_none() {
        blockers.push(reason::NO_ANCHOR);
    }
    if !probe.registered {
        blockers.push(reason::NOT_REGISTERED);
    }
    if probe.locked {
        blockers.push(reason::LOCKED);
    }
    if probe.prunable {
        blockers.push(reason::PRUNABLE);
    }
    if probe.detached {
        blockers.push(reason::DETACHED_HEAD);
    } else if !probe.branch_resolves_to_head {
        blockers.push(reason::BRANCH_NOT_RESOLVING);
    }
    if probe.in_progress_operation {
        blockers.push(reason::IN_PROGRESS_OPERATION);
    }
    if probe.nested_layout {
        blockers.push(reason::NESTED_LAYOUT);
    }
    if probe.unsupported_layout.is_some() {
        blockers.push(reason::UNSUPPORTED_LAYOUT);
    }
    match &probe.changes {
        Some(changes) => {
            if changes.tracked_modified > 0 {
                blockers.push(reason::TRACKED_CHANGES);
            }
            if changes.staged > 0 {
                blockers.push(reason::STAGED_CHANGES);
            }
            if changes.conflicted > 0 {
                blockers.push(reason::CONFLICTS);
            }
            if changes.untracked > 0 {
                blockers.push(reason::UNTRACKED_FILES);
            }
            if changes.completeness != Some(CheckCompleteness::Complete) {
                blockers.push(reason::CHANGES_INCOMPLETE);
                checks = CheckCompleteness::Incomplete;
            }
        }
        None => {
            blockers.push(reason::CHANGES_INCOMPLETE);
            checks = CheckCompleteness::Incomplete;
        }
    }
    let requires_acknowledgement = match &probe.ignored {
        Some(inventory) => {
            if inventory.completeness != CheckCompleteness::Complete {
                blockers.push(reason::IGNORED_INCOMPLETE);
                checks = CheckCompleteness::Incomplete;
            }
            inventory.total_entries > 0
        }
        None => {
            // No inventory at all only counts as complete when Git reported zero ignored paths.
            let ignored_paths = probe
                .changes
                .as_ref()
                .map(|changes| changes.ignored_paths)
                .unwrap_or(usize::MAX);
            if ignored_paths > 0 {
                blockers.push(reason::IGNORED_INCOMPLETE);
                checks = CheckCompleteness::Incomplete;
            }
            false
        }
    };
    (blockers, checks, requires_acknowledgement)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::workspaces::application::worktree_cleanup_models::{
        IgnoredInventory, WorktreeChangeSummary,
    };
    use crate::contexts::workspaces::domain::{ManagedWorktree, WorktreeIdentity};

    fn identity() -> WorktreeIdentity {
        WorktreeIdentity {
            canonical_root: "/repo-feature".to_string(),
            git_dir: "/repo/.git/worktrees/repo-feature".to_string(),
            common_dir: "/repo/.git".to_string(),
            branch: Some("vanehub/feature".to_string()),
            head: Some("abc".to_string()),
            fs_identity: Some("1:2".to_string()),
        }
    }

    fn verified_record() -> ManagedWorktree {
        let mut record = ManagedWorktree::provisioning(
            "wt".to_string(),
            WorktreeOrigin::OrdinarySession,
            "/repo".to_string(),
            "/repo-feature".to_string(),
            None,
            "t0".to_string(),
        )
        .expect("record");
        record
            .confirm_created(identity(), "t1".to_string())
            .expect("confirm");
        record
    }

    fn clean_probe() -> WorktreeProbe {
        WorktreeProbe {
            identity: Some(identity()),
            root_exists: true,
            is_linked: true,
            registered: true,
            branch_resolves_to_head: true,
            anchor: Some("/repo".to_string()),
            changes: Some(WorktreeChangeSummary {
                completeness: Some(CheckCompleteness::Complete),
                ..WorktreeChangeSummary::default()
            }),
            ignored: Some(IgnoredInventory {
                total_entries: 0,
                samples: Vec::new(),
                samples_truncated: false,
                completeness: CheckCompleteness::Complete,
                fingerprint: "empty".to_string(),
            }),
            ..WorktreeProbe::default()
        }
    }

    fn inspection(probe: WorktreeProbe) -> WorktreeInspection {
        WorktreeInspection {
            record: verified_record(),
            probe,
            identity_matches: true,
        }
    }

    fn complete_references() -> ReferenceSummary {
        ReferenceSummary {
            external_count: 0,
            completeness: Some(CheckCompleteness::Complete),
        }
    }

    #[test]
    fn a_clean_verified_registered_worktree_may_be_removed() {
        let evaluation = evaluate_cleanup(&inspection(clean_probe()), complete_references(), false);
        assert!(evaluation.allows_removal());
        assert!(evaluation.blockers.is_empty());
        assert!(!evaluation.requires_ignored_acknowledgement);
        assert_eq!(evaluation.checks, CheckCompleteness::Complete);
    }

    #[test]
    fn keep_is_always_allowed_even_when_everything_is_wrong() {
        let evaluation = evaluate_cleanup(
            &inspection(WorktreeProbe::failed(reason::GIT_UNAVAILABLE)),
            ReferenceSummary::default(),
            true,
        );
        assert_eq!(
            evaluation.allowed_policies,
            vec![WorktreeCleanupPolicy::Keep]
        );
        assert!(evaluation.blockers.contains(&reason::GIT_UNAVAILABLE));
        assert!(evaluation.blockers.contains(&reason::GATE_HELD));
        assert!(evaluation.blockers.contains(&reason::REFERENCES_INCOMPLETE));
        assert_eq!(evaluation.checks, CheckCompleteness::Incomplete);
    }

    #[test]
    fn every_non_ignored_change_kind_blocks_and_ignored_only_requires_acknowledgement() {
        for (summary, expected) in [
            (
                WorktreeChangeSummary {
                    tracked_modified: 1,
                    completeness: Some(CheckCompleteness::Complete),
                    ..WorktreeChangeSummary::default()
                },
                reason::TRACKED_CHANGES,
            ),
            (
                WorktreeChangeSummary {
                    staged: 1,
                    completeness: Some(CheckCompleteness::Complete),
                    ..WorktreeChangeSummary::default()
                },
                reason::STAGED_CHANGES,
            ),
            (
                WorktreeChangeSummary {
                    conflicted: 1,
                    completeness: Some(CheckCompleteness::Complete),
                    ..WorktreeChangeSummary::default()
                },
                reason::CONFLICTS,
            ),
            (
                WorktreeChangeSummary {
                    untracked: 1,
                    completeness: Some(CheckCompleteness::Complete),
                    ..WorktreeChangeSummary::default()
                },
                reason::UNTRACKED_FILES,
            ),
            (
                WorktreeChangeSummary {
                    completeness: Some(CheckCompleteness::Incomplete),
                    ..WorktreeChangeSummary::default()
                },
                reason::CHANGES_INCOMPLETE,
            ),
        ] {
            let mut probe = clean_probe();
            probe.changes = Some(summary);
            let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
            assert!(!evaluation.allows_removal(), "{expected}");
            assert_eq!(evaluation.blockers, vec![expected]);
        }

        let mut probe = clean_probe();
        probe.ignored = Some(IgnoredInventory {
            total_entries: 3,
            samples: Vec::new(),
            samples_truncated: false,
            completeness: CheckCompleteness::Complete,
            fingerprint: "fp".to_string(),
        });
        let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
        assert!(evaluation.allows_removal());
        assert!(evaluation.requires_ignored_acknowledgement);
    }

    #[test]
    fn incomplete_or_missing_inventories_never_read_as_empty() {
        let mut probe = clean_probe();
        probe.ignored = Some(IgnoredInventory {
            total_entries: 10_000,
            samples: Vec::new(),
            samples_truncated: true,
            completeness: CheckCompleteness::Incomplete,
            fingerprint: "fp".to_string(),
        });
        let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
        assert_eq!(evaluation.blockers, vec![reason::IGNORED_INCOMPLETE]);
        assert_eq!(evaluation.checks, CheckCompleteness::Incomplete);

        let mut probe = clean_probe();
        probe.ignored = None;
        probe.changes = Some(WorktreeChangeSummary {
            ignored_paths: 2,
            completeness: Some(CheckCompleteness::Complete),
            ..WorktreeChangeSummary::default()
        });
        let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
        assert_eq!(evaluation.blockers, vec![reason::IGNORED_INCOMPLETE]);

        let mut probe = clean_probe();
        probe.changes = None;
        let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
        assert!(evaluation.blockers.contains(&reason::CHANGES_INCOMPLETE));
    }

    #[test]
    fn topology_and_registration_problems_each_block() {
        type Case = (fn(&mut WorktreeProbe), &'static str);
        let cases: Vec<Case> = vec![
            (|probe| probe.root_exists = false, reason::DIRECTORY_MISSING),
            (|probe| probe.identity = None, reason::NOT_GIT_WORKTREE),
            (
                |probe| probe.is_linked = false,
                reason::MAIN_OR_BARE_WORKSPACE,
            ),
            (|probe| probe.anchor = None, reason::NO_ANCHOR),
            (|probe| probe.registered = false, reason::NOT_REGISTERED),
            (|probe| probe.locked = true, reason::LOCKED),
            (|probe| probe.prunable = true, reason::PRUNABLE),
            (|probe| probe.detached = true, reason::DETACHED_HEAD),
            (
                |probe| probe.branch_resolves_to_head = false,
                reason::BRANCH_NOT_RESOLVING,
            ),
            (
                |probe| probe.in_progress_operation = true,
                reason::IN_PROGRESS_OPERATION,
            ),
            (|probe| probe.nested_layout = true, reason::NESTED_LAYOUT),
            (
                |probe| probe.unsupported_layout = Some("submodule"),
                reason::UNSUPPORTED_LAYOUT,
            ),
        ];
        for (mutate, expected) in cases {
            let mut probe = clean_probe();
            mutate(&mut probe);
            let evaluation = evaluate_cleanup(&inspection(probe), complete_references(), false);
            assert!(!evaluation.allows_removal(), "{expected}");
            assert!(evaluation.blockers.contains(&expected), "{expected}");
        }
    }

    #[test]
    fn provenance_identity_and_reference_facts_block_independently() {
        let mut unverified = inspection(clean_probe());
        unverified.record.provenance =
            crate::contexts::workspaces::domain::WorktreeProvenance::LegacyUnverified;
        let evaluation = evaluate_cleanup(&unverified, complete_references(), false);
        assert_eq!(evaluation.blockers, vec![reason::PROVENANCE_UNVERIFIED]);

        let mut loop_owned = inspection(clean_probe());
        loop_owned.record.origin = WorktreeOrigin::Loop;
        let evaluation = evaluate_cleanup(&loop_owned, complete_references(), false);
        assert!(evaluation.blockers.contains(&reason::ORIGIN_NOT_ORDINARY));

        let mut drifted = inspection(clean_probe());
        drifted.identity_matches = false;
        let evaluation = evaluate_cleanup(&drifted, complete_references(), false);
        assert_eq!(evaluation.blockers, vec![reason::IDENTITY_MISMATCH]);

        let evaluation = evaluate_cleanup(
            &inspection(clean_probe()),
            ReferenceSummary {
                external_count: 1,
                completeness: Some(CheckCompleteness::Complete),
            },
            false,
        );
        assert_eq!(evaluation.blockers, vec![reason::EXTERNAL_REFERENCES]);

        let evaluation = evaluate_cleanup(
            &inspection(clean_probe()),
            ReferenceSummary {
                external_count: 0,
                completeness: Some(CheckCompleteness::Incomplete),
            },
            false,
        );
        assert_eq!(evaluation.blockers, vec![reason::REFERENCES_INCOMPLETE]);
        assert_eq!(evaluation.checks, CheckCompleteness::Incomplete);
    }

    #[test]
    fn a_removing_record_is_not_offered_again() {
        let mut inspection = inspection(clean_probe());
        inspection
            .record
            .begin_removal("t2".to_string())
            .expect("removing");
        let evaluation = evaluate_cleanup(&inspection, complete_references(), false);
        assert_eq!(evaluation.blockers, vec![reason::RESOURCE_STATUS]);
    }

    #[test]
    fn policy_literals_round_trip_and_reject_unknown_values() {
        assert_eq!(
            WorktreeCleanupPolicy::parse("remove-safe"),
            Some(WorktreeCleanupPolicy::RemoveSafe)
        );
        assert_eq!(
            WorktreeCleanupPolicy::parse(WorktreeCleanupPolicy::Keep.as_str()),
            Some(WorktreeCleanupPolicy::Keep)
        );
        assert_eq!(WorktreeCleanupPolicy::parse("force"), None);
    }
}
