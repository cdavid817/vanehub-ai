use super::transaction::{path_exists, FileTransactions};
use crate::contexts::tooling::skills::application::{
    OverlayKey, OverlayPinSnapshot, OverlayPinStatePort, OverlayUsageSnapshot,
    OverlayUsageStatePort, SkillApplicationError, SkillUsageActivity, SkillUsageIdentity,
    SkillUsageMutation, SkillUsageRead, SkillUsageRepository, SkillUsageSummary,
};
use crate::contexts::tooling::skills::domain::{OverlayScope, SkillId, SkillLocation, SkillScope};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const USAGE_SCHEMA_VERSION: u32 = 1;
const MAX_USAGE_BYTES: u64 = 1_048_576;
const MAX_BACKUPS: usize = 5;
const MAX_BACKUP_BYTES: u64 = 4_194_304;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UsageDocument {
    version: u32,
    revision: u64,
    records: BTreeMap<String, UsageRecord>,
}

impl Default for UsageDocument {
    fn default() -> Self {
        Self {
            version: USAGE_SCHEMA_VERSION,
            revision: 0,
            records: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UsageRecord {
    view_count: u64,
    use_count: u64,
    last_viewed_at: Option<String>,
    last_used_at: Option<String>,
    revision_witness: Option<String>,
    #[serde(default)]
    patch_count: u64,
    #[serde(default)]
    overlay_mutation_count: u64,
    #[serde(default)]
    last_patched_at: Option<String>,
    #[serde(default)]
    last_overlay_mutation_at: Option<String>,
    #[serde(default)]
    pinned: bool,
}

impl FilesystemSkillUsageRepository {
    fn overlay_sidecar_path(
        &self,
        workspace_identity: Option<&str>,
    ) -> Result<PathBuf, SkillApplicationError> {
        match workspace_identity {
            Some(workspace) => Ok(Path::new(workspace).join(".vanehub/skills/.usage.json")),
            None => Ok(self.home_root.join(".vanehub/skills/.usage.json")),
        }
    }

    fn overlay_record(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<(UsageDocument, UsageRecord), SkillApplicationError> {
        let path = self.overlay_sidecar_path(workspace_identity)?;
        let (document, _) = self.read_or_recover(&path)?;
        let record = document
            .records
            .get(&format!("overlay:{}", canonical_skill_id.as_str()))
            .cloned()
            .unwrap_or_default();
        Ok((document, record))
    }
}

impl OverlayPinStatePort for FilesystemSkillUsageRepository {
    fn pin_snapshot(
        &self,
        canonical_skill_id: &SkillId,
        workspace_identity: Option<&str>,
    ) -> Result<OverlayPinSnapshot, SkillApplicationError> {
        let _guard = self.writer.lock().map_err(lock_error)?;
        let (document, record) = self.overlay_record(canonical_skill_id, workspace_identity)?;
        Ok(OverlayPinSnapshot {
            pinned: record.pinned,
            revision_witness: format!("pin-{}-{}", document.revision, record.pinned),
        })
    }
}

impl OverlayUsageStatePort for FilesystemSkillUsageRepository {
    fn usage_snapshot(
        &self,
        key: &OverlayKey,
    ) -> Result<OverlayUsageSnapshot, SkillApplicationError> {
        let workspace = match key.scope {
            OverlayScope::Project => key.workspace_identity.as_deref(),
            OverlayScope::System | OverlayScope::User => None,
        };
        let _guard = self.writer.lock().map_err(lock_error)?;
        let (_, record) = self.overlay_record(&key.canonical_skill_id, workspace)?;
        Ok(OverlayUsageSnapshot {
            patch_count: record.patch_count,
            overlay_mutation_count: record.overlay_mutation_count,
            last_patched_at: record.last_patched_at,
            last_overlay_mutation_at: record.last_overlay_mutation_at,
            revision_witness: record
                .revision_witness
                .unwrap_or_else(|| "usage-0".to_string()),
        })
    }
}

impl From<&UsageRecord> for SkillUsageSummary {
    fn from(record: &UsageRecord) -> Self {
        Self {
            view_count: record.view_count,
            use_count: record.use_count,
            last_viewed_at: record.last_viewed_at.clone(),
            last_used_at: record.last_used_at.clone(),
            revision_witness: record.revision_witness.clone(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct FilesystemSkillUsageRepository {
    home_root: PathBuf,
    transactions: Arc<FileTransactions>,
    writer: Arc<Mutex<()>>,
    backup_sequence: Arc<AtomicU64>,
    #[cfg(test)]
    fail_before_replace: Arc<std::sync::atomic::AtomicBool>,
}

impl FilesystemSkillUsageRepository {
    pub(crate) fn new() -> Self {
        Self::with_home_root(super::default_home_root())
    }

    pub(crate) fn with_home_root(home_root: PathBuf) -> Self {
        Self {
            home_root,
            transactions: Arc::new(FileTransactions::default()),
            writer: Arc::new(Mutex::new(())),
            backup_sequence: Arc::new(AtomicU64::new(0)),
            #[cfg(test)]
            fail_before_replace: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn sidecar_path(&self, location: &SkillLocation) -> Result<PathBuf, SkillApplicationError> {
        match location.scope {
            SkillScope::Global => Ok(self.home_root.join(".vanehub/skills/.usage.json")),
            SkillScope::Workspace => location
                .workspace_path
                .as_deref()
                .map(|workspace| Path::new(workspace).join(".vanehub/skills/.usage.json"))
                .ok_or_else(|| {
                    SkillApplicationError::Validation(
                        "Workspace usage requires a workspace path".to_string(),
                    )
                }),
        }
    }

    fn read_or_recover(&self, path: &Path) -> Result<(UsageDocument, bool), SkillApplicationError> {
        if !path_exists(path) {
            return Ok((UsageDocument::default(), false));
        }
        let metadata = std::fs::metadata(path).map_err(filesystem_error)?;
        if metadata.len() > MAX_USAGE_BYTES {
            return self.recover_corrupt(path);
        }
        let bytes = std::fs::read(path).map_err(filesystem_error)?;
        match serde_json::from_slice::<UsageDocument>(&bytes) {
            Ok(document) if document.version == USAGE_SCHEMA_VERSION => Ok((document, false)),
            _ => self.recover_corrupt(path),
        }
    }

    fn recover_corrupt(&self, path: &Path) -> Result<(UsageDocument, bool), SkillApplicationError> {
        let backup_dir = path
            .parent()
            .ok_or_else(|| SkillApplicationError::Filesystem("Usage path has no parent".into()))?
            .join(".usage-backups");
        std::fs::create_dir_all(&backup_dir).map_err(filesystem_error)?;
        let sequence = self.backup_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let backup = backup_dir.join(format!("usage-{timestamp}-{sequence}.json"));
        std::fs::rename(path, &backup).map_err(filesystem_error)?;
        let empty = UsageDocument::default();
        if let Err(error) = self.write_document(path, &empty, None) {
            let _ = std::fs::rename(&backup, path);
            return Err(error);
        }
        self.prune_backups(&backup_dir)?;
        Ok((empty, true))
    }

    fn write_document(
        &self,
        path: &Path,
        document: &UsageDocument,
        expected_revision: Option<u64>,
    ) -> Result<(), SkillApplicationError> {
        let parent = path
            .parent()
            .ok_or_else(|| SkillApplicationError::Filesystem("Usage path has no parent".into()))?;
        std::fs::create_dir_all(parent).map_err(filesystem_error)?;
        if let Some(expected) = expected_revision {
            let current = std::fs::read(path).map_err(filesystem_error)?;
            let current: UsageDocument = serde_json::from_slice(&current).map_err(json_error)?;
            if current.revision != expected {
                return Err(SkillApplicationError::ConcurrentModification(
                    "usage-sidecar".to_string(),
                ));
            }
        }
        let serialized = serde_json::to_vec_pretty(document).map_err(json_error)?;
        if serialized.len() as u64 > MAX_USAGE_BYTES {
            return Err(SkillApplicationError::Filesystem(
                "Skill usage sidecar exceeds its size limit".to_string(),
            ));
        }
        let sequence = self.backup_sequence.fetch_add(1, Ordering::Relaxed) + 1;
        let temporary = parent.join(format!(".usage.tmp-{}-{sequence}", std::process::id()));
        std::fs::write(&temporary, serialized).map_err(filesystem_error)?;
        #[cfg(test)]
        if self
            .fail_before_replace
            .swap(false, std::sync::atomic::Ordering::SeqCst)
        {
            let _ = std::fs::remove_file(&temporary);
            return Err(SkillApplicationError::Filesystem(
                "injected usage write failure".to_string(),
            ));
        }
        // The temporary file is already on disk by this point, so a failure to open a
        // transaction has to clean it up the same way every other failure below does.
        let transaction = match self.transactions.begin() {
            Ok(transaction) => transaction,
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                return Err(error);
            }
        };
        if let Err(error) = self
            .transactions
            .stage_replace_or_create(&transaction, path)
        {
            let _ = std::fs::remove_file(&temporary);
            self.transactions.rollback(transaction);
            return Err(error);
        }
        match std::fs::rename(&temporary, path) {
            Ok(()) => {
                self.transactions.commit(transaction);
                Ok(())
            }
            Err(error) => {
                let _ = std::fs::remove_file(&temporary);
                self.transactions.rollback(transaction);
                Err(filesystem_error(error))
            }
        }
    }

    fn prune_backups(&self, directory: &Path) -> Result<(), SkillApplicationError> {
        let mut backups = std::fs::read_dir(directory)
            .map_err(filesystem_error)?
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let metadata = entry.metadata().ok()?;
                metadata.is_file().then_some((entry.path(), metadata.len()))
            })
            .collect::<Vec<_>>();
        backups.sort_by(|left, right| left.0.cmp(&right.0));
        let mut total = backups.iter().map(|(_, size)| *size).sum::<u64>();
        while backups.len() > MAX_BACKUPS || total > MAX_BACKUP_BYTES {
            let (path, size) = backups.remove(0);
            std::fs::remove_file(path).map_err(filesystem_error)?;
            total = total.saturating_sub(size);
        }
        Ok(())
    }

    #[cfg(test)]
    fn inject_write_failure(&self) {
        self.fail_before_replace
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }
}

impl SkillUsageRepository for FilesystemSkillUsageRepository {
    fn summaries(
        &self,
        location: &SkillLocation,
        identities: &[SkillUsageIdentity],
    ) -> Result<SkillUsageRead, SkillApplicationError> {
        let _guard = self.writer.lock().map_err(lock_error)?;
        let (document, recovered_corrupt_state) =
            self.read_or_recover(&self.sidecar_path(location)?)?;
        let summaries = identities
            .iter()
            .filter_map(|identity| {
                document
                    .records
                    .get(&usage_key(identity))
                    .map(|record| (identity.clone(), SkillUsageSummary::from(record)))
            })
            .collect();
        Ok(SkillUsageRead {
            summaries,
            recovered_corrupt_state,
        })
    }

    fn bump(
        &self,
        location: &SkillLocation,
        identity: &SkillUsageIdentity,
        activity: SkillUsageActivity,
        timestamp: &str,
        revision_witness: &str,
    ) -> Result<SkillUsageMutation, SkillApplicationError> {
        let _guard = self.writer.lock().map_err(lock_error)?;
        let path = self.sidecar_path(location)?;
        let existed = path_exists(&path);
        let (mut document, recovered_corrupt_state) = self.read_or_recover(&path)?;
        let expected_revision = document.revision;
        let record = document.records.entry(usage_key(identity)).or_default();
        match activity {
            SkillUsageActivity::View => {
                record.view_count = record.view_count.saturating_add(1);
                record.last_viewed_at = Some(timestamp.to_string());
            }
            SkillUsageActivity::Use => {
                record.use_count = record.use_count.saturating_add(1);
                record.last_used_at = Some(timestamp.to_string());
            }
        }
        record.revision_witness = Some(revision_witness.to_string());
        let summary = SkillUsageSummary::from(&*record);
        document.revision = document.revision.saturating_add(1);
        self.write_document(
            &path,
            &document,
            (existed || recovered_corrupt_state).then_some(expected_revision),
        )?;
        Ok(SkillUsageMutation {
            summary,
            recovered_corrupt_state,
        })
    }
}

fn usage_key(identity: &SkillUsageIdentity) -> String {
    format!("{}:{}", identity.layer.as_str(), identity.id.as_str())
}

fn filesystem_error(error: std::io::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

fn json_error(error: serde_json::Error) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

fn lock_error(error: std::sync::PoisonError<impl Sized>) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::domain::{SkillId, SkillLayer};
    use crate::test_support::TempDirectory;
    use std::sync::Arc;

    fn global() -> SkillLocation {
        SkillLocation::new(SkillScope::Global, None).expect("global")
    }

    fn project(path: &Path) -> SkillLocation {
        SkillLocation::new(SkillScope::Workspace, path.to_str()).expect("workspace")
    }

    fn identity(layer: SkillLayer) -> SkillUsageIdentity {
        SkillUsageIdentity {
            id: SkillId::parse("usage-skill").expect("id"),
            layer,
        }
    }

    #[test]
    fn project_and_non_project_activity_use_their_bounded_sidecars() {
        let home = TempDirectory::new("usage-home");
        let workspace = TempDirectory::new("usage-workspace");
        let repository = FilesystemSkillUsageRepository::with_home_root(home.path().to_path_buf());
        repository
            .bump(
                &global(),
                &identity(SkillLayer::System),
                SkillUsageActivity::Use,
                "now",
                "r1",
            )
            .expect("global bump");
        repository
            .bump(
                &project(workspace.path()),
                &identity(SkillLayer::Project),
                SkillUsageActivity::View,
                "now",
                "r2",
            )
            .expect("project bump");
        assert!(home.path().join(".vanehub/skills/.usage.json").is_file());
        assert!(workspace
            .path()
            .join(".vanehub/skills/.usage.json")
            .is_file());
        let global_document: serde_json::Value = serde_json::from_slice(
            &std::fs::read(home.path().join(".vanehub/skills/.usage.json")).expect("sidecar"),
        )
        .expect("versioned JSON");
        assert_eq!(global_document["version"], 1);
        assert_eq!(global_document["revision"], 1);
        assert_eq!(
            global_document["records"]["system:usage-skill"]["patchCount"],
            0
        );
        assert_eq!(
            global_document["records"]["system:usage-skill"]["overlayMutationCount"],
            0
        );
    }

    #[test]
    fn concurrent_updates_are_serialized_and_leave_no_temporary_file() {
        let home = TempDirectory::new("usage-concurrent");
        let repository = Arc::new(FilesystemSkillUsageRepository::with_home_root(
            home.path().to_path_buf(),
        ));
        let mut threads = Vec::new();
        for index in 0..24 {
            let repository = repository.clone();
            threads.push(std::thread::spawn(move || {
                repository.bump(
                    &global(),
                    &identity(SkillLayer::User),
                    SkillUsageActivity::Use,
                    &format!("t{index}"),
                    "r1",
                )
            }));
        }
        for thread in threads {
            thread.join().expect("thread").expect("bump");
        }
        let read = repository
            .summaries(&global(), &[identity(SkillLayer::User)])
            .expect("read");
        assert_eq!(read.summaries[&identity(SkillLayer::User)].use_count, 24);
        let names = std::fs::read_dir(home.path().join(".vanehub/skills"))
            .expect("directory")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().to_string())
            .collect::<Vec<_>>();
        assert!(!names.iter().any(|name| name.starts_with(".usage.tmp")));
    }

    #[test]
    fn corrupt_state_is_backed_up_replaced_and_retained_within_limits() {
        let home = TempDirectory::new("usage-corrupt");
        let repository = FilesystemSkillUsageRepository::with_home_root(home.path().to_path_buf());
        let sidecar = home.path().join(".vanehub/skills/.usage.json");
        std::fs::create_dir_all(sidecar.parent().expect("parent")).expect("directory");
        for index in 0..8 {
            std::fs::write(&sidecar, format!("not-json-{index}")).expect("corrupt");
            let read = repository.summaries(&global(), &[]).expect("recover");
            assert!(read.recovered_corrupt_state);
        }
        let active: UsageDocument =
            serde_json::from_slice(&std::fs::read(&sidecar).expect("active"))
                .expect("valid active");
        assert_eq!(active, UsageDocument::default());
        let backups = std::fs::read_dir(home.path().join(".vanehub/skills/.usage-backups"))
            .expect("backups")
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), MAX_BACKUPS);
    }

    #[test]
    fn failed_replacement_preserves_the_previous_valid_document() {
        let home = TempDirectory::new("usage-write-failure");
        let repository = FilesystemSkillUsageRepository::with_home_root(home.path().to_path_buf());
        repository
            .bump(
                &global(),
                &identity(SkillLayer::User),
                SkillUsageActivity::View,
                "first",
                "r1",
            )
            .expect("first");
        repository.inject_write_failure();
        repository
            .bump(
                &global(),
                &identity(SkillLayer::User),
                SkillUsageActivity::View,
                "second",
                "r2",
            )
            .expect_err("failure");
        let read = repository
            .summaries(&global(), &[identity(SkillLayer::User)])
            .expect("read");
        let summary = &read.summaries[&identity(SkillLayer::User)];
        assert_eq!(summary.view_count, 1);
        assert_eq!(summary.revision_witness.as_deref(), Some("r1"));
    }
}
