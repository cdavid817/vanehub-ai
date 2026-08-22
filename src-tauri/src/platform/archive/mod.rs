//! Bounded reading of archives that arrived from outside the application.
//!
//! Skills' Overlay import, and every package format that follows it, needs the same handful of
//! answers before a single byte reaches disk: is this really one archive, what does it claim to
//! contain, is any of it a link or an escape, and does it stay inside its budgets while being
//! expanded. Those answers do not belong to any one bounded context, and a context that owns them
//! becomes a context every other context has to import.
//!
//! So they live here, in the outer technology layer the composition root already owns. The rules
//! are deliberately mechanical: what is *admissible* in a particular package — which names are
//! expected, which schema version is supported — stays with the context that defines that package.

mod entry_path;
#[cfg(test)]
mod tests;
mod zip_reader;

pub(crate) use entry_path::is_safe_archive_entry_path;
pub(crate) use zip_reader::{
    ends_at_the_central_directory_record, extract_zip_entries, inspect_zip_entries,
};

use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveEntryKind {
    File,
    Directory,
    SymbolicLink,
    HardLink,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveEntry {
    pub(crate) path: String,
    pub(crate) kind: ArchiveEntryKind,
    /// The size the archive *declares*. Believed only for budgeting; extraction re-checks it.
    pub(crate) expanded_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ArchiveLimits {
    pub(crate) maximum_compressed_bytes: u64,
    pub(crate) maximum_expanded_bytes: u64,
    pub(crate) maximum_entries: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArchiveRejectionReason {
    CompressedSize,
    ExpandedSize,
    EntryCount,
    /// Two entries name the same file, or a file that should have been new already exists. Both
    /// mean the same thing to a caller: something is already at that name.
    DuplicatePath,
    UnsafePath,
    LinkEntry,
    EntryTooLarge,
    /// The bytes are not one well-formed archive, or they do not agree with what was declared.
    Format,
    EncryptedEntry,
    UnsupportedCompression,
}

/// Why an archive was refused, and which entry did it.
///
/// A struct rather than an enum carrying payloads, because the entry name is absent for exactly
/// the same reasons everywhere — the rejection is about the archive as a whole, or the name could
/// not be decoded at all — and an `Option` per variant would say that ten times.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ArchiveRejection {
    pub(crate) reason: ArchiveRejectionReason,
    pub(crate) entry: Option<String>,
}

impl ArchiveRejection {
    pub(crate) const fn new(reason: ArchiveRejectionReason) -> Self {
        Self {
            reason,
            entry: None,
        }
    }

    pub(crate) fn at(reason: ArchiveRejectionReason, entry: &str) -> Self {
        Self {
            reason,
            entry: Some(entry.to_string()),
        }
    }
}

/// Creating or removing the private directory an extraction runs inside failed.
///
/// One shape, not an `io::Error`: a caller cannot do anything different for a permission error
/// than for a name collision, and the underlying message may name a path that does not belong in
/// a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct StagingFailure;

/// Refuses an archive whose compressed size is already past its budget, before it is parsed.
pub(crate) fn check_compressed_size(
    byte_length: u64,
    limits: ArchiveLimits,
) -> Result<(), ArchiveRejection> {
    if byte_length > limits.maximum_compressed_bytes {
        return Err(ArchiveRejection::new(
            ArchiveRejectionReason::CompressedSize,
        ));
    }
    Ok(())
}

/// Applies the count, link, path, duplicate, per-entry, and running-total rules in one pass.
///
/// One pass, in entry order, is behavior a caller can depend on: the first entry that breaks a
/// rule decides the answer, and the rules within an entry run in the order written here. Callers
/// surface the resulting code to an operator, so reordering them changes what the operator is
/// told about an archive that breaks several rules at once.
///
/// `entry_budget` returns the per-entry cap, or `None` for an entry that is not subject to one —
/// a manifest the caller expects to be large, or a kind that carries no bytes of its own. Every
/// entry counts toward `maximum_expanded_bytes` regardless.
pub(crate) fn validate_entries(
    entries: &[ArchiveEntry],
    limits: ArchiveLimits,
    entry_budget: impl Fn(&ArchiveEntry) -> Option<u64>,
) -> Result<(), ArchiveRejection> {
    if entries.len() > limits.maximum_entries {
        return Err(ArchiveRejection::new(ArchiveRejectionReason::EntryCount));
    }
    let mut seen = BTreeSet::new();
    let mut expanded_bytes = 0_u64;
    for entry in entries {
        let refuse = |reason| Err(ArchiveRejection::at(reason, &entry.path));
        if matches!(
            entry.kind,
            ArchiveEntryKind::SymbolicLink | ArchiveEntryKind::HardLink
        ) {
            return refuse(ArchiveRejectionReason::LinkEntry);
        }
        if !is_safe_archive_entry_path(&entry.path) {
            return refuse(ArchiveRejectionReason::UnsafePath);
        }
        if !seen.insert(entry.path.clone()) {
            return refuse(ArchiveRejectionReason::DuplicatePath);
        }
        if entry_budget(entry).is_some_and(|budget| entry.expanded_bytes > budget) {
            return refuse(ArchiveRejectionReason::EntryTooLarge);
        }
        expanded_bytes = expanded_bytes
            .checked_add(entry.expanded_bytes)
            .ok_or_else(|| ArchiveRejection::new(ArchiveRejectionReason::ExpandedSize))?;
        if expanded_bytes > limits.maximum_expanded_bytes {
            return Err(ArchiveRejection::new(ArchiveRejectionReason::ExpandedSize));
        }
    }
    Ok(())
}

/// Runs `operation` inside a directory this call creates and this call removes.
///
/// The directory must not already exist. Extraction writes files it expects to be new, so
/// inheriting whatever an earlier run or another process left behind would silently mix two
/// packages together. Cleanup runs on every path, and a cleanup that fails turns a successful
/// operation into a failure: leaving unreviewed bytes behind is not a success.
pub(crate) fn with_isolated_staging<T, E>(
    staging: &Path,
    operation: impl FnOnce(&Path) -> Result<T, E>,
) -> Result<T, E>
where
    E: From<StagingFailure>,
{
    let Some(parent) = staging.parent() else {
        return Err(StagingFailure.into());
    };
    if std::fs::create_dir_all(parent).is_err() || std::fs::create_dir(staging).is_err() {
        return Err(StagingFailure.into());
    }
    let result = operation(staging);
    let cleanup = std::fs::remove_dir_all(staging);
    match (result, cleanup) {
        (Ok(value), Ok(())) => Ok(value),
        (Err(error), _) => Err(error),
        (Ok(_), Err(_)) => Err(StagingFailure.into()),
    }
}
