// The package reader that calls this lands with the install flow in Task Group 4; see
// `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What may be inside a `.vhext`, decided before any of it is written to disk.
//!
//! Two passes, and the split matters. The first looks only at what the archive *says*: names,
//! kinds, declared sizes. It runs before extraction, so a package that should not be unpacked
//! never is. The second runs once the manifest has been decoded and answers the questions that
//! need both halves in view — does every declared path exist, and is every executable one the
//! runtime the manifest actually declared.
//!
//! The entry names are held to `PortablePackagePath`, the same rule manifest-declared paths get.
//! `crate::platform::archive`'s entry-name check is the mechanical floor beneath it — it refuses
//! separators and traversal on every platform — and is deliberately weaker: it admits NUL bytes,
//! Windows device names, and trailing dots, which this layer refuses. A package is written out to
//! a real filesystem, so it gets the strict rule.

use super::{
    ExtensionManifestV1, PathRejection, PortablePackagePath, RuntimeKind,
    MAX_PACKAGE_PATH_CHARACTERS,
};
use std::collections::BTreeMap;

/// The manifest, at the package root, under exactly this name.
pub(crate) const PACKAGE_MANIFEST_ENTRY: &str = "vanehub-extension.yaml";

/// Top-level directories a package may use, and what each is for.
///
/// An allowlist rather than a denylist. A package that puts files somewhere unexpected is either
/// built by a tool this build does not understand or is trying to reach somewhere it should not,
/// and both are better answered with "not a place packages put things" than with a guess.
pub(crate) const PACKAGE_DIRECTORIES: [&str; 4] = ["runtime", "schemas", "skills", "assets"];

/// Root files a package may carry besides the manifest. Documentation, and nothing that runs.
pub(crate) const PACKAGE_ROOT_FILES: [&str; 4] = ["LICENSE", "LICENSE.md", "README.md", "NOTICE"];

/// Suffixes that make a file executable, or loadable as code, on some platform this application
/// runs on.
///
/// Checked in addition to the Unix permission bits, because a ZIP written on Windows records no
/// mode at all and `.exe` is every bit as executable for having arrived without one.
const EXECUTABLE_SUFFIXES: [&str; 10] = [
    ".exe", ".dll", ".so", ".dylib", ".bat", ".cmd", ".com", ".ps1", ".sh", ".scr",
];

/// The ceilings from `design.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ExtensionPackageLimits {
    pub(crate) maximum_compressed_bytes: u64,
    pub(crate) maximum_expanded_bytes: u64,
    pub(crate) maximum_entries: usize,
    pub(crate) maximum_entry_bytes: u64,
    /// A declared JSON Schema is a description, not a payload. Result and log ceilings belong to
    /// the runtime trust profile rather than here, because they bound what an extension *produces*
    /// rather than what it ships.
    pub(crate) maximum_schema_bytes: u64,
    pub(crate) maximum_path_characters: usize,
    /// Expanded bytes per compressed byte, across the archive. A cheap pre-filter over what the
    /// central directory *declares*; extraction is what actually enforces a budget, because the
    /// central directory is written by whoever produced the file.
    pub(crate) maximum_compression_ratio: u64,
}

pub(crate) const DEFAULT_EXTENSION_PACKAGE_LIMITS: ExtensionPackageLimits =
    ExtensionPackageLimits {
        maximum_compressed_bytes: 64 * 1024 * 1024,
        maximum_expanded_bytes: 256 * 1024 * 1024,
        maximum_entries: 2_048,
        maximum_entry_bytes: 128 * 1024 * 1024,
        maximum_schema_bytes: 256 * 1024,
        maximum_path_characters: MAX_PACKAGE_PATH_CHARACTERS,
        maximum_compression_ratio: 100,
    };

/// One entry, as the archive describes it.
///
/// The domain's own shape rather than the archive reader's: a domain type may not reach into
/// `crate::platform`, and a package layout rule has no business knowing what a ZIP central
/// directory is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageArchiveEntry {
    pub(crate) path: String,
    pub(crate) is_directory: bool,
    pub(crate) expanded_bytes: u64,
    /// Unix permission bits, when the archive recorded any.
    pub(crate) unix_mode: Option<u32>,
}

