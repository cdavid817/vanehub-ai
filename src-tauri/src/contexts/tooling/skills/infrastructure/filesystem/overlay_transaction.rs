#![cfg_attr(not(test), allow(dead_code))]

use super::overlay_history::FilesystemOverlayHistoryRepository;
use super::overlay_layout::OverlayStorageLayout;
use super::overlay_manifest::{parse_overlay_manifest, serialize_overlay_manifest};
use super::overlay_payload::OverlayPayloadStore;
use crate::contexts::tooling::skills::application::{
    OverlayHistoryAction, OverlayTransactionExecutor, OverlayTransactionOutcome,
    OverlayTransactionPlan, SkillApplicationError,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};

#[cfg(test)]
use std::sync::mpsc::{Receiver, Sender};

const TRANSACTION_SCHEMA_VERSION: u32 = 1;
const USAGE_SCHEMA_VERSION: u32 = 1;
const MAX_USAGE_BYTES: usize = 1_048_576;
static NEXT_TRANSACTION_ID: AtomicU64 = AtomicU64::new(0);
static OVERLAY_LOCKS: OnceLock<Mutex<BTreeMap<String, Weak<Mutex<()>>>>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayTransactionInterruption {
    PayloadStaging,
    ManifestSwap,
    HistoryAppend,
    UsageUpdate,
    CommitMarker,
    Cleanup,
}

#[derive(Clone)]
pub(crate) struct FilesystemOverlayTransactionExecutor {
    home_root: PathBuf,
    #[cfg(test)]
    interruption: Arc<Mutex<Option<OverlayTransactionInterruption>>>,
    #[cfg(test)]
    pause_after_lock: Arc<Mutex<Option<TestPause>>>,
}

impl FilesystemOverlayTransactionExecutor {
    pub(crate) fn new() -> Self {
        Self::with_home_root(super::default_home_root())
    }

    pub(crate) fn with_home_root(home_root: PathBuf) -> Self {
        Self {
            home_root,
            #[cfg(test)]
            interruption: Arc::new(Mutex::new(None)),
            #[cfg(test)]
            pause_after_lock: Arc::new(Mutex::new(None)),
        }
    }

    #[cfg(test)]
    pub(crate) fn inject_interruption_once(&self, point: OverlayTransactionInterruption) {
        if let Ok(mut interruption) = self.interruption.lock() {
            *interruption = Some(point);
        }
    }

    #[cfg(test)]
    pub(crate) fn inject_pause_after_lock_once(&self, entered: Sender<()>, release: Receiver<()>) {
        if let Ok(mut pause) = self.pause_after_lock.lock() {
            *pause = Some(TestPause { entered, release });
        }
    }

