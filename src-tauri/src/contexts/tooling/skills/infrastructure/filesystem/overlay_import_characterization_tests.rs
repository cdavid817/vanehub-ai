//! What the Overlay importer accepts and refuses today, pinned before task 2.1 moves its
//! archive, path, and content-address primitives to `crate::platform`.
//!
//! The tests deliberately go through `parse_overlay_import_archive` and
//! `validate_overlay_import_probe` rather than the private helpers. Those two entry points are
//! what Skills keeps after the extraction, so a move that changes an internal boundary but leaves
//! the observable answer intact is allowed, and one that changes the answer is not.
//!
//! Several assertions below record a *gap* rather than a guarantee. A characterization test that
//! only pins the pleasant half of the behavior is how a rewrite quietly tightens or loosens a
//! rule; the gaps are the part most likely to be "fixed" by accident during a move.

use super::overlay_import::{
    parse_overlay_import_archive, validate_overlay_import_probe, OverlayImportProbe,
    OverlayImportValidationError,
};
use super::overlay_manifest::serialize_overlay_manifest;
use crate::contexts::tooling::skills::domain::{
    OverlayBaseWitness, OverlayDocument, OverlayFile, OverlayScope, OverlayTrust, SkillId,
    DEFAULT_OVERLAY_LIMITS, OVERLAY_SCHEMA_VERSION,
};
use crate::platform::archive::{with_isolated_staging, ArchiveEntry, ArchiveEntryKind};
use crate::test_support::TempDirectory;
use sha2::{Digest, Sha256};
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

const MANIFEST_ENTRY: &str = "overlay.json";
const PAYLOAD_PREFIX: &str = "payloads/sha256/";

type Rejection = OverlayImportValidationError;

fn entry(path: &str, kind: ArchiveEntryKind, expanded_bytes: u64) -> ArchiveEntry {
    ArchiveEntry {
        path: path.to_string(),
        kind,
        expanded_bytes,
    }
}

fn file(path: &str, expanded_bytes: u64) -> ArchiveEntry {
    entry(path, ArchiveEntryKind::File, expanded_bytes)
}

fn probe(
    entries: &[ArchiveEntry],
    compressed_bytes: u64,
    mutation_count: usize,
    schema_version: u32,
) -> Result<(), Rejection> {
    validate_overlay_import_probe(
        &OverlayImportProbe {
            schema_version,
            compressed_bytes,
            mutation_count,
            entries,
        },
        DEFAULT_OVERLAY_LIMITS,
    )
}

fn paths_only(paths: &[&str]) -> Result<(), Rejection> {
    let entries = paths.iter().map(|path| file(path, 0)).collect::<Vec<_>>();
    probe(&entries, 1, 0, OVERLAY_SCHEMA_VERSION)
}

