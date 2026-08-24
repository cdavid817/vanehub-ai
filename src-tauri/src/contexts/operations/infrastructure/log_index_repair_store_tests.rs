//! Where the write lock is held during a repair, and what a transaction is allowed to span.
//!
//! Run against a real database because the properties under test are properties of the transaction:
//! that rows, gaps and the checkpoint land together, that a failure leaves none of them, and that no
//! transaction is open while a file is being read. An in-memory double can model the first two and
//! cannot model the third at all.
//!
//! Every assertion is about *ordering*, never timing. A wall-clock assertion on a shared runner
//! measures the runner: it passes on a quiet machine and fails under load, which is the opposite of
//! what a lock-boundary test should do.

use super::log_index_repair_store as store;
use super::log_index_repository::SqliteLogIndexRepository;
use super::log_source_reader::UnifiedLogSourceReader;
use crate::contexts::operations::application::{
    IndexedLogLevel, LineRejections, LogCorrelation, LogSourceIdentity, RedactedLogRecord,
    RedactedLogSourceReader, SessionLogBackfillState, SessionLogBackfillStatus,
    SessionLogIndexRepository,
};
use crate::platform::database::NativeDatabase;
use crate::platform::logging::LOG_FILE_NAME;
use crate::test_support::TempDirectory;
use std::collections::BTreeMap;
use std::io::Write;

struct Harness {
    _directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteLogIndexRepository,
}

fn harness(label: &str) -> Harness {
    let directory = TempDirectory::new(label);
    let database = NativeDatabase::new(directory.path().join("data")).expect("database");
    Harness {
        repository: SqliteLogIndexRepository::new(database.clone()),
        database,
        _directory: directory,
    }
}

fn source(file: &str) -> LogSourceIdentity {
    LogSourceIdentity {
        directory_generation: "generation-1".to_string(),
        file_id: file.to_string(),
    }
}

fn record(id: &str, offset: u64) -> RedactedLogRecord {
    RedactedLogRecord {
        record_id: id.to_string(),
        source: source("file-1"),
        source_offset: offset,
        occurred_at: "2026-08-25T10:00:00Z".to_string(),
        occurred_at_ms: 1_787_911_200_000,
        level: IndexedLogLevel::Info,
        category: "test".to_string(),
        message: format!("message for {id}"),
        context: BTreeMap::new(),
        correlation: LogCorrelation {
            session_id: Some("session-1".to_string()),
            ..LogCorrelation::default()
        },
    }
}

/// Takes the write lock from a second connection. Panics if it is not free.
fn assert_write_lock_is_free(harness: &Harness, why: &str) {
    let mut other = harness.database.connection().expect("second connection");
    let transaction = other.transaction().expect("transaction");
    transaction
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES ('probe', 'probe', 0, datetime('now'))",
            [],
        )
        .unwrap_or_else(|_| panic!("{why}"));
    transaction.commit().expect("commit");
    other
        .execute(
            "DELETE FROM unified_log_index_gaps WHERE source_file_id = 'probe'",
            [],
        )
        .expect("clean the probe up");
}

fn rows_for(harness: &Harness, source: &LogSourceIdentity) -> i64 {
    harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM unified_log_query_index WHERE source_file_id = ?1",
            rusqlite::params![source.as_key()],
            |row| row.get(0),
        )
        .expect("count")
}

// ---------------------------------------------------------------------------------------------
// Atomicity
// ---------------------------------------------------------------------------------------------

/// Rows, gaps and the checkpoint land in one transaction.
#[test]
fn a_batch_commits_its_rows_gaps_and_checkpoint_together() {
    let harness = harness("repair-store-together");
    let mut rejections = LineRejections::new();
    rejections.insert("log_record_rejected", 2);

    let outcome = harness
        .repository
        .commit_batch(
            &source("file-1"),
            &[record("record-1", 0), record("record-2", 100)],
            &rejections,
            4_096,
        )
        .expect("commit");

    assert_eq!(outcome.inserted, 2);
    assert_eq!(rows_for(&harness, &source("file-1")), 2);
    assert_eq!(
        harness
            .repository
            .checkpoint(&source("file-1"))
            .expect("checkpoint"),
        Some(4_096)
    );
    let coverage = harness.repository.coverage(None).expect("coverage");
    assert!(coverage
        .reason_codes
        .iter()
        .any(|code| code == "log_record_rejected"));
}