    fn execute_locked(
        &self,
        plan: OverlayTransactionPlan,
    ) -> Result<OverlayTransactionOutcome, SkillApplicationError> {
        self.recover_locked(&plan.key)?;
        validate_plan(&self.home_root, &plan)?;
        let layout = OverlayStorageLayout::resolve(&self.home_root, &plan.key)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        let paths = TransactionPaths::resolve(&self.home_root, &plan)?;
        prepare_backups(&paths, &layout)?;
        let transaction_id = format!(
            "overlay-{}-{}",
            std::process::id(),
            NEXT_TRANSACTION_ID.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut marker = TransactionMarker {
            schema_version: TRANSACTION_SCHEMA_VERSION,
            phase: TransactionPhase::Prepared,
            transaction_id: transaction_id.clone(),
            manifest_existed: layout.manifest_path.is_file(),
            history_existed: layout.history_root.is_dir(),
            usage_existed: paths.usage_path.is_file(),
            payloads: plan
                .payload_additions
                .iter()
                .map(|payload| PayloadMarker {
                    content_hash: payload.content_hash.clone(),
                    existed: layout
                        .payload_root
                        .join("sha256")
                        .join(&payload.content_hash)
                        .is_file(),
                })
                .collect(),
        };
        write_json_atomic(&paths.marker_path, &marker)?;
        self.interrupt(OverlayTransactionInterruption::PayloadStaging)?;

        let payload_store = OverlayPayloadStore::with_home_root(self.home_root.clone());
        let mut stages = Vec::new();
        for write in &plan.payload_additions {
            stages.push(payload_store.stage(&plan.key, write, &transaction_id)?);
        }
        for stage in stages {
            payload_store.publish(stage)?;
        }
        self.interrupt(OverlayTransactionInterruption::ManifestSwap)?;

        let manifest_bytes = serialize_overlay_manifest(&plan.next_manifest.document)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        write_bytes_atomic(&layout.manifest_path, &manifest_bytes)?;
        self.interrupt(OverlayTransactionInterruption::HistoryAppend)?;

        let history = FilesystemOverlayHistoryRepository::with_home_root(self.home_root.clone());
        let history_event = history.append_verified(&plan.key, plan.history_event.clone())?;
        self.interrupt(OverlayTransactionInterruption::UsageUpdate)?;

        let usage_witness = update_usage(&paths.usage_path, &plan)?;
        self.interrupt(OverlayTransactionInterruption::CommitMarker)?;

        marker.phase = TransactionPhase::Committed;
        write_json_atomic(&paths.marker_path, &marker)?;
        self.interrupt(OverlayTransactionInterruption::Cleanup)?;
        cleanup_transaction(&paths, &layout, &marker)?;

        Ok(OverlayTransactionOutcome {
            committed_revision: plan.next_manifest.document.revision(),
            document_hash: plan.next_manifest.document_hash,
            history_event_hash: history_event.event_hash,
            usage_revision_witness: usage_witness,
        })
    }

    fn recover_locked(
        &self,
        key: &crate::contexts::tooling::skills::application::OverlayKey,
    ) -> Result<(), SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(&self.home_root, key)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        let transaction_root =
            transaction_root(&layout, key.scope.as_str(), key.canonical_skill_id.as_str())?;
        let marker_path = transaction_root.join("marker.json");
        if !marker_path.is_file() {
            if transaction_root.exists() {
                fs::remove_dir_all(transaction_root).map_err(filesystem_error)?;
            }
            return Ok(());
        }
        let marker: TransactionMarker =
            serde_json::from_slice(&fs::read(&marker_path).map_err(filesystem_error)?)
                .map_err(json_error)?;
        if marker.schema_version != TRANSACTION_SCHEMA_VERSION {
            return Err(SkillApplicationError::Filesystem(
                "Unsupported Overlay transaction marker version".to_string(),
            ));
        }
        let usage_path = usage_path(&self.home_root, key)?;
        let paths = TransactionPaths::from_root(transaction_root, usage_path);
        match marker.phase {
            TransactionPhase::Prepared => rollback_transaction(&paths, &layout, &marker),
            TransactionPhase::Committed => cleanup_transaction(&paths, &layout, &marker),
        }
    }

    fn interrupt(
        &self,
        point: OverlayTransactionInterruption,
    ) -> Result<(), SkillApplicationError> {
        #[cfg(test)]
        {
            let mut interruption = self.interruption.lock().map_err(lock_error)?;
            if interruption.as_ref() == Some(&point) {
                *interruption = None;
                return Err(SkillApplicationError::Filesystem(format!(
                    "injected Overlay transaction interruption: {point:?}"
                )));
            }
        }
        let _ = point;
        Ok(())
    }

    #[cfg(test)]
    fn pause_after_lock(&self) -> Result<(), SkillApplicationError> {
        let pause = self.pause_after_lock.lock().map_err(lock_error)?.take();
        if let Some(pause) = pause {
            pause.entered.send(()).map_err(channel_error)?;
            pause.release.recv().map_err(channel_error)?;
        }
        Ok(())
    }
}

impl OverlayTransactionExecutor for FilesystemOverlayTransactionExecutor {
    fn manifest_snapshot(
        &self,
        document: crate::contexts::tooling::skills::domain::OverlayDocument,
    ) -> Result<
        crate::contexts::tooling::skills::application::OverlayManifestSnapshot,
        SkillApplicationError,
    > {
        let bytes = serialize_overlay_manifest(&document)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        Ok(
            crate::contexts::tooling::skills::application::OverlayManifestSnapshot {
                document,
                document_hash: sha256(&bytes),
            },
        )
    }

