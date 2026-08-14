use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationApplyArtifactEvidence, DelegationApplyPlan, DelegationChangeFile,
    DelegationChangeKind, DelegationChangeSetCapture,
};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

struct Port {
    mismatch: bool,
    staged: AtomicBool,
}

impl DelegationApplyStagingPort for Port {
    fn inspect_paths(
        &self,
        _: &DelegationApplyPlan,
        expectations: &[DelegationApplyPathExpectation],
    ) -> Result<Vec<DelegationApplyPathWitness>, ()> {
        Ok(expectations
            .iter()
            .map(|item| DelegationApplyPathWitness {
                path: item.path.clone(),
                exists: if self.mismatch {
                    !item.must_exist
                } else {
                    item.must_exist
                },
                mode: item.expected_mode.clone(),
                git_hash: item.expected_git_hash.clone(),
            })
            .collect())
    }

    fn stage_recovery_capsule(
        &self,
        _: &DelegationApplyPlan,
        apply_attempt_id: &str,
        _: &[DelegationApplyPathExpectation],
    ) -> Result<DelegationRecoveryCapsule, ()> {
        self.staged.store(true, Ordering::Release);
        Ok(DelegationRecoveryCapsule {
            apply_attempt_id: apply_attempt_id.into(),
            reference: "recovery/apply-1".into(),
            witness_hash: format!("sha256:{}", "a".repeat(64)),
        })
    }
}

fn file(kind: DelegationChangeKind, path: &str, previous: Option<&str>) -> DelegationChangeFile {
    let added = kind == DelegationChangeKind::Added;
    let deleted = kind == DelegationChangeKind::Deleted;
    DelegationChangeFile {
        path: path.into(),
        previous_path: previous.map(str::to_owned),
        kind,
        before_mode: (!added).then(|| "100644".into()),
        after_mode: (!deleted).then(|| "100644".into()),
        before_git_hash: (!added).then(|| "before".into()),
        after_git_hash: (!deleted).then(|| "after".into()),
        binary: false,
    }
}

fn plan() -> DelegationApplyPlan {
    DelegationApplyPlan {
        target_root: PathBuf::from("C:/repo"),
        artifact: DelegationApplyArtifactEvidence {
            artifact_id: "artifact-1".into(),
            content_hash: format!("sha256:{}", "a".repeat(64)),
            repository_identity: "repository-1".into(),
            capture: DelegationChangeSetCapture {
                base_commit: "c".repeat(40),
                files: vec![
                    file(DelegationChangeKind::Added, "add.txt", None),
                    file(DelegationChangeKind::Modified, "modify.txt", None),
                    file(DelegationChangeKind::Deleted, "delete.txt", None),
                    file(
                        DelegationChangeKind::Renamed,
                        "renamed.txt",
                        Some("old.txt"),
                    ),
                ],
                canonical_patch: b"patch".to_vec(),
                diff_hash: format!("sha256:{}", "b".repeat(64)),
            },
            applyable: true,
            integrity_verified: true,
        },
        approval_input_hash: format!("sha256:{}", "d".repeat(64)),
    }
}

#[test]
fn verifies_all_operation_paths_before_sealing_recovery_capsule() {
    let port = Arc::new(Port {
        mismatch: false,
        staged: AtomicBool::new(false),
    });
    let capsule = DelegationApplyStagingService::new(port.clone())
        .stage(&plan(), "apply-1")
        .expect("capsule");
    assert_eq!(capsule.reference, "recovery/apply-1");
    assert!(port.staged.load(Ordering::Acquire));
}

#[test]
fn concurrent_path_mutation_prevents_capsule_and_apply_admission() {
    let port = Arc::new(Port {
        mismatch: true,
        staged: AtomicBool::new(false),
    });
    assert!(matches!(
        DelegationApplyStagingService::new(port.clone()).stage(&plan(), "apply-1"),
        Err(DelegationApplyStagingError::ConcurrentMutation)
    ));
    assert!(!port.staged.load(Ordering::Acquire));
}
