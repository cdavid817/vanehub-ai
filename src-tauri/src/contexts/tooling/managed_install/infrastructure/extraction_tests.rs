// Included through `#[path]` from extraction.rs.
//
// The archive kind has no production caller yet -- `add-lsp-java-jdtls` is the change that adds
// one. It ships with these rather than as an untested affordance, because an unpacker that has
// never been shown to refuse an escaping entry is not a bound, it is an intention.
use super::*;

use std::io::Cursor;
use zip::write::SimpleFileOptions;

fn limits() -> ExtractionLimits {
    ExtractionLimits {
        max_total_bytes: 1024,
        max_entries: 8,
    }
}

/// Builds a zip in memory with the given entries, then writes it beside a returned handle.
fn archive_with(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("artifact.zip");
    let mut buffer = Vec::new();
    {
        let mut writer = zip::ZipWriter::new(Cursor::new(&mut buffer));
        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .expect("start entry");
            std::io::Write::write_all(&mut writer, bytes).expect("write entry");
        }
        writer.finish().expect("finish archive");
    }
    std::fs::write(&path, &buffer).expect("write archive");
    (directory, path)
}

#[test]
fn a_clean_archive_extracts_and_reports_its_directory() {
    let (_fixture, archive) = archive_with(&[("bin/server", b"payload"), ("lib/a.jar", b"jar")]);

    let extracted = extract_zip(&archive, limits()).expect("extracted");

    let root = extracted.directory.path();
    assert_eq!(
        std::fs::read_to_string(root.join("bin/server")).expect("entry readable"),
        "payload"
    );
    assert!(root.join("lib/a.jar").exists());
}

#[test]
fn an_entry_with_a_parent_component_is_refused() {
    // The one a leading-slash check misses: it normalizes out of the destination while looking
    // like an ordinary relative path.
    let (_fixture, archive) = archive_with(&[("a/../../escaped", b"x")]);

    let error = extract_zip(&archive, limits()).expect_err("refused");

    assert!(matches!(error, ManagedInstallError::Refused(_)));
    assert!(error.to_string().contains("parent-directory"));
}

#[test]
fn an_absolute_entry_is_refused() {
    let (_fixture, archive) = archive_with(&[("/etc/implanted", b"x")]);

    let error = extract_zip(&archive, limits()).expect_err("refused");

    assert!(error.to_string().contains("absolute"));
}

#[test]
fn nothing_survives_a_refused_archive() {
    // The first entry is legitimate and the second escapes. The guard owns the destination, so
    // dropping it on the refusal takes the already-written entry with it: a half-unpacked tool is
    // worse than none, because it looks installed.
    let (_fixture, archive) = archive_with(&[("bin/server", b"payload"), ("../escaped", b"x")]);

    let mut guard = ExtractionGuard::new(limits()).expect("guard");
    let destination = guard.destination().to_path_buf();
    let admitted = guard.admit("bin/server").expect("admitted");
    guard
        .write_entry(&admitted, Cursor::new(b"payload"))
        .expect("written");
    assert!(admitted.exists());
    let error = guard.admit("../escaped").expect_err("refused");
    drop(guard);

    assert!(error.to_string().contains("parent-directory"));
    assert!(!destination.exists());
    assert!(extract_zip(&archive, limits()).is_err());
}

#[test]
fn exceeding_the_byte_ceiling_is_refused_while_writing() {
    let bounded = ExtractionLimits {
        max_total_bytes: 16,
        max_entries: 8,
    };
    let (_fixture, archive) = archive_with(&[("big", &[b'x'; 4096])]);

    let error = extract_zip(&archive, bounded).expect_err("refused");

    // An archive's compressed size says nothing about its expanded size, which is why the ceiling
    // is checked while writing rather than against the downloaded file.
    assert!(error.to_string().contains("16 byte extraction ceiling"));
}

#[test]
fn exceeding_the_entry_count_is_refused() {
    let bounded = ExtractionLimits {
        max_total_bytes: 1024,
        max_entries: 2,
    };
    let (_fixture, archive) = archive_with(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);

    let error = extract_zip(&archive, bounded).expect_err("refused");

    assert!(error.to_string().contains("2 entry limit"));
}

#[test]
fn limits_that_bound_nothing_are_refused_before_a_destination_exists() {
    for unbounded in [
        ExtractionLimits {
            max_total_bytes: 0,
            max_entries: 8,
        },
        ExtractionLimits {
            max_total_bytes: 1024,
            max_entries: 0,
        },
    ] {
        assert!(!unbounded.is_bounded());
        let error = ExtractionGuard::new(unbounded).expect_err("refused");
        assert!(error.to_string().contains("no extraction limits"));
    }
}
