//! Where the database write lock is held, and where it must not be.
//!
//! The failure these guard against does not look like a bug when it happens: the application simply
//! becomes unresponsive while a disk is slow, and the reason is a transaction that was opened before
//! the slow thing started. So the boundary is tested by *ordering* — which side of the transaction
//! an operation happens on — rather than by timing it, because a wall-clock assertion on a shared
//! runner measures the runner.

use super::log_index_repository::SqliteLogIndexRepository;
use crate::contexts::operations::application::{
    IndexedLogLevel, LogCorrelation, LogSourceIdentity, RedactedLogRecord, RedactedLogSourceReader,
    SessionLogIndexRepository,
};
use crate::contexts::operations::infrastructure::log_source_reader::UnifiedLogSourceReader;
use crate::platform::database::NativeDatabase;
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::io::Write;

fn record(id: &str, offset: u64) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: id.to_string(),
        source: LogSourceIdentity {
            directory_generation: "generation-1".to_string(),
            file_id: "file-1".to_string(),
        },
        source_offset: offset,
        occurred_at: "2026-08-24T10:00:00Z".to_string(),
        occurred_at_ms: 1_787_824_800_000,
        level: IndexedLogLevel::Info,
        category: "test".to_string(),
        message: "hello".to_string(),
        context: BTreeMap::new(),
        correlation: LogCorrelation::default(),
    }
}

/// Reading and parsing a source file happens with no transaction open.
///
/// Proven by holding the database's write lock from a *different* connection for the whole read: if
/// the reader took a transaction, it would block here and never return. That it returns is the
/// evidence, and it needs no clock.
#[test]
fn reading_a_source_file_does_not_wait_on_the_database_write_lock() {
    let fixture = TempDirectory::new("log-lock-read");
    let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
    let logs = fixture.path().join("logs");
    std::fs::create_dir_all(&logs).expect("log dir");
    let mut file = std::fs::File::create(logs.join(LOG_FILE_NAME)).expect("log file");
    writeln!(
        file,
        r#"{{"timestamp":"2026-08-24T10:00:00Z","level":"info","category":"t","message":"m","context":{{}},"recordId":"record-1"}}"#
    )
    .expect("write");
    drop(file);

    // A second connection holding an exclusive write transaction for the duration.
    let mut blocker = database.connection().expect("blocking connection");
    let held = blocker.transaction().expect("hold the write lock");
    held.execute(
        "INSERT INTO unified_log_index_gaps
             (source_file_id, reason_code, dropped_count, observed_at)
         VALUES ('file-x', 'holding', 0, datetime('now'))",
        [],
    )
    .expect("take the write lock");

    let reader = UnifiedLogSourceReader::new(logs);
    let source = reader.sources().expect("sources").remove(0);
    let batch = reader
        .read_batch(&source, 0, 10, 10_000)
        .expect("the read completed while the write lock was held elsewhere");

    assert_eq!(batch.records.len(), 1);
    drop(held);
}

/// One failed insert leaves no transaction open behind it.
///
/// A transaction that outlived its failure would hold the write lock until the connection was
/// dropped, and every later write in the application would queue behind a call that had already
/// returned an error to its caller.
#[test]
fn a_failed_insert_leaves_no_transaction_holding_the_write_lock() {
    let fixture = TempDirectory::new("log-lock-failed-insert");
    let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
    let repository = SqliteLogIndexRepository::new(database.clone());

    // A record whose id is already taken by a different witness: the conflict path commits, and the
    // insert path is never reached.
    repository.insert(&record("record-1", 0)).expect("first");
    let mut moved = record("record-1", 4096);
    moved.message = "different".to_string();
    repository.insert(&moved).expect("conflict");

    // If either path had left a transaction open, this exclusive write from another connection
    // would fail rather than succeed.
    let mut other = database.connection().expect("second connection");
    let transaction = other.transaction().expect("transaction");
    transaction
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES ('file-y', 'probe', 0, datetime('now'))",
            [],
        )
        .expect("the write lock was free");
    transaction.commit().expect("commit");
}

/// A query holds nothing once it has answered.
///
/// The page is materialised before the caller sees it, so a slow consumer of the result cannot keep
/// a read transaction open against the index while it renders.
#[test]
fn a_query_releases_its_connection_before_returning_the_page() {
    let fixture = TempDirectory::new("log-lock-query");
    let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
    let repository = SqliteLogIndexRepository::new(database.clone());
    repository.insert(&record("record-1", 0)).expect("insert");

    let page = repository
        .query(
            &crate::contexts::operations::application::IndexedSessionLogQuery {
                scope: Default::default(),
                filters: Default::default(),
                cursor: None,
                limit: Some(10),
            },
        )
        .expect("query");

    // Still holding the page, take the write lock from another connection. A statement or
    // transaction left open by the query would make this fail.
    let mut other = database.connection().expect("second connection");
    let transaction = other.transaction().expect("transaction");
    transaction
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES ('file-z', 'probe', 0, datetime('now'))",
            [],
        )
        .expect("the write lock was free while a page was held");
    transaction.commit().expect("commit");
    assert_eq!(page.items.len(), 1);
}

/// A batch is prepared entirely before any database work begins.
///
/// The reader returns owned records; nothing in them borrows a file handle or a statement, so the
/// caller cannot accidentally keep the source open across its inserts.
#[test]
fn a_source_batch_is_fully_materialised_before_any_insert() {
    let fixture = TempDirectory::new("log-lock-batch");
    let logs = fixture.path().join("logs");
    std::fs::create_dir_all(&logs).expect("log dir");
    let mut file = std::fs::File::create(logs.join(LOG_FILE_NAME)).expect("log file");
    for index in 0..3 {
        writeln!(
            file,
            r#"{{"timestamp":"2026-08-24T10:00:00Z","level":"info","category":"t","message":"m","context":{{}},"recordId":"record-{index}"}}"#
        )
        .expect("write");
    }
    drop(file);
    let reader = UnifiedLogSourceReader::new(logs.clone());
    let source = reader.sources().expect("sources").remove(0);

    let batch = reader.read_batch(&source, 0, 10, 100_000).expect("batch");
    // The file can be removed before a single row is written: the batch is already whole.
    std::fs::remove_file(logs.join(LOG_FILE_NAME)).expect("remove the source");

    let database = NativeDatabase::new(fixture.path().join("data")).expect("database");
    let repository = SqliteLogIndexRepository::new(database);
    for entry in &batch.records {
        repository
            .insert(entry)
            .expect("insert after the file is gone");
    }
    assert_eq!(batch.records.len(), 3);
}
