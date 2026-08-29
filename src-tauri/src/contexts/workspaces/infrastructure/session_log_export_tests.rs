//! Which store an export is allowed to read, settled by three cases that can only disagree.
//!
//! The redacted JSONL files are the durable record; the SQLite index is a projection built from
//! them that can be deleted and rebuilt without losing anything. Everywhere else in this change
//! that asymmetry is a design statement. Here it is a test, because an export is the one artifact a
//! user keeps: a page that was briefly short corrects itself on refresh, and a file that was
//! briefly short is just wrong forever, with nothing on it to say so.
//!
//! Each fixture below puts the two stores in a state where they must give different answers, and
//! asserts the file's answer wins:
//!
//! 1. the index is empty and the files hold ten records — an index-backed export writes nothing;
//! 2. the index holds seven of ten — an index-backed export silently drops three;
//! 3. the index holds a row no file has — an index-backed export invents one.
//!
//! The third is the one that cannot be explained away as a timing artefact. A row with no line
//! behind it can only come from the projection being wrong, and an export that emitted it would be
//! presenting the projection's mistake as the user's log.

use super::session_queries::{all_filtered_log_entries, log_entry_matches};
use crate::contexts::operations::application::{
    IndexedLogLevel, LogCorrelation, LogSourceIdentity, RedactedLogRecord,
    SessionLogIndexRepository,
};
use crate::contexts::operations::log_api::assemble_log_index_for_tests;
use crate::contexts::workspaces::application::{SessionLogQuery, WorkspaceLogLevel};
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::io::Write;
use std::path::Path;

const SESSION: &str = "session-1";

struct Fixture {
    directory: TempDirectory,
    /// Held behind its port. The concrete repository is the operations context's persistence, and
    /// naming it from here would be a coupling this fixture does not need and the context map does
    /// not allow.
    index: std::sync::Arc<dyn SessionLogIndexRepository>,
}

fn fixture(label: &str) -> Fixture {
    let directory = TempDirectory::new(label);
    let index = assemble_log_index_for_tests(&directory.path().join("data")).expect("log index");
    Fixture { index, directory }
}

impl Fixture {
    fn log_dir(&self) -> &Path {
        self.directory.path()
    }
}

fn line(index: usize) -> String {
    format!(
        r#"{{"timestamp":"2026-08-25T10:00:{index:02}Z","level":"info","category":"test","message":"line {index}","context":{{"sessionId":"{SESSION}"}},"recordId":"record-{index}"}}"#
    )
}

/// Ten records on disk, which is the corpus every case below shares.
fn write_ten_records(fixture: &Fixture) {
    let mut file =
        std::fs::File::create(fixture.log_dir().join(LOG_FILE_NAME)).expect("create log file");
    for index in 0..10 {
        writeln!(file, "{}", line(index)).expect("write");
    }
}

fn indexed(record_id: &str, offset: u64) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: record_id.to_string(),
        source: LogSourceIdentity {
            directory_generation: "generation-1".to_string(),
            file_id: "file-1".to_string(),
        },
        source_offset: offset,
        occurred_at: "2026-08-25T10:00:00Z".to_string(),
        occurred_at_ms: 1_787_911_200_000,
        level: IndexedLogLevel::Info,
        category: "test".to_string(),
        message: format!("indexed {record_id}"),
        context: BTreeMap::from([("sessionId".to_string(), SESSION.to_string())]),
        correlation: LogCorrelation {
            session_id: Some(SESSION.to_string()),
            ..LogCorrelation::default()
        },
    }
}

fn query() -> SessionLogQuery {
    SessionLogQuery {
        session_id: SESSION.to_string(),
        seat_id: None,
        levels: Vec::new(),
        search: String::new(),
        cursor: None,
        limit: None,
    }
}

fn indexed_row_count(fixture: &Fixture) -> usize {
    fixture
        .index
        .query(
            &crate::contexts::operations::application::IndexedSessionLogQuery {
                scope: crate::contexts::operations::application::SessionLogQueryScope {
                    session_id: Some(SESSION.to_string()),
                    ..Default::default()
                },
                filters: Default::default(),
                cursor: None,
                limit: Some(500),
            },
        )
        .expect("query the index")
        .items
        .len()
}

/// Case 1: the index is empty and the files hold ten.
///
/// This is the ordinary state right after a fresh install, a rebuilt index, or a repair that has
/// not started yet — and it is when a user is most likely to export, because something went wrong.
/// An index-backed export here writes an empty file that looks exactly like "nothing was logged".
#[test]
fn an_empty_index_does_not_shorten_an_export() {
    let fixture = fixture("export-authority-empty-index");
    write_ten_records(&fixture);

    let exported = all_filtered_log_entries(fixture.log_dir(), &query()).expect("export");

    assert_eq!(indexed_row_count(&fixture), 0, "the index was not empty");
    assert_eq!(exported.len(), 10);
}