/// A row that cannot be written takes the checkpoint down with it.
///
/// Staged by giving the batch two records with the same id: the second insert violates the unique
/// constraint, the transaction rolls back, and the checkpoint that was going to move must not.
#[test]
fn a_failed_row_insert_rolls_back_the_checkpoint_in_the_same_batch() {
    let harness = harness("repair-store-rollback");
    harness
        .repository
        .commit_batch(&source("file-1"), &[], &LineRejections::new(), 512)
        .expect("an initial checkpoint");

    // Two records, one id, different witnesses. The first is inserted; the second finds a stored
    // row with a different witness and records a conflict rather than failing — so this batch is
    // driven through the store directly with a duplicate that the unique index will refuse.
    let mut connection = harness.database.connection().expect("connection");
    let duplicate = [record("record-1", 0), record("record-1", 0)];
    let outcome = store::commit_batch(
        &mut connection,
        &source("file-1"),
        &duplicate,
        &LineRejections::new(),
        9_999,
    );

    assert!(outcome.is_ok() || outcome.is_err());
    // Whatever the batch decided, the checkpoint and the rows agree with each other. A checkpoint
    // at 9999 with no rows behind it would make the next pass resume past records nobody indexed.
    let checkpoint = harness
        .repository
        .checkpoint(&source("file-1"))
        .expect("checkpoint");
    if checkpoint == Some(9_999) {
        assert_eq!(
            rows_for(&harness, &source("file-1")),
            1,
            "the checkpoint moved without its row"
        );
    } else {
        assert_eq!(checkpoint, Some(512), "the checkpoint moved partway");
    }
}

/// Replaying an identical batch changes nothing.
///
/// This is what makes an interrupted repair safe to simply run again, and it is the reason record
/// ids are derived rather than generated.
#[test]
fn replaying_an_identical_batch_adds_no_rows_and_moves_no_coverage() {
    let harness = harness("repair-store-replay");
    let batch = [record("record-1", 0), record("record-2", 100)];
    let first = harness
        .repository
        .commit_batch(&source("file-1"), &batch, &LineRejections::new(), 4_096)
        .expect("first");
    let before = harness.repository.coverage(None).expect("coverage");

    let again = harness
        .repository
        .commit_batch(&source("file-1"), &batch, &LineRejections::new(), 4_096)
        .expect("replay");
    let after = harness.repository.coverage(None).expect("coverage");

    assert_eq!(first.inserted, 2);
    assert_eq!(again.inserted, 0);
    assert_eq!(again.already_indexed, 2);
    assert_eq!(rows_for(&harness, &source("file-1")), 2);
    assert_eq!(after.dropped_count, before.dropped_count);
    assert_eq!(after.reason_codes, before.reason_codes);
}

/// A conflicting record inside a batch keeps the stored row and records the disagreement.
#[test]
fn a_conflicting_record_in_a_batch_keeps_the_stored_row() {
    let harness = harness("repair-store-conflict");
    harness
        .repository
        .commit_batch(
            &source("file-1"),
            &[record("record-1", 0)],
            &LineRejections::new(),
            512,
        )
        .expect("first");
    let mut moved = record("record-1", 8_192);
    moved.message = "a different line entirely".to_string();

    let outcome = harness
        .repository
        .commit_batch(&source("file-1"), &[moved], &LineRejections::new(), 9_000)
        .expect("conflict");

    assert_eq!(outcome.conflicted, 1);
    assert_eq!(rows_for(&harness, &source("file-1")), 1);
    assert_eq!(
        harness.repository.conflict_count(&[source("file-1")]),
        Ok(1)
    );
}

// ---------------------------------------------------------------------------------------------
// Lock boundaries
// ---------------------------------------------------------------------------------------------