    fn execute(
        &self,
        plan: OverlayTransactionPlan,
    ) -> Result<OverlayTransactionOutcome, SkillApplicationError> {
        let lock = overlay_lock(&self.home_root, &plan.key)?;
        let _guard = lock.lock().map_err(lock_error)?;
        #[cfg(test)]
        self.pause_after_lock()?;
        self.execute_locked(plan)
    }

    fn recover(
        &self,
        key: &crate::contexts::tooling::skills::application::OverlayKey,
    ) -> Result<(), SkillApplicationError> {
        let lock = overlay_lock(&self.home_root, key)?;
        let _guard = lock.lock().map_err(lock_error)?;
        self.recover_locked(key)
    }
}

#[cfg(test)]
struct TestPause {
    entered: Sender<()>,
    release: Receiver<()>,
}

fn overlay_lock(
    home_root: &Path,
    key: &crate::contexts::tooling::skills::application::OverlayKey,
) -> Result<Arc<Mutex<()>>, SkillApplicationError> {
    let layout = OverlayStorageLayout::resolve(home_root, key)
        .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
    let identity = lock_identity(&layout.history_root);
    let registry = OVERLAY_LOCKS.get_or_init(|| Mutex::new(BTreeMap::new()));
    let mut locks = registry.lock().map_err(lock_error)?;
    locks.retain(|_, lock| lock.strong_count() > 0);
    if let Some(lock) = locks.get(&identity).and_then(Weak::upgrade) {
        return Ok(lock);
    }
    let lock = Arc::new(Mutex::new(()));
    locks.insert(identity, Arc::downgrade(&lock));
    Ok(lock)
}

