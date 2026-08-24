use std::fs;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use super::legacy_memory_document::parse_legacy_document;
use super::markdown_memory_repository::{DERIVED_INDEX_FILE_NAME, QUARANTINE_DIRECTORY_NAME};
use super::memory_directory_lock::{ensure_directory, is_lock_file, MemoryDirectoryLock};
use super::memory_document::{peek_kind, DocumentKind};
use crate::contexts::personalization::application::{
    DiscoveredLegacySource, LegacyMemoryFields, LegacyMemorySourcePort,
    PersonalizationApplicationError,
};
use crate::contexts::personalization::domain::{LegacySourceFingerprint, LegacySourceLocator};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// Byte-for-byte copies of every legacy source, written before anything is removed.
///
/// A directory rather than a suffix in place, so the backup can never be re-enumerated as a source:
/// enumeration skips directories outright, which is one rule instead of a growing list of name
/// patterns to exclude.
pub(crate) const LEGACY_BACKUP_DIRECTORY_NAME: &str = "legacy-backup";

/// Suffix of the temporary file the v2 store writes before renaming it into place. Excluded here
/// for the same reason the store classifies it transient: it is half of someone else's write.
const TEMPORARY_SUFFIX: &str = ".tmp";

const MEMORY_EXTENSION: &str = "md";

/// Bounded search for a free quarantine name. A collision means the same file name was quarantined
/// before; going round forever instead would turn a naming clash into a hang.
const QUARANTINE_NAME_ATTEMPTS: usize = 1_000;

fn storage(error: impl std::fmt::Display) -> PersonalizationApplicationError {
    PersonalizationApplicationError::Storage(error.to_string())
}

/// The pre-v2 memory directory, read for migration only.
///
/// Operates on the same root as the v2 store because that is where the files are: v1 and v2 write
/// into one flat directory, and what separates them is the declared schema version in the header,
/// never the shape of the filename. A v1 memory's name was its file stem, and a name like
/// `use-pnpm` is also a perfectly valid v2 memory id.
#[derive(Clone)]
pub(crate) struct FileLegacyMemorySource {
    root: PathBuf,
    /// The same lock the v2 store takes, so a migration write and an ordinary write cannot
    /// interleave inside one directory. Acquired per operation rather than for a whole run: the
    /// journal makes each step resumable, so a contended step is deferred rather than corrupting.
    lock: Arc<MemoryDirectoryLock>,
    /// Test-only injection points, keyed by legacy file name.
    ///
    /// Seams rather than real filesystem conditions because the real ones are not portable: a
    /// read-only file is deletable on Linux and not on Windows, and a full disk cannot be produced
    /// on a developer's machine at all. The recovery properties under test have to be reproducible
    /// everywhere.
    #[cfg(test)]
    failures: Arc<std::sync::Mutex<std::collections::BTreeSet<(LegacyOperation, String)>>>,
}

/// Which legacy-source operation a test has asked to fail.
#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LegacyOperation {
    Backup,
    ReadBackup,
    Remove,
    Quarantine,
}

impl FileLegacyMemorySource {
    pub(crate) fn new(root: PathBuf, lock: Arc<MemoryDirectoryLock>) -> Result<Self> {
        ensure_directory(&root)?;
        Ok(Self {
            root,
            lock,
            #[cfg(test)]
            failures: Arc::new(std::sync::Mutex::new(std::collections::BTreeSet::new())),
        })
    }

    /// Marks one operation on one file to fail on every attempt until it is cleared.
    #[cfg(test)]
    pub(crate) fn inject_failure(&self, operation: LegacyOperation, file_name: &str) {
        self.failures
            .lock()
            .expect("legacy failures")
            .insert((operation, file_name.to_string()));
    }

    #[cfg(test)]
    pub(crate) fn clear_failure(&self, operation: LegacyOperation, file_name: &str) {
        self.failures
            .lock()
            .expect("legacy failures")
            .remove(&(operation, file_name.to_string()));
    }

