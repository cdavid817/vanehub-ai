// The install flow that calls this lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Reading a `.vhext` from bytes to a decoded, checked package.
//!
//! The order is the point. Nothing is written to disk until the archive has said what it contains
//! and the domain has agreed that all of it is admissible, so a package that would refuse
//! extraction never gets extracted. Only then are bytes unpacked into a directory this call
//! creates and this call removes, and only from those bytes is a manifest read.
//!
//! ```text
//! bytes -> compressed budget -> exact end -> entry inspection -> layout -> extract -> manifest
//!                                                                                     |
//!                                                          integrity + declared paths -+
//! ```
//!
//! Every step here is glue. The archive mechanics belong to `crate::platform::archive` and every
//! rule belongs to the domain; what this file owns is the sequence and the mapping between them.

use crate::contexts::tooling::extension_platform::domain::{
    check_integrity, check_manifest_against_layout, inspect_package_layout, ExtensionManifestV1,
    ExtensionManifestV1Decoder, ExtensionPackageLimits, IntegrityViolation, ManifestDecodeError,
    PackageArchiveEntry, PackageLayout, PackageLayoutRejection, PackageLayoutViolation,
    VersionedExtensionManifest, EXTENSION_MANIFEST_YAML_LIMITS, PACKAGE_MANIFEST_ENTRY,
};
use crate::platform::archive::{
    check_compressed_size, ends_at_the_central_directory_record, extract_zip_entries,
    inspect_zip_entries, ArchiveEntry, ArchiveEntryKind, ArchiveLimits, ArchiveRejection,
    ArchiveRejectionReason, StagingFailure,
};
use semver::Version;
use std::path::Path;
use vanehub_bounded_yaml::parse_block;

/// Why a package could not be read.
///
/// Three families, kept apart because they call for different things from a publisher: the bytes
/// are not one well-formed archive, the archive's contents are not an admissible package, or the
/// manifest inside it does not describe a valid extension.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PackageReadError {
    Archive(ArchiveRejection),
    Layout(PackageLayoutViolation),
    Manifest(ManifestDecodeError),
    /// The manifest and the package disagree, or the manifest contradicts itself. Reported as a
    /// list, because a publisher fixing a package wants all of them.
    Consistency {
        layout: Vec<PackageLayoutViolation>,
        integrity: Vec<IntegrityViolation>,
    },
    /// Creating or removing the directory the package was unpacked into failed.
    Staging,
}

impl PackageReadError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Archive(rejection) => match rejection.reason {
                ArchiveRejectionReason::CompressedSize => "package_compressed_size",
                ArchiveRejectionReason::ExpandedSize => "package_expanded_size",
                ArchiveRejectionReason::EntryCount => "package_entry_count",
                ArchiveRejectionReason::DuplicatePath => "package_duplicate_path",
                ArchiveRejectionReason::UnsafePath => "package_unsafe_path",
                ArchiveRejectionReason::LinkEntry => "package_link_entry",
                ArchiveRejectionReason::EntryTooLarge => "package_entry_too_large",
                ArchiveRejectionReason::Format => "package_archive_format",
                ArchiveRejectionReason::EncryptedEntry => "package_encrypted_entry",
                ArchiveRejectionReason::UnsupportedCompression => "package_unsupported_compression",
            },
            Self::Layout(violation) => violation.reason.code(),
            Self::Manifest(error) => error.code(),
            Self::Consistency { .. } => "package_inconsistent",
            Self::Staging => "package_staging_failure",
        }
    }
}

impl From<ArchiveRejection> for PackageReadError {
    fn from(rejection: ArchiveRejection) -> Self {
        Self::Archive(rejection)
    }
}

impl From<StagingFailure> for PackageReadError {
    fn from(_: StagingFailure) -> Self {
        Self::Staging
    }
}

impl From<PackageLayoutViolation> for PackageReadError {
    fn from(violation: PackageLayoutViolation) -> Self {
        Self::Layout(violation)
    }
}

/// A package whose structure and manifest have both been checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReadPackage {
    pub(crate) manifest: ExtensionManifestV1,
    pub(crate) layout: PackageLayout,
}