fn sha256_hex(content: &[u8]) -> String {
    Sha256::digest(content)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn zip_package(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in entries {
        writer.start_file(*name, options).expect("start ZIP entry");
        writer.write_all(content).expect("write ZIP entry");
    }
    writer.finish().expect("finish ZIP").into_inner()
}

fn manifest_for(content: &[u8]) -> Vec<u8> {
    let content_hash = sha256_hex(content);
    let mut document = OverlayDocument::new(
        SkillId::parse("imported-overlay").expect("Skill id"),
        OverlayScope::User,
        None,
        OverlayBaseWitness::new("system:imported-overlay:v1", "base-text", "base-package")
            .expect("base witness"),
        OverlayTrust::trusted_local(1),
        "2026-08-11T00:00:00Z",
    )
    .expect("Overlay document");
    document.files.push(
        OverlayFile::new(
            "file-1",
            "references/import.md",
            "text/markdown",
            content.len() as u64,
            &content_hash,
            &format!("sha256/{content_hash}"),
            "2026-08-11T00:00:00Z",
        )
        .expect("Overlay file"),
    );
    serialize_overlay_manifest(&document).expect("manifest")
}

fn valid_package(content: &[u8]) -> Vec<u8> {
    zip_package(&[
        (MANIFEST_ENTRY, &manifest_for(content)),
        (&format!("{PAYLOAD_PREFIX}{}", sha256_hex(content)), content),
    ])
}

fn import(archive: &[u8], label: &str) -> Result<(), Rejection> {
    let quarantine = TempDirectory::new(label);
    let outcome = parse_overlay_import_archive(
        archive,
        quarantine.path(),
        "characterization.zip",
        DEFAULT_OVERLAY_LIMITS,
    )
    .map(|_| ());
    assert_eq!(
        std::fs::read_dir(quarantine.path())
            .expect("quarantine root")
            .count(),
        0,
        "staging must never survive the call, whatever the answer"
    );
    outcome
}

#[test]
fn probe_checks_its_limits_in_a_fixed_order() {
    // Each case violates every rule below it as well; the reported code is the first check, not
    // the worst problem. Callers surface this code to an operator, so the order is behavior.
    let oversize_link = entry("../escape.md", ArchiveEntryKind::SymbolicLink, u64::MAX);
    assert_eq!(
        probe(
            std::slice::from_ref(&oversize_link),
            DEFAULT_OVERLAY_LIMITS.maximum_import_bytes + 1,
            DEFAULT_OVERLAY_LIMITS.maximum_mutations + 1,
            OVERLAY_SCHEMA_VERSION + 1,
        ),
        Err(Rejection::CompressedSize)
    );
    assert_eq!(
        probe(
            std::slice::from_ref(&oversize_link),
            1,
            DEFAULT_OVERLAY_LIMITS.maximum_mutations + 1,
            OVERLAY_SCHEMA_VERSION + 1,
        ),
        Err(Rejection::UnsupportedVersion)
    );
    assert_eq!(
        probe(
            std::slice::from_ref(&oversize_link),
            1,
            DEFAULT_OVERLAY_LIMITS.maximum_mutations + 1,
            OVERLAY_SCHEMA_VERSION,
        ),
        Err(Rejection::MutationCount)
    );
    // The entry rules are ordered inside one entry — link, path, duplicate, size — and the entries
    // themselves are walked in order, so a size problem in the first entry is reported ahead of a
    // duplicate introduced by the second. The distinction matters: this is one pass, not four.
    assert_eq!(
        probe(&[oversize_link], 1, 0, OVERLAY_SCHEMA_VERSION),
        Err(Rejection::LinkEntry)
    );
    assert_eq!(
        probe(
            &[file("../escape.md", u64::MAX)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Err(Rejection::UnsafePath)
    );
    assert_eq!(
        probe(
            &[file("a.md", 0), file("a.md", u64::MAX)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION,
        ),
        Err(Rejection::DuplicatePath),
        "the duplicate is noticed before that same entry is measured"
    );
    assert_eq!(
        probe(
            &[file("a.md", u64::MAX), file("a.md", 0)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION,
        ),
        Err(Rejection::SupportingFileSize),
        "the first offending entry decides the answer, so the duplicate is never reached"
    );
}

#[test]
fn probe_accepts_each_limit_exactly_and_refuses_one_past_it() {
    let limits = DEFAULT_OVERLAY_LIMITS;
    assert_eq!(
        probe(&[], limits.maximum_import_bytes, 0, OVERLAY_SCHEMA_VERSION),
        Ok(())
    );
    assert_eq!(
        probe(
            &[],
            limits.maximum_import_bytes + 1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Err(Rejection::CompressedSize)
    );

    assert_eq!(
        probe(&[], 1, limits.maximum_mutations, OVERLAY_SCHEMA_VERSION),
        Ok(())
    );
    assert_eq!(
        probe(&[], 1, limits.maximum_mutations + 1, OVERLAY_SCHEMA_VERSION),
        Err(Rejection::MutationCount)
    );

    let at_entry_limit = (0..limits.maximum_archive_entries)
        .map(|index| file(&format!("references/{index}.md"), 0))
        .collect::<Vec<_>>();
    assert_eq!(probe(&at_entry_limit, 1, 0, OVERLAY_SCHEMA_VERSION), Ok(()));
    let past_entry_limit = (0..=limits.maximum_archive_entries)
        .map(|index| file(&format!("references/{index}.md"), 0))
        .collect::<Vec<_>>();
    assert_eq!(
        probe(&past_entry_limit, 1, 0, OVERLAY_SCHEMA_VERSION),
        Err(Rejection::EntryCount)
    );

    assert_eq!(
        probe(
            &[file("a.md", limits.maximum_supporting_file_bytes)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Ok(())
    );
    assert_eq!(
        probe(
            &[file("a.md", limits.maximum_supporting_file_bytes + 1)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Err(Rejection::SupportingFileSize)
    );
}

#[test]
fn only_the_manifest_escapes_the_per_file_limit_and_only_files_are_measured() {
    let limits = DEFAULT_OVERLAY_LIMITS;
    // The manifest is exempt from the per-file cap but not from the expanded total.
    assert_eq!(
        probe(
            &[file(MANIFEST_ENTRY, limits.maximum_expanded_import_bytes)],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Ok(())
    );
    assert_eq!(
        probe(
            &[file(
                MANIFEST_ENTRY,
                limits.maximum_expanded_import_bytes + 1
            )],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Err(Rejection::ExpandedSize)
    );
    // A directory carrying a declared size is not checked against the per-file cap, yet its bytes
    // still count toward the expanded total.
    assert_eq!(
        probe(
            &[entry(
                "references",
                ArchiveEntryKind::Directory,
                limits.maximum_supporting_file_bytes + 1
            )],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Ok(())
    );
    assert_eq!(
        probe(
            &[
                entry("a", ArchiveEntryKind::Directory, u64::MAX),
                entry("b", ArchiveEntryKind::Directory, u64::MAX),
            ],
            1,
            0,
            OVERLAY_SCHEMA_VERSION
        ),
        Err(Rejection::ExpandedSize),
        "the running total saturates into a rejection rather than wrapping"
    );
}

#[test]
fn archive_paths_are_accepted_or_refused_by_component() {
    for accepted in [
        "a.md",
        "references/a.md",
        "a/b/c/d/e/f/g/h/i/j.md",
        "references/",
        "payloads/sha256/",
        "UPPER.MD",
        "ünïcode.md",
        "name with spaces.md",
    ] {
        assert_eq!(
            paths_only(&[accepted]),
            Ok(()),
            "{accepted} should be accepted"
        );
    }

    for refused in [
        "",
        "/",
        "/absolute.md",
        "\\absolute.md",
        "references\\a.md",
        "C:/a.md",
        "a:b.md",
        "../escape.md",
        "a/../b.md",
        "a/./b.md",
        "a//b.md",
        ".hidden",
        "references/.hidden",
        ".git/config",
        "..",
    ] {
        assert_eq!(
            paths_only(&[refused]),
            Err(Rejection::UnsafePath),
            "{refused} should be refused"
        );
    }
}

#[test]
fn the_archive_path_check_stops_at_traversal_and_separators() {
    // Recorded as a gap, not a guarantee. These are refused later — by the entry profile, which
    // only admits `overlay.json` and `payloads/sha256/<64 lowercase hex>` — but the path check
    // itself passes them, so nothing downstream of an extraction may assume otherwise.
    for accepted in [
        "trailing-space.md ",
        "trailing-dot.md.",
        "CON",
        "aux.md",
        "stream.md:$DATA",
        "\u{7f}del.md",
    ] {
        let outcome = paths_only(&[accepted]);
        if accepted.contains(':') {
            assert_eq!(outcome, Err(Rejection::UnsafePath), "{accepted}");
        } else {
            assert_eq!(outcome, Ok(()), "{accepted} passes the path check today");
        }
    }
    assert_eq!(
        paths_only(&["nul\0byte.md"]),
        Ok(()),
        "a NUL byte in an entry name is not rejected here"
    );
}

#[test]
fn a_well_formed_package_imports_and_leaves_no_residue() {
    assert_eq!(
        import(&valid_package(b"Use bounded retries."), "chr-ok"),
        Ok(())
    );
}

#[test]
fn the_end_record_must_be_the_last_thing_in_the_file() {
    let mut trailing = valid_package(b"safe");
    trailing.extend_from_slice(b"trailing");
    assert_eq!(
        import(&trailing, "chr-trailing"),
        Err(Rejection::TrailingData)
    );

    // A prefix leaves the end record where it belongs, so the trailing-data check passes and the
    // non-zero archive offset is what refuses it. Different code, different diagnostic.
    let mut prefixed = b"MZ prepended".to_vec();
    prefixed.extend_from_slice(&valid_package(b"safe"));
    assert_eq!(
        import(&prefixed, "chr-prefixed"),
        Err(Rejection::ArchiveFormat)
    );

    assert_eq!(import(b"", "chr-empty"), Err(Rejection::TrailingData));
    assert_eq!(
        import(b"not a zip", "chr-garbage"),
        Err(Rejection::TrailingData)
    );
}

#[test]
fn an_archive_comment_is_not_trailing_data() {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    let content: &[u8] = b"safe";
    writer
        .start_file(MANIFEST_ENTRY, options)
        .expect("manifest entry");
    writer
        .write_all(&manifest_for(content))
        .expect("manifest bytes");
    writer
        .start_file(format!("{PAYLOAD_PREFIX}{}", sha256_hex(content)), options)
        .expect("payload entry");
    writer.write_all(content).expect("payload bytes");
    writer
        .set_comment("a comment the end record accounts for")
        .expect("set archive comment");
    let archive = writer.finish().expect("finish ZIP").into_inner();

    assert_eq!(import(&archive, "chr-comment"), Ok(()));
}

#[test]
fn only_the_manifest_and_lowercase_sha256_payloads_are_admitted_entries() {
    let content: &[u8] = b"safe";
    let manifest = manifest_for(content);
    let hash = sha256_hex(content);

    for rejected_name in [
        "readme.md",
        "payloads/loose.bin",
        &format!("payloads/sha256/{}", hash.to_uppercase()),
        &format!("payloads/sha256/{}", &hash[..63]),
        &format!("payloads/sha256/{hash}0"),
        &format!("payloads/sha512/{hash}"),
    ] {
        let archive = zip_package(&[
            (MANIFEST_ENTRY, &manifest),
            (&format!("{PAYLOAD_PREFIX}{hash}"), content),
            (rejected_name, b"extra"),
        ]);
        assert_eq!(
            import(&archive, "chr-profile"),
            Err(Rejection::UnexpectedEntry),
            "{rejected_name} should not be an admissible entry"
        );
    }
}

#[test]
fn an_archive_without_a_manifest_reports_the_missing_manifest() {
    let content: &[u8] = b"safe";
    let archive = zip_package(&[(&format!("{PAYLOAD_PREFIX}{}", sha256_hex(content)), content)]);
    assert_eq!(
        import(&archive, "chr-no-manifest"),
        Err(Rejection::MissingManifest)
    );
}

#[test]
fn a_declared_size_that_does_not_match_the_stream_is_caught_during_extraction() {
    // The probe trusts the central directory, which an attacker writes. Extraction is what makes
    // that safe: it copies at most the limit plus one byte and then insists the copied length is
    // exactly what was declared. Whoever moves this code must keep both halves.
    let content: &[u8] = b"safe";
    let mut archive = valid_package(content);
    let understated = shrink_declared_sizes(&mut archive, content.len() as u32);
    assert!(
        understated,
        "the fixture must actually patch a declared size"
    );

    assert_eq!(
        import(&archive, "chr-size-lie"),
        Err(Rejection::ArchiveFormat)
    );
}

/// Rewrites every declared uncompressed size equal to `actual` down to one byte, in both the local
/// headers and the central directory, leaving the compressed stream untouched.
fn shrink_declared_sizes(archive: &mut [u8], actual: u32) -> bool {
    const LOCAL_SIGNATURE: &[u8] = b"PK\x03\x04";
    const CENTRAL_SIGNATURE: &[u8] = b"PK\x01\x02";
    const LOCAL_UNCOMPRESSED_OFFSET: usize = 22;
    const CENTRAL_UNCOMPRESSED_OFFSET: usize = 24;

    let declared = actual.to_le_bytes();
    let mut patched = false;
    for index in 0..archive.len().saturating_sub(4) {
        let field = match &archive[index..index + 4] {
            LOCAL_SIGNATURE => index + LOCAL_UNCOMPRESSED_OFFSET,
            CENTRAL_SIGNATURE => index + CENTRAL_UNCOMPRESSED_OFFSET,
            _ => continue,
        };
        if archive.get(field..field + 4) == Some(&declared[..]) {
            archive[field..field + 4].copy_from_slice(&1_u32.to_le_bytes());
            patched = true;
        }
    }
    patched
}

#[test]
fn staging_is_removed_whether_the_operation_succeeds_or_fails() {
    let home = TempDirectory::new("chr-staging");

    let created = home.path().join("nested/import-ok");
    assert_eq!(
        with_isolated_staging(&created, |root| -> Result<bool, Rejection> {
            std::fs::write(root.join("partial.bin"), b"partial")
                .map_err(|_| Rejection::UnsafePath)?;
            Ok(root.join("partial.bin").is_file())
        }),
        Ok(true),
        "the callback runs inside a directory that exists"
    );
    assert!(!created.exists());

    let failed = home.path().join("nested/import-failed");
    assert_eq!(
        with_isolated_staging(&failed, |root| -> Result<(), Rejection> {
            std::fs::write(root.join("partial.bin"), b"partial")
                .map_err(|_| Rejection::UnsafePath)?;
            Err(Rejection::ExpandedSize)
        }),
        Err(Rejection::ExpandedSize)
    );
    assert!(!failed.exists());

    // A second call on the same path refuses rather than reusing an existing directory, and the
    // shared staging failure still reaches an Overlay operator as `import-unsafe-path`.
    std::fs::create_dir_all(&created).expect("pre-existing staging");
    assert_eq!(
        with_isolated_staging(&created, |_| -> Result<(), Rejection> { Ok(()) }),
        Err(Rejection::UnsafePath)
    );
}
