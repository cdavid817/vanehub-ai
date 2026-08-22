use super::*;
use crate::test_support::TempDirectory;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipWriter};

type Reason = ArchiveRejectionReason;

const LIMITS: ArchiveLimits = ArchiveLimits {
    maximum_compressed_bytes: 1_024,
    maximum_expanded_bytes: 4_096,
    maximum_entries: 8,
};

fn entry(path: &str, kind: ArchiveEntryKind, expanded_bytes: u64) -> ArchiveEntry {
    ArchiveEntry {
        path: path.to_string(),
        kind,
        expanded_bytes,
        unix_mode: None,
    }
}

fn file(path: &str, expanded_bytes: u64) -> ArchiveEntry {
    entry(path, ArchiveEntryKind::File, expanded_bytes)
}

fn unbounded(_: &ArchiveEntry) -> Option<u64> {
    None
}

fn zip_of(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, content) in entries {
        writer.start_file(*name, options).expect("start ZIP entry");
        writer.write_all(content).expect("write ZIP entry");
    }
    writer.finish().expect("finish ZIP").into_inner()
}

fn inspect(archive: &[u8]) -> Result<Vec<ArchiveEntry>, ArchiveRejection> {
    inspect_zip_entries(archive, |_| Ok(()))
}

/// Rewrites a two-byte header field in both the local header and the central directory, so the
/// archive stays self-consistent while declaring something the writer cannot produce.
///
/// Returns every value it replaced. A caller that asserts on those has proof it patched the field
/// it meant to: a wrong offset corrupts the archive, and a corrupt archive is refused for reasons
/// that look exactly like the ones these tests are trying to demonstrate.
fn patch_headers(
    archive: &mut [u8],
    local_offset: usize,
    central_offset: usize,
    replace: impl Fn(u16) -> u16,
) -> Vec<u16> {
    let mut replaced = Vec::new();
    for index in 0..archive.len().saturating_sub(4) {
        let field = match &archive[index..index + 4] {
            b"PK\x03\x04" => index + local_offset,
            b"PK\x01\x02" => index + central_offset,
            _ => continue,
        };
        let Some(slice) = archive.get_mut(field..field + 2) else {
            continue;
        };
        let previous = u16::from_le_bytes([slice[0], slice[1]]);
        slice.copy_from_slice(&replace(previous).to_le_bytes());
        replaced.push(previous);
    }
    replaced
}

#[test]
fn entry_names_are_accepted_or_refused_by_component() {
    for accepted in [
        "a.md",
        "references/a.md",
        "a/b/c/d/e/f/g/h/i/j.md",
        "references/",
        "UPPER.MD",
        "ünïcode.md",
        "name with spaces.md",
    ] {
        assert!(is_safe_archive_entry_path(accepted), "{accepted}");
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
        assert!(!is_safe_archive_entry_path(refused), "{refused}");
    }
}

#[test]
fn the_entry_name_rule_is_a_floor_and_not_a_whole_policy() {
    // Pinned as a gap. A consumer that writes these names out on Windows needs the stricter
    // declared-path rule as well; nothing here may be read as covering that.
    for accepted in [
        "trailing-space.md ",
        "trailing-dot.md.",
        "CON",
        "aux.md",
        "nul\0byte.md",
        "\u{7f}del.md",
    ] {
        assert!(is_safe_archive_entry_path(accepted), "{accepted}");
    }
}

#[test]
fn the_end_record_must_be_the_last_thing_in_the_buffer() {
    let archive = zip_of(&[("a.md", b"content")]);
    assert!(ends_at_the_central_directory_record(&archive));

    let mut trailing = archive.clone();
    trailing.extend_from_slice(b"appended");
    assert!(!ends_at_the_central_directory_record(&trailing));

    let mut prefixed = b"MZ stub".to_vec();
    prefixed.extend_from_slice(&archive);
    assert!(
        ends_at_the_central_directory_record(&prefixed),
        "a prefix leaves the end record where it belongs; the archive offset is what catches it"
    );

    assert!(!ends_at_the_central_directory_record(b""));
    assert!(!ends_at_the_central_directory_record(b"PK\x05\x06"));

    let mut writer = ZipWriter::new(Cursor::new(Vec::new()));
    writer
        .start_file("a.md", SimpleFileOptions::default())
        .expect("entry");
    writer.write_all(b"content").expect("bytes");
    writer
        .set_comment("declared in the end record")
        .expect("comment");
    let commented = writer.finish().expect("finish ZIP").into_inner();
    assert!(
        ends_at_the_central_directory_record(&commented),
        "a comment is part of the record, not trailing data"
    );
}

