// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Paths a manifest may point at, validated as text before they become paths.
//!
//! The raw string is checked first, deliberately. `Path::components()` treats a backslash as an
//! ordinary filename character on Unix, so `..\..\etc\passwd` analyses as a single innocuous
//! component on a Linux CI runner and as a traversal on Windows. A rule expressed only in terms of
//! components is therefore green on one platform and a vulnerability on another.
//!
//! Nothing here normalizes. A path that is not portable is refused, because "repaired" input is
//! input the operator never reviewed and the signature never covered.

use super::ExtensionPathError;
use std::path::{Component, Path};

/// Long enough for a realistic `skills/<name>/references/<file>.md`, short enough that a hostile
/// manifest cannot use path length as an amplification vector.
pub(crate) const MAX_PACKAGE_PATH_CHARACTERS: usize = 240;
const MAX_PACKAGE_PATH_SEGMENTS: usize = 24;

/// Windows treats these as devices regardless of extension or directory, so a package containing
/// one cannot be extracted there even though it unpacks cleanly elsewhere.
const WINDOWS_RESERVED_NAMES: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// A relative, forward-slash path inside a package snapshot.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct PortablePackagePath(String);

impl PortablePackagePath {
    pub(crate) fn parse(value: &str) -> Result<Self, ExtensionPathError> {
        reject_raw_string(value)?;
        reject_segments(value)?;
        // Only now is it safe to involve `Path`, and only as a cross-check: every rejection above
        // is already decided on text that means the same thing on every platform.
        debug_assert!(Path::new(value)
            .components()
            .all(|component| matches!(component, Component::Normal(_))));
        Ok(Self(value.to_string()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    /// Lower-cased, for collision detection between two declared paths. Case-insensitive
    /// filesystems make `Skills/A.md` and `skills/a.md` the same file; a package declaring both is
    /// ambiguous rather than merely redundant.
    pub(crate) fn case_folded(&self) -> String {
        self.0.to_lowercase()
    }
}

fn reject_raw_string(value: &str) -> Result<(), ExtensionPathError> {
    let refuse = |reason: PathRejection| {
        Err(ExtensionPathError {
            path: value.chars().take(MAX_PACKAGE_PATH_CHARACTERS).collect(),
            reason,
        })
    };

    if value.is_empty() {
        return refuse(PathRejection::Empty);
    }
    if value.chars().count() > MAX_PACKAGE_PATH_CHARACTERS {
        return refuse(PathRejection::TooLong);
    }
    if value.contains('\0') {
        return refuse(PathRejection::NulByte);
    }
    if value.contains('\\') {
        return refuse(PathRejection::Backslash);
    }
    if value.chars().any(char::is_control) {
        return refuse(PathRejection::ControlCharacter);
    }
    // `//server/share` before the single-slash rule: a UNC path also starts with `/`, and naming
    // it "absolute" would send a publisher looking for a leading slash to remove.
    if value.starts_with("//") {
        return refuse(PathRejection::UncPrefix);
    }
    if value.starts_with('/') {
        return refuse(PathRejection::Absolute);
    }
    if has_drive_prefix(value) {
        return refuse(PathRejection::DrivePrefix);
    }
    // NTFS alternate data streams: `file.txt:hidden` writes bytes nothing else lists.
    if value.contains(':') {
        return refuse(PathRejection::AlternateDataStream);
    }
    Ok(())
}

fn reject_segments(value: &str) -> Result<(), ExtensionPathError> {
    let refuse = |reason: PathRejection| {
        Err(ExtensionPathError {
            path: value.chars().take(MAX_PACKAGE_PATH_CHARACTERS).collect(),
            reason,
        })
    };

    let segments: Vec<&str> = value.split('/').collect();
    if segments.len() > MAX_PACKAGE_PATH_SEGMENTS {
        return refuse(PathRejection::TooDeep);
    }
    for segment in segments {
        if segment.is_empty() {
            return refuse(PathRejection::EmptySegment);
        }
        if segment == "." {
            return refuse(PathRejection::CurrentDirectorySegment);
        }
        if segment == ".." {
            return refuse(PathRejection::ParentDirectorySegment);
        }
        // Windows strips trailing dots and spaces, so `a. ` and `a` become the same entry there
        // while staying distinct in the manifest.
        if segment.ends_with('.') || segment.ends_with(' ') || segment.starts_with(' ') {
            return refuse(PathRejection::TrailingDotOrSpace);
        }
        if is_windows_reserved(segment) {
            return refuse(PathRejection::WindowsReservedName);
        }
    }
    Ok(())
}

/// `c:/x` and `c:x`. Checked on the raw string because a drive-relative path is not absolute and
/// would otherwise pass every other rule.
fn has_drive_prefix(value: &str) -> bool {
    let mut characters = value.chars();
    matches!(
        (characters.next(), characters.next()),
        (Some(letter), Some(':')) if letter.is_ascii_alphabetic()
    )
}

fn is_windows_reserved(segment: &str) -> bool {
    // The device name is reserved with any extension, so compare the stem.
    let stem = segment.split('.').next().unwrap_or(segment);
    WINDOWS_RESERVED_NAMES
        .iter()
        .any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// Why a declared path is not portable. One variant per rule so a diagnostic can say which.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PathRejection {
    Empty,
    TooLong,
    TooDeep,
    NulByte,
    Backslash,
    ControlCharacter,
    Absolute,
    UncPrefix,
    DrivePrefix,
    AlternateDataStream,
    EmptySegment,
    CurrentDirectorySegment,
    ParentDirectorySegment,
    TrailingDotOrSpace,
    WindowsReservedName,
}

impl PathRejection {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::TooLong => "too_long",
            Self::TooDeep => "too_deep",
            Self::NulByte => "nul_byte",
            Self::Backslash => "backslash",
            Self::ControlCharacter => "control_character",
            Self::Absolute => "absolute",
            Self::UncPrefix => "unc_prefix",
            Self::DrivePrefix => "drive_prefix",
            Self::AlternateDataStream => "alternate_data_stream",
            Self::EmptySegment => "empty_segment",
            Self::CurrentDirectorySegment => "current_directory_segment",
            Self::ParentDirectorySegment => "parent_directory_segment",
            Self::TrailingDotOrSpace => "trailing_dot_or_space",
            Self::WindowsReservedName => "windows_reserved_name",
        }
    }
}