    #[cfg(test)]
    fn injected(&self, operation: LegacyOperation, file_name: &str) -> bool {
        self.failures
            .lock()
            .expect("legacy failures")
            .contains(&(operation, file_name.to_string()))
    }

    fn guard_injection(&self, operation_label: &str, file_name: &str) -> Result<()> {
        #[cfg(test)]
        {
            let operation = match operation_label {
                "backup" => LegacyOperation::Backup,
                "read_backup" => LegacyOperation::ReadBackup,
                "remove" => LegacyOperation::Remove,
                _ => LegacyOperation::Quarantine,
            };
            if self.injected(operation, file_name) {
                return Err(storage(format!("{operation_label}_injected_failure")));
            }
        }
        #[cfg(not(test))]
        {
            let _ = (operation_label, file_name);
        }
        Ok(())
    }

    fn backup_root(&self) -> PathBuf {
        self.root.join(LEGACY_BACKUP_DIRECTORY_NAME)
    }

    fn quarantine_root(&self) -> PathBuf {
        self.root.join(QUARANTINE_DIRECTORY_NAME)
    }

    /// The one file name a locator names, rejecting anything that is not a plain name in this
    /// directory.
    ///
    /// The separator check runs as a string comparison before component analysis because `\` is a
    /// separator only on Windows: on Linux `a\b.md` parses as one ordinary file name, so component
    /// analysis alone would accept on one platform what it rejects on the other.
    fn file_name_of(locator: &LegacySourceLocator) -> Result<String> {
        let LegacySourceLocator::MarkdownFile {
            normalized_relative_path,
        } = locator
        else {
            return Err(storage(
                "this source reads Markdown files; a row locator has no file",
            ));
        };
        let raw = normalized_relative_path.as_str();
        if raw.contains('/') || raw.contains('\\') {
            return Err(storage("a legacy file name must not contain a separator"));
        }
        let mut components = Path::new(raw).components();
        let Some(Component::Normal(name)) = components.next() else {
            return Err(storage("a legacy file name must be a plain file name"));
        };
        if components.next().is_some() {
            return Err(storage("a legacy file name must be a plain file name"));
        }
        name.to_str()
            .map(str::to_string)
            .ok_or_else(|| storage("a legacy file name is not valid UTF-8"))
    }

    fn source_path(&self, locator: &LegacySourceLocator) -> Result<(String, PathBuf)> {
        let file_name = Self::file_name_of(locator)?;
        let path = self.root.join(&file_name);
        // A source that has already been removed is resolvable but absent; the caller decides
        // whether that is an error. Only an existing path can be canonicalized.
        if path.exists() && !resolves_inside(&self.root, &path) {
            return Err(storage(
                "a legacy source resolves outside the memory directory",
            ));
        }
        Ok((file_name, path))
    }
}

/// Whether a path that exists resolves to somewhere inside `root`.
///
/// Canonicalizing both sides is what covers a symlink or a Windows junction pointing out of the
/// directory. A path that cannot be canonicalized is treated as outside: the cost of a false
/// negative is one file left in place for a human to look at, while a false positive would let
/// migration read, copy, and delete something outside the directory it owns.
fn resolves_inside(root: &Path, path: &Path) -> bool {
    let (Ok(canonical_root), Ok(canonical_path)) = (root.canonicalize(), path.canonicalize())
    else {
        return false;
    };
    canonical_path.starts_with(canonical_root)
}

/// Whether a directory entry is a candidate legacy source, by name alone.
///
/// Everything decided here is decided without opening the file. Whether an actual `.md` file is v1
/// or v2 needs its header and is decided in `enumerate_sources`.
fn is_candidate_name(file_name: &str) -> bool {
    if file_name == DERIVED_INDEX_FILE_NAME || is_lock_file(file_name) {
        return false;
    }
    if file_name.ends_with(TEMPORARY_SUFFIX) || file_name.ends_with(".lock") {
        return false;
    }
    file_name.ends_with(&format!(".{MEMORY_EXTENSION}"))
}