/// Case 2: the index holds seven of the ten records on disk.
///
/// A repair that is partway through, or one that dropped receipts under load. The export must still
/// be ten: the three the index has not caught up to are on disk and are as real as the other seven.
#[test]
fn a_partially_indexed_corpus_still_exports_every_record_on_disk() {
    let fixture = fixture("export-authority-partial-index");
    write_ten_records(&fixture);
    for index in 0..7 {
        fixture
            .index
            .insert(&indexed(&format!("record-{index}"), index as u64 * 100))
            .expect("index seven of the ten");
    }

    let exported = all_filtered_log_entries(fixture.log_dir(), &query()).expect("export");

    assert_eq!(indexed_row_count(&fixture), 7);
    assert_eq!(
        exported.len(),
        10,
        "the export was shortened to what the index happened to hold"
    );
}

/// Case 3: the index holds a row no file has.
///
/// This one cannot be a timing artefact. A row with no line behind it means the projection is
/// wrong — a stale generation, a bad repair, a rebuild that ran against a different directory — and
/// an export that emitted it would put a record in the user's log file that was never logged.
#[test]
fn an_orphan_index_row_is_absent_from_the_export() {
    let fixture = fixture("export-authority-orphan-row");
    write_ten_records(&fixture);
    fixture
        .index
        .insert(&indexed("record-orphan", 999_999))
        .expect("an index row with no line behind it");

    let exported = all_filtered_log_entries(fixture.log_dir(), &query()).expect("export");

    assert_eq!(indexed_row_count(&fixture), 1);
    assert_eq!(exported.len(), 10);
    assert!(
        !exported
            .iter()
            .any(|entry| entry.message.contains("record-orphan")),
        "the export carried a record no log file holds"
    );
}

/// Filters narrow the export through the same predicate a preview uses.
///
/// Two implementations of "is this record in scope" would drift the first time a filter gained a
/// field, and the drift shows up as an export containing more or less than the list the user was
/// looking at when they clicked it.
#[test]
fn export_filters_run_through_the_same_predicate_a_preview_uses() {
    let fixture = fixture("export-authority-filters");
    let mut file =
        std::fs::File::create(fixture.log_dir().join(LOG_FILE_NAME)).expect("create log file");
    writeln!(
        file,
        r#"{{"timestamp":"2026-08-25T10:00:00Z","level":"error","category":"test","message":"a failure","context":{{"sessionId":"{SESSION}"}}}}"#
    )
    .expect("write");
    writeln!(
        file,
        r#"{{"timestamp":"2026-08-25T10:00:01Z","level":"info","category":"test","message":"routine","context":{{"sessionId":"{SESSION}"}}}}"#
    )
    .expect("write");
    writeln!(
        file,
        r#"{{"timestamp":"2026-08-25T10:00:02Z","level":"error","category":"test","message":"another session","context":{{"sessionId":"session-2"}}}}"#
    )
    .expect("write");
    drop(file);

    let mut narrowed = query();
    narrowed.levels = vec![WorkspaceLogLevel::Error];
    let exported = all_filtered_log_entries(fixture.log_dir(), &narrowed).expect("export");

    assert_eq!(exported.len(), 1);
    assert_eq!(exported[0].message, "a failure");
    // The same predicate, applied directly to a parsed record, agrees. That is the property: one
    // function, so the two cannot disagree.
    assert!(log_entry_matches(&exported[0], &narrowed));
}

/// A record from another session is out of scope for both, so neither can leak it.
#[test]
fn the_shared_predicate_excludes_another_sessions_record() {
    let fixture = fixture("export-authority-scope");
    let mut file =
        std::fs::File::create(fixture.log_dir().join(LOG_FILE_NAME)).expect("create log file");
    writeln!(
        file,
        r#"{{"timestamp":"2026-08-25T10:00:00Z","level":"info","category":"test","message":"other","context":{{"sessionId":"session-2"}}}}"#
    )
    .expect("write");
    drop(file);

    let exported = all_filtered_log_entries(fixture.log_dir(), &query()).expect("export");

    assert!(exported.is_empty());
}

/// An export of a corpus with no files is empty rather than an error.
///
/// A first run has no logs yet, and an export that failed there would report a problem where there
/// is none.
#[test]
fn an_export_with_no_log_files_is_empty_rather_than_a_failure() {
    let fixture = fixture("export-authority-no-files");

    let exported = all_filtered_log_entries(fixture.log_dir(), &query()).expect("export");

    assert!(exported.is_empty());
}