/// The file-type field of a Unix mode, and the two values a package entry may carry.
const UNIX_FILE_TYPE_MASK: u32 = 0o170_000;
const UNIX_REGULAR_FILE: u32 = 0o100_000;
const UNIX_DIRECTORY: u32 = 0o040_000;

impl PackageArchiveEntry {
    /// Whether the recorded mode says this is something other than a file or a directory.
    ///
    /// ZIP has no entry kind for a device, a socket, or a FIFO; what it has is the Unix mode an
    /// archiver copied out of `stat`, and an extractor that ignores the file-type bits will happily
    /// create whatever it names. A mode of zero means the archive recorded none, which is ordinary
    /// for anything written on Windows and is not a claim about the type.
    fn is_unsupported_kind(&self) -> bool {
        self.unix_mode.is_some_and(|mode| {
            let file_type = mode & UNIX_FILE_TYPE_MASK;
            file_type != 0 && file_type != UNIX_REGULAR_FILE && file_type != UNIX_DIRECTORY
        })
    }

    /// Whether this entry would land on disk as something that can be run.
    fn is_executable(&self) -> bool {
        let by_mode = self.unix_mode.is_some_and(|mode| mode & 0o111 != 0);
        let lowered = self.path.to_ascii_lowercase();
        by_mode
            || EXECUTABLE_SUFFIXES
                .iter()
                .any(|suffix| lowered.ends_with(suffix))
    }
}

/// Why a package's contents are not admissible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageLayoutViolation {
    /// The entry that broke the rule, already bounded by whatever rejected it.
    pub(crate) entry: String,
    pub(crate) reason: PackageLayoutRejection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PackageLayoutRejection {
    /// The name is not a portable path. Carries which rule it broke, because "escapes the package"
    /// and "is a device name on Windows" call for different fixes.
    UnsafePath(PathRejection),
    /// Not the manifest, not a documented root file, and not under a directory packages use.
    UnexpectedLocation,
    MissingManifest,
    EntryTooLarge,
    ExpandedSize,
    EntryCount,
    CompressedSize,
    /// Declared expansion far outstrips declared compression.
    CompressionRatio,
    /// Two entries are the same file on a case-insensitive filesystem.
    CaseCollision {
        other: String,
    },
    /// Two entries are the same file after Unicode normalization.
    UnicodeCollision {
        other: String,
    },
    /// The same name twice.
    DuplicatePath,
    /// A file that can be run, which the manifest did not declare as its runtime entry.
    UndeclaredExecutable,
    /// A declared schema far larger than a schema.
    SchemaTooLarge,
    /// Not a regular file or a directory: a device, a socket, a FIFO, or a link.
    UnsupportedEntryKind,
    /// The manifest points at something the package does not contain.
    DeclaredPathMissing,
}

impl PackageLayoutRejection {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::UnsafePath(_) => "package_unsafe_path",
            Self::UnexpectedLocation => "package_unexpected_location",
            Self::MissingManifest => "package_missing_manifest",
            Self::EntryTooLarge => "package_entry_too_large",
            Self::ExpandedSize => "package_expanded_size",
            Self::EntryCount => "package_entry_count",
            Self::CompressedSize => "package_compressed_size",
            Self::CompressionRatio => "package_compression_ratio",
            Self::CaseCollision { .. } => "package_case_collision",
            Self::UnicodeCollision { .. } => "package_unicode_collision",
            Self::DuplicatePath => "package_duplicate_path",
            Self::UndeclaredExecutable => "package_undeclared_executable",
            Self::SchemaTooLarge => "package_schema_too_large",
            Self::UnsupportedEntryKind => "package_unsupported_entry_kind",
            Self::DeclaredPathMissing => "package_declared_path_missing",
        }
    }
}