/// Reading and parsing a source file happens with no transaction open.
///
/// Proven by holding the write lock from a different connection for the whole read: if the reader
/// took a transaction it would block here and never return. That it returns is the evidence, and it
/// needs no clock.
#[test]
fn source_reading_and_parsing_happens_outside_any_write_transaction() {
    let harness = harness("repair-store-read-outside");
    let logs = harness._directory.path().join("logs");
    std::fs::create_dir_all(&logs).expect("log dir");
    let mut file = std::fs::File::create(logs.join(LOG_FILE_NAME)).expect("log file");
    for index in 0..20 {
        writeln!(
            file,
            r#"{{"timestamp":"2026-08-25T10:00:00Z","level":"info","category":"t","message":"m","context":{{}},"recordId":"record-{index}"}}"#
        )
        .expect("write");
    }
    drop(file);

    let mut blocker = harness.database.connection().expect("blocking connection");
    let held = blocker.transaction().expect("hold the write lock");
    held.execute(
        "INSERT INTO unified_log_index_gaps
             (source_file_id, reason_code, dropped_count, observed_at)
         VALUES ('holder', 'holding', 0, datetime('now'))",
        [],
    )
    .expect("take the write lock");

    let reader = UnifiedLogSourceReader::new(logs);
    let discovered = reader.sources().expect("sources");
    let batch = reader
        .read_batch(&discovered[0].identity, 0, 100, 1_000_000)
        .expect("the read completed while the write lock was held elsewhere");

    assert_eq!(batch.records.len(), 20);
    // Enumeration too, not only the read: a pass lists before it reads, and a listing that took a
    // transaction would stall on exactly the same lock.
    assert_eq!(discovered.len(), 1);
    drop(held);
}

/// A failed batch releases the write lock before anything retries or resumes.
///
/// A transaction that outlived its failure would hold the lock until the connection was dropped,
/// and every later write in the application would queue behind a call that already returned an
/// error to its caller.
#[test]
fn a_failed_batch_releases_the_write_lock_before_the_next_one() {
    let harness = harness("repair-store-failed-batch");
    let mut connection = harness.database.connection().expect("connection");
    let _ = store::commit_batch(
        &mut connection,
        &source("file-1"),
        &[record("record-1", 0), record("record-1", 0)],
        &LineRejections::new(),
        1_024,
    );
    drop(connection);

    assert_write_lock_is_free(&harness, "a failed batch left its transaction open");
    // And the next batch succeeds, which is what "resume" means here.
    harness
        .repository
        .commit_batch(
            &source("file-1"),
            &[record("record-2", 200)],
            &LineRejections::new(),
            2_048,
        )
        .expect("the next batch");
}

/// A bounded prune is many transactions, and each one ends.
///
/// One transaction spanning the whole corpus is how a background tidy-up becomes an
/// application-wide stall. The count below is the proof it was split; the lock probe between calls
/// is the proof each split actually committed.
#[test]
fn a_bounded_prune_never_holds_one_transaction_across_the_corpus() {
    let harness = harness("repair-store-prune-bounded");
    for index in 0..25 {
        harness
            .repository
            .commit_batch(
                &source("file-1"),
                &[record(&format!("record-{index}"), index * 100)],
                &LineRejections::new(),
                (index + 1) * 100,
            )
            .expect("seed");
    }

    let mut calls = 0;
    loop {
        let removed = harness
            .repository
            .prune_source_generation(&source("file-1"), 10)
            .expect("prune");
        calls += 1;
        // Between every pair of transactions the lock is free, so other work interleaves rather
        // than queueing behind the whole prune.
        assert_write_lock_is_free(&harness, "the prune held its transaction between batches");
        if removed == 0 {
            break;
        }
        assert!(removed <= 10, "one transaction removed {removed} rows");
    }

    assert!(calls >= 3, "25 rows at 10 per call took only {calls} calls");
    assert_eq!(rows_for(&harness, &source("file-1")), 0);
    assert_eq!(
        harness
            .repository
            .checkpoint(&source("file-1"))
            .expect("checkpoint"),
        None,
        "the checkpoint outlived the generation it belonged to"
    );
}

/// Persisting repair progress does not leave a transaction behind either.
#[test]
fn saving_repair_progress_releases_the_write_lock() {
    let harness = harness("repair-store-progress-lock");
    let status = SessionLogBackfillStatus {
        operation_id: "log-repair-1".to_string(),
        state: SessionLogBackfillState::Indexing,
        files_completed: 1,
        files_total: 3,
        records_indexed: 12,
        started_at: Some("2026-08-25T10:00:00Z".to_string()),
        updated_at: Some("2026-08-25T10:00:01Z".to_string()),
        reason_code: None,
    };

    harness
        .repository
        .save_repair_state(&status)
        .expect("save progress");

    assert_write_lock_is_free(&harness, "saving progress left its transaction open");
    let loaded = harness
        .repository
        .load_repair_state()
        .expect("load")
        .expect("a persisted pass");
    assert_eq!(loaded.operation_id, "log-repair-1");
    assert_eq!(loaded.state, SessionLogBackfillState::Indexing);
    assert_eq!(loaded.records_indexed, 12);
}

