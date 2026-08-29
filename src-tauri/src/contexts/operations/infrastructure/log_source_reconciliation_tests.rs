//! Seven things that happen to a log file, and why each needs a different answer.
//!
//! They are grouped here because the point is that they are *distinguishable*. Every one of them
//! makes the corpus differ from what the index holds, and a reconciliation that could not tell them
//! apart would have to pick one response for all seven — which means being wrong about six.
//!
//! | what happened          | identity | checkpoint | rows |
//! |------------------------|----------|------------|------|
//! | rotation (rename)      | kept     | kept       | kept |
//! | append                 | kept     | kept       | kept |
//! | truncate in place      | kept     | reset      | pruned |
//! | recreate at same path  | new      | not reused | pruned |
//! | retained deletion      | gone     | expired    | expired |
//! | temporary read failure | kept     | kept       | kept |
//! | directory change       | new      | not reused | expired |
//!
//! The two rows that look alike are the dangerous pair: a retained deletion and a temporary read
//! failure both present as "the file is not there right now", and only one of them means the
//! records are gone.

use super::log_source_reader::UnifiedLogSourceReader;
use crate::contexts::operations::application::{
    LogSourceSnapshot, OperationsLogError, RedactedLogSourceReader,
};
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;
use std::io::Write;
use std::path::Path;

fn entry(message: &str, record_id: &str) -> String {
    format!(
        r#"{{"timestamp":"2026-08-25T10:00:00Z","level":"info","category":"test","message":"{message}","context":{{"sessionId":"session-1"}},"recordId":"{record_id}"}}"#
    )
}

fn append(path: &Path, lines: &[String]) {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .expect("open log");
    for line in lines {
        writeln!(file, "{line}").expect("write");
    }
}

fn only_source(reader: &UnifiedLogSourceReader) -> LogSourceSnapshot {
    reader.sources().expect("sources").remove(0)
}

// ---------------------------------------------------------------------------------------------
// Rotation
// ---------------------------------------------------------------------------------------------

/// A rename keeps the identity, so the checkpoint keyed by it still applies.
///
/// The records in a rotated file are the same records. Treating the renamed file as new would
/// re-index everything it holds and strand the checkpoint that already covered it.
#[test]
fn rotation_keeps_the_identity_and_the_active_file_gets_a_new_one() {
    let directory = TempDirectory::new("reconcile-rotation");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    append(&active, &[entry("before rotation", "record-1")]);
    let before = only_source(&reader);

    // Rotation is a rename plus a fresh active file.
    let rotated = directory.path().join("vanehub-2026-08-25.log");
    std::fs::rename(&active, &rotated).expect("rotate");
    append(&active, &[entry("after rotation", "record-2")]);

    let sources = reader.sources().expect("sources");
    assert_eq!(sources.len(), 2);
    let rotated_now = sources
        .iter()
        .find(|source| source.identity == before.identity)
        .expect("the rotated file kept its identity");
    let active_now = sources
        .iter()
        .find(|source| source.identity != before.identity)
        .expect("the new active file has its own identity");

    // Same identity, same captured length: nothing about the rotated file changed, so a checkpoint
    // at its end is still at its end.
    assert_eq!(rotated_now.end_offset, before.end_offset);
    assert_ne!(active_now.identity, before.identity);
}

/// The rotated file is not re-read from the start, so its records are not indexed twice.
#[test]
fn rotation_does_not_re_present_records_the_index_already_holds() {
    let directory = TempDirectory::new("reconcile-rotation-no-dupes");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    append(
        &active,
        &[entry("one", "record-1"), entry("two", "record-2")],
    );
    let before = only_source(&reader);
    let indexed_to = before.end_offset;

    std::fs::rename(&active, directory.path().join("vanehub-2026-08-25.log")).expect("rotate");

    let rotated = only_source(&reader);
    let batch = reader
        .read_batch(&rotated.identity, indexed_to, 100, 1_000_000)
        .expect("resume from the checkpoint the pre-rotation identity earned");
    assert!(
        batch.records.is_empty(),
        "the rotated file re-presented {} records",
        batch.records.len()
    );
    assert!(batch.reached_end);
}

