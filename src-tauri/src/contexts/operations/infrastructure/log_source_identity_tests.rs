//! The mixed corpus a real installation actually has, and the three things that happen to a file.
//!
//! Split from the reader's own tests because these two subjects are about what survives *between*
//! runs — a replayed repair, a restart, a rotation — rather than about parsing one batch.

use super::log_source_reader::UnifiedLogSourceReader;
use crate::contexts::operations::application::RedactedLogSourceReader;
use crate::platform::logging::{active_file_id, LOG_FILE_NAME};
use crate::test_support::TempDirectory;
use std::io::Write;
use std::path::Path;

fn write_lines(path: &Path, lines: &[&str]) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open log");
    for line in lines {
        writeln!(file, "{line}").expect("write line");
    }
}

fn entry(message: &str, record_id: Option<&str>) -> String {
    let id = record_id
        .map(|value| format!(r#","recordId":"{value}""#))
        .unwrap_or_default();
    format!(
        r#"{{"timestamp":"2026-08-24T10:00:00Z","level":"info","category":"test","message":"{message}","context":{{"sessionId":"session-1"}}{id}}}"#
    )
}

/// Everything at once, read twice. The second pass must produce exactly the same identities as the
/// first — that is what makes a repair safe to run again, which is the only way a repair that was
/// interrupted can ever finish.
#[test]
fn a_mixed_file_yields_stable_identities_on_every_pass() {
    let fixture = TempDirectory::new("log-source-mixed");
    let path = fixture.path().join(LOG_FILE_NAME);
    write_lines(
        &path,
        &[
            &entry("before ids", None),
            &entry("with an id", Some("record-new")),
            // The same text twice in one file: a retry loop logging one failure.
            &entry("before ids", None),
            "{not json at all",
        ],
    );
    // A trailing line the writer has not finished.
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("append");
    write!(file, "{}", entry("half written", None)).expect("partial");
    drop(file);
    let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
    let source = reader.sources().expect("sources").remove(0);

    let first = reader
        .read_batch(&source, 0, 100, 1_000_000)
        .expect("first");
    let replay = reader
        .read_batch(&source, 0, 100, 1_000_000)
        .expect("replay");

    let ids: Vec<&str> = first
        .records
        .iter()
        .map(|record| record.record_id.as_str())
        .collect();
    assert_eq!(ids.len(), 3, "three complete, well-formed lines");
    assert_eq!(ids[1], "record-new", "a stored id is used as-is");
    assert!(ids[0].starts_with("legacy:v1:"));
    // Identical text at two offsets is two records, not one.
    assert_ne!(ids[0], ids[2]);
    assert_eq!(first.rejected, 1, "the malformed complete line is counted");
    // Deriving the same ids again is what makes a repeated repair pass add no duplicate row.
    assert_eq!(
        ids,
        replay
            .records
            .iter()
            .map(|record| record.record_id.as_str())
            .collect::<Vec<_>>()
    );
    // The partial trailing line is left for the next pass.
    assert!(first.reached_end);
}

/// The same text in two different files is two records. Without the source in the identity they
/// would collapse, and one file's history would silently absorb the other's.
#[test]
fn the_same_line_in_two_files_is_two_records() {
    let fixture = TempDirectory::new("log-source-two-files");
    write_lines(
        &fixture.path().join("vanehub-2026-08-23.log"),
        &[&entry("identical", None)],
    );
    write_lines(
        &fixture.path().join(LOG_FILE_NAME),
        &[&entry("identical", None)],
    );
    let reader = UnifiedLogSourceReader::new(fixture.path().to_path_buf());
    let sources = reader.sources().expect("sources");
    assert_eq!(sources.len(), 2);

    let rotated = reader
        .read_batch(&sources[0], 0, 10, 100_000)
        .expect("rotated");
    let active = reader
        .read_batch(&sources[1], 0, 10, 100_000)
        .expect("active");

    assert_ne!(
        rotated.records[0].record_id, active.records[0].record_id,
        "the same text in two files collapsed into one record"
    );
}

/// Restarting the application must not turn every file into a new source. If it did, every launch
/// would re-index the whole corpus and every checkpoint would be stranded.
#[test]
fn an_unchanged_file_keeps_its_identity_across_a_restart() {
    let fixture = TempDirectory::new("log-source-restart");
    let path = fixture.path().join(LOG_FILE_NAME);
    write_lines(&path, &[&entry("first", Some("record-1"))]);

    let before = active_file_id(&path);
    // A restart is a fresh read of the same bytes; nothing about the process carries over.
    let after = active_file_id(&path);
    // Appending is not replacing: a growing file is the same file.
    write_lines(&path, &[&entry("second", Some("record-2"))]);
    let grown = active_file_id(&path);

    assert_eq!(before, after);
    assert_eq!(before, grown);
}

/// Rotation renames a file whose records are the same records. Treating it as new would re-index
/// everything it holds and leave its checkpoint pointing at nothing.
#[test]
fn a_rotated_file_keeps_the_identity_it_had_under_its_old_name() {
    let fixture = TempDirectory::new("log-source-rotation");
    let active = fixture.path().join(LOG_FILE_NAME);
    write_lines(&active, &[&entry("before rotation", Some("record-1"))]);
    let before = active_file_id(&active);

    let rotated = fixture.path().join("vanehub-2026-08-24.log");
    std::fs::rename(&active, &rotated).expect("rotate");

    assert_eq!(active_file_id(&rotated), before);
}

/// A path reused for unrelated bytes is a new generation. Resuming the old checkpoint here would
/// read from a byte offset that no longer means anything.
#[test]
fn a_recreated_file_at_the_same_path_is_a_new_generation() {
    let fixture = TempDirectory::new("log-source-truncate");
    let path = fixture.path().join(LOG_FILE_NAME);
    write_lines(&path, &[&entry("original", Some("record-1"))]);
    let before = active_file_id(&path);

    std::fs::remove_file(&path).expect("remove");
    write_lines(&path, &[&entry("recreated", Some("record-2"))]);

    assert_ne!(active_file_id(&path), before);
}

/// A directory change replaces the corpus. Old checkpoints must not attach to new sources, and rows
/// indexed from the old directory must not let the new one claim to be complete.
#[test]
fn a_different_directory_is_a_different_generation() {
    let first = TempDirectory::new("log-source-dir-a");
    let second = TempDirectory::new("log-source-dir-b");
    write_lines(
        &first.path().join(LOG_FILE_NAME),
        &[&entry("shared", Some("record-1"))],
    );
    write_lines(
        &second.path().join(LOG_FILE_NAME),
        &[&entry("shared", Some("record-1"))],
    );

    let left = UnifiedLogSourceReader::new(first.path().to_path_buf())
        .sources()
        .expect("left")
        .remove(0);
    let right = UnifiedLogSourceReader::new(second.path().to_path_buf())
        .sources()
        .expect("right")
        .remove(0);

    // Identical content, so the file half matches; the directory generation is what separates them,
    // and it is what a checkpoint is keyed by.
    assert_ne!(left.directory_generation, right.directory_generation);
    assert_ne!(left.as_key(), right.as_key());
}

/// A source identity crosses into the UI inside coverage and diagnostics. It must not carry the
/// user's filesystem layout with it.
#[test]
fn a_source_identity_names_no_path() {
    let fixture = TempDirectory::new("log-source-no-path");
    write_lines(
        &fixture.path().join(LOG_FILE_NAME),
        &[&entry("hello", Some("record-1"))],
    );
    let source = UnifiedLogSourceReader::new(fixture.path().to_path_buf())
        .sources()
        .expect("sources")
        .remove(0);

    let key = source.as_key();
    let root = fixture.path().to_string_lossy().to_string();
    assert!(!key.contains(&root));
    assert!(!key.contains(LOG_FILE_NAME));
    assert!(!key.contains(std::path::MAIN_SEPARATOR));
}

/// An export tells the user which logs it covers, not where they live.
#[test]
fn export_sources_name_files_rather_than_paths() {
    let fixture = TempDirectory::new("log-source-export-names");
    write_lines(
        &fixture.path().join(LOG_FILE_NAME),
        &[&entry("hello", Some("record-1"))],
    );

    let sources = UnifiedLogSourceReader::new(fixture.path().to_path_buf())
        .export_sources()
        .expect("export sources");

    assert_eq!(sources, [LOG_FILE_NAME.to_string()]);
    assert!(!sources[0].contains(std::path::MAIN_SEPARATOR));
}