fn lock_identity(history_root: &Path) -> String {
    let identity = history_root.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        identity.to_lowercase()
    } else {
        identity
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TransactionPhase {
    Prepared,
    Committed,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct TransactionMarker {
    schema_version: u32,
    phase: TransactionPhase,
    transaction_id: String,
    manifest_existed: bool,
    history_existed: bool,
    usage_existed: bool,
    payloads: Vec<PayloadMarker>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct PayloadMarker {
    content_hash: String,
    existed: bool,
}

struct TransactionPaths {
    root: PathBuf,
    marker_path: PathBuf,
    manifest_backup: PathBuf,
    history_backup: PathBuf,
    usage_path: PathBuf,
    usage_backup: PathBuf,
}

impl TransactionPaths {
    fn resolve(
        home_root: &Path,
        plan: &OverlayTransactionPlan,
    ) -> Result<Self, SkillApplicationError> {
        let layout = OverlayStorageLayout::resolve(home_root, &plan.key)
            .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
        let root = transaction_root(
            &layout,
            plan.key.scope.as_str(),
            plan.key.canonical_skill_id.as_str(),
        )?;
        Ok(Self::from_root(root, usage_path(home_root, &plan.key)?))
    }

    fn from_root(root: PathBuf, usage_path: PathBuf) -> Self {
        Self {
            marker_path: root.join("marker.json"),
            manifest_backup: root.join("manifest.backup"),
            history_backup: root.join("history.backup"),
            usage_backup: root.join("usage.backup"),
            root,
            usage_path,
        }
    }
}

fn validate_plan(
    home_root: &Path,
    plan: &OverlayTransactionPlan,
) -> Result<(), SkillApplicationError> {
    let layout = OverlayStorageLayout::resolve(home_root, &plan.key)
        .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
    if plan.next_manifest.document.canonical_skill_id != plan.key.canonical_skill_id
        || plan.next_manifest.document.scope() != plan.key.scope
        || plan.next_manifest.document.workspace_identity()
            != plan.key.workspace_identity.as_deref()
    {
        return Err(SkillApplicationError::Validation(
            "Overlay transaction manifest identity does not match its key".to_string(),
        ));
    }
    let bytes = serialize_overlay_manifest(&plan.next_manifest.document)
        .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
    if sha256(&bytes) != plan.next_manifest.document_hash {
        return Err(SkillApplicationError::Validation(
            "Overlay transaction manifest hash does not match serialized content".to_string(),
        ));
    }
    let current = read_manifest_state(&layout.manifest_path)?;
    let current_revision = current.as_ref().map(|(document, _)| document.revision());
    let current_hash = current.as_ref().map(|(_, hash)| hash.as_str());
    if current_revision != plan.expected_revision
        || current_hash != plan.expected_document_hash.as_deref()
    {
        return Err(SkillApplicationError::ConcurrentModification(
            plan.key.canonical_skill_id.as_str().to_string(),
        ));
    }
    let trust_promotion = current
        .as_ref()
        .is_some_and(|(document, hash)| is_exact_trust_promotion(document, hash, plan));
    let imported_creation = current.is_none()
        && plan.history_event.action == OverlayHistoryAction::Import
        && plan.next_manifest.document.revision() > 0;
    let expected_next = if trust_promotion {
        plan.expected_revision.unwrap_or_default()
    } else if imported_creation {
        plan.next_manifest.document.revision()
    } else {
        plan.expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| {
                SkillApplicationError::Validation("Overlay revision overflow".to_string())
            })?
    };
    if plan.next_manifest.document.revision() != expected_next
        || plan.history_event.prior_revision != plan.expected_revision
        || plan.history_event.next_revision != expected_next
        || plan.history_event.next_document_hash != plan.next_manifest.document_hash
    {
        return Err(SkillApplicationError::Validation(
            "Overlay transaction revision and history witnesses are inconsistent".to_string(),
        ));
    }
    validate_usage_witness(&usage_path(home_root, &plan.key)?, plan)
}

fn is_exact_trust_promotion(
    current: &crate::contexts::tooling::skills::domain::OverlayDocument,
    current_hash: &str,
    plan: &OverlayTransactionPlan,
) -> bool {
    if plan.history_event.action != OverlayHistoryAction::Promote
        || plan.expected_revision != Some(current.revision())
        || plan.expected_document_hash.as_deref() != Some(current_hash)
    {
        return false;
    }
    let mut expected = current.clone();
    expected
        .promote_import(
            current.revision(),
            current_hash,
            &plan.next_manifest.document.updated_at,
        )
        .is_ok()
        && expected == plan.next_manifest.document
}

fn prepare_backups(
    paths: &TransactionPaths,
    layout: &OverlayStorageLayout,
) -> Result<(), SkillApplicationError> {
    if paths.root.exists() {
        return Err(SkillApplicationError::ConcurrentModification(
            "overlay-transaction".to_string(),
        ));
    }
    fs::create_dir_all(&paths.root).map_err(filesystem_error)?;
    if layout.manifest_path.is_file() {
        fs::copy(&layout.manifest_path, &paths.manifest_backup).map_err(filesystem_error)?;
    }
    if layout.history_root.is_dir() {
        copy_directory(&layout.history_root, &paths.history_backup)?;
    }
    if paths.usage_path.is_file() {
        fs::copy(&paths.usage_path, &paths.usage_backup).map_err(filesystem_error)?;
    }
    Ok(())
}

fn rollback_transaction(
    paths: &TransactionPaths,
    layout: &OverlayStorageLayout,
    marker: &TransactionMarker,
) -> Result<(), SkillApplicationError> {
    restore_file(
        &layout.manifest_path,
        &paths.manifest_backup,
        marker.manifest_existed,
    )?;
    restore_directory(
        &layout.history_root,
        &paths.history_backup,
        marker.history_existed,
    )?;
    restore_file(&paths.usage_path, &paths.usage_backup, marker.usage_existed)?;
    for payload in &marker.payloads {
        if !payload.existed {
            remove_file_if_exists(
                &layout
                    .payload_root
                    .join("sha256")
                    .join(&payload.content_hash),
            )?;
        }
    }
    remove_staging(layout, &marker.transaction_id)?;
    fs::remove_dir_all(&paths.root).map_err(filesystem_error)
}

fn cleanup_transaction(
    paths: &TransactionPaths,
    layout: &OverlayStorageLayout,
    marker: &TransactionMarker,
) -> Result<(), SkillApplicationError> {
    remove_staging(layout, &marker.transaction_id)?;
    if paths.root.exists() {
        fs::remove_dir_all(&paths.root).map_err(filesystem_error)?;
    }
    Ok(())
}

fn update_usage(
    path: &Path,
    plan: &OverlayTransactionPlan,
) -> Result<String, SkillApplicationError> {
    let mut document = read_usage(path)?;
    let key = format!("overlay:{}", plan.key.canonical_skill_id.as_str());
    let record = document.records.entry(key).or_default();
    record.patch_count = record
        .patch_count
        .saturating_add(plan.usage_delta.patch_count_delta);
    record.overlay_mutation_count = record
        .overlay_mutation_count
        .saturating_add(plan.usage_delta.overlay_mutation_count_delta);
    if plan.usage_delta.patch_count_delta > 0 {
        record.last_patched_at = Some(plan.usage_delta.timestamp.clone());
    }
    if plan.usage_delta.overlay_mutation_count_delta > 0 {
        record.last_overlay_mutation_at = Some(plan.usage_delta.timestamp.clone());
    }
    document.revision = document.revision.saturating_add(1);
    let witness = format!("usage-{}", document.revision);
    record.revision_witness = Some(witness.clone());
    let bytes = serde_json::to_vec_pretty(&document).map_err(json_error)?;
    if bytes.len() > MAX_USAGE_BYTES {
        return Err(SkillApplicationError::Filesystem(
            "Skill usage sidecar exceeds its size limit".to_string(),
        ));
    }
    write_bytes_atomic(path, &bytes)?;
    Ok(witness)
}

fn validate_usage_witness(
    path: &Path,
    plan: &OverlayTransactionPlan,
) -> Result<(), SkillApplicationError> {
    let document = read_usage(path)?;
    let key = format!("overlay:{}", plan.key.canonical_skill_id.as_str());
    let current = document
        .records
        .get(&key)
        .and_then(|record| record.revision_witness.as_deref())
        .unwrap_or("usage-0");
    if current == plan.usage_delta.expected_revision_witness {
        Ok(())
    } else {
        Err(SkillApplicationError::ConcurrentModification(
            "usage-sidecar".to_string(),
        ))
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UsageDocumentWire {
    version: u32,
    revision: u64,
    records: BTreeMap<String, UsageRecordWire>,
}

impl Default for UsageDocumentWire {
    fn default() -> Self {
        Self {
            version: USAGE_SCHEMA_VERSION,
            revision: 0,
            records: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct UsageRecordWire {
    #[serde(default)]
    view_count: u64,
    #[serde(default)]
    use_count: u64,
    #[serde(default)]
    last_viewed_at: Option<String>,
    #[serde(default)]
    last_used_at: Option<String>,
    #[serde(default)]
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

fn read_usage(path: &Path) -> Result<UsageDocumentWire, SkillApplicationError> {
    if !path.is_file() {
        return Ok(UsageDocumentWire::default());
    }
    let document: UsageDocumentWire =
        serde_json::from_slice(&fs::read(path).map_err(filesystem_error)?).map_err(json_error)?;
    if document.version != USAGE_SCHEMA_VERSION {
        return Err(SkillApplicationError::Filesystem(
            "Unsupported Skill usage sidecar version".to_string(),
        ));
    }
    Ok(document)
}

fn read_manifest_state(
    path: &Path,
) -> Result<
    Option<(
        crate::contexts::tooling::skills::domain::OverlayDocument,
        String,
    )>,
    SkillApplicationError,
> {
    if !path.is_file() {
        return Ok(None);
    }
    let bytes = fs::read(path).map_err(filesystem_error)?;
    let document = parse_overlay_manifest(&bytes)
        .map_err(|error| SkillApplicationError::Filesystem(error.to_string()))?;
    Ok(Some((document, sha256(&bytes))))
}

fn usage_path(
    home_root: &Path,
    key: &crate::contexts::tooling::skills::application::OverlayKey,
) -> Result<PathBuf, SkillApplicationError> {
    match key.scope {
        crate::contexts::tooling::skills::domain::OverlayScope::System
        | crate::contexts::tooling::skills::domain::OverlayScope::User => {
            Ok(home_root.join(".vanehub/skills/.usage.json"))
        }
        crate::contexts::tooling::skills::domain::OverlayScope::Project => key
            .workspace_identity
            .as_deref()
            .map(|workspace| Path::new(workspace).join(".vanehub/skills/.usage.json"))
            .ok_or_else(|| {
                SkillApplicationError::Validation(
                    "Project Overlay usage requires a workspace identity".to_string(),
                )
            }),
    }
}

fn transaction_root(
    layout: &OverlayStorageLayout,
    scope: &str,
    skill_id: &str,
) -> Result<PathBuf, SkillApplicationError> {
    let overlay_root = layout.payload_root.parent().ok_or_else(|| {
        SkillApplicationError::Filesystem("Overlay payload root has no parent".to_string())
    })?;
    Ok(overlay_root
        .join(".transactions")
        .join(format!("{scope}-{skill_id}")))
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), SkillApplicationError> {
    write_bytes_atomic(path, &serde_json::to_vec_pretty(value).map_err(json_error)?)
}

fn write_bytes_atomic(path: &Path, bytes: &[u8]) -> Result<(), SkillApplicationError> {
    let parent = path.parent().ok_or_else(|| {
        SkillApplicationError::Filesystem("Overlay transaction path has no parent".to_string())
    })?;
    fs::create_dir_all(parent).map_err(filesystem_error)?;
    let temporary = parent.join(format!(".overlay-tmp-{}", std::process::id()));
    remove_file_if_exists(&temporary)?;
    let mut file = crate::platform::private_relay_fs::create_new_private_file(&temporary)
        .map_err(filesystem_error)?;
    file.write_all(bytes).map_err(filesystem_error)?;
    file.sync_all().map_err(filesystem_error)?;
    remove_file_if_exists(path)?;
    fs::rename(&temporary, path).map_err(filesystem_error)
}

fn copy_directory(source: &Path, target: &Path) -> Result<(), SkillApplicationError> {
    fs::create_dir_all(target).map_err(filesystem_error)?;
    for entry in fs::read_dir(source).map_err(filesystem_error)? {
        let entry = entry.map_err(filesystem_error)?;
        let file_type = entry.file_type().map_err(filesystem_error)?;
        if file_type.is_symlink() {
            return Err(SkillApplicationError::Filesystem(
                "Overlay transaction backup refused a symbolic link".to_string(),
            ));
        }
        let destination = target.join(entry.file_name());
        if file_type.is_dir() {
            copy_directory(&entry.path(), &destination)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), destination).map_err(filesystem_error)?;
        }
    }
    Ok(())
}

fn restore_file(target: &Path, backup: &Path, existed: bool) -> Result<(), SkillApplicationError> {
    remove_file_if_exists(target)?;
    if existed {
        let parent = target.parent().ok_or_else(|| {
            SkillApplicationError::Filesystem("Restore target has no parent".to_string())
        })?;
        fs::create_dir_all(parent).map_err(filesystem_error)?;
        fs::copy(backup, target).map_err(filesystem_error)?;
    }
    Ok(())
}

fn restore_directory(
    target: &Path,
    backup: &Path,
    existed: bool,
) -> Result<(), SkillApplicationError> {
    if target.exists() {
        fs::remove_dir_all(target).map_err(filesystem_error)?;
    }
    if existed {
        copy_directory(backup, target)?;
    }
    Ok(())
}

fn remove_staging(
    layout: &OverlayStorageLayout,
    transaction_id: &str,
) -> Result<(), SkillApplicationError> {
    let staging = layout.payload_root.join(".staging").join(transaction_id);
    if staging.exists() {
        fs::remove_dir_all(staging).map_err(filesystem_error)?;
    }
    Ok(())
}

fn remove_file_if_exists(path: &Path) -> Result<(), SkillApplicationError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(filesystem_error(error)),
    }
}

fn sha256(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
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
fn channel_error(error: impl std::fmt::Display) -> SkillApplicationError {
    SkillApplicationError::Filesystem(error.to_string())
}
