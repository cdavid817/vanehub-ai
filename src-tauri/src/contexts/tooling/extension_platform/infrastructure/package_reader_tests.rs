//! Reading a real `.vhext`, and refusing the ones that should not be unpacked.

use super::{read_extension_package, PackageReadError, ReadPackage};
use crate::contexts::tooling::extension_platform::domain::{
    PackageLayoutRejection, DEFAULT_EXTENSION_PACKAGE_LIMITS, PACKAGE_MANIFEST_ENTRY,
};
use crate::platform::archive::ArchiveRejectionReason;
use crate::test_support::TempDirectory;
use semver::Version;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const MANIFEST: &str = "\
schema_version: 1
id: acme.git-guardian
display_name: Git Guardian
publisher: acme
version: 1.2.0
min_vanehub_version: \">=0.9.0\"
runtime:
  kind: wasm-module
  entry: runtime/guardian.wasm
  trust_profile: strict
";

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in entries {
        writer.start_file(*name, options).expect("start ZIP entry");
        writer.write_all(content).expect("write ZIP entry");
    }
    writer.finish().expect("finish ZIP").into_inner()
}

fn valid_package() -> Vec<u8> {
    zip_of(&[
        (PACKAGE_MANIFEST_ENTRY, MANIFEST.as_bytes()),
        ("runtime/guardian.wasm", b"\0asm\x01\0\0\0"),
        ("README.md", b"# Git Guardian\n"),
    ])
}

fn read(archive: &[u8], label: &str) -> Result<(), PackageReadError> {
    let home = TempDirectory::new(label);
    let staging = home.path().join("staging");
    let outcome = read_extension_package(
        archive,
        &staging,
        &Version::parse("1.0.0").expect("version"),
        DEFAULT_EXTENSION_PACKAGE_LIMITS,
    )
    .map(|_| ());
    assert!(
        !staging.exists(),
        "staging must not survive the call, whatever the answer"
    );
    outcome
}

#[test]
fn a_well_formed_package_reads_back_its_manifest_and_leaves_no_residue() {
    let home = TempDirectory::new("package-reader-ok");
    let staging = home.path().join("staging");

    let package: ReadPackage = read_extension_package(
        &valid_package(),
        &staging,
        &Version::parse("1.0.0").expect("version"),
        DEFAULT_EXTENSION_PACKAGE_LIMITS,
    )
    .expect("package should read");

    assert_eq!(package.manifest.id.as_str(), "acme.git-guardian");
    assert_eq!(
        package
            .manifest
            .runtime
            .entry
            .as_ref()
            .map(|entry| entry.as_str()),
        Some("runtime/guardian.wasm")
    );
    assert!(!staging.exists());
}

#[test]
fn a_package_whose_layout_is_refused_is_never_unpacked() {
    // The check that matters most about ordering: an archive with a traversal entry must be
    // refused from its own description, before a single byte is written anywhere.
    let home = TempDirectory::new("package-reader-not-unpacked");
    let staging = home.path().join("staging");
    let archive = zip_of(&[
        (PACKAGE_MANIFEST_ENTRY, MANIFEST.as_bytes()),
        ("runtime/CON", b"device"),
    ]);

    let outcome = read_extension_package(
        &archive,
        &staging,
        &Version::parse("1.0.0").expect("version"),
        DEFAULT_EXTENSION_PACKAGE_LIMITS,
    );

    assert!(matches!(outcome, Err(PackageReadError::Layout(_))));
    assert!(
        !staging.exists(),
        "the staging directory is never even created for a package refused on its description"
    );
}

#[test]
fn trailing_bytes_after_the_archive_are_refused() {
    let mut trailing = valid_package();
    trailing.extend_from_slice(b"appended");

    let outcome = read(&trailing, "package-reader-trailing");
    assert!(matches!(
        outcome,
        Err(PackageReadError::Archive(rejection))
            if rejection.reason == ArchiveRejectionReason::Format
    ));
}

