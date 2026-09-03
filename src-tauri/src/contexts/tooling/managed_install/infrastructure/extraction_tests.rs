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
    let admitted = guard
        .admit("bin/server", ArchiveEntryKind::File)
        .expect("admitted");
    guard
        .write_entry(&admitted, Cursor::new(b"payload"))
        .expect("written");
    assert!(admitted.exists());
    let error = guard
        .admit("../escaped", ArchiveEntryKind::File)
        .expect_err("refused");
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

/// Builds a tar.gz in memory with the given entries, then writes it beside a returned handle.
fn tar_archive_with(entries: &[(&str, &[u8])]) -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("artifact.tar.gz");
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        for (name, bytes) in entries {
            let mut header = tar::Header::new_gnu();
            header.set_size(bytes.len() as u64);
            header.set_mode(0o644);
            // Written into the raw name field rather than through `set_path`, because the crate's
            // builder refuses to emit an absolute or `..` path -- which is exactly why a fixture
            // built through it could not exercise the guard. A hostile archive was not produced by
            // a well-behaved builder either.
            let raw = name.as_bytes();
            header.as_old_mut().name[..raw.len()].copy_from_slice(raw);
            header.set_cksum();
            builder.append(&header, *bytes).expect("append entry");
        }
        builder.finish().expect("finish tar");
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &tar_bytes).expect("gzip");
    std::fs::write(&path, encoder.finish().expect("finish gzip")).expect("write archive");
    (directory, path)
}

/// A tar carrying one symbolic link and nothing else.
fn tar_archive_with_symlink() -> (tempfile::TempDir, std::path::PathBuf) {
    let directory = tempfile::tempdir().expect("fixture directory");
    let path = directory.path().join("linked.tar.gz");
    let mut tar_bytes = Vec::new();
    {
        let mut builder = tar::Builder::new(&mut tar_bytes);
        let mut header = tar::Header::new_gnu();
        header.set_entry_type(tar::EntryType::Symlink);
        header.set_size(0);
        header.set_mode(0o777);
        // Points inside the destination, which is the case a naive containment check admits.
        let name = b"bin/alias";
        header.as_old_mut().name[..name.len()].copy_from_slice(name);
        let target = b"server";
        header.as_old_mut().linkname[..target.len()].copy_from_slice(target);
        header.set_cksum();
        builder
            .append(&header, std::io::empty())
            .expect("append link");
        builder.finish().expect("finish tar");
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    std::io::Write::write_all(&mut encoder, &tar_bytes).expect("gzip");
    std::fs::write(&path, encoder.finish().expect("finish gzip")).expect("write archive");
    (directory, path)
}

#[test]
fn a_clean_tar_gz_extracts_and_reports_its_directory() {
    let (_fixture, archive) =
        tar_archive_with(&[("bin/server", b"payload"), ("lib/a.jar", b"jar")]);

    let extracted = extract_tar_gz(&archive, limits()).expect("extracted");

    let root = extracted.directory.path();
    assert_eq!(
        std::fs::read_to_string(root.join("bin/server")).expect("entry readable"),
        "payload"
    );
    assert!(root.join("lib/a.jar").exists());
}

#[test]
fn both_formats_refuse_the_same_escaping_entry() {
    // The point of one guard: the containment decision is shared, not reimplemented per format.
    // If a second adapter grew its own check, this is the test that would catch the drift.
    let escaping = "a/../../escaped";
    let (_zip_fixture, zip) = archive_with(&[(escaping, b"x")]);
    let (_tar_fixture, tar) = tar_archive_with(&[(escaping, b"x")]);

    for error in [
        extract_zip(&zip, limits()).expect_err("zip refused"),
        extract_tar_gz(&tar, limits()).expect_err("tar refused"),
    ] {
        assert!(matches!(error, ManagedInstallError::Refused(_)));
        assert!(error.to_string().contains("parent-directory"));
    }
}

#[test]
fn an_absolute_tar_entry_is_refused() {
    let (_fixture, archive) = tar_archive_with(&[("/etc/implanted", b"x")]);

    assert!(extract_tar_gz(&archive, limits())
        .expect_err("refused")
        .to_string()
        .contains("absolute"));
}

#[test]
fn a_tar_exceeding_the_byte_ceiling_is_refused_while_writing() {
    let bounded = ExtractionLimits {
        max_total_bytes: 16,
        max_entries: 8,
    };
    let (_fixture, archive) = tar_archive_with(&[("big", &[b'x'; 4096])]);

    assert!(extract_tar_gz(&archive, bounded)
        .expect_err("refused")
        .to_string()
        .contains("16 byte extraction ceiling"));
}

#[test]
fn a_tar_exceeding_the_entry_count_is_refused() {
    let bounded = ExtractionLimits {
        max_total_bytes: 1024,
        max_entries: 2,
    };
    let (_fixture, archive) = tar_archive_with(&[("a", b"1"), ("b", b"2"), ("c", b"3")]);

    assert!(extract_tar_gz(&archive, bounded)
        .expect_err("refused")
        .to_string()
        .contains("2 entry limit"));
}

#[test]
fn a_link_entry_is_refused_even_when_it_points_inside() {
    let (_fixture, archive) = tar_archive_with_symlink();

    let error = extract_tar_gz(&archive, limits()).expect_err("refused");

    // Refused regardless of target: a link resolves at use, so one pointing inside the destination
    // today points outside it after something else moves.
    assert!(matches!(error, ManagedInstallError::Refused(_)));
    assert!(error.to_string().contains("neither a file nor a directory"));
}

#[test]
fn the_guard_refuses_a_link_before_it_computes_a_destination() {
    let mut guard = ExtractionGuard::new(limits()).expect("guard");

    let error = guard
        .admit("bin/alias", ArchiveEntryKind::Other)
        .expect_err("refused");

    // Refused before the entry counter moves, so a stream of links cannot exhaust the budget and
    // report the wrong reason.
    assert!(error.to_string().contains("neither a file nor a directory"));
    assert!(guard.admit("bin/server", ArchiveEntryKind::File).is_ok());
}
