//! Reading a ZIP archive that arrived from outside the application.
//!
//! Every function here treats the archive as hostile input: the central directory is written by
//! whoever produced the file, so nothing it declares is believed without a matching check against
//! the bytes that are actually there.

use super::{ArchiveEntry, ArchiveEntryKind, ArchiveRejection, ArchiveRejectionReason};
use std::io::{Cursor, Read};
use std::path::Path;
use zip::{CompressionMethod, ZipArchive};

/// True when the end-of-central-directory record, plus the comment it declares, is the last thing
/// in the buffer.
///
/// A ZIP reader locates that record by scanning backwards, so bytes appended after it are ignored
/// by the reader while remaining part of the file a signature or hash covers. Refusing the
/// mismatch keeps "what was verified" and "what was read" the same sequence of bytes.
pub(crate) fn ends_at_the_central_directory_record(bytes: &[u8]) -> bool {
    const END_RECORD_BYTES: usize = 22;
    const MAXIMUM_COMMENT_BYTES: usize = u16::MAX as usize;
    if bytes.len() < END_RECORD_BYTES {
        return false;
    }
    let start = bytes
        .len()
        .saturating_sub(END_RECORD_BYTES + MAXIMUM_COMMENT_BYTES);
    (start..=bytes.len() - END_RECORD_BYTES)
        .rev()
        .find(|offset| bytes[*offset..].starts_with(b"PK\x05\x06"))
        .is_some_and(|offset| {
            let comment_length =
                u16::from_le_bytes([bytes[offset + 20], bytes[offset + 21]]) as usize;
            offset + END_RECORD_BYTES + comment_length == bytes.len()
        })
}

/// Enumerates every entry without expanding any of it.
///
/// `accept` is the caller's profile — which names and kinds belong in *this* kind of package. It
/// runs per entry, in archive order, so a caller's own rejection is reported at the first entry
/// that breaks it rather than after the whole archive has been walked.
pub(crate) fn inspect_zip_entries<E>(
    archive_bytes: &[u8],
    mut accept: impl FnMut(&ArchiveEntry) -> Result<(), E>,
) -> Result<Vec<ArchiveEntry>, E>
where
    E: From<ArchiveRejection>,
{
    let malformed = || ArchiveRejection::new(ArchiveRejectionReason::Format);
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes)).map_err(|_| malformed())?;
    // A non-zero offset means something was prepended — a self-extracting stub, or a second file
    // the reader silently skipped past.
    if archive.offset() != 0 {
        return Err(malformed().into());
    }
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        let file = archive.by_index(index).map_err(|_| malformed())?;
        // `name()` is the reader's sanitized interpretation; `name_raw()` is what the archive
        // actually says. A difference means the two disagree about which file this is, which is
        // the whole mechanism behind a traversal that reads back as harmless.
        let name = std::str::from_utf8(file.name_raw())
            .map_err(|_| ArchiveRejection::new(ArchiveRejectionReason::UnsafePath))?;
        if name != file.name() || file.enclosed_name().is_none() {
            return Err(ArchiveRejection::at(ArchiveRejectionReason::UnsafePath, name).into());
        }
        if file.encrypted() {
            return Err(ArchiveRejection::at(ArchiveRejectionReason::EncryptedEntry, name).into());
        }
        if !matches!(
            file.compression(),
            CompressionMethod::Stored | CompressionMethod::Deflated
        ) {
            return Err(
                ArchiveRejection::at(ArchiveRejectionReason::UnsupportedCompression, name).into(),
            );
        }
        let kind = if file.is_symlink() {
            ArchiveEntryKind::SymbolicLink
        } else if file.is_dir() {
            ArchiveEntryKind::Directory
        } else if file.is_file() {
            ArchiveEntryKind::File
        } else {
            ArchiveEntryKind::HardLink
        };
        let entry = ArchiveEntry {
            path: name.to_string(),
            kind,
            expanded_bytes: file.size(),
        };
        accept(&entry)?;
        entries.push(entry);
    }
    Ok(entries)
}

/// Writes every file entry beneath `destination`, never writing more than `entry_budget` allows
/// and never trusting the size the archive declared.
///
/// The budget is enforced by reading one byte past it and refusing if that byte arrives, so a
/// stream that expands without bound is stopped rather than measured. The declared size is then
/// compared against what was actually copied, because the central directory an attacker wrote is
/// what the caller's earlier limit checks were based on.
pub(crate) fn extract_zip_entries(
    archive_bytes: &[u8],
    destination: &Path,
    entry_budget: impl Fn(&str) -> u64,
) -> Result<(), ArchiveRejection> {
    let malformed = || ArchiveRejection::new(ArchiveRejectionReason::Format);
    let mut archive = ZipArchive::new(Cursor::new(archive_bytes)).map_err(|_| malformed())?;
    for index in 0..archive.len() {
        let mut source = archive.by_index(index).map_err(|_| malformed())?;
        if source.is_dir() {
            continue;
        }
        let name = std::str::from_utf8(source.name_raw())
            .map_err(|_| ArchiveRejection::new(ArchiveRejectionReason::UnsafePath))?
            .to_string();
        let target = destination.join(&name);
        let parent = target
            .parent()
            .ok_or_else(|| ArchiveRejection::at(ArchiveRejectionReason::UnsafePath, &name))?;
        std::fs::create_dir_all(parent)
            .map_err(|_| ArchiveRejection::at(ArchiveRejectionReason::UnsafePath, &name))?;
        let mut output = crate::platform::private_relay_fs::create_new_private_file(&target)
            .map_err(|_| ArchiveRejection::at(ArchiveRejectionReason::DuplicatePath, &name))?;
        let budget = entry_budget(&name);
        let declared_bytes = source.size();
        let copied = std::io::copy(&mut source.by_ref().take(budget + 1), &mut output)
            .map_err(|_| malformed())?;
        if copied > budget {
            return Err(ArchiveRejection::at(
                ArchiveRejectionReason::EntryTooLarge,
                &name,
            ));
        }
        if copied != declared_bytes {
            return Err(malformed());
        }
    }
    Ok(())
}