#[test]
fn inspection_classifies_entries_and_reports_the_declared_size() {
    let archive = zip_of(&[("overlay.json", b"{}"), ("payloads/a.bin", b"12345")]);
    let entries = inspect(&archive).expect("inspected");
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.path.as_str(), entry.kind, entry.expanded_bytes))
            .collect::<Vec<_>>(),
        vec![
            ("overlay.json", ArchiveEntryKind::File, 2),
            ("payloads/a.bin", ArchiveEntryKind::File, 5),
        ]
    );
}

#[test]
fn inspection_refuses_a_prepended_stub() {
    let mut prefixed = b"MZ stub".to_vec();
    prefixed.extend_from_slice(&zip_of(&[("a.md", b"content")]));
    assert_eq!(
        inspect(&prefixed),
        Err(ArchiveRejection::new(Reason::Format))
    );
}

#[test]
fn an_encrypted_entry_never_reaches_the_caller() {
    // No writer in this build produces one, so the encryption bit is set by hand — which is how it
    // would arrive anyway.
    const LOCAL_FLAG_OFFSET: usize = 6;
    const CENTRAL_FLAG_OFFSET: usize = 8;
    let mut archive = zip_of(&[("a.md", b"content")]);
    assert!(inspect(&archive).is_ok(), "the fixture starts readable");

    let previous = patch_headers(
        &mut archive,
        LOCAL_FLAG_OFFSET,
        CENTRAL_FLAG_OFFSET,
        |flag| flag | 0x0001,
    );
    assert_eq!(previous.len(), 2, "one local header and one central record");
    assert!(
        previous.iter().all(|flag| flag & 0x0001 == 0),
        "the offset must point at a flag field that did not already say encrypted: {previous:?}"
    );

    // `zip = 8.6.0` refuses the entry inside `by_index`, before this module's own encryption check
    // is reached, so the answer is the generic format rejection rather than `EncryptedEntry`. The
    // check in `zip_reader` is therefore a guard against a future reader that is more permissive,
    // not the one doing the work today. Pinned so that a reader upgrade which starts handing us
    // encrypted entries shows up here as a changed answer.
    assert_eq!(
        inspect(&archive),
        Err(ArchiveRejection::new(Reason::Format))
    );
}

#[test]
fn a_compression_method_this_build_cannot_read_never_reaches_the_caller() {
    const LOCAL_METHOD_OFFSET: usize = 8;
    const CENTRAL_METHOD_OFFSET: usize = 10;
    const DEFLATED: u16 = 8;
    const LZMA: u16 = 14;
    let mut archive = zip_of(&[("a.md", b"content")]);
    assert!(inspect(&archive).is_ok(), "the fixture starts readable");

    let previous = patch_headers(
        &mut archive,
        LOCAL_METHOD_OFFSET,
        CENTRAL_METHOD_OFFSET,
        |_| LZMA,
    );
    assert_eq!(
        previous,
        vec![DEFLATED, DEFLATED],
        "the offset must point at the compression method the fixture was written with"
    );

    // Same shape as the encrypted case: the pinned reader refuses the method first, so this module
    // never gets to say `UnsupportedCompression`. The algorithm is beside the point — what matters
    // is that an unreadable method is refused up front instead of surfacing later as an
    // unexplained read failure part-way through an extraction.
    assert_eq!(
        inspect(&archive),
        Err(ArchiveRejection::new(Reason::Format))
    );
}

