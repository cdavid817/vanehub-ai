use crate::contexts::tooling::skills::application::{
    SkillApplicationError, SkillFilesystemTransaction,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

#[derive(Default)]
pub(super) struct FileTransactions {
    next_id: AtomicU64,
    next_backup: AtomicU64,
    journals: Mutex<BTreeMap<String, Vec<UndoAction>>>,
}

impl FileTransactions {
    pub(super) fn begin(&self) -> SkillFilesystemTransaction {
        let sequence = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let transaction = SkillFilesystemTransaction {
            id: format!("skill-fs-{}-{sequence}", std::process::id()),
        };
        self.journals
            .lock()
            .expect("Skill filesystem transaction journals")
            .insert(transaction.id.clone(), Vec::new());
        transaction
    }

    pub(super) fn checkpoint(
        &self,
        transaction: &SkillFilesystemTransaction,
    ) -> Result<usize, SkillApplicationError> {
        self.journals
            .lock()
            .map_err(lock_error)?
            .get(&transaction.id)
            .map(Vec::len)
            .ok_or_else(|| missing_transaction(transaction))
    }

    pub(super) fn stage_replace_or_create(
        &self,
        transaction: &SkillFilesystemTransaction,
        target: &Path,
    ) -> Result<(), SkillApplicationError> {
        if path_exists(target) {
            let backup = self.transient_backup(target, transaction)?;
            std::fs::rename(target, &backup).map_err(filesystem_error)?;
            self.push(
                transaction,
                UndoAction::Replaced {
                    target: target.to_path_buf(),
                    backup,
                    retain_backup_on_commit: false,
                },
            )
        } else {
            self.push(transaction, UndoAction::Created(target.to_path_buf()))
        }
    }

    pub(super) fn stage_permanent_replacement(
        &self,
        transaction: &SkillFilesystemTransaction,
        target: &Path,
        backup: &Path,
    ) -> Result<(), SkillApplicationError> {
        if let Some(parent) = backup.parent() {
            std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        }
        if path_exists(backup) {
            return Err(SkillApplicationError::Filesystem(format!(
                "Skill backup path already exists: {}",
                backup.display()
            )));
        }
        std::fs::rename(target, backup).map_err(filesystem_error)?;
        self.push(
            transaction,
            UndoAction::Replaced {
                target: target.to_path_buf(),
                backup: backup.to_path_buf(),
                retain_backup_on_commit: true,
            },
        )
    }

    pub(super) fn stage_remove(
        &self,
        transaction: &SkillFilesystemTransaction,
        target: &Path,
    ) -> Result<bool, SkillApplicationError> {
        if !path_exists(target) {
            return Ok(false);
        }
        let backup = self.transient_backup(target, transaction)?;
        std::fs::rename(target, &backup).map_err(filesystem_error)?;
        self.push(
            transaction,
            UndoAction::Removed {
                target: target.to_path_buf(),
                backup,
            },
        )?;
        Ok(true)
    }

    pub(super) fn rollback_to(&self, transaction: &SkillFilesystemTransaction, checkpoint: usize) {
        let actions = self
            .journals
            .lock()
            .ok()
            .and_then(|mut journals| {
                let journal = journals.get_mut(&transaction.id)?;
                (checkpoint <= journal.len()).then(|| journal.drain(checkpoint..).collect())
            })
            .unwrap_or_default();
        rollback_actions(actions);
    }

    pub(super) fn commit(&self, transaction: SkillFilesystemTransaction) {
        let actions = self
            .journals
            .lock()
            .ok()
            .and_then(|mut journals| journals.remove(&transaction.id))
            .unwrap_or_default();
        for action in actions {
            match action {
                UndoAction::Replaced {
                    backup,
                    retain_backup_on_commit: false,
                    ..
                }
                | UndoAction::Removed { backup, .. } => {
                    let _ = remove_path(&backup);
                }
                UndoAction::Created(_)
                | UndoAction::Replaced {
                    retain_backup_on_commit: true,
                    ..
                } => {}
            }
        }
    }

    pub(super) fn rollback(&self, transaction: SkillFilesystemTransaction) {
        let actions = self
            .journals
            .lock()
            .ok()
            .and_then(|mut journals| journals.remove(&transaction.id))
            .unwrap_or_default();
        rollback_actions(actions);
    }

    fn push(
        &self,
        transaction: &SkillFilesystemTransaction,
        action: UndoAction,
    ) -> Result<(), SkillApplicationError> {
        self.journals
            .lock()
            .map_err(lock_error)?
            .get_mut(&transaction.id)
            .ok_or_else(|| missing_transaction(transaction))?
            .push(action);
        Ok(())
    }

    fn transient_backup(
        &self,
        target: &Path,
        transaction: &SkillFilesystemTransaction,
    ) -> Result<PathBuf, SkillApplicationError> {
        let parent = target.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Skill path has no parent".to_string())
        })?;
        let name = target
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("skill");
        loop {
            let sequence = self.next_backup.fetch_add(1, Ordering::Relaxed) + 1;
            let candidate = parent.join(format!(
                ".{name}.vanehub-transaction-{}-{sequence}",
                transaction.id
            ));
            if !path_exists(&candidate) {
                return Ok(candidate);
            }
        }
    }
}

enum UndoAction {
    Created(PathBuf),
    Replaced {
        target: PathBuf,
        backup: PathBuf,
        retain_backup_on_commit: bool,
    },
    Removed {
        target: PathBuf,
        backup: PathBuf,
    },
}

fn rollback_actions(actions: Vec<UndoAction>) {
    for action in actions.into_iter().rev() {
        match action {
            UndoAction::Created(target) => {
                let _ = remove_path(&target);
            }
            UndoAction::Replaced { target, backup, .. } => {
                let _ = remove_path(&target);
                let _ = std::fs::rename(backup, target);
            }
            UndoAction::Removed { target, backup } => {
                let _ = std::fs::rename(backup, target);
            }
        }
    }
}

pub(super) fn path_exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

pub(super) fn remove_path(path: &Path) -> std::io::Result<()> {
    let metadata = match std::fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.file_type().is_symlink() {
        std::fs::remove_file(path).or_else(|_| std::fs::remove_dir(path))
    } else if metadata.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

fn missing_transaction(transaction: &SkillFilesystemTransaction) -> SkillApplicationError {
    SkillApplicationError::Filesystem(format!(
        "Unknown Skill filesystem transaction: {}",
        transaction.id
    ))
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

fn lock_error(error: std::sync::PoisonError<impl Sized>) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
