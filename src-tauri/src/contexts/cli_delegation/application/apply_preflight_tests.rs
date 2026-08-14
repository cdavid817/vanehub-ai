use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationChangeFile, DelegationChangeKind, DelegationChangeSetCapture,
};

struct ArtifactPort {
    integrity: bool,
}

impl DelegationApplyArtifactPort for ArtifactPort {
    fn load_apply_evidence(
        &self,
        artifact_id: &str,
    ) -> Result<DelegationApplyArtifactEvidence, ()> {
        Ok(DelegationApplyArtifactEvidence {
            artifact_id: artifact_id.into(),
            content_hash: hash('a'),
            repository_identity: "repository-1".into(),
            capture: capture(),
            applyable: true,
            integrity_verified: self.integrity,
        })
    }
}

struct TargetPort {
    head: String,
    clean: bool,
}

impl DelegationApplyTargetPort for TargetPort {
    fn inspect_target(&self, root: &Path) -> Result<DelegationApplyTargetWitness, ()> {
        Ok(DelegationApplyTargetWitness {
            canonical_root: root.into(),
            repository_identity: "repository-1".into(),
            head_commit: self.head.clone(),
            worktree_clean: self.clean,
            index_clean: self.clean,
            path_compatible: true,
        })
    }
}

struct OncePort(bool);

impl DelegationApplyOncePort for OncePort {
    fn is_available(&self, _: &str, _: &str) -> Result<bool, ()> {
        Ok(self.0)
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
            after_mode: Some("100644".into()),
            before_git_hash: Some("before".into()),
            after_git_hash: Some("after".into()),
            binary: false,
        }],
        canonical_patch: b"patch".to_vec(),
        diff_hash: hash('b'),
    }
}

fn request() -> DelegationApplyPreflightRequest {
    DelegationApplyPreflightRequest {
        target_root: PathBuf::from("C:/repo"),
        artifact_id: "artifact-1".into(),
        expected_content_hash: hash('a'),
        expected_diff_hash: hash('b'),
        expected_repository_identity: "repository-1".into(),
        expected_base_commit: "c".repeat(40),
        approval_input_hash: hash('d'),
    }
}

fn service(
    integrity: bool,
    head: String,
    clean: bool,
    available: bool,
) -> DelegationApplyPreflightService {
    DelegationApplyPreflightService::new(
        Arc::new(ArtifactPort { integrity }),
        Arc::new(TargetPort { head, clean }),
        Arc::new(OncePort(available)),
    )
}

#[test]
fn admits_only_exact_integral_clean_and_unconsumed_target() {
    let plan = service(true, "c".repeat(40), true, true)
        .preflight(request())
        .expect("plan");
    assert_eq!(plan.target_root, PathBuf::from("C:/repo"));
    assert_eq!(plan.artifact.capture.diff_hash, hash('b'));
    assert_eq!(plan.approval_input_hash, hash('d'));
}

#[test]
fn rejects_tampering_stale_base_dirty_target_and_replay_before_mutation() {
    assert!(matches!(
        service(false, "c".repeat(40), true, true).preflight(request()),
        Err(DelegationApplyPreflightError::IntegrityFailure)
    ));
    assert!(matches!(
        service(true, "e".repeat(40), true, true).preflight(request()),
        Err(DelegationApplyPreflightError::StaleBase)
    ));
    assert!(matches!(
        service(true, "c".repeat(40), false, true).preflight(request()),
        Err(DelegationApplyPreflightError::DirtyTarget)
    ));
    assert!(matches!(
        service(true, "c".repeat(40), true, false).preflight(request()),
        Err(DelegationApplyPreflightError::ApprovalConsumed)
    ));
}
