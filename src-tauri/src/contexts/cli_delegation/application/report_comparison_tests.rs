use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationAgentReportV1, DelegationHostEvidence, DelegationReportOutcome,
};

fn report() -> DelegationAgentReportV1 {
    DelegationAgentReportV1 {
        schema_version: 1,
        outcome: DelegationReportOutcome::Completed,
        summary: "Updated src/lib.rs".into(),
        findings: vec![],
        actions_taken: vec!["edited source".into()],
        verification_claims: vec!["tests passed".into()],
        risks: vec![],
        follow_ups: vec![],
        limitations: vec![],
    }
}

fn host() -> DelegationHostEvidence {
    DelegationHostEvidence {
        base_commit: "abc".into(),
        changed_files: vec!["src/lib.rs".into()],
        diff_hash: Some("hash".into()),
        exit_code: 0,
        observed_actions: vec!["edited source".into(), "tests passed".into()],
        policy_violations: vec![],
        cleanup_succeeded: true,
    }
}

#[test]
fn exact_host_observations_corroborate_claims_without_promoting_their_role() {
    assert!(DelegationReportComparator::compare(&report(), &host()).is_empty());
}

#[test]
fn surfaces_unobserved_claims_unreported_files_and_host_failures_as_warnings() {
    let mut host = host();
    host.changed_files.push("assets/icon.bin".into());
    host.observed_actions.clear();
    host.exit_code = 7;
    host.policy_violations.push("outside_write".into());
    host.cleanup_succeeded = false;

    let warnings = DelegationReportComparator::compare(&report(), &host);

    assert_eq!(warnings.len(), 6);
    assert!(warnings.contains(
        &DelegationEvidenceWarning::HostChangedFileNotProviderReported {
            path: "assets/icon.bin".into(),
        }
    ));
    assert!(warnings.contains(
        &DelegationEvidenceWarning::ProviderOutcomeConflictsWithHost {
            host_reason: "non_zero_exit".into(),
        }
    ));
}
