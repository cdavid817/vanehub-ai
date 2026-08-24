use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use super::memory_directory_lock::{ensure_directory, is_lock_file, MemoryDirectoryLock};
use super::memory_document::{compose, normalize_body, parse, peek_kind, DocumentKind};
use crate::contexts::personalization::application::{
    CreateMemoryInput, DeleteMemoryOutcome, MemoryIdGeneratorPort, MemoryMaintenanceRepository,
    MemoryRepository, PersonalizationApplicationError, ResetCounts, UpdateMemoryPatch,
};
use crate::contexts::personalization::domain::{
    MaintenanceFailure, MaintenancePhase, MemoryId, MemoryRecord, MemoryScope, MemoryScopeFilter,
    MemoryStatus, OwnedEntryClassification, ReconcileMemoryOutcome, ResetMemoryOutcome,
    ResetMemoryRequest, StorageEntry,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Derived, never a memory. Excluded from every classification so it can never be listed, injected,
/// or deleted as if it were one.
pub(crate) const DERIVED_INDEX_FILE_NAME: &str = "MEMORY.md";

/// Malformed and unsafe entries are moved here rather than deleted. Losing a user's text because
/// this build could not parse it is worse than any amount of maintenance noise.
///
/// Shared with legacy migration rather than duplicated: one directory is one place for a user to
/// look, and two would mean a file could be quarantined into whichever the last writer preferred.
pub(crate) const QUARANTINE_DIRECTORY_NAME: &str = "quarantine";

const MEMORY_EXTENSION: &str = "md";
/// Suffix used by the temporary file an update writes before it is renamed into place.
const TEMPORARY_SUFFIX: &str = ".tmp";

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

/// Markdown-backed authoritative memory store.
///
/// The directory is flat and every filename is `<memory-id>.md`, so traversal is impossible by
/// construction rather than by filtering `..`: `MemoryId` already rejects separators, dots, and
/// every character outside the generated charset. The canonicalization check below is what closes
/// the remaining case, a symlink pointing out of the directory.
#[derive(Clone)]
pub(crate) struct MarkdownMemoryRepository {
    root: PathBuf,
    ids: Arc<dyn MemoryIdGeneratorPort>,
    /// Serializes every mutation of this directory, in this process and across processes.
    ///
    /// Shared as an `Arc` so migration, reconciliation, and ordinary writes contend on one lock
    /// rather than three. See `MemoryDirectoryLock` for the lock order every path follows.
    lock: Arc<MemoryDirectoryLock>,
    /// Test-only injection point for a removal that fails.
    ///
    /// A seam rather than a real filesystem condition because the real ones are not portable: a
    /// read-only file is deletable on Linux and not on Windows, and an open file is deletable on
    /// POSIX and not on Windows. Partial-failure semantics have to be reproducible on every
    /// developer's machine, so they are injected here and the platform-specific behaviors are left
    /// to the platform tests in task 3.10.
    #[cfg(test)]
    delete_failures: Arc<Mutex<std::collections::BTreeSet<String>>>,
}

impl MarkdownMemoryRepository {
    pub(crate) fn new(root: PathBuf, ids: Arc<dyn MemoryIdGeneratorPort>) -> Result<Self> {
        ensure_directory(&root)?;
        let lock = Arc::new(MemoryDirectoryLock::new(&root));
        Ok(Self {
            root,
            ids,
            lock,
            #[cfg(test)]
            delete_failures: Arc::new(Mutex::new(std::collections::BTreeSet::new())),
        })
    }

    /// Builds a repository that shares an existing directory lock.
    ///
    /// Migration needs to hold the directory for a whole multi-stage run while still using the
    /// repository's own write paths. Handing it the same lock is what keeps that from
    /// self-deadlocking on a second acquisition.
    pub(crate) fn with_shared_lock(
        root: PathBuf,
        ids: Arc<dyn MemoryIdGeneratorPort>,
        lock: Arc<MemoryDirectoryLock>,
    ) -> Result<Self> {
        ensure_directory(&root)?;
        Ok(Self {
            root,
            ids,
            lock,
            #[cfg(test)]
            delete_failures: Arc::new(Mutex::new(std::collections::BTreeSet::new())),
        })
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn lock(&self) -> Arc<MemoryDirectoryLock> {
        self.lock.clone()
    }

    /// Marks a file name whose removal must fail on the next attempt.
    #[cfg(test)]
    pub(crate) fn inject_delete_failure(&self, file_name: &str) {
        self.delete_failures
            .lock()
            .expect("delete failures")
            .insert(file_name.to_string());
    }

    fn removal_is_injected_to_fail(&self, file_name: &str) -> bool {
        #[cfg(test)]
        {
            return self
                .delete_failures
                .lock()
                .expect("delete failures")
                .contains(file_name);
        }
        #[cfg(not(test))]
        {
            let _ = file_name;
            false
        }
    }

    fn quarantine_root(&self) -> PathBuf {
        self.root.join(QUARANTINE_DIRECTORY_NAME)
    }

    /// Resolves an id-derived filename inside the memory directory.
    ///
    /// The separator check runs as a string comparison before component analysis because `\` is a
    /// separator only on Windows: on Linux `a\b.md` parses as one ordinary file name, so component
    /// analysis alone would accept on one platform what it rejects on another.
    fn resolve(&self, file_name: &str) -> Result<PathBuf> {
        if file_name.contains('/') || file_name.contains('\\') {
            return Err(storage(format!(
                "memory file name {file_name:?} must not contain a directory separator"
            )));
        }
        let candidate = Path::new(file_name);
        let mut components = candidate.components();
        let Some(Component::Normal(name)) = components.next() else {
            return Err(storage(format!(
                "memory file name {file_name:?} must be a plain file name"
            )));
        };
        if components.next().is_some() {
            return Err(storage(format!(
                "memory file name {file_name:?} must not contain a directory separator"
            )));
        }
        let path = self.root.join(name);
        if !is_memory_file(&path) {
            return Err(storage(format!(
                "memory file name {file_name:?} must name a .md file that is not the derived index"
            )));
        }
        // Only meaningful for a file that already exists: a path that does not exist yet cannot be
        // canonicalized, and its parent is the root we just joined from.
        if path.exists() {
            let canonical_root = self.root.canonicalize().map_err(|error| {
                storage(format!("memory directory cannot be resolved: {error}"))
            })?;
            let canonical_path = path.canonicalize().map_err(|error| {
                storage(format!("memory {file_name:?} cannot be resolved: {error}"))
            })?;
            if !canonical_path.starts_with(&canonical_root) {
                return Err(storage(format!(
                    "memory {file_name:?} resolves outside the memory directory"
                )));
            }
        }
        Ok(path)
    }

    fn path_for(&self, id: &MemoryId) -> Result<PathBuf> {
        self.resolve(&format!("{id}.{MEMORY_EXTENSION}"))
    }

    fn read_record(&self, path: &Path) -> Result<MemoryRecord> {
        let raw = fs::read_to_string(path)
            .map_err(|error| storage(format!("memory cannot be read: {error}")))?;
        parse(&raw)
    }

    /// Creates the file with create-new semantics: an existing path is an error, never a
    /// replacement. That is what makes a duplicate display name produce a second record rather
    /// than silently overwriting the first.
    ///
    /// `File::create_new` rather than `OpenOptions`: the two are equivalent here, but the
    /// architecture fitness rule that keeps append-log construction inside the platform adapter
    /// matches `OpenOptions::new()` syntactically, and this store is not an append log.
    fn write_new(&self, path: &Path, contents: &str) -> Result<()> {
        let mut file = fs::File::create_new(path)
            .map_err(|error| storage(format!("memory cannot be created: {error}")))?;
        file.write_all(contents.as_bytes())
            .map_err(|error| storage(format!("memory cannot be written: {error}")))?;
        file.sync_all()
            .map_err(|error| storage(format!("memory cannot be flushed: {error}")))?;
        Ok(())
    }

    /// Replaces an existing file atomically: write a sibling temporary, flush it to disk, then
    /// rename over the target. A crash can leave the temporary behind — enumeration classifies it
    /// as transient — but never a half-written memory at the real path.
    fn replace_atomically(&self, path: &Path, contents: &str) -> Result<()> {
        let temporary = path.with_extension(format!("{MEMORY_EXTENSION}{TEMPORARY_SUFFIX}"));
        {
            let mut file = fs::File::create(&temporary).map_err(|error| {
                storage(format!("memory temporary file cannot be created: {error}"))
            })?;
            file.write_all(contents.as_bytes())
                .map_err(|error| storage(format!("memory cannot be written: {error}")))?;
            file.sync_all()
                .map_err(|error| storage(format!("memory cannot be flushed: {error}")))?;
        }
        fs::rename(&temporary, path).map_err(|error| {
            // Leave the temporary in place: it holds the only copy of the new content, and
            // enumeration will surface it rather than silently discarding the user's edit.
            storage(format!("memory cannot be replaced: {error}"))
        })
    }

    fn quarantine(&self, file_name: &str) -> Result<()> {
        let quarantine_root = self.quarantine_root();
        fs::create_dir_all(&quarantine_root)
            .map_err(|error| storage(format!("quarantine directory is unavailable: {error}")))?;
        let source = self.root.join(file_name);
        let destination = quarantine_root.join(file_name);
        fs::rename(&source, &destination)
            .map_err(|error| storage(format!("memory cannot be quarantined: {error}")))
    }
}

fn is_memory_file(path: &Path) -> bool {
    if path.extension().and_then(|extension| extension.to_str()) != Some(MEMORY_EXTENSION) {
        return false;
    }
    path.file_name().and_then(|name| name.to_str()) != Some(DERIVED_INDEX_FILE_NAME)
}

/// Classifies a directory entry by explicit rule, from its name and a bounded read of its head.
///
/// Deliberately independent of whether the file parses in full: the previous store decided what
/// existed by what it could read, so a malformed file was invisible to the reset that was supposed
/// to remove it. Full parsing happens afterwards, and only to tell a valid v2 file from a broken
/// one.
///
/// The name alone cannot decide the format. A v1 memory's filename was derived from its display
/// name, and a name like `use-pnpm` is also a perfectly valid memory id — so the declared schema
/// version in the header is what separates the two.
fn classify(file_name: &str, head: Option<&str>) -> OwnedEntryClassification {
    if file_name == DERIVED_INDEX_FILE_NAME {
        return OwnedEntryClassification::Derived;
    }
    if file_name.ends_with(TEMPORARY_SUFFIX) || file_name.ends_with(".lock") {
        return OwnedEntryClassification::Transient;
    }
    if !file_name.ends_with(&format!(".{MEMORY_EXTENSION}")) {
        return OwnedEntryClassification::Foreign;
    }
    match head.map(peek_kind) {
        Some(DocumentKind::V2) => OwnedEntryClassification::ValidV2,
        Some(DocumentKind::Legacy) => OwnedEntryClassification::LegacyV1,
        // Unreadable, or the head could not be read at all. Neither a legacy memory (which always
        // had frontmatter) nor a usable v2 file.
        Some(DocumentKind::Unreadable) | None => OwnedEntryClassification::MalformedV2,
    }
}

/// Reads only as far as the closing frontmatter delimiter.
fn read_head(path: &Path) -> Option<String> {
    use std::io::{BufRead, BufReader};

    let file = fs::File::open(path).ok()?;
    let mut lines = Vec::new();
    for line in BufReader::new(file).lines().take(HEAD_LINE_LIMIT) {
        let line = line.ok()?;
        let is_delimiter = line.trim_end() == "---";
        lines.push(line);
        if is_delimiter && lines.len() > 1 {
            break;
        }
    }
    Some(lines.join("\n"))
}

/// Head lines read for classification. Comfortably larger than the frontmatter this writer
/// produces, small enough that a large body is never pulled in to decide what a file is.
const HEAD_LINE_LIMIT: usize = 64;

impl MemoryRepository for MarkdownMemoryRepository {
    fn get(&self, id: &MemoryId) -> Result<Option<MemoryRecord>> {
        let path = self.path_for(id)?;
        if !path.exists() {
            return Ok(None);
        }
        self.read_record(&path).map(Some)
    }

    fn create(&self, input: CreateMemoryInput, now: DateTime<Utc>) -> Result<MemoryRecord> {
        let id = self.ids.generate();
        self.create_with_id(&id, input, now, now)
    }

    fn create_with_id(
        &self,
        id: &MemoryId,
        input: CreateMemoryInput,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<MemoryRecord> {
        let _guard = self.lock.acquire_with_retry()?;

        let record = MemoryRecord {
            id: id.clone(),
            name: input.name,
            description: input.description,
            memory_type: input.memory_type,
            // Normalized here rather than at the file boundary so the record this returns is the
            // record the caller will read back, byte for byte.
            content: normalize_body(&input.content),
            scope: input.scope,
            audience: input.audience,
            status: input.status,
            source: input.source,
            provenance: input.provenance,
            sensitivity: input.sensitivity,
            // Starts at 1 so that "revision 0" is never a valid stored value a stale client could
            // guess its way past.
            revision: 1,
            created_at,
            updated_at,
            verified_at: None,
            last_used_at: None,
            use_count: 0,
        };
        record.validate()?;

        let path = self.path_for(&record.id)?;
        self.write_new(&path, &compose(&record))?;
        Ok(record)
    }

    fn update(
        &self,
        id: &MemoryId,
        expected_revision: u64,
        patch: UpdateMemoryPatch,
        now: DateTime<Utc>,
    ) -> Result<MemoryRecord> {
        let _guard = self.lock.acquire_with_retry()?;

        let path = self.path_for(id)?;
        if !path.exists() {
            return Err(PersonalizationApplicationError::NotFound);
        }
        let mut record = self.read_record(&path)?;
        if record.revision != expected_revision {
            return Err(PersonalizationApplicationError::RevisionConflict(
                crate::contexts::personalization::domain::RevisionConflict {
                    expected: expected_revision,
                    current: record.revision,
                },
            ));
        }

        if let Some(name) = patch.name {
            record.name = name;
        }
        if let Some(description) = patch.description {
            record.description = description;
        }
        if let Some(memory_type) = patch.memory_type {
            record.memory_type = memory_type;
        }
        if let Some(content) = patch.content {
            record.content = normalize_body(&content);
        }
        if let Some(scope) = patch.scope {
            record.scope = scope;
        }
        if let Some(audience) = patch.audience {
            record.audience = audience;
        }
        if let Some(status) = patch.status {
            record.status = status;
        }
        if let Some(sensitivity) = patch.sensitivity {
            record.sensitivity = sensitivity;
        }
        record.revision = record.revision.saturating_add(1);
        record.updated_at = now;
        record.validate()?;

        self.replace_atomically(&path, &compose(&record))?;
        Ok(record)
    }

    fn delete(&self, id: &MemoryId, expected_revision: Option<u64>) -> Result<DeleteMemoryOutcome> {
        let _guard = self.lock.acquire_with_retry()?;

        let path = self.path_for(id)?;
        if !path.exists() {
            // Already gone is the caller's desired end state, and reporting it as a failure would
            // stop reconciliation from converging.
            return Ok(DeleteMemoryOutcome::default());
        }
        if let Some(expected) = expected_revision {
            let record = self.read_record(&path)?;
            if record.revision != expected {
                return Err(PersonalizationApplicationError::RevisionConflict(
                    crate::contexts::personalization::domain::RevisionConflict {
                        expected,
                        current: record.revision,
                    },
                ));
            }
        }
        if self.removal_is_injected_to_fail(&format!("{id}.{MEMORY_EXTENSION}")) {
            return Err(storage("memory cannot be deleted: injected failure"));
        }
        fs::remove_file(&path)
            .map_err(|error| storage(format!("memory cannot be deleted: {error}")))?;
        Ok(DeleteMemoryOutcome {
            deleted_file: true,
            ..DeleteMemoryOutcome::default()
        })
    }
}

impl MemoryMaintenanceRepository for MarkdownMemoryRepository {
    /// Every application-owned entry, with no cap and no dependence on parsing.
    ///
    /// This is the operation the old `scan()` could not be: it stopped at 200 files and dropped
    /// anything whose frontmatter would not parse, and destructive work reused it. A reset built
    /// on that leaves files behind; a reset built on this does not.
    fn enumerate_owned_entries(&self) -> Result<Vec<StorageEntry>> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            // A directory that does not exist yet is an empty store, not a failure.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(storage(format!(
                    "memory directory {} cannot be read: {error}",
                    self.root.display()
                )))
            }
        };

        let mut owned = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // The quarantine directory is enumerated separately by the operations that are
                // allowed to touch it.
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            // The lock file is excluded by name rather than by extension, so it can never be
            // classified, counted, or deleted as if it were a memory. Deleting it would be worse
            // than noise: the next two holders would each create their own and both believe they
            // owned the directory.
            if is_lock_file(file_name) {
                continue;
            }
            let head = read_head(&path);
            let mut classification = classify(file_name, head.as_deref());
            let mut memory_id = None;
            if matches!(classification, OwnedEntryClassification::ValidV2) {
                match self.read_record(&path) {
                    Ok(record) => memory_id = Some(record.id),
                    // The name is a valid id but the file will not parse: a torn write, an
                    // external edit, or a build that cannot read this schema version. All three
                    // must be visible, and none may be activated.
                    Err(_) => classification = OwnedEntryClassification::MalformedV2,
                }
            }
            owned.push(StorageEntry {
                file_name: file_name.to_string(),
                classification,
                memory_id,
            });
        }

        if let Ok(quarantined) = fs::read_dir(self.quarantine_root()) {
            for entry in quarantined.flatten() {
                let Some(file_name) = entry
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str().map(String::from))
                else {
                    continue;
                };
                owned.push(StorageEntry {
                    file_name,
                    classification: OwnedEntryClassification::Quarantined,
                    memory_id: None,
                });
            }
        }

        owned.sort_by(|left, right| left.file_name.cmp(&right.file_name));
        Ok(owned)
    }

    fn count_for_reset(
        &self,
        scope: &MemoryScopeFilter,
        statuses: &[MemoryStatus],
    ) -> Result<ResetCounts> {
        let mut counts = ResetCounts::default();
        for entry in self.enumerate_owned_entries()? {
            if !entry.classification.is_resettable() {
                continue;
            }
            match entry.classification {
                OwnedEntryClassification::ValidV2 => {
                    let path = self.root.join(&entry.file_name);
                    let Ok(record) = self.read_record(&path) else {
                        counts.malformed += 1;
                        counts.matched += 1;
                        continue;
                    };
                    if !matches_filter(&record, scope, statuses) {
                        continue;
                    }
                    counts.matched += 1;
                    match record.scope {
                        MemoryScope::Global => counts.global += 1,
                        MemoryScope::Workspace { .. } => counts.workspace += 1,
                    }
                    if matches!(record.status, MemoryStatus::Candidate) {
                        counts.candidates += 1;
                    }
                }
                // An entry whose scope cannot be established counts as matched only for an
                // unrestricted reset; a scoped reset reports it as a failure rather than guessing.
                OwnedEntryClassification::MalformedV2
                | OwnedEntryClassification::LegacyV1
                | OwnedEntryClassification::Quarantined => {
                    counts.malformed += 1;
                    if matches!(scope, MemoryScopeFilter::Any) {
                        counts.matched += 1;
                    }
                }
                _ => {}
            }
        }
        Ok(counts)
    }

    fn reset(
        &self,
        request: &ResetMemoryRequest,
        now: DateTime<Utc>,
    ) -> Result<ResetMemoryOutcome> {
        request
            .authorize(now)
            .map_err(PersonalizationApplicationError::ResetRefused)?;

        let _guard = self.lock.acquire_with_retry()?;

        let mut outcome = ResetMemoryOutcome::default();
        let unrestricted = matches!(request.scope, MemoryScopeFilter::Any);

        for entry in self.enumerate_owned_entries()? {
            if !entry.classification.is_resettable() {
                continue;
            }
            let (directory, is_quarantined) =
                if matches!(entry.classification, OwnedEntryClassification::Quarantined) {
                    (self.quarantine_root(), true)
                } else {
                    (self.root.clone(), false)
                };
            let path = directory.join(&entry.file_name);

            let should_delete = match entry.classification {
                OwnedEntryClassification::ValidV2 => match self.read_record(&path) {
                    Ok(record) => matches_filter(&record, &request.scope, &request.statuses),
                    Err(_) => unrestricted,
                },
                // Scope is unknowable for these. An unrestricted reset removes them, which is what
                // makes "reset everything" actually mean everything; a scoped reset reports them
                // rather than guessing which side of the boundary they fall on.
                _ => {
                    if !unrestricted {
                        outcome.failures.push(MaintenanceFailure {
                            memory_id: entry.memory_id.clone(),
                            phase: MaintenancePhase::UnclassifiableEntry,
                        });
                        false
                    } else {
                        true
                    }
                }
            };

            if !should_delete {
                continue;
            }
            outcome.matched += 1;
            if self.removal_is_injected_to_fail(&entry.file_name) {
                outcome.failures.push(MaintenanceFailure {
                    memory_id: entry.memory_id.clone(),
                    phase: MaintenancePhase::AuthoritativeFile,
                });
                continue;
            }
            match fs::remove_file(&path) {
                Ok(()) => {
                    if is_quarantined {
                        outcome.removed_quarantine_entries += 1;
                    } else {
                        outcome.deleted_files += 1;
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(_) => outcome.failures.push(MaintenanceFailure {
                    memory_id: entry.memory_id.clone(),
                    phase: MaintenancePhase::AuthoritativeFile,
                }),
            }
        }

        Ok(outcome)
    }

    fn reconcile(&self, _now: DateTime<Utc>) -> Result<ReconcileMemoryOutcome> {
        let _guard = self.lock.acquire_with_retry()?;

        let mut outcome = ReconcileMemoryOutcome::default();
        for entry in self.enumerate_owned_entries()? {
            outcome.scanned_entries += 1;
            if matches!(entry.classification, OwnedEntryClassification::MalformedV2) {
                match self.quarantine(&entry.file_name) {
                    Ok(()) => outcome.quarantined_entries += 1,
                    Err(_) => outcome.failures.push(MaintenanceFailure {
                        memory_id: entry.memory_id.clone(),
                        phase: MaintenancePhase::Quarantine,
                    }),
                }
            }
        }
        Ok(outcome)
    }
}

fn matches_filter(
    record: &MemoryRecord,
    scope: &MemoryScopeFilter,
    statuses: &[MemoryStatus],
) -> bool {
    if !scope.matches(&record.scope) {
        return false;
    }
    statuses.is_empty() || statuses.contains(&record.status)
}
