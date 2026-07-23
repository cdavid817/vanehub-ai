use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LoopCheckOutcome {
    Passed,
    Failed,
    TimedOut,
    Cancelled,
    Error,
}

impl LoopCheckOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Passed => "passed",
            Self::Failed => "failed",
            Self::TimedOut => "timed-out",
            Self::Cancelled => "cancelled",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct LoopRequiredCheckObservation {
    pub(crate) command_id: String,
    pub(crate) outcome: LoopCheckOutcome,
    pub(crate) exit_code: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopObjectiveFingerprints {
    pub(crate) diff: String,
    pub(crate) required_check_failures: String,
    passing_required_checks: BTreeSet<String>,
}

impl LoopObjectiveFingerprints {
    pub(crate) fn rehydrate(diff: String, required_check_failures: String) -> Self {
        Self {
            diff,
            required_check_failures,
            passing_required_checks: BTreeSet::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct LoopRevisionProgress {
    pub(crate) progressed: bool,
    pub(crate) repeated_diff: bool,
    pub(crate) repeated_required_check_failures: bool,
    pub(crate) has_new_passing_required_evidence: bool,
}

pub(crate) fn fingerprint_objective_state(
    diff: &str,
    checks: &[LoopRequiredCheckObservation],
) -> LoopObjectiveFingerprints {
    let normalized_diff = diff.replace("\r\n", "\n").replace('\r', "\n");
    let mut observations = checks.to_vec();
    observations.sort();
    observations.dedup();

    let passing_required_checks = observations
        .iter()
        .filter(|check| check.outcome == LoopCheckOutcome::Passed)
        .map(|check| check.command_id.clone())
        .collect();
    let mut failures = String::new();
    for check in observations
        .iter()
        .filter(|check| check.outcome != LoopCheckOutcome::Passed)
    {
        append_field(&mut failures, &check.command_id);
        append_field(&mut failures, check.outcome.as_str());
        append_field(
            &mut failures,
            &check
                .exit_code
                .map(|code| code.to_string())
                .unwrap_or_else(|| "none".to_string()),
        );
    }

    LoopObjectiveFingerprints {
        diff: sha256(normalized_diff.as_bytes()),
        required_check_failures: sha256(failures.as_bytes()),
        passing_required_checks,
    }
}

pub(crate) fn assess_revision_progress(
    previous: Option<&LoopObjectiveFingerprints>,
    current: &LoopObjectiveFingerprints,
) -> LoopRevisionProgress {
    let Some(previous) = previous else {
        return LoopRevisionProgress {
            progressed: true,
            repeated_diff: false,
            repeated_required_check_failures: false,
            has_new_passing_required_evidence: !current.passing_required_checks.is_empty(),
        };
    };
    let repeated_diff = previous.diff == current.diff;
    let repeated_required_check_failures =
        previous.required_check_failures == current.required_check_failures;
    let has_new_passing_required_evidence = current
        .passing_required_checks
        .difference(&previous.passing_required_checks)
        .next()
        .is_some();
    LoopRevisionProgress {
        progressed: !repeated_diff
            || !repeated_required_check_failures
            || has_new_passing_required_evidence,
        repeated_diff,
        repeated_required_check_failures,
        has_new_passing_required_evidence,
    }
}

fn append_field(output: &mut String, value: &str) {
    output.push_str(&value.len().to_string());
    output.push(':');
    output.push_str(value);
    output.push(';');
}

fn sha256(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(command_id: &str, outcome: LoopCheckOutcome) -> LoopRequiredCheckObservation {
        LoopRequiredCheckObservation {
            command_id: command_id.to_string(),
            outcome,
            exit_code: (outcome == LoopCheckOutcome::Failed).then_some(1),
        }
    }

    #[test]
    fn fingerprints_are_stable_across_line_endings_and_check_order() {
        let first = fingerprint_objective_state(
            "diff --git a/a b/a\r\n+change\r\n",
            &[
                check("lint", LoopCheckOutcome::Failed),
                check("test", LoopCheckOutcome::Passed),
            ],
        );
        let second = fingerprint_objective_state(
            "diff --git a/a b/a\n+change\n",
            &[
                check("test", LoopCheckOutcome::Passed),
                check("lint", LoopCheckOutcome::Failed),
            ],
        );

        assert_eq!(first, second);
        assert_eq!(first.diff.len(), 64);
        assert_eq!(first.required_check_failures.len(), 64);
    }

    #[test]
    fn only_repeated_objective_state_without_new_pass_is_no_progress() {
        let previous =
            fingerprint_objective_state("same diff", &[check("lint", LoopCheckOutcome::Failed)]);
        let repeated =
            fingerprint_objective_state("same diff", &[check("lint", LoopCheckOutcome::Failed)]);
        assert!(!assess_revision_progress(Some(&previous), &repeated).progressed);

        let changed_diff = fingerprint_objective_state(
            "different diff",
            &[check("lint", LoopCheckOutcome::Failed)],
        );
        assert!(assess_revision_progress(Some(&previous), &changed_diff).progressed);

        let new_pass = fingerprint_objective_state(
            "same diff",
            &[
                check("lint", LoopCheckOutcome::Failed),
                check("test", LoopCheckOutcome::Passed),
            ],
        );
        let assessment = assess_revision_progress(Some(&previous), &new_pass);
        assert!(assessment.progressed);
        assert!(assessment.has_new_passing_required_evidence);
    }

    #[test]
    fn all_failure_outcomes_contribute_to_failure_fingerprint() {
        let outcomes = [
            LoopCheckOutcome::Failed,
            LoopCheckOutcome::TimedOut,
            LoopCheckOutcome::Cancelled,
            LoopCheckOutcome::Error,
        ];
        let fingerprints = outcomes
            .into_iter()
            .map(|outcome| {
                fingerprint_objective_state("diff", &[check("test", outcome)])
                    .required_check_failures
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(fingerprints.len(), outcomes.len());
    }
}
