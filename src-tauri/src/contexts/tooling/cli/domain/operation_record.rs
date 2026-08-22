//! What is persisted about a finished CLI operation.
//!
//! The record is safe by construction rather than by review: every field is a typed identifier, a
//! version, or an enum. There is no `String` a path, a credential, or a fragment of process output
//! could be assigned to, so the redaction rule cannot be forgotten at one of the three boundaries
//! that consume this -- operation storage, the frontend DTO, and the unified log.
//!
//! Versions are the one free-form value, and they are `NormalizedCliVersion`, which has already
//! rejected anything that is not a version.

use super::action::CliActionKind;
use super::ids::{CliSourceId, CliToolId};
use super::phase::CliOperationPhase;
use super::snapshot::CliMutationOutcome;
use super::version::NormalizedCliVersion;

/// How the external process ended.
///
/// Separate from the outcome: `Exited { code: 0 }` says the command reported success, while the
/// outcome says whether the machine was verified to match. Collapsing the two is how a successful
/// command with a failed verification gets reported as a clean success.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliOperationTermination {
    /// No process was started -- refused during preflight, or cancelled before spawning.
    NotStarted,
    Exited {
        code: i32,
    },
    /// The process ran but reported no exit code, so nothing can be concluded from one.
    ExitedWithoutCode,
    TimedOut,
    Cancelled,
}

impl CliOperationTermination {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::NotStarted => "not-started",
            Self::Exited { .. } => "exited",
            Self::ExitedWithoutCode => "exited-without-code",
            Self::TimedOut => "timed-out",
            Self::Cancelled => "cancelled",
        }
    }

    pub(crate) fn exit_code(self) -> Option<i32> {
        match self {
            Self::Exited { code } => Some(code),
            _ => None,
        }
    }
}

/// Why an operation could not be verified after the fact.
///
/// Recorded rather than inferred: "we did not look" and "we looked and could not tell" lead to the
/// same stale snapshot but to different advice for the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CliVerificationWarning {
    /// A package manager was writing the same resource, so detection would have observed a
    /// half-written tree and reported it as the machine's state.
    DetectionSkippedWhileBusy,
    /// Detection ran and failed. The last-known data is kept, labelled stale.
    DetectionFailed,
    /// Detection succeeded but the installed version is not the one the plan targeted.
    TargetVersionNotObserved,
}

impl CliVerificationWarning {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DetectionSkippedWhileBusy => "detection-skipped-while-busy",
            Self::DetectionFailed => "detection-failed",
            Self::TargetVersionNotObserved => "target-version-not-observed",
        }
    }
}

/// The persisted context of one lifecycle operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliOperationRecord {
    pub(crate) operation_id: String,
    pub(crate) agent_id: Option<CliToolId>,
    pub(crate) source_id: Option<CliSourceId>,
    pub(crate) action: Option<CliActionKind>,
    /// The version the plan targeted, already normalized. Never a raw user string.
    pub(crate) target_version: Option<NormalizedCliVersion>,
    /// The version observed after the operation, when detection produced one.
    pub(crate) observed_version: Option<NormalizedCliVersion>,
    /// The phase the operation reached. On a failure this is where it stopped.
    pub(crate) phase: CliOperationPhase,
    pub(crate) termination: CliOperationTermination,
    pub(crate) elapsed_ms: u64,
    pub(crate) outcome: Option<CliMutationOutcome>,
    pub(crate) warnings: Vec<CliVerificationWarning>,
    /// Whether retained output hit its ceiling. The output itself is not part of this record.
    pub(crate) output_truncated: bool,
}

impl CliOperationRecord {
    /// A record for an operation that ended before any process ran.
    pub(crate) fn unstarted(operation_id: String, phase: CliOperationPhase) -> Self {
        Self {
            operation_id,
            agent_id: None,
            source_id: None,
            action: None,
            target_version: None,
            observed_version: None,
            phase,
            termination: CliOperationTermination::NotStarted,
            elapsed_ms: 0,
            outcome: None,
            warnings: Vec::new(),
            output_truncated: false,
        }
    }

