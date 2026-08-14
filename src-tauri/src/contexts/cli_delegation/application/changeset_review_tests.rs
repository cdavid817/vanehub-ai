use super::*;
use serde_json::json;

struct Artifact(Vec<u8>, String);

impl DelegationChangeSetReviewPort for Artifact {
    fn load(&self, _: &str, max_bytes: usize) -> Result<DelegationChangeSetPayload, ()> {
        if self.0.len() > max_bytes {
            return Err(());
        }
        Ok(DelegationChangeSetPayload {
            content_hash: self.1.clone(),
            bytes: self.0.clone(),
        })
    }
}

fn manifest(patch: &[u8]) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": 1,
        "artifact_identity": "changeset-1",
        "delegation_id": "delegation-1",
        "attempt_id": "attempt-1",
        "repository_identity": "repository-1",
        "base_commit": "base",
        "provider": "codex_cli",
        "cli_fingerprint": "cli-v1",
        "adapter_fingerprint": "adapter-v1",
        "prompt_schema_fingerprint": "schema-v1",
        "files": [{
            "path": "src/lib.rs", "previous_path": null, "kind": "modified",
            "before_mode": "100644", "after_mode": "100644",
            "before_git_hash": "before", "after_git_hash": "after", "binary": false
        }, {
            "path": "assets/icon.bin", "previous_path": null, "kind": "added",
            "before_mode": null, "after_mode": "100644",
            "before_git_hash": null, "after_git_hash": "after-bin", "binary": true
        }],
        "patch_base64": STANDARD.encode(patch),
        "diff_hash": sha256(patch),
        "provider_report": {
            "schema_version": 1, "outcome": "completed", "summary": "done",
            "findings": [], "actions_taken": [], "verification_claims": [],
            "risks": [], "follow_ups": [], "limitations": []
        },
        "host_evidence": {
            "base_commit": "base", "changed_files": ["src/lib.rs", "assets/icon.bin"],
            "diff_hash": sha256(patch), "exit_code": 0, "observed_actions": [],
            "policy_violations": [], "cleanup_succeeded": true
        },
        "evidence_warnings": [],
        "risk_classification": "review_required",
        "limitations": [],
        "applyable": true
    }))
    .expect("manifest")
}

fn request() -> DelegationChangeSetReviewRequest {
    DelegationChangeSetReviewRequest {
        artifact_id: "artifact-1".into(),
        file_offset: 0,
        file_limit: 1,
        diff_offset: 0,
        diff_limit: 5,
    }
}

#[test]
fn pages_complete_file_and_utf8_diff_evidence_with_binary_notices() {
    let bytes = manifest("é-change\n".as_bytes());
    let reviewer =
        DelegationChangeSetReviewer::new(Arc::new(Artifact(bytes.clone(), sha256(&bytes))));

    let page = reviewer.review(request()).expect("review");

    assert_eq!(page.file_count, 2);
    assert_eq!(page.binary_file_count, 1);
    assert_eq!(page.files.len(), 1);
    assert_eq!(page.next_file_offset, Some(1));
    assert_eq!(page.diff_encoding, DelegationDiffEncoding::Utf8);
    assert_eq!(page.diff_data, "é-ch");
    assert_eq!(page.next_diff_offset, Some(5));
    assert!(page.integrity_verified);
    assert!(!page.complete_page);
}

#[test]
fn returns_exact_base64_for_non_utf8_diff_and_rejects_tampering() {
    let bytes = manifest(&[0, 255, 1]);
    let mut complete = request();
    complete.file_limit = 2;
    complete.diff_limit = 3;
    let reviewer =
        DelegationChangeSetReviewer::new(Arc::new(Artifact(bytes.clone(), sha256(&bytes))));
    let page = reviewer.review(complete).expect("review");
    assert_eq!(page.diff_encoding, DelegationDiffEncoding::Base64);
    assert_eq!(page.diff_data, "AP8B");
    assert!(page.complete_page);

    let tampered = DelegationChangeSetReviewer::new(Arc::new(Artifact(
        bytes,
        format!("sha256:{}", "0".repeat(64)),
    )));
    assert!(matches!(
        tampered.review(request()),
        Err(DelegationChangeSetReviewError::IntegrityFailure)
    ));
}