impl LegacyMemorySourcePort for FileLegacyMemorySource {
    fn enumerate_sources(&self) -> Result<Vec<DiscoveredLegacySource>> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            // A directory that does not exist yet holds no legacy sources, which is not a failure.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => {
                return Err(storage(format!(
                    "the memory directory cannot be read: {error}"
                )))
            }
        };

        let mut discovered = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            // Skips the backup and quarantine directories in one rule, rather than by name. Both
            // hold copies of sources, and re-enumerating either would migrate the same text twice.
            if path.is_dir() {
                continue;
            }
            let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            if !is_candidate_name(file_name) {
                continue;
            }
            let locator = LegacySourceLocator::markdown(file_name)?;

            // Checked before the file is opened. A link pointing out of the directory is journalled
            // and quarantined — which renames the link, never its target — and its bytes are never
            // read, so nothing outside the directory is copied into a backup or a hash.
            if !resolves_inside(&self.root, &path) {
                discovered.push(DiscoveredLegacySource {
                    locator,
                    fingerprint: None,
                    fields: None,
                });
                continue;
            }

            let Ok(bytes) = fs::read(&path) else {
                // Unreadable now, possibly readable later. Recorded with no fingerprint so nothing
                // downstream can mistake an unread file for one whose contents are known.
                discovered.push(DiscoveredLegacySource {
                    locator,
                    fingerprint: None,
                    fields: None,
                });
                continue;
            };
            let fingerprint = Some(LegacySourceFingerprint::of(&bytes));
            let Ok(text) = String::from_utf8(bytes) else {
                discovered.push(DiscoveredLegacySource {
                    locator,
                    fingerprint,
                    fields: None,
                });
                continue;
            };
            // A file that declares the current schema version belongs to the v2 store. Deciding
            // this from the header rather than the filename is the whole point: v1 named files
            // after their display name, and those names are also valid v2 ids.
            if peek_kind(&text) == DocumentKind::V2 {
                continue;
            }

            discovered.push(DiscoveredLegacySource {
                locator,
                fingerprint,
                fields: parse_legacy_document(&text).map(|document| LegacyMemoryFields {
                    name: document.name,
                    description: document.description,
                    memory_type: document.memory_type,
                    content: document.body,
                    source_agent_id: document.agent_id,
                    folder: document.folder,
                    save_source: document.save_source,
                    source_relative_path: Some(file_name.to_string()),
                    created_at: document
                        .created_at
                        .as_deref()
                        .and_then(parse_legacy_timestamp),
                    modified_at: entry
                        .metadata()
                        .ok()
                        .and_then(|metadata| metadata.modified().ok())
                        .map(chrono::DateTime::<chrono::Utc>::from),
                }),
            });
        }

        // Sorted so a run migrates in the same order every time. Directory iteration order is not
        // specified, and an unstable order would make an interrupted run resume differently than it
        // started, which is exactly what a resume must not do.
        discovered.sort_by(|left, right| left.locator.cmp(&right.locator));
        Ok(discovered)
    }

    fn read_raw(&self, locator: &LegacySourceLocator) -> Result<Vec<u8>> {
        let (_, path) = self.source_path(locator)?;
        fs::read(&path).map_err(|error| storage(format!("a legacy source cannot be read: {error}")))
    }

    fn write_backup(&self, locator: &LegacySourceLocator, bytes: &[u8]) -> Result<String> {
        let (file_name, _) = self.source_path(locator)?;
        self.guard_injection("backup", &file_name)?;
        let _guard = self.lock.acquire_with_retry()?;

        let backup_root = self.backup_root();
        fs::create_dir_all(&backup_root).map_err(|error| {
            storage(format!(
                "the legacy backup directory is unavailable: {error}"
            ))
        })?;
        // Written whole rather than appended, and overwritten rather than created new: a resumed run
        // that was interrupted before it journalled the backup writes it again, and the second copy
        // has to replace the first rather than fail.
        fs::write(backup_root.join(&file_name), bytes)
            .map_err(|error| storage(format!("a legacy backup cannot be written: {error}")))?;
        Ok(format!("{LEGACY_BACKUP_DIRECTORY_NAME}/{file_name}"))
    }

    fn read_backup(&self, relative_path: &str) -> Result<Vec<u8>> {
        let file_name = relative_path
            .strip_prefix(&format!("{LEGACY_BACKUP_DIRECTORY_NAME}/"))
            .ok_or_else(|| storage("a backup path must name a file in the backup directory"))?;
        if file_name.is_empty() || file_name.contains('/') || file_name.contains('\\') {
            return Err(storage("a backup path must be a plain file name"));
        }
        self.guard_injection("read_backup", file_name)?;
        let path = self.backup_root().join(file_name);
        if !resolves_inside(&self.backup_root(), &path) {
            return Err(storage("a backup resolves outside the backup directory"));
        }
        fs::read(&path).map_err(|error| storage(format!("a legacy backup cannot be read: {error}")))
    }

    fn remove_source(&self, locator: &LegacySourceLocator) -> Result<()> {
        let (file_name, path) = self.source_path(locator)?;
        self.guard_injection("remove", &file_name)?;
        let _guard = self.lock.acquire_with_retry()?;

        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            // A source that is already gone is the end state this call was asking for, which is
            // what lets a run interrupted between the removal and its journal entry resume.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(storage(format!(
                "a legacy source cannot be removed: {error}"
            ))),
        }
    }

    fn quarantine_source(&self, locator: &LegacySourceLocator) -> Result<String> {
        let (file_name, path) = self.source_path(locator)?;
        self.guard_injection("quarantine", &file_name)?;
        let _guard = self.lock.acquire_with_retry()?;

        let quarantine_root = self.quarantine_root();
        fs::create_dir_all(&quarantine_root).map_err(|error| {
            storage(format!("the quarantine directory is unavailable: {error}"))
        })?;
        let destination_name = free_quarantine_name(&quarantine_root, &file_name)?;
        // A rename rather than a copy-then-delete: the file exists at exactly one of the two paths
        // at every instant, so an interruption can never leave the user's text in neither.
        fs::rename(&path, quarantine_root.join(&destination_name))
            .map_err(|error| storage(format!("a legacy source cannot be quarantined: {error}")))?;
        Ok(format!("{QUARANTINE_DIRECTORY_NAME}/{destination_name}"))
    }
}

/// The instant a v1 `created` value names.
///
/// An unparseable value degrades to absent rather than failing the migration: v1 wrote this field
/// from a clock, but a hand-written file could carry anything, and a memory is worth more than its
/// timestamp.
fn parse_legacy_timestamp(raw: &str) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(raw.trim())
        .ok()
        .map(|parsed| parsed.with_timezone(&chrono::Utc))
}

/// A quarantine file name that is not already taken.
///
/// Overwriting would destroy the earlier quarantined file, which is the one outcome quarantine
/// exists to prevent — so a clash gets a suffix rather than a replacement.
fn free_quarantine_name(quarantine_root: &Path, file_name: &str) -> Result<String> {
    if !quarantine_root.join(file_name).exists() {
        return Ok(file_name.to_string());
    }
    let (stem, extension) = match file_name.rsplit_once('.') {
        Some((stem, extension)) => (stem, format!(".{extension}")),
        None => (file_name, String::new()),
    };
    for attempt in 1..=QUARANTINE_NAME_ATTEMPTS {
        let candidate = format!("{stem}-{attempt}{extension}");
        if !quarantine_root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(storage("no free quarantine name is available"))
}
