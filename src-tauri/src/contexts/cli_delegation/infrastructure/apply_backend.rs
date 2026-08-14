use super::apply_preflight_backend::RecoveryEntry;
use super::GitDelegationChangeSetCapture;
use crate::contexts::agent_runtime::application::{
    ChangeSetApplyRecord, ChangeSetStatus, RecoveryRecord, RecoveryStatus,
};
use crate::contexts::agent_runtime::infrastructure::SqliteNativeToolRepository;
use crate::contexts::cli_delegation::application::{
    DelegationApplyPlan, DelegationApplyRecoveryPort, DelegationChangeSetCapturePort,
    DelegationExactApplyPort, DelegationExactApplyRequest, DelegationExactApplyWitness,
    DelegationPostApplyVerificationPort, DelegationPostApplyWitness, DelegationRecoveryCapsule,
    DelegationRecoveryOutcome,
};
use crate::platform::git::GitAdapter;
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const GIT_TIMEOUT: Duration = Duration::from_secs(30);

pub(super) struct NativeApplyBackend {
    pub(super) recovery_root: PathBuf,
    pub(super) repository: Arc<SqliteNativeToolRepository>,
}

impl NativeApplyBackend {
    pub(super) fn new(
        recovery_root: PathBuf,
        repository: Arc<SqliteNativeToolRepository>,
    ) -> Result<Self, String> {
        fs::create_dir_all(&recovery_root)
            .map_err(|_| "apply recovery is unavailable".to_owned())?;
        Ok(Self {
            recovery_root,
            repository,
        })
    }

    pub(super) fn capsule_root(&self, id: &str) -> PathBuf {
        self.recovery_root.join(id)
    }

    pub(super) fn run(&self, root: &Path, args: &[&str]) -> Result<Vec<u8>, ()> {
        let output = GitAdapter::default()
            .execute(
                root,
                &args
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect::<Vec<_>>(),
                GIT_TIMEOUT,
            )
            .map_err(|_| ())?;
        output.status.success().then_some(output.stdout).ok_or(())
    }
}
impl DelegationExactApplyPort for NativeApplyBackend {
    fn apply_exact(
        &self,
        request: DelegationExactApplyRequest<'_>,
    ) -> Result<DelegationExactApplyWitness, ()> {
        let root = self.capsule_root(&request.capsule.apply_attempt_id);
        let patch = root.join("changes.patch");
        fs::write(&patch, &request.plan.artifact.capture.canonical_patch).map_err(|_| ())?;
        let patch_arg = patch.to_str().ok_or(())?;
        self.run(
            &request.plan.target_root,
            &["apply", "--binary", "--whitespace=nowarn", "--", patch_arg],
        )?;
        let control = root.join("control");
        fs::create_dir_all(&control).map_err(|_| ())?;
        let capture = GitDelegationChangeSetCapture::new()
            .capture(
                &request.plan.target_root,
                &control,
                &request.plan.artifact.capture.base_commit,
            )
            .map_err(|_| ())?;
        let complete_patch_applied = capture == request.plan.artifact.capture;
        Ok(DelegationExactApplyWitness {
            applied_diff_hash: capture.diff_hash,
            complete_patch_applied,
            index_unchanged: self
                .run(&request.plan.target_root, &["diff", "--cached", "--quiet"])
                .is_ok(),
            network_used: false,
            history_operation_used: false,
            partial_success: false,
        })
    }
}

impl DelegationPostApplyVerificationPort for NativeApplyBackend {
    fn capture_post_apply(
        &self,
        plan: &DelegationApplyPlan,
    ) -> Result<DelegationPostApplyWitness, ()> {
        let control = self.recovery_root.join("verification-control");
        fs::create_dir_all(&control).map_err(|_| ())?;
        let capture = GitDelegationChangeSetCapture::new()
            .capture(
                &plan.target_root,
                &control,
                &plan.artifact.capture.base_commit,
            )
            .map_err(|_| ())?;
        let head = String::from_utf8(self.run(&plan.target_root, &["rev-parse", "HEAD"])?)
            .map_err(|_| ())?;
        Ok(DelegationPostApplyWitness {
            capture,
            head_commit: head.trim().to_owned(),
            index_clean: self
                .run(&plan.target_root, &["diff", "--cached", "--quiet"])
                .is_ok(),
            mutation_lease_held: true,
        })
    }

