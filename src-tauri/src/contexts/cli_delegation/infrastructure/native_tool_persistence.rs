use crate::contexts::agent_runtime::api::{
    ChangeSetFileRecord, ChangeSetRecord, DelegationAttemptRecord, DelegationMode as StoredMode,
    DelegationRecord, DelegationStatus as StoredStatus, DelegationTarget as StoredTarget,
    FileChangeKind, NativeToolPersistencePort,
};
use crate::contexts::cli_delegation::application::{
    DelegationChangeKind, DelegationChangeSetCapture, DelegationMode,
};
use std::sync::Arc;

pub(super) struct DelegationPersistence {
    repository: Arc<dyn NativeToolPersistencePort>,
}

pub(super) struct AttemptUpdate<'a> {
    pub(super) session_id: &'a str,
    pub(super) delegation_id: &'a str,
    pub(super) attempt_id: &'a str,
    pub(super) mode: DelegationMode,
    pub(super) task_hash: &'a str,
    pub(super) status: StoredStatus,
    pub(super) created_at: &'a str,
    pub(super) updated_at: &'a str,
    pub(super) summary: Option<&'a str>,
    pub(super) report_artifact_id: Option<&'a str>,
    pub(super) change_set_artifact_id: Option<&'a str>,
    pub(super) error_code: Option<&'a str>,
}

impl DelegationPersistence {
    pub(super) fn new(repository: Arc<dyn NativeToolPersistencePort>) -> Self {
        Self { repository }
    }

    pub(super) fn save_attempt(&self, update: AttemptUpdate<'_>) -> Result<(), &'static str> {
        self.repository
            .save_delegation(&DelegationRecord {
                contract_version: 1,
                id: update.delegation_id.to_owned(),
                session_id: update.session_id.to_owned(),
                task_hash: update.task_hash.to_owned(),
                status: update.status,
                created_at: update.created_at.to_owned(),
                updated_at: update.updated_at.to_owned(),
            })
            .map_err(|_| "storage_failure")?;
        self.repository
            .save_delegation_attempt(&DelegationAttemptRecord {
                contract_version: 1,
                id: update.attempt_id.to_owned(),
                delegation_id: update.delegation_id.to_owned(),
                attempt_number: 1,
                target: StoredTarget::ClaudeCode,
                mode: stored_mode(update.mode),
                status: update.status,
                safe_summary: update.summary.map(str::to_owned),
                report_artifact_id: update.report_artifact_id.map(str::to_owned),
                change_set_artifact_id: update.change_set_artifact_id.map(str::to_owned),
                error_code: update.error_code.map(str::to_owned),
                started_at: Some(update.created_at.to_owned()),
                completed_at: matches!(
                    update.status,
                    StoredStatus::Succeeded | StoredStatus::Failed | StoredStatus::Cancelled
                )
                .then(|| update.updated_at.to_owned()),
            })
            .map_err(|_| "storage_failure")
    }

    pub(super) fn insert_change_set(
        &self,
        artifact_id: &str,
        content_hash: &str,
        repository_identity: &str,
        attempt_id: &str,
        capture: &DelegationChangeSetCapture,
        created_at: &str,
    ) -> Result<(), &'static str> {
        self.repository
            .insert_change_set(&ChangeSetRecord {
                contract_version: 1,
                artifact_id: artifact_id.to_owned(),
                content_hash: content_hash.to_owned(),
                repository_identity: repository_identity.to_owned(),
                base_commit: capture.base_commit.clone(),
                attempt_id: attempt_id.to_owned(),
                files: capture.files.iter().map(stored_file).collect(),
                warnings: Vec::new(),
                created_at: created_at.to_owned(),
            })
            .map_err(|_| "storage_failure")
    }
}

fn stored_mode(mode: DelegationMode) -> StoredMode {
    match mode {
        DelegationMode::Analyze => StoredMode::Analyze,
        DelegationMode::Edit => StoredMode::Edit,
    }
}

fn stored_file(
    file: &crate::contexts::cli_delegation::application::DelegationChangeFile,
) -> ChangeSetFileRecord {
    ChangeSetFileRecord {
        path: file.path.clone(),
        change_kind: match file.kind {
            DelegationChangeKind::Added => FileChangeKind::Add,
            DelegationChangeKind::Deleted => FileChangeKind::Delete,
            DelegationChangeKind::Renamed => FileChangeKind::Rename,
            DelegationChangeKind::Modified | DelegationChangeKind::TypeChanged => {
                FileChangeKind::Modify
            }
        },
        old_hash: file.before_git_hash.clone(),
        new_hash: file.after_git_hash.clone(),
        binary: file.binary,
        mode: file.after_mode.clone().or_else(|| file.before_mode.clone()),
    }
}