/// Every layout rejection, for the catalog. The path family is registered through
/// `ALL_PATH_REJECTIONS` and is represented here by one member.
pub(crate) fn all_package_layout_rejections() -> Vec<PackageLayoutRejection> {
    vec![
        PackageLayoutRejection::UnsafePath(PathRejection::Backslash),
        PackageLayoutRejection::UnexpectedLocation,
        PackageLayoutRejection::MissingManifest,
        PackageLayoutRejection::EntryTooLarge,
        PackageLayoutRejection::ExpandedSize,
        PackageLayoutRejection::EntryCount,
        PackageLayoutRejection::CompressedSize,
        PackageLayoutRejection::CompressionRatio,
        PackageLayoutRejection::CaseCollision {
            other: String::new(),
        },
        PackageLayoutRejection::UnicodeCollision {
            other: String::new(),
        },
        PackageLayoutRejection::DuplicatePath,
        PackageLayoutRejection::UndeclaredExecutable,
        PackageLayoutRejection::SchemaTooLarge,
        PackageLayoutRejection::UnsupportedEntryKind,
        PackageLayoutRejection::DeclaredPathMissing,
    ]
}

/// What a package contains, once its structure is known to be admissible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PackageLayout {
    files: Vec<PortablePackagePath>,
    executables: Vec<PortablePackagePath>,
}

impl PackageLayout {
    pub(crate) fn files(&self) -> &[PortablePackagePath] {
        &self.files
    }

    pub(crate) fn executables(&self) -> &[PortablePackagePath] {
        &self.executables
    }

    pub(crate) fn contains(&self, path: &PortablePackagePath) -> bool {
        self.files.iter().any(|file| file == path)
    }
}

/// Reads the archive's own description of itself and decides whether any of it may be unpacked.
///
/// One violation, not a list. Everything here is a reason not to touch the file at all, and a
/// caller that kept going to collect more would be reading further into an archive it has already
/// decided to refuse.
pub(crate) fn inspect_package_layout(
    entries: &[PackageArchiveEntry],
    compressed_bytes: u64,
    limits: ExtensionPackageLimits,
) -> Result<PackageLayout, PackageLayoutViolation> {
    let whole = |reason| PackageLayoutViolation {
        entry: String::new(),
        reason,
    };

    if compressed_bytes > limits.maximum_compressed_bytes {
        return Err(whole(PackageLayoutRejection::CompressedSize));
    }
    if entries.len() > limits.maximum_entries {
        return Err(whole(PackageLayoutRejection::EntryCount));
    }

    let mut files = Vec::new();
    let mut executables = Vec::new();
    let mut by_exact: BTreeMap<String, ()> = BTreeMap::new();
    let mut by_case: BTreeMap<String, String> = BTreeMap::new();
    let mut by_composition: BTreeMap<String, String> = BTreeMap::new();
    let mut expanded_bytes = 0_u64;
    let mut manifest_seen = false;

    for entry in entries {
        let at = |reason| PackageLayoutViolation {
            entry: bounded(&entry.path, limits),
            reason,
        };

        // A ZIP directory entry ends in `/`, which is an empty final segment to a path rule that
        // knows nothing about ZIP. Strip it here rather than teaching the path rule about archives.
        let name = entry.path.strip_suffix('/').unwrap_or(&entry.path);
        let path = PortablePackagePath::parse(name)
            .map_err(|error| at(PackageLayoutRejection::UnsafePath(error.reason)))?;

        if by_exact.insert(path.as_str().to_string(), ()).is_some() {
            return Err(at(PackageLayoutRejection::DuplicatePath));
        }
        if let Some(other) = by_case.insert(path.case_folded(), path.as_str().to_string()) {
            return Err(at(PackageLayoutRejection::CaseCollision { other }));
        }
        if let Some(other) =
            by_composition.insert(path.composition_folded(), path.as_str().to_string())
        {
            return Err(at(PackageLayoutRejection::UnicodeCollision { other }));
        }

        if entry.is_unsupported_kind() {
            return Err(at(PackageLayoutRejection::UnsupportedEntryKind));
        }
        if !is_admissible_location(path.as_str(), entry.is_directory) {
            return Err(at(PackageLayoutRejection::UnexpectedLocation));
        }

        expanded_bytes = expanded_bytes
            .checked_add(entry.expanded_bytes)
            .ok_or_else(|| whole(PackageLayoutRejection::ExpandedSize))?;
        if expanded_bytes > limits.maximum_expanded_bytes {
            return Err(whole(PackageLayoutRejection::ExpandedSize));
        }

        if entry.is_directory {
            continue;
        }
        if entry.expanded_bytes > limits.maximum_entry_bytes {
            return Err(at(PackageLayoutRejection::EntryTooLarge));
        }
        if path.as_str().starts_with("schemas/")
            && entry.expanded_bytes > limits.maximum_schema_bytes
        {
            return Err(at(PackageLayoutRejection::SchemaTooLarge));
        }
        if path.as_str() == PACKAGE_MANIFEST_ENTRY {
            manifest_seen = true;
        }
        if entry.is_executable() {
            executables.push(path.clone());
        }
        files.push(path);
    }

    if !manifest_seen {
        return Err(whole(PackageLayoutRejection::MissingManifest));
    }
    if exceeds_compression_ratio(expanded_bytes, compressed_bytes, limits) {
        return Err(whole(PackageLayoutRejection::CompressionRatio));
    }

    Ok(PackageLayout { files, executables })
}