#[derive(Debug, PartialEq, Eq)]
enum ProfileError {
    Archive(ArchiveRejection),
    Unexpected(String),
}

impl From<ArchiveRejection> for ProfileError {
    fn from(rejection: ArchiveRejection) -> Self {
        Self::Archive(rejection)
    }
}

#[test]
fn the_callers_profile_runs_per_entry_in_archive_order() {
    let archive = zip_of(&[("expected.md", b"a"), ("surprise.md", b"b")]);
    let mut seen = Vec::new();
    let outcome = inspect_zip_entries(&archive, |entry| {
        seen.push(entry.path.clone());
        if entry.path == "expected.md" {
            Ok(())
        } else {
            Err(ProfileError::Unexpected(entry.path.clone()))
        }
    });

    assert_eq!(
        outcome,
        Err(ProfileError::Unexpected("surprise.md".to_string()))
    );
    assert_eq!(
        seen,
        vec!["expected.md".to_string(), "surprise.md".to_string()],
        "inspection stops at the first entry the profile refuses"
    );
}

#[test]
fn compressed_size_is_refused_one_byte_past_the_budget() {
    assert_eq!(
        check_compressed_size(LIMITS.maximum_compressed_bytes, LIMITS),
        Ok(())
    );
    assert_eq!(
        check_compressed_size(LIMITS.maximum_compressed_bytes + 1, LIMITS),
        Err(ArchiveRejection::new(Reason::CompressedSize))
    );
}

#[test]
fn entry_validation_runs_one_pass_in_a_fixed_order() {
    let refusal = |reason, path: &str| Err(ArchiveRejection::at(reason, path));

    let too_many = (0..=LIMITS.maximum_entries)
        .map(|index| file(&format!("{index}.md"), 0))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_entries(&too_many, LIMITS, unbounded),
        Err(ArchiveRejection::new(Reason::EntryCount))
    );
    assert_eq!(
        validate_entries(&too_many[..LIMITS.maximum_entries], LIMITS, unbounded),
        Ok(())
    );

    // Within one entry: link, then path, then duplicate, then budget.
    let link = entry("../escape.md", ArchiveEntryKind::SymbolicLink, u64::MAX);
    assert_eq!(
        validate_entries(std::slice::from_ref(&link), LIMITS, |_| Some(0)),
        refusal(Reason::LinkEntry, "../escape.md")
    );
    assert_eq!(
        validate_entries(&[file("../escape.md", u64::MAX)], LIMITS, |_| Some(0)),
        refusal(Reason::UnsafePath, "../escape.md")
    );
    assert_eq!(
        validate_entries(
            &[file("a.md", 0), file("a.md", u64::MAX)],
            LIMITS,
            |_| Some(0)
        ),
        refusal(Reason::DuplicatePath, "a.md")
    );
    // Across entries: the first one that breaks a rule decides, so a later duplicate is never
    // reached.
    assert_eq!(
        validate_entries(&[file("a.md", 1), file("a.md", 0)], LIMITS, |_| Some(0)),
        refusal(Reason::EntryTooLarge, "a.md")
    );
}

#[test]
fn every_entry_counts_toward_the_total_even_when_it_has_no_budget_of_its_own() {
    let exempt = |entry: &ArchiveEntry| (entry.path != "manifest.json").then_some(4);

    assert_eq!(
        validate_entries(
            &[file("manifest.json", LIMITS.maximum_expanded_bytes)],
            LIMITS,
            exempt
        ),
        Ok(())
    );
    assert_eq!(
        validate_entries(
            &[file("manifest.json", LIMITS.maximum_expanded_bytes + 1)],
            LIMITS,
            exempt
        ),
        Err(ArchiveRejection::new(Reason::ExpandedSize))
    );
    assert_eq!(
        validate_entries(&[file("a.md", 5)], LIMITS, exempt),
        Err(ArchiveRejection::at(Reason::EntryTooLarge, "a.md"))
    );
    assert_eq!(
        validate_entries(
            &[
                entry("a/", ArchiveEntryKind::Directory, u64::MAX),
                entry("b/", ArchiveEntryKind::Directory, u64::MAX),
            ],
            LIMITS,
            unbounded
        ),
        Err(ArchiveRejection::new(Reason::ExpandedSize)),
        "the running total saturates into a rejection rather than wrapping"
    );
}

