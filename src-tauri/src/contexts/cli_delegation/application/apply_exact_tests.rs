use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationApplyArtifactEvidence, DelegationChangeFile, DelegationChangeKind,
    DelegationChangeSetCapture,
};
use std::path::PathBuf;

struct Port(DelegationExactApplyWitness);

impl DelegationExactApplyPort for Port {
    fn apply_exact(
        &self,
        _: DelegationExactApplyRequest<'_>,
    ) -> Result<DelegationExactApplyWitness, ()> {
        Ok(self.0.clone())
    }
}

fn hash(character: char) -> String {
    format!("sha256:{}", character.to_string().repeat(64))
}

fn plan() -> DelegationApplyPlan {
    DelegationApplyPlan {
        target_root: PathBuf::from("C:/repo"),
        artifact: DelegationApplyArtifactEvidence {
            artifact_id: "artifact-1".into(),
            content_hash: hash('a'),
            repository_identity: "repository-1".into(),
            capture: DelegationChangeSetCapture {
                base_commit: "c".repeat(40),
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
                canonical_patch: b"patch".to_vec(),
                diff_hash: hash('b'),
            },
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

fn witness() -> DelegationExactApplyWitness {
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
fn accepts_only_complete_exact_unstaged_offline_application() {
    let result = DelegationExactApplyService::new(Arc::new(Port(witness())))
        .apply(&plan(), &capsule())
        .expect("exact apply");
    assert_eq!(result.applied_diff_hash, hash('b'));
}

#[test]
fn rejects_partial_history_network_index_and_hash_failures() {
    let mut cases = Vec::new();
    let mut partial = witness();
    partial.partial_success = true;
    cases.push((partial, DelegationExactApplyError::IncompleteApplication));
    let mut network = witness();
    network.network_used = true;
    cases.push((network, DelegationExactApplyError::ForbiddenSideEffect));
    let mut history = witness();
    history.history_operation_used = true;
    cases.push((history, DelegationExactApplyError::ForbiddenSideEffect));
    let mut index = witness();
    index.index_unchanged = false;
    cases.push((index, DelegationExactApplyError::ForbiddenSideEffect));
    let mut hash_mismatch = witness();
    hash_mismatch.applied_diff_hash = hash('f');
    cases.push((hash_mismatch, DelegationExactApplyError::IntegrityFailure));

    for (witness, expected) in cases {
        assert_eq!(
            DelegationExactApplyService::new(Arc::new(Port(witness)))
                .apply(&plan(), &capsule())
                .expect_err("rejected"),
            expected
        );
    }
}