/// A package that claims to expand far beyond what it could plausibly hold.
///
/// An empty archive is exempt: zero expanded bytes is not a bomb, and the ratio of nothing to
/// anything is not a number worth acting on.
fn exceeds_compression_ratio(
    expanded_bytes: u64,
    compressed_bytes: u64,
    limits: ExtensionPackageLimits,
) -> bool {
    if expanded_bytes == 0 {
        return false;
    }
    match compressed_bytes.checked_mul(limits.maximum_compression_ratio) {
        Some(ceiling) => expanded_bytes > ceiling,
        // The ceiling overflowed, which means the archive is enormous and the expanded total
        // cannot possibly exceed it.
        None => false,
    }
}

fn is_admissible_location(path: &str, is_directory: bool) -> bool {
    match path.split_once('/') {
        Some((top, _)) => PACKAGE_DIRECTORIES.contains(&top),
        // A root entry: the manifest, a documented file, or one of the known directories itself.
        None if is_directory => PACKAGE_DIRECTORIES.contains(&path),
        None => path == PACKAGE_MANIFEST_ENTRY || PACKAGE_ROOT_FILES.contains(&path),
    }
}

fn bounded(value: &str, limits: ExtensionPackageLimits) -> String {
    value.chars().take(limits.maximum_path_characters).collect()
}

/// The second pass: what the manifest claims about the package, checked against what is in it.
///
/// Runs after decoding, because until then there is no list of declared paths and no runtime entry
/// to compare an executable against.
pub(crate) fn check_manifest_against_layout(
    manifest: &ExtensionManifestV1,
    layout: &PackageLayout,
) -> Vec<PackageLayoutViolation> {
    let mut violations = Vec::new();

    let mut declared: Vec<&PortablePackagePath> = manifest.contributes.declared_paths();
    if let Some(entry) = manifest.runtime.entry.as_ref() {
        declared.push(entry);
    }
    for path in &declared {
        if !layout.contains(path) {
            violations.push(PackageLayoutViolation {
                entry: path.as_str().to_string(),
                reason: PackageLayoutRejection::DeclaredPathMissing,
            });
        }
    }

    // A package may carry exactly one thing that runs, and only if it said so. A `.dll` beside a
    // declared `.wasm` is not cargo: it is a second program nobody reviewed, sitting inside a
    // snapshot the runtime can reach.
    let runtime_entry = manifest
        .runtime
        .entry
        .as_ref()
        .filter(|_| manifest.runtime.kind != RuntimeKind::None);
    for executable in layout.executables() {
        if runtime_entry != Some(executable) {
            violations.push(PackageLayoutViolation {
                entry: executable.as_str().to_string(),
                reason: PackageLayoutRejection::UndeclaredExecutable,
            });
        }
    }

    violations
}