#[test]
fn a_package_missing_its_manifest_is_refused_before_extraction() {
    let archive = zip_of(&[("runtime/guardian.wasm", b"\0asm")]);

    let outcome = read(&archive, "package-reader-no-manifest");
    assert!(matches!(
        outcome,
        Err(PackageReadError::Layout(violation))
            if violation.reason == PackageLayoutRejection::MissingManifest
    ));
}

#[test]
fn a_manifest_that_does_not_decode_is_reported_as_a_manifest_problem() {
    let archive = zip_of(&[(
        PACKAGE_MANIFEST_ENTRY,
        b"schema_version: 1\nid: acme.git-guardian\n",
    )]);

    let outcome = read(&archive, "package-reader-bad-manifest");
    match outcome {
        Err(PackageReadError::Manifest(error)) => {
            // The decoder reads identity before display, so `publisher` is the first field
            // it misses rather than the first one absent from the text.
            assert_eq!(
                (error.field(), error.code()),
                ("publisher", "missing_field")
            );
        }
        other => panic!("expected a manifest rejection, got {other:?}"),
    }
}

#[test]
fn a_manifest_that_points_outside_the_package_is_reported_with_everything_else_wrong() {
    // Reported as a list rather than the first problem: a publisher fixing a package wants all of
    // them, and nothing is executed to find the next one.
    let manifest = MANIFEST.replace("runtime/guardian.wasm", "runtime/missing.wasm");
    let archive = zip_of(&[
        (PACKAGE_MANIFEST_ENTRY, manifest.as_bytes()),
        ("runtime/guardian.wasm", b"\0asm"),
    ]);

    let outcome = read(&archive, "package-reader-dangling");
    match outcome {
        Err(PackageReadError::Consistency { layout, .. }) => {
            let codes: Vec<&str> = layout
                .iter()
                .map(|violation| violation.reason.code())
                .collect();
            assert!(
                codes.contains(&"package_declared_path_missing"),
                "{codes:?}"
            );
        }
        other => panic!("expected a consistency rejection, got {other:?}"),
    }
}

#[test]
fn an_executable_the_manifest_did_not_declare_is_refused() {
    let archive = zip_of(&[
        (PACKAGE_MANIFEST_ENTRY, MANIFEST.as_bytes()),
        ("runtime/guardian.wasm", b"\0asm"),
        ("runtime/helper.dll", b"MZ"),
    ]);

    let outcome = read(&archive, "package-reader-undeclared-executable");
    match outcome {
        Err(PackageReadError::Consistency { layout, .. }) => {
            assert_eq!(
                layout
                    .iter()
                    .map(|violation| violation.reason.code())
                    .collect::<Vec<_>>(),
                vec!["package_undeclared_executable"]
            );
        }
        other => panic!("expected a consistency rejection, got {other:?}"),
    }
}

#[test]
fn a_package_larger_than_its_compressed_budget_is_refused_before_it_is_parsed() {
    let tiny = crate::contexts::tooling::extension_platform::domain::ExtensionPackageLimits {
        maximum_compressed_bytes: 8,
        ..DEFAULT_EXTENSION_PACKAGE_LIMITS
    };
    let home = TempDirectory::new("package-reader-oversize");

    let outcome = read_extension_package(
        &valid_package(),
        &home.path().join("staging"),
        &Version::parse("1.0.0").expect("version"),
        tiny,
    );

    assert!(matches!(
        outcome,
        Err(PackageReadError::Archive(rejection))
            if rejection.reason == ArchiveRejectionReason::CompressedSize
    ));
}

#[test]
fn every_failure_carries_a_distinct_stable_code() {
    let mut codes = Vec::new();
    for (archive, label) in [
        (valid_package(), "code-ok"),
        (zip_of(&[("runtime/a.wasm", b"x")]), "code-no-manifest"),
        (
            zip_of(&[(PACKAGE_MANIFEST_ENTRY, b"schema_version: 1\n")]),
            "code-bad-manifest",
        ),
    ] {
        if let Err(error) = read(&archive, label) {
            codes.push(error.code());
        }
    }

    assert_eq!(codes, vec!["package_missing_manifest", "missing_field"]);
}
