use super::*;
use serde_json::json;

fn report() -> Value {
    json!({
        "schema_version": 1,
        "outcome": "completed",
        "summary": "Analysis completed.",
        "findings": ["One finding"],
        "actions_taken": ["Inspected code"],
        "verification_claims": ["Tests passed"],
        "risks": [],
        "follow_ups": [],
        "limitations": ["No live provider call"]
    })
}

#[test]
fn provider_claims_remain_explicitly_untrusted() {
    let normalized = DelegationReportNormalizer::normalize(report()).expect("valid");
    assert_eq!(
        DelegationReportNormalizer::provider_claims(&normalized),
        vec![DelegationVerificationClaim {
            role: DelegationEvidenceRole::ProviderReported,
            claim: "Tests passed".into(),
        }]
    );
    let host = DelegationHostEvidence {
        base_commit: "abc".into(),
        changed_files: vec!["src/lib.rs".into()],
        diff_hash: Some("sha256:host".into()),
        exit_code: 0,
        observed_actions: Vec::new(),
        policy_violations: Vec::new(),
        cleanup_succeeded: true,
    };
    assert_eq!(host.changed_files, vec!["src/lib.rs"]);
}

#[test]
fn schema_drift_versions_and_unbounded_fields_fail_closed() {
    let mut unknown = report();
    unknown["extra"] = json!(true);
    assert_eq!(
        DelegationReportNormalizer::normalize(unknown),
        Err(DelegationReportError::InvalidSchema)
    );
    let mut version = report();
    version["schema_version"] = json!(2);
    assert_eq!(
        DelegationReportNormalizer::normalize(version),
        Err(DelegationReportError::InvalidVersion)
    );
    let mut oversized = report();
    oversized["summary"] = json!("x".repeat(MAX_SUMMARY_BYTES + 1));
    assert_eq!(
        DelegationReportNormalizer::normalize(oversized),
        Err(DelegationReportError::LimitExceeded)
    );
}
