//! Unpacking a verified archive into a directory this process owns.
//!
//! Split the same way the download is: the part that must not be duplicated is the containment and
//! the limits, not the format. `ExtractionGuard` owns those, and a format adapter feeds it one
//! entry at a time. A second archive format is a second adapter, not a second set of bounds.
//!
//! The containment check runs on each entry's **resolved** path rather than on its name. Scanning
//! for a leading separator or a literal `..` misses `a/../../b`, which normalizes out of the
//! destination while looking ordinary.

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};

use crate::contexts::tooling::managed_install::domain::error::ManagedInstallError;
use crate::platform::logging::redact_text;

/// What an archive may expand to. Declared by the contributor for the same reason the download
/// ceiling is: an archive's compressed size says nothing about its expanded size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the archive kind ships tested and without a caller; add-lsp-java-jdtls is the consumer"
    )
)]
pub(crate) struct ExtractionLimits {
    pub(crate) max_total_bytes: u64,
    pub(crate) max_entries: usize,
}

impl ExtractionLimits {
    pub(crate) const fn is_bounded(&self) -> bool {
        self.max_total_bytes > 0 && self.max_entries > 0
    }
}

/// A directory this process created and will remove unless extraction completes.
#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the archive kind ships tested and without a caller; add-lsp-java-jdtls is the consumer"
    )
)]
pub(crate) struct ExtractedArchive {
    pub(crate) directory: tempfile::TempDir,
}

/// Enforces containment and the limits while a format adapter walks an archive.
///
/// Holds the destination so a failure anywhere removes everything already written: a partially
/// unpacked tool is worse than none, because it looks installed.
#[derive(Debug)]
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the archive kind ships tested and without a caller; add-lsp-java-jdtls is the consumer"
    )
)]
pub(crate) struct ExtractionGuard {
    destination: tempfile::TempDir,
    limits: ExtractionLimits,
    written_bytes: u64,
    written_entries: usize,
}

impl ExtractionGuard {
    pub(crate) fn new(limits: ExtractionLimits) -> Result<Self, ManagedInstallError> {
        if !limits.is_bounded() {
            return Err(ManagedInstallError::Refused(
                "the archive declares no extraction limits".to_string(),
            ));
        }
        let destination = tempfile::Builder::new()
            .prefix("vanehub-managed-extract-")
            .tempdir()
            .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
        Ok(Self {
            destination,
            limits,
            written_bytes: 0,
            written_entries: 0,
        })
    }

    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "reached by the format adapter a consumer adds")
    )]
    pub(crate) fn destination(&self) -> &Path {
        self.destination.path()
    }

    /// Admits one entry and returns where it may be written, or refuses.
    ///
    /// The path is resolved against the destination and then checked to still be inside it, which
    /// is the only check that catches a name that normalizes its way out.
    pub(crate) fn admit(&mut self, entry_name: &str) -> Result<PathBuf, ManagedInstallError> {
        self.written_entries += 1;
        if self.written_entries > self.limits.max_entries {
            return Err(ManagedInstallError::Refused(format!(
                "the archive exceeded its {} entry limit",
                self.limits.max_entries
            )));
        }
        let candidate = Path::new(entry_name);
        // Rejected before resolution, because a drive-relative or root-relative name would
        // otherwise replace the destination entirely when joined.
        if candidate.is_absolute() || candidate.has_root() {
            return Err(ManagedInstallError::Refused(
                "the archive contains an absolute entry path".to_string(),
            ));
        }
        let mut resolved = self.destination.path().to_path_buf();
        for component in candidate.components() {
            match component {
                Component::Normal(part) => resolved.push(part),
                // `..` is refused rather than popped. Popping would let an archive walk to the
                // destination's parent and back into a sibling it was never given.
                Component::ParentDir => {
                    return Err(ManagedInstallError::Refused(
                        "the archive contains a parent-directory entry path".to_string(),
                    ))
                }
                Component::CurDir => {}
                Component::Prefix(_) | Component::RootDir => {
                    return Err(ManagedInstallError::Refused(
                        "the archive contains an absolute entry path".to_string(),
                    ))
                }
            }
        }
        // Belt and braces: the loop above already refuses every escaping component, and this
        // catches a platform where resolution does something the components did not describe.
        if !resolved.starts_with(self.destination.path()) {
            return Err(ManagedInstallError::Refused(
                "the archive entry resolves outside the destination".to_string(),
            ));
        }
        Ok(resolved)
    }

    /// Writes one admitted entry under the total-bytes ceiling, enforced while reading.
    pub(crate) fn write_entry(
        &mut self,
        path: &Path,
        mut body: impl Read,
    ) -> Result<(), ManagedInstallError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
        }
        let mut file = std::fs::File::create(path)
            .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
        let mut buffer = vec![0_u8; 64 * 1024];
        loop {
            let read = body
                .read(&mut buffer)
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
            if read == 0 {
                break;
            }
            self.written_bytes += read as u64;
            if self.written_bytes > self.limits.max_total_bytes {
                return Err(ManagedInstallError::Refused(format!(
                    "the archive exceeded its {} byte extraction ceiling",
                    self.limits.max_total_bytes
                )));
            }
            file.write_all(&buffer[..read])
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
        }
        file.flush()
            .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))
    }

    /// Hands the destination to the caller. Only reachable once every entry has been admitted and
    /// written, so a guard dropped on any failure path takes the partial directory with it.
    pub(crate) fn finish(self) -> ExtractedArchive {
        ExtractedArchive {
            directory: self.destination,
        }
    }
}

/// Unpacks a verified zip archive.
///
/// Zip because it needs no dependency this build does not already carry. A format that does --
/// `tar.gz` is the obvious next one -- is a new adapter plus a dependency decision, and both
/// belong to the change that has a consumer for them rather than to this one.
#[cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the archive kind ships tested and without a caller; add-lsp-java-jdtls is the consumer"
    )
)]
pub(crate) fn extract_zip(
    archive: &Path,
    limits: ExtractionLimits,
) -> Result<ExtractedArchive, ManagedInstallError> {
    let file = std::fs::File::open(archive)
        .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
    let mut zip = zip::ZipArchive::new(file)
        .map_err(|error| ManagedInstallError::Refused(redact_text(&error.to_string())))?;
    let mut guard = ExtractionGuard::new(limits)?;

    for index in 0..zip.len() {
        let mut entry = zip
            .by_index(index)
            .map_err(|error| ManagedInstallError::Refused(redact_text(&error.to_string())))?;
        // `name()` is the archive's own string. It is what the guard resolves, and the guard is
        // what decides whether it may be written -- this loop never joins it to a path itself.
        let name = entry.name().to_owned();
        if name.ends_with('/') {
            // A directory entry carries no bytes, but it still counts against the entry limit and
            // still has to be inside the destination.
            let path = guard.admit(&name)?;
            std::fs::create_dir_all(&path)
                .map_err(|error| ManagedInstallError::Transfer(redact_text(&error.to_string())))?;
            continue;
        }
        let path = guard.admit(&name)?;
        guard.write_entry(&path, &mut entry)?;
    }
    Ok(guard.finish())
}

#[cfg(test)]
#[path = "extraction_tests.rs"]
mod tests;
