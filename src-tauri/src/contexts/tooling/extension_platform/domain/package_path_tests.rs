//! Portable package path rules.
//!
//! Table-driven, one row per rule, because the value of this type is that no case was forgotten.
//! Several of these pass trivially on Windows and matter only on Linux, or the reverse — which is
//! exactly why they are asserted on text rather than on `Path`.

use super::{PathRejection, PortablePackagePath, MAX_PACKAGE_PATH_CHARACTERS};

fn rejection(value: &str) -> PathRejection {
    PortablePackagePath::parse(value)
        .expect_err("path should be rejected")
        .reason
}

#[test]
fn ordinary_relative_paths_are_accepted_unchanged() {
    for value in [
        "SKILL.md",
        "skills/guarded-reviewer/SKILL.md",
        "runtime/git_guardian.wasm",
        "schemas/git-status-input.json",
        "a",
        "a/b/c/d/e/f",
        "file.name.with.dots.md",
        "unicode-名前.md",
    ] {
        let path = PortablePackagePath::parse(value)
            .unwrap_or_else(|error| panic!("{value} should parse: {error}"));
        // Never normalized: what the manifest declared is what the signature covered.
        assert_eq!(path.as_str(), value);
    }
}

#[test]
fn a_backslash_is_rejected_on_every_platform() {
    // The rule this whole type exists for. `Path::components()` treats a backslash as an ordinary
    // filename character on Unix, so a component-only check passes this on a Linux runner and
    // fails on Windows — green locally, a traversal in production, or the reverse.
    for value in [
        r"..\..\etc\passwd",
        r"skills\SKILL.md",
        r"a\b",
        "mixed/separators\\file.md",
    ] {
        assert_eq!(
            rejection(value),
            PathRejection::Backslash,
            "{value:?} must be rejected as a backslash on every platform"
        );
    }
}

#[test]
fn traversal_and_current_directory_segments_are_rejected_rather_than_resolved() {
    assert_eq!(rejection(".."), PathRejection::ParentDirectorySegment);
    assert_eq!(rejection("../etc"), PathRejection::ParentDirectorySegment);
    assert_eq!(
        rejection("skills/../../etc"),
        PathRejection::ParentDirectorySegment
    );
    // `a/../a` resolves to `a`, and is still refused: normalizing it would accept input the
    // operator never reviewed.
    assert_eq!(rejection("a/../a"), PathRejection::ParentDirectorySegment);
    assert_eq!(rejection("."), PathRejection::CurrentDirectorySegment);
    assert_eq!(rejection("./a"), PathRejection::CurrentDirectorySegment);
}

#[test]
fn absolute_unc_and_drive_relative_paths_are_each_named() {
    assert_eq!(rejection("/etc/passwd"), PathRejection::Absolute);
    assert_eq!(rejection("//server/share/file"), PathRejection::UncPrefix);
    // Drive-relative is not absolute and would pass every other rule.
    assert_eq!(rejection("c:/windows"), PathRejection::DrivePrefix);
    assert_eq!(rejection("C:file.md"), PathRejection::DrivePrefix);
}

#[test]
fn an_alternate_data_stream_is_rejected() {
    // `file.txt:hidden` writes bytes that nothing lists on NTFS.
    assert_eq!(
        rejection("file.txt:hidden"),
        PathRejection::AlternateDataStream
    );
    assert_eq!(
        rejection("skills/a.md:stream"),
        PathRejection::AlternateDataStream
    );
}

#[test]
fn windows_device_names_are_rejected_with_or_without_an_extension() {
    for value in ["con", "CON", "nul.txt", "skills/aux", "com1", "LPT9.md"] {
        assert_eq!(
            rejection(value),
            PathRejection::WindowsReservedName,
            "{value} is a device name on Windows"
        );
    }
    // Only the exact device stem is reserved; a longer name is fine.
    assert!(PortablePackagePath::parse("console.md").is_ok());
    assert!(PortablePackagePath::parse("nullable").is_ok());
}

#[test]
fn trailing_dots_and_spaces_are_rejected_because_windows_strips_them() {
    for value in ["a.", "a ", " a", "skills/a./b", "dir /file"] {
        assert_eq!(
            rejection(value),
            PathRejection::TrailingDotOrSpace,
            "{value:?} would collapse to a different name on Windows"
        );
    }
}

#[test]
fn empty_segments_nul_bytes_and_control_characters_are_rejected() {
    assert_eq!(rejection(""), PathRejection::Empty);
    assert_eq!(rejection("a//b"), PathRejection::EmptySegment);
    assert_eq!(rejection("a/"), PathRejection::EmptySegment);
    assert_eq!(rejection("a\0b"), PathRejection::NulByte);
    assert_eq!(rejection("a\nb"), PathRejection::ControlCharacter);
    assert_eq!(rejection("a\tb"), PathRejection::ControlCharacter);
}

#[test]
fn length_and_depth_are_bounded() {
    let at_limit = "a".repeat(MAX_PACKAGE_PATH_CHARACTERS);
    assert!(PortablePackagePath::parse(&at_limit).is_ok());
    assert_eq!(
        rejection(&"a".repeat(MAX_PACKAGE_PATH_CHARACTERS + 1)),
        PathRejection::TooLong
    );

    let deep = (0..25).map(|_| "a").collect::<Vec<_>>().join("/");
    assert_eq!(rejection(&deep), PathRejection::TooDeep);
}

#[test]
fn case_folding_exposes_collisions_on_case_insensitive_filesystems() {
    let upper = PortablePackagePath::parse("Skills/A.md").expect("parse");
    let lower = PortablePackagePath::parse("skills/a.md").expect("parse");

    // Distinct declarations, one file on macOS and Windows. A manifest containing both is
    // ambiguous, and only the folded form makes that visible.
    assert_ne!(upper, lower);
    assert_eq!(upper.case_folded(), lower.case_folded());
}

#[test]
fn a_rejection_carries_a_bounded_path_and_a_specific_reason_code() {
    let hostile = format!("{}/../x", "a".repeat(1_000));
    let error = PortablePackagePath::parse(&hostile).expect_err("rejected");

    assert_eq!(error.code(), "invalid_package_path");
    assert_eq!(error.reason_code(), "too_long");
    assert_eq!(error.path.chars().count(), MAX_PACKAGE_PATH_CHARACTERS);
}

#[test]
fn every_rejection_reason_has_its_own_code() {
    let reasons = [
        PathRejection::Empty,
        PathRejection::TooLong,
        PathRejection::TooDeep,
        PathRejection::NulByte,
        PathRejection::Backslash,
        PathRejection::ControlCharacter,
        PathRejection::Absolute,
        PathRejection::UncPrefix,
        PathRejection::DrivePrefix,
        PathRejection::AlternateDataStream,
        PathRejection::EmptySegment,
        PathRejection::CurrentDirectorySegment,
        PathRejection::ParentDirectorySegment,
        PathRejection::TrailingDotOrSpace,
        PathRejection::WindowsReservedName,
    ];

    let mut codes: Vec<&str> = reasons.iter().map(|reason| reason.as_str()).collect();
    let total = codes.len();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), total, "each reason needs a distinct code");
}