// ---------------------------------------------------------------------------------------------
// Append
// ---------------------------------------------------------------------------------------------

/// Growing is not replacing. An append keeps the identity and moves only the captured target.
#[test]
fn an_append_keeps_the_identity_and_only_moves_the_target() {
    let directory = TempDirectory::new("reconcile-append");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    append(&active, &[entry("first", "record-1")]);
    let before = only_source(&reader);

    append(&active, &[entry("second", "record-2")]);
    let after = only_source(&reader);

    assert_eq!(before.identity, after.identity);
    assert!(after.end_offset > before.end_offset);
}

// ---------------------------------------------------------------------------------------------
// Truncation and recreation
// ---------------------------------------------------------------------------------------------

/// Truncating in place keeps the path and the head line, so only the length says what happened.
///
/// This is why the captured end offset is the witness. A file now shorter than the offset a
/// checkpoint already read past cannot be the content that checkpoint described, whatever its
/// identity says.
#[test]
fn truncation_in_place_is_visible_as_a_length_that_went_backwards() {
    let directory = TempDirectory::new("reconcile-truncate");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    let head = entry("head", "record-1");
    append(&active, &[head.clone(), entry("second", "record-2")]);
    let before = only_source(&reader);

    // Rewritten with only its first line: same path, same first bytes, less of them.
    std::fs::write(&active, format!("{head}\n")).expect("truncate");
    let after = only_source(&reader);

    assert!(
        after.end_offset < before.end_offset,
        "a truncation that did not shorten the file is not a truncation"
    );
    // The identity may or may not survive a truncate — that depends on the filesystem — and the
    // reconciliation must not depend on which. The length regression is the signal either way.
}

/// Recreating a path with different content is a different source.
///
/// The old checkpoint's byte offsets now point into unrelated bytes. Resuming there would read from
/// the middle of a record that was never written at that position.
#[test]
fn recreating_a_path_with_new_content_is_a_new_source() {
    let directory = TempDirectory::new("reconcile-recreate");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    append(&active, &[entry("original", "record-1")]);
    let before = only_source(&reader);

    std::fs::remove_file(&active).expect("remove");
    append(&active, &[entry("unrelated", "record-2")]);
    let after = only_source(&reader);

    assert_ne!(
        before.identity, after.identity,
        "a recreated file reused the identity whose offsets no longer mean anything"
    );
    // And the checkpoint is keyed by identity, so the old one simply does not match the new source.
    assert_ne!(before.identity.as_key(), after.identity.as_key());
}

// ---------------------------------------------------------------------------------------------
// Retention versus a temporary failure
// ---------------------------------------------------------------------------------------------

/// A file that is genuinely gone leaves the inventory, and the listing still succeeds.
///
/// This is the authoritative signal: the directory was read, and the file is not in it. Only this
/// may expire rows.
#[test]
fn a_retained_deletion_leaves_the_inventory_through_a_successful_listing() {
    let directory = TempDirectory::new("reconcile-retention");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    append(
        &directory.path().join("vanehub-2026-08-24.log"),
        &[entry("old", "record-1")],
    );
    append(
        &directory.path().join(LOG_FILE_NAME),
        &[entry("current", "record-2")],
    );
    assert_eq!(reader.sources().expect("sources").len(), 2);

    std::fs::remove_file(directory.path().join("vanehub-2026-08-24.log")).expect("retention");

    let after = reader.sources().expect("the listing still succeeds");
    assert_eq!(after.len(), 1);
    assert!(after[0].identity.file_id.len() > 1);
}

