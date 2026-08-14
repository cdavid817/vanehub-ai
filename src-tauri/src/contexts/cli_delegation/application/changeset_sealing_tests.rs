use super::*;
use crate::contexts::cli_delegation::application::{DelegationChangeKind, DelegationReportOutcome};
use std::sync::Mutex;

struct Artifacts(Mutex<Option<serde_json::Value>>);

impl DelegationChangeSetArtifactPort for Artifacts {
    fn seal_json(
        &self,
        _: &str,
        _: &str,
        _: &str,
        value: &serde_json::Value,
    ) -> Result<(String, String), ()> {
        *self.0.lock().map_err(|_| ())? = Some(value.clone());
        Ok(("artifact-1".into(), format!("sha256:{}", "b".repeat(64))))
    }
}

fn request(patch: Vec<u8>) -> DelegationChangeSetSealRequest {
    let diff_hash = sha256(&patch);
    DelegationChangeSetSealRequest {
        artifact_identity: "changeset-1".into(),
        delegation_id: "delegation-1".into(),
        attempt_id: "attempt-1".into(),
        repository_identity: "repository-1".into(),
        provider: DelegationTarget::CodexCli,
        cli_fingerprint: "sha256:cli".into(),
        adapter_fingerprint: "adapter-v1".into(),
        prompt_schema_fingerprint: "schema-v1".into(),
        capture: DelegationChangeSetCapture {
            base_commit: "base".into(),
            files: vec![DelegationChangeFile {
                path: "src/lib.rs".into(),
                previous_path: None,
                kind: DelegationChangeKind::Modified,
                before_mode: Some("100644".into()),
                after_mode: Some("100644".into()),
                before_git_hash: Some("before".into()),
                after_git_hash: Some("after".into()),
                binary: false,
            }],
            canonical_patch: patch,
            diff_hash: diff_hash.clone(),
        },
        provider_report: DelegationAgentReportV1 {
            schema_version: 1,
            outcome: DelegationReportOutcome::Completed,
            summary: "Completed".into(),
            findings: Vec::new(),
            actions_taken: Vec::new(),
            verification_claims: Vec::new(),
            risks: Vec::new(),
            follow_ups: Vec::new(),
            limitations: Vec::new(),
        },
        host_evidence: DelegationHostEvidence {
            base_commit: "base".into(),
            changed_files: vec!["src/lib.rs".into()],
            diff_hash: Some(diff_hash.clone()),
            exit_code: 0,
            observed_actions: Vec::new(),
            policy_violations: Vec::new(),
            cleanup_succeeded: true,
        },
        risk_classification: "review_required".into(),
        limitations: Vec::new(),
        created_at: "2026-08-14T00:00:00Z".into(),
    }
}

#[test]
fn seals_complete_binary_capable_manifest_as_one_artifact() {
    let artifacts = Arc::new(Artifacts(Mutex::new(None)));
    let sealed = DelegationChangeSetSealer::new(artifacts.clone())
        .seal(request(vec![0, 1, 2, 255]))
        .expect("sealed");
    assert_eq!(sealed.artifact_id, "artifact-1");
    let value = artifacts.0.lock().expect("lock").clone().expect("value");
    assert_eq!(value["schema_version"], 1);
    assert_eq!(value["applyable"], true);
    assert_eq!(value["patch_base64"], "AAEC/w==");
    assert_eq!(
        value["evidence_warnings"][0]["code"],
        "host_changed_file_not_provider_reported"
    );
}

#[test]
fn patch_tampering_prevents_artifact_creation() {
    let artifacts = Arc::new(Artifacts(Mutex::new(None)));
    let mut value = request(b"patch".to_vec());
    value.capture.canonical_patch.push(b'!');
    assert_eq!(
        DelegationChangeSetSealer::new(artifacts.clone()).seal(value),
        Err(DelegationChangeSetSealError::IntegrityFailure)
    );
    assert!(artifacts.0.lock().expect("lock").is_none());
}