/// Reads a `.vhext` and answers what is in it.
///
/// `staging` is where the archive is unpacked; the directory must not already exist and does not
/// survive the call. Extraction happens only after everything checkable without it has passed.
pub(crate) fn read_extension_package(
    archive_bytes: &[u8],
    staging: &Path,
    application_version: &Version,
    limits: ExtensionPackageLimits,
) -> Result<ReadPackage, PackageReadError> {
    check_compressed_size(archive_bytes.len() as u64, archive_limits(limits))?;
    if !ends_at_the_central_directory_record(archive_bytes) {
        return Err(PackageReadError::Archive(ArchiveRejection::new(
            ArchiveRejectionReason::Format,
        )));
    }

    // The profile refuses links and anything that is not a file or a directory. Everything else a
    // package may or may not contain is the domain's question, asked next over the whole list.
    let entries: Vec<ArchiveEntry> = inspect_zip_entries(archive_bytes, refuse_links)?;
    let layout = inspect_package_layout(
        &entries.iter().map(package_entry).collect::<Vec<_>>(),
        archive_bytes.len() as u64,
        limits,
    )?;

    crate::platform::archive::with_isolated_staging(staging, |root| {
        extract_zip_entries(archive_bytes, root, |_| limits.maximum_entry_bytes)?;
        let manifest = decode_manifest(root, application_version)?;

        let layout_violations = check_manifest_against_layout(&manifest, &layout);
        let integrity_violations = check_integrity(&manifest);
        if !layout_violations.is_empty() || !integrity_violations.is_empty() {
            return Err(PackageReadError::Consistency {
                layout: layout_violations,
                integrity: integrity_violations,
            });
        }

        Ok(ReadPackage {
            manifest,
            layout: layout.clone(),
        })
    })
}

/// The archive-level budgets, in the shape the shared reader takes them.
///
/// Only the three the shared reader knows about. Everything else — per-entry ceilings, the schema
/// cap, the compression ratio — is applied by the domain over the whole entry list, where it can
/// see which entry is which.
fn archive_limits(limits: ExtensionPackageLimits) -> ArchiveLimits {
    ArchiveLimits {
        maximum_compressed_bytes: limits.maximum_compressed_bytes,
        maximum_expanded_bytes: limits.maximum_expanded_bytes,
        maximum_entries: limits.maximum_entries,
    }
}

/// A package may contain files and directories. A link is neither, and following one is how an
/// extraction writes outside the directory it was given.
fn refuse_links(entry: &ArchiveEntry) -> Result<(), PackageReadError> {
    match entry.kind {
        ArchiveEntryKind::File | ArchiveEntryKind::Directory => Ok(()),
        ArchiveEntryKind::SymbolicLink | ArchiveEntryKind::HardLink => {
            Err(PackageReadError::Archive(ArchiveRejection::at(
                ArchiveRejectionReason::LinkEntry,
                &entry.path,
            )))
        }
    }
}

fn package_entry(entry: &ArchiveEntry) -> PackageArchiveEntry {
    PackageArchiveEntry {
        path: entry.path.clone(),
        is_directory: entry.kind == ArchiveEntryKind::Directory,
        expanded_bytes: entry.expanded_bytes,
        unix_mode: entry.unix_mode,
    }
}

fn decode_manifest(
    root: &Path,
    application_version: &Version,
) -> Result<ExtensionManifestV1, PackageReadError> {
    let bytes = std::fs::read(root.join(PACKAGE_MANIFEST_ENTRY)).map_err(|_| {
        // The layout pass already established the entry exists, so a read failure here is a
        // filesystem problem rather than a package problem.
        PackageReadError::Staging
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|_| {
        PackageReadError::Layout(PackageLayoutViolation {
            entry: PACKAGE_MANIFEST_ENTRY.to_string(),
            reason: PackageLayoutRejection::UnexpectedLocation,
        })
    })?;
    let document = parse_block(text, EXTENSION_MANIFEST_YAML_LIMITS).map_err(|error| {
        PackageReadError::Manifest(ManifestDecodeError::new(
            "",
            crate::contexts::tooling::extension_platform::domain::DecodeReason::MalformedDocument {
                code: error.code(),
            },
        ))
    })?;
    let decoded = ExtensionManifestV1Decoder::new(application_version.clone())
        .decode(&document)
        .map_err(PackageReadError::Manifest)?;
    match decoded {
        VersionedExtensionManifest::V1(manifest) => Ok(manifest),
    }
}