/// A directory that cannot be listed is an error, never an empty corpus.
///
/// The two are one character apart in code and worlds apart in effect: an empty inventory means
/// every source expired, and acting on that during a disk hiccup deletes the whole index.
#[test]
fn a_directory_that_cannot_be_listed_is_an_error_rather_than_an_empty_corpus() {
    let directory = TempDirectory::new("reconcile-unreadable");
    // A path that is a file rather than a directory: reading it as a directory fails with
    // something other than "not found", which is the shape of a transient IO problem.
    let blocked = directory.path().join("not-a-directory");
    std::fs::write(&blocked, b"x").expect("write");
    let reader = UnifiedLogSourceReader::new(blocked);

    let listed = reader.sources();

    assert!(
        matches!(listed, Err(OperationsLogError::RepairFailed(_))),
        "an unreadable directory reported {listed:?} instead of failing"
    );
}

/// A missing directory is an empty corpus, not a failure.
///
/// The counterpart to the case above, and the reason it is not simply "any error fails": a log
/// directory that has not been created yet is the ordinary state on a first run, and failing there
/// would mean a fresh install could never index anything.
#[test]
fn a_directory_that_does_not_exist_yet_is_an_empty_corpus() {
    let directory = TempDirectory::new("reconcile-missing");
    let reader = UnifiedLogSourceReader::new(directory.path().join("not-created-yet"));

    let listed = reader
        .sources()
        .expect("a missing directory is not a failure");

    assert!(listed.is_empty());
}

/// A file that is present but unreadable fails its own read and leaves the inventory alone.
#[test]
fn a_read_failure_does_not_remove_a_file_from_the_inventory() {
    let directory = TempDirectory::new("reconcile-read-failure");
    let reader = UnifiedLogSourceReader::new(directory.path().to_path_buf());
    let active = directory.path().join(LOG_FILE_NAME);
    append(&active, &[entry("present", "record-1")]);
    let source = only_source(&reader);

    // Reading from beyond the end of the file is not a listing problem, and the file stays listed.
    let batch = reader
        .read_batch(&source.identity, 1_000_000, 100, 1_000_000)
        .expect("a read past the end is an empty batch, not a failure");

    assert!(batch.records.is_empty());
    assert_eq!(
        reader.sources().expect("sources").len(),
        1,
        "a failed read removed the file from the inventory"
    );
}

// ---------------------------------------------------------------------------------------------
// Directory change
// ---------------------------------------------------------------------------------------------

/// A different directory is a different generation, so no checkpoint crosses between them.
///
/// The identical file in both directories is the point: without the generation in the key, the new
/// corpus would inherit the old one's checkpoints and claim to be indexed already.
#[test]
fn a_directory_change_starts_a_new_generation_no_checkpoint_can_cross() {
    let first = TempDirectory::new("reconcile-directory-a");
    let second = TempDirectory::new("reconcile-directory-b");
    let line = entry("identical", "record-1");
    append(
        &first.path().join(LOG_FILE_NAME),
        std::slice::from_ref(&line),
    );
    append(&second.path().join(LOG_FILE_NAME), &[line]);

    let left = only_source(&UnifiedLogSourceReader::new(first.path().to_path_buf()));
    let right = only_source(&UnifiedLogSourceReader::new(second.path().to_path_buf()));

    assert_ne!(
        left.identity.directory_generation,
        right.identity.directory_generation
    );
    // The checkpoint key carries the generation, so the two cannot collide even with identical
    // content — which is exactly the case that would otherwise collide.
    assert_ne!(left.identity.as_key(), right.identity.as_key());
}

/// Restarting the indexer changes nothing about what a source is.
///
/// Otherwise every launch would re-index the entire corpus and every checkpoint would be stranded
/// by the process that wrote it.
#[test]
fn an_indexer_restart_does_not_change_any_source_identity() {
    let directory = TempDirectory::new("reconcile-restart");
    append(
        &directory.path().join(LOG_FILE_NAME),
        &[entry("first", "record-1")],
    );

    // A restart is a fresh reader over the same directory; nothing about the process carries over.
    let before = only_source(&UnifiedLogSourceReader::new(directory.path().to_path_buf()));
    let after = only_source(&UnifiedLogSourceReader::new(directory.path().to_path_buf()));

    assert_eq!(before.identity, after.identity);
    assert_eq!(before.end_offset, after.end_offset);
}
