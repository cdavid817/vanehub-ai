use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationApplyArtifactEvidence, DelegationChangeFile, DelegationChangeKind,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

struct Port {
    capture: DelegationChangeSetCapture,
    lease: bool,
    consumed: bool,
    completion_called: AtomicBool,
}

impl DelegationPostApplyVerificationPort for Port {
    fn capture_post_apply(
        &self,
        _: &DelegationApplyPlan,
    ) -> Result<DelegationPostApplyWitness, ()> {
        Ok(DelegationPostApplyWitness {
            capture: self.capture.clone(),
            head_commit: "c".repeat(40),
            index_clean: true,
            mutation_lease_held: self.lease,
        })
    }

    fn record_success_and_consume(&self, _: &str, _: &str, _: &str, _: &str) -> Result<bool, ()> {
        self.completion_called.store(true, Ordering::Release);
        Ok(self.consumed)
    }
}

fn hash(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn capture() -> DelegationChangeSetCapture {
    DelegationChangeSetCapture {
        base_commit: "c".repeat(40),
        files: vec![DelegationChangeFile {
            path: "src/lib.rs".into(),
            previous_path: None,
            kind: DelegationChangeKind::Modified,
            before_mode: Some("100644".into()),
            after_mode: Some("100755".into()),
            before_git_hash: Some("before".into()),
            after_git_hash: Some("after".into()),
            binary: false,
        }],
        canonical_patch: b"patch".to_vec(),
        diff_hash: hash('b'),
    }
}

fn plan() -> DelegationApplyPlan {
    DelegationApplyPlan {
        target_root: PathBuf::from("C:/repo"),
        artifact: DelegationApplyArtifactEvidence {
            artifact_id: "artifact-1".into(),
            content_hash: hash('a'),
            repository_identity: "repository-1".into(),
            capture: capture(),
            applyable: true,
            integrity_verified: true,
        },
        approval_input_hash: hash('d'),
    }
}

fn capsule() -> DelegationRecoveryCapsule {
    DelegationRecoveryCapsule {
        apply_attempt_id: "apply-1".into(),
        reference: "recovery/apply-1".into(),
        witness_hash: hash('e'),
    }
}

fn application() -> DelegationExactApplyWitness {
    DelegationExactApplyWitness {
        applied_diff_hash: hash('b'),
        complete_patch_applied: true,
        index_unchanged: true,
        network_used: false,
        history_operation_used: false,
        partial_success: false,
    }
}

#[test]
fn consumes_once_only_capability_after_complete_tree_and_metadata_match() {
    let port = Arc::new(Port {
        capture: capture(),
        lease: true,
        consumed: true,
        completion_called: AtomicBool::new(false),
    });
    DelegationPostApplyVerificationService::new(port.clone())
        .verify_and_complete(&plan(), &capsule(), &application())
        .expect("complete");
    assert!(port.completion_called.load(Ordering::Acquire));
}

#[test]
fn mismatch_or_lease_loss_never_consumes_approval_as_success() {
    let mut mismatch = capture();
    mismatch.files[0].after_mode = Some("100644".into());
    for (actual, lease, expected) in [
        (
            mismatch,
            true,
            DelegationPostApplyVerificationError::TreeMismatch,
        ),
        (
            capture(),
            false,
            DelegationPostApplyVerificationError::LeaseLost,
        ),
    ] {
        let port = Arc::new(Port {
            capture: actual,
            lease,
            consumed: true,
            completion_called: AtomicBool::new(false),
        });
        assert_eq!(
            DelegationPostApplyVerificationService::new(port.clone())
                .verify_and_complete(&plan(), &capsule(), &application())
                .expect_err("rejected"),
            expected
        );
        assert!(!port.completion_called.load(Ordering::Acquire));
    }
}