#[test]
fn extraction_writes_under_the_destination_and_stops_at_the_budget() {
    let home = TempDirectory::new("platform-archive-extract");
    let destination = home.path().join("staging");
    std::fs::create_dir_all(&destination).expect("destination");

    let archive = zip_of(&[("overlay.json", b"{}"), ("payloads/a.bin", b"12345")]);
    let entries = inspect(&archive).expect("inspected");
    assert_eq!(
        extract_zip_entries(&archive, &entries, &destination, |_| 8),
        Ok(())
    );
    assert_eq!(
        std::fs::read(destination.join("payloads/a.bin")).expect("payload"),
        b"12345"
    );

    let tight = home.path().join("tight");
    std::fs::create_dir_all(&tight).expect("tight destination");
    assert_eq!(
        extract_zip_entries(&archive, &entries, &tight, |name| {
            if name == "overlay.json" {
                8
            } else {
                4
            }
        }),
        Err(ArchiveRejection::at(
            Reason::EntryTooLarge,
            "payloads/a.bin"
        ))
    );
}

#[test]
fn extraction_refuses_a_declared_size_the_stream_does_not_match() {
    const LOCAL_UNCOMPRESSED_OFFSET: usize = 22;
    const CENTRAL_UNCOMPRESSED_OFFSET: usize = 24;
    let home = TempDirectory::new("platform-archive-size-lie");
    let destination = home.path().join("staging");
    std::fs::create_dir_all(&destination).expect("destination");

    let mut archive = zip_of(&[("a.bin", b"12345")]);
    let previous = patch_declared_sizes(
        &mut archive,
        LOCAL_UNCOMPRESSED_OFFSET,
        CENTRAL_UNCOMPRESSED_OFFSET,
        1,
    );
    assert_eq!(
        previous,
        vec![5, 5],
        "the offset must point at the uncompressed size the fixture was written with"
    );

    let entries = inspect(&archive).expect("inspected");
    assert_eq!(
        extract_zip_entries(&archive, &entries, &destination, |_| 64),
        Err(ArchiveRejection::new(Reason::Format))
    );
}

/// The size fields are four bytes wide, unlike the flag and method fields `patch_headers` covers.
fn patch_declared_sizes(
    archive: &mut [u8],
    local_offset: usize,
    central_offset: usize,
    declared: u32,
) -> Vec<u32> {
    let mut replaced = Vec::new();
    for index in 0..archive.len().saturating_sub(4) {
        let field = match &archive[index..index + 4] {
            b"PK\x03\x04" => index + local_offset,
            b"PK\x01\x02" => index + central_offset,
            _ => continue,
        };
        let Some(slice) = archive.get_mut(field..field + 4) else {
            continue;
        };
        let mut previous = [0_u8; 4];
        previous.copy_from_slice(slice);
        slice.copy_from_slice(&declared.to_le_bytes());
        replaced.push(u32::from_le_bytes(previous));
    }
    replaced
}

#[test]
fn extraction_refuses_to_overwrite_something_already_there() {
    let home = TempDirectory::new("platform-archive-collision");
    let destination = home.path().join("staging");
    std::fs::create_dir_all(&destination).expect("destination");
    std::fs::write(destination.join("a.bin"), b"existing").expect("pre-existing file");

    let archive = zip_of(&[("a.bin", b"12345")]);
    let entries = inspect(&archive).expect("inspected");
    assert_eq!(
        extract_zip_entries(&archive, &entries, &destination, |_| 64),
        Err(ArchiveRejection::at(Reason::DuplicatePath, "a.bin"))
    );
}

impl From<StagingFailure> for ProfileError {
    fn from(_: StagingFailure) -> Self {
        Self::Unexpected("staging".to_string())
    }
}