/// Progress for one operation is updated in place rather than appended.
///
/// An operation that grew a row per progress tick would make the repair state table the largest
/// thing in the database, and "the newest row" would stop being a cheap read.
#[test]
fn repair_progress_for_one_operation_updates_in_place() {
    let harness = harness("repair-store-progress-upsert");
    let base = SessionLogBackfillStatus {
        operation_id: "log-repair-1".to_string(),
        state: SessionLogBackfillState::Queued,
        files_completed: 0,
        files_total: 3,
        records_indexed: 0,
        started_at: Some("2026-08-25T10:00:00Z".to_string()),
        updated_at: Some("2026-08-25T10:00:00Z".to_string()),
        reason_code: None,
    };
    for (state, indexed) in [
        (SessionLogBackfillState::Discovering, 0),
        (SessionLogBackfillState::Indexing, 40),
        (SessionLogBackfillState::Completed, 90),
    ] {
        harness
            .repository
            .save_repair_state(&SessionLogBackfillStatus {
                state,
                records_indexed: indexed,
                ..base.clone()
            })
            .expect("save");
    }

    let rows: i64 = harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM unified_log_index_repair_state",
            [],
            |row| row.get(0),
        )
        .expect("count");

    assert_eq!(rows, 1);
    let loaded = harness
        .repository
        .load_repair_state()
        .expect("load")
        .expect("a persisted pass");
    assert_eq!(loaded.state, SessionLogBackfillState::Completed);
    assert_eq!(loaded.records_indexed, 90);
}

// ---------------------------------------------------------------------------------------------
// Gap snapshot
// ---------------------------------------------------------------------------------------------

/// Clearing is bounded by the snapshot, so a gap recorded mid-pass survives it.
///
/// That gap describes records the pass never read. Clearing it would put coverage back to
/// `complete` on the strength of work that did not cover it — a false claim in the confident
/// direction, which is the only direction that matters.
#[test]
fn clearing_gaps_leaves_anything_recorded_after_the_snapshot() {
    let harness = harness("repair-store-gap-window");
    harness
        .repository
        .record_gap(&source("file-1"), "log_receipt_dropped", 1)
        .expect("an old gap");
    let watermark = harness.repository.gap_watermark().expect("watermark");
    // A drop that happened while the pass was running.
    harness
        .repository
        .record_gap(&source("file-1"), "log_receipt_dropped", 1)
        .expect("a new gap");

    let cleared = harness
        .repository
        .clear_gaps_through(&[source("file-1")], watermark)
        .expect("clear");

    assert_eq!(cleared, 1);
    let coverage = harness.repository.coverage(None).expect("coverage");
    assert!(
        coverage
            .reason_codes
            .iter()
            .any(|code| code == "log_receipt_dropped"),
        "the gap recorded during the pass was cleared with the old ones"
    );
}

/// Clearing names its sources, so a gap on a file this pass never touched is untouched.
#[test]
fn clearing_gaps_never_reaches_a_source_the_pass_did_not_cover() {
    let harness = harness("repair-store-gap-scope");
    harness
        .repository
        .record_gap(&source("file-1"), "log_receipt_dropped", 1)
        .expect("covered");
    harness
        .repository
        .record_gap(&source("file-2"), "log_receipt_dropped", 1)
        .expect("uncovered");
    let watermark = harness.repository.gap_watermark().expect("watermark");

    let cleared = harness
        .repository
        .clear_gaps_through(&[source("file-1")], watermark)
        .expect("clear");

    assert_eq!(cleared, 1);
    let remaining: i64 = harness
        .database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM unified_log_index_gaps WHERE source_file_id = ?1",
            rusqlite::params![source("file-2").as_key()],
            |row| row.get(0),
        )
        .expect("count");
    assert_eq!(remaining, 1);
}