    fn record_success_and_consume(
        &self,
        id: &str,
        artifact_id: &str,
        approval_input_hash: &str,
        _: &str,
    ) -> Result<bool, ()> {
        if !self
            .repository
            .is_change_set_available(artifact_id)
            .map_err(|_| ())?
        {
            return Ok(false);
        }
        let now = Utc::now().to_rfc3339();
        self.repository
            .save_apply_attempt(&ChangeSetApplyRecord {
                contract_version: 1,
                id: id.to_owned(),
                change_set_artifact_id: artifact_id.to_owned(),
                target_repository_identity: "verified-target".to_owned(),
                expected_base_commit: "verified-base".to_owned(),
                approval_input_hash: approval_input_hash.to_owned(),
                status: ChangeSetStatus::Succeeded,
                error_code: None,
                consumed_at: Some(now.clone()),
                created_at: now.clone(),
                updated_at: now,
            })
            .map_err(|_| ())?;
        Ok(true)
    }
}

impl DelegationApplyRecoveryPort for NativeApplyBackend {
    fn restore_from_capsule(
        &self,
        plan: &DelegationApplyPlan,
        capsule: &DelegationRecoveryCapsule,
    ) -> Result<(), ()> {
        let root = PathBuf::from(&capsule.reference);
        let bytes = fs::read(root.join("manifest.json")).map_err(|_| ())?;
        if sha256(&bytes) != capsule.witness_hash {
            return Err(());
        }
        let entries: Vec<RecoveryEntry> = serde_json::from_slice(&bytes).map_err(|_| ())?;
        for entry in entries {
            let target = plan.target_root.join(&entry.path);
            if target.exists() {
                fs::remove_file(&target).map_err(|_| ())?;
            }
            if entry.existed {
                if let Some(parent) = target.parent() {
                    fs::create_dir_all(parent).map_err(|_| ())?;
                }
                fs::copy(root.join("files").join(&entry.path), target).map_err(|_| ())?;
            }
        }
        Ok(())
    }

    fn verify_pre_apply_witness(
        &self,
        plan: &DelegationApplyPlan,
        _: &DelegationRecoveryCapsule,
    ) -> Result<bool, ()> {
        Ok(self
            .run(
                &plan.target_root,
                &["status", "--porcelain=v1", "--untracked-files=all"],
            )?
            .is_empty())
    }

    fn persist_recovery(
        &self,
        id: &str,
        outcome: DelegationRecoveryOutcome,
        reference: Option<&str>,
    ) -> Result<(), ()> {
        let status = match outcome {
            DelegationRecoveryOutcome::RolledBack => RecoveryStatus::RolledBack,
            DelegationRecoveryOutcome::ManualRecoveryRequired => {
                RecoveryStatus::ManualRecoveryRequired
            }
        };
        self.repository
            .save_recovery(&RecoveryRecord {
                contract_version: 1,
                apply_attempt_id: id.to_owned(),
                status,
                recovery_reference: reference.map(str::to_owned),
                safe_instructions: vec!["Review the affected files before continuing.".to_owned()],
                updated_at: Utc::now().to_rfc3339(),
            })
            .map_err(|_| ())
    }

    fn remove_capsule(&self, capsule: &DelegationRecoveryCapsule) -> Result<(), ()> {
        fs::remove_dir_all(&capsule.reference).map_err(|_| ())
    }
}

fn sha256(bytes: &[u8]) -> String {
    let hex = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("sha256:{hex}")
}