#[test]
fn staging_is_created_for_the_operation_and_removed_afterwards() {
    let home = TempDirectory::new("platform-archive-staging");

    let used = home.path().join("nested/run-1");
    assert_eq!(
        with_isolated_staging(&used, |root| -> Result<bool, ProfileError> {
            std::fs::write(root.join("partial.bin"), b"partial").expect("write inside staging");
            Ok(root.is_dir())
        }),
        Ok(true)
    );
    assert!(!used.exists(), "staging does not survive a success");

    let failed = home.path().join("nested/run-2");
    assert_eq!(
        with_isolated_staging(&failed, |root| -> Result<(), ProfileError> {
            std::fs::write(root.join("partial.bin"), b"partial").expect("write inside staging");
            Err(ProfileError::Unexpected("operation".to_string()))
        }),
        Err(ProfileError::Unexpected("operation".to_string()))
    );
    assert!(!failed.exists(), "staging does not survive a failure");

    std::fs::create_dir_all(&used).expect("pre-existing staging");
    assert_eq!(
        with_isolated_staging(&used, |_| -> Result<(), ProfileError> { Ok(()) }),
        Err(ProfileError::Unexpected("staging".to_string())),
        "an existing directory is refused rather than reused"
    );
}

#[test]
fn an_archive_with_more_than_one_end_record_is_refused_as_ambiguous() {
    // Two end records mean two readings. A backward-scanning reader takes the last, a
    // forward-scanning one takes the first, and the two can describe different files -- which is a
    // parser differential a signature cannot resolve, because a publisher can sign an archive that
    // is genuinely ambiguous.
    let archive = zip_of(&[("a.md", b"content")]);
    assert_eq!(count_end_records(&archive), 1);

    let doubled = [archive.clone(), archive.clone()].concat();
    assert_eq!(count_end_records(&doubled), 2);
    assert_eq!(
        inspect(&doubled),
        Err(ArchiveRejection::new(Reason::Ambiguous))
    );

    // And an archive with none at all is refused too, rather than falling through to whatever the
    // reader would make of it.
    assert_eq!(count_end_records(b"not an archive"), 0);
    assert_eq!(
        inspect(b"not an archive"),
        Err(ArchiveRejection::new(Reason::Ambiguous))
    );
}

#[test]
fn the_end_record_count_ignores_a_signature_that_is_not_a_record() {
    // `PK\x05\x06` occurs inside compressed data by chance. A count that took the four signature
    // bytes at face value would refuse ordinary archives, so the record has to be self-consistent:
    // zero disk numbers, matching entry counts, and a central directory inside the buffer.
    let mut planted = zip_of(&[("a.md", b"content")]);
    let insert_at = 8;
    // Disk numbers deliberately non-zero, which no real end record has.
    let mut decoy = b"PK\x05\x06".to_vec();
    decoy.extend_from_slice(&[0xff; 18]);
    planted.splice(insert_at..insert_at, decoy);

    assert_eq!(
        count_end_records(&planted),
        1,
        "the decoy is not counted, so a real archive is not refused for containing one"
    );
}

#[test]
fn extraction_refuses_an_entry_that_is_not_what_inspection_saw() {
    // The two passes re-open the archive independently, which is the one place this module could
    // have its own parser differential. Handing extraction the inspected list and checking each
    // index against it is what closes that.
    let home = TempDirectory::new("platform-archive-view");
    let destination = home.path().join("staging");
    std::fs::create_dir_all(&destination).expect("destination");
    let archive = zip_of(&[("a.md", b"content")]);
    let mut entries = inspect(&archive).expect("inspected");

    entries[0].path = "b.md".to_string();
    assert_eq!(
        extract_zip_entries(&archive, &entries, &destination, |_| 64),
        Err(ArchiveRejection::at(Reason::Ambiguous, "a.md"))
    );

    let short = Vec::new();
    assert_eq!(
        extract_zip_entries(&archive, &short, &destination, |_| 64),
        Err(ArchiveRejection::new(Reason::Ambiguous))
    );
}
