use super::*;
use crate::contexts::cli_delegation::application::{
    DelegationApplyArtifactEvidence, DelegationChangeFile, DelegationChangeKind,
    DelegationChangeSetCapture,
};
use std::path::PathBuf;
use std::sync::Mutex;

struct Port {
    restore: bool,
    verify: bool,
    persisted: Mutex<Vec<(DelegationRecoveryOutcome, Option<String>)>>,
    removed: Mutex<bool>,
}

impl DelegationApplyRecoveryPort for Port {
    fn restore_from_capsule(
        &self,
        _: &DelegationApplyPlan,
        _: &DelegationRecoveryCapsule,
    ) -> Result<(), ()> {
        self.restore.then_some(()).ok_or(())
    }

    fn verify_pre_apply_witness(
        &self,
        _: &DelegationApplyPlan,
        _: &DelegationRecoveryCapsule,
    ) -> Result<bool, ()> {
        Ok(self.verify)
    }

    fn persist_recovery(
        &self,
        _: &str,
        outcome: DelegationRecoveryOutcome,
        reference: Option<&str>,
    ) -> Result<(), ()> {
        self.persisted
            .lock()
            .map_err(|_| ())?
            .push((outcome, reference.map(str::to_owned)));
        Ok(())
    }

    fn remove_capsule(&self, _: &DelegationRecoveryCapsule) -> Result<(), ()> {
        *self.removed.lock().map_err(|_| ())? = true;
        Ok(())
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

#[test]
fn verified_rollback_is_persisted_before_capsule_removal() {
    let port = Arc::new(Port {
        restore: true,
        verify: true,
        persisted: Mutex::new(Vec::new()),
        removed: Mutex::new(false),
    });
    let outcome = DelegationApplyRecoveryService::new(port.clone())
        .recover(&plan(), &capsule())
        .expect("recovered");
    assert_eq!(outcome, DelegationRecoveryOutcome::RolledBack);
    assert_eq!(port.persisted.lock().expect("persisted")[0].1, None);
    assert!(*port.removed.lock().expect("removed"));
}

#[test]
fn unproven_restore_retains_capsule_and_requires_manual_recovery() {
    let port = Arc::new(Port {
        restore: false,
        verify: false,
        persisted: Mutex::new(Vec::new()),
        removed: Mutex::new(false),
    });
    let outcome = DelegationApplyRecoveryService::new(port.clone())
        .recover(&plan(), &capsule())
        .expect("recorded");
    assert_eq!(outcome, DelegationRecoveryOutcome::ManualRecoveryRequired);
    assert_eq!(
        port.persisted.lock().expect("persisted")[0].1.as_deref(),
        Some("recovery/apply-1")
    );
    assert!(!*port.removed.lock().expect("removed"));
}