    /// Whether the user should be told something beyond "done".
    pub(crate) fn warrants_attention(&self) -> bool {
        !self.warnings.is_empty()
            || self
                .outcome
                .is_some_and(CliMutationOutcome::warrants_warning)
            || !matches!(
                self.termination,
                CliOperationTermination::Exited { code: 0 }
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_termination_reports_an_exit_code_only_when_one_exists() {
        assert_eq!(
            CliOperationTermination::Exited { code: 0 }.exit_code(),
            Some(0)
        );
        assert_eq!(
            CliOperationTermination::Exited { code: 137 }.exit_code(),
            Some(137)
        );
        // A timeout, a cancellation and a process that never reported one are all distinct from
        // "exited 0", and none of them may be read as a code.
        assert_eq!(CliOperationTermination::TimedOut.exit_code(), None);
        assert_eq!(CliOperationTermination::Cancelled.exit_code(), None);
        assert_eq!(CliOperationTermination::NotStarted.exit_code(), None);
        assert_eq!(CliOperationTermination::ExitedWithoutCode.exit_code(), None);
    }

    #[test]
    fn every_termination_and_warning_has_a_distinct_wire_string() {
        let terminations = [
            CliOperationTermination::NotStarted,
            CliOperationTermination::Exited { code: 0 },
            CliOperationTermination::ExitedWithoutCode,
            CliOperationTermination::TimedOut,
            CliOperationTermination::Cancelled,
        ];
        let mut labels = terminations
            .iter()
            .map(|termination| termination.as_str())
            .collect::<Vec<_>>();
        let total = labels.len();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), total);

        let warnings = [
            CliVerificationWarning::DetectionSkippedWhileBusy,
            CliVerificationWarning::DetectionFailed,
            CliVerificationWarning::TargetVersionNotObserved,
        ];
        let mut warning_labels = warnings
            .iter()
            .map(|warning| warning.as_str())
            .collect::<Vec<_>>();
        let warning_total = warning_labels.len();
        warning_labels.sort_unstable();
        warning_labels.dedup();
        assert_eq!(warning_labels.len(), warning_total);
    }

    #[test]
    fn the_exit_code_does_not_decide_the_outcome_on_its_own() {
        // A command that exits 0 and a verification that could not confirm it are both true at
        // once. The record keeps them as separate fields precisely so the pair is representable.
        let record = CliOperationRecord {
            termination: CliOperationTermination::Exited { code: 0 },
            outcome: Some(CliMutationOutcome::AppliedUnverified),
            warnings: vec![CliVerificationWarning::DetectionFailed],
            ..CliOperationRecord::unstarted("op-1".to_string(), CliOperationPhase::Completed)
        };

        assert_eq!(record.termination.exit_code(), Some(0));
        assert!(record.warrants_attention());
    }

    #[test]
    fn a_clean_verified_operation_needs_no_warning() {
        let record = CliOperationRecord {
            termination: CliOperationTermination::Exited { code: 0 },
            outcome: Some(CliMutationOutcome::Verified),
            ..CliOperationRecord::unstarted("op-1".to_string(), CliOperationPhase::Completed)
        };
        assert!(!record.warrants_attention());
    }

    #[test]
    fn a_cancelled_operation_always_warrants_attention() {
        let record = CliOperationRecord {
            termination: CliOperationTermination::Cancelled,
            outcome: Some(CliMutationOutcome::Cancelled),
            ..CliOperationRecord::unstarted("op-1".to_string(), CliOperationPhase::Downloading)
        };
        assert!(record.warrants_attention());
        assert_eq!(record.phase, CliOperationPhase::Downloading);
    }

    #[test]
    fn an_unstarted_record_claims_nothing_about_the_machine() {
        let record =
            CliOperationRecord::unstarted("op-1".to_string(), CliOperationPhase::Preflight);

        assert_eq!(record.outcome, None);
        assert_eq!(record.observed_version, None);
        assert_eq!(record.elapsed_ms, 0);
        assert!(record.warnings.is_empty());
        assert!(!record.output_truncated);
        // Not-started is not exit-zero, so it is never mistaken for a quiet success.
        assert!(record.warrants_attention());
    }
}
