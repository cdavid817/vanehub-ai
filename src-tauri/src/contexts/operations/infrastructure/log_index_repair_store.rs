//! The storage a bounded repair needs: batch commits, persisted progress, and bounded pruning.
//!
//! Split from the query repository because the two have opposite shapes. A query reads one page and
//! holds nothing; a repair writes in bounded batches and has to be resumable across a restart. The
//! one rule they share is the reason this module exists at all: **rows, gaps and the checkpoint
//! commit together or not at all.** A checkpoint ahead of its rows makes the next pass resume past
//! records that were never indexed, and nothing afterwards can tell — the offset says they were
//! read, and "read" is the only claim a checkpoint makes.

use super::log_index_repository::{context_json, storage_error};
use crate::contexts::operations::application::{
    LineRejections, LogBatchCommit, LogSourceIdentity, OperationsLogError, RedactedLogRecord,
    SessionLogBackfillState, SessionLogBackfillStatus,
};
use rusqlite::{params, Connection, OptionalExtension};

/// Writes one batch's rows, gaps and checkpoint inside a single transaction.
///
/// The witness check happens inside the transaction rather than before it, for the same reason the
/// single-record path does it there: reading first and inserting after leaves a window where two
/// callers both see "absent" and both insert, and the unique constraint reports that as a storage
/// failure rather than as the retry it is.
pub(crate) fn commit_batch(
    connection: &mut Connection,
    source: &LogSourceIdentity,
    records: &[RedactedLogRecord],
    rejections: &LineRejections,
    next_offset: u64,
) -> Result<LogBatchCommit, OperationsLogError> {
    let transaction = connection.transaction().map_err(storage_error)?;
    let mut outcome = LogBatchCommit::default();
    for record in records {
        let stored: Option<(String, i64)> = transaction
            .query_row(
                "SELECT source_file_id, source_offset FROM unified_log_query_index
                 WHERE record_id = ?1",
                params![record.record_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(storage_error)?;
        match stored {
            Some((file_id, offset))
                if file_id == record.source.as_key() && offset == record.source_offset as i64 =>
            {
                // The same line, seen again. That is what a deterministic id is for, so a repeated
                // pass over the same file adds nothing and is not an error.
                outcome.already_indexed += 1;
            }
            Some(_) => {
                // One id, two different lines. The stored row wins — a reader may already have
                // cited it — and coverage stops claiming to be whole.
                record_gap_in(&transaction, source, "log_identity_conflict", 1)?;
                outcome.conflicted += 1;
            }
            None => {
                insert_record(&transaction, record)?;
                outcome.inserted += 1;
            }
        }
    }
    for (reason, count) in rejections {
        record_gap_in(&transaction, source, reason, *count)?;
    }
    save_checkpoint_in(&transaction, source, next_offset)?;
    transaction.commit().map_err(storage_error)?;
    Ok(outcome)
}

fn insert_record(
    connection: &Connection,
    record: &RedactedLogRecord,
) -> Result<(), OperationsLogError> {
    connection
        .execute(
            "INSERT INTO unified_log_query_index (
                record_id, source_file_id, source_offset, occurred_at, occurred_at_ms,
                level, category, message, safe_context_json,
                session_id, run_id, trace_id, span_id, operation_id, agent_id, seat_id,
                redaction_applied
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, 1)",
            params![
                record.record_id,
                record.source.as_key(),
                record.source_offset as i64,
                record.occurred_at,
                record.occurred_at_ms,
                record.level.token(),
                record.category,
                record.message,
                context_json(&record.context),
                record.correlation.session_id,
                record.correlation.run_id,
                record.correlation.trace_id,
                record.correlation.span_id,
                record.correlation.operation_id,
                record.correlation.agent_id,
                record.correlation.seat_id,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn record_gap_in(
    connection: &Connection,
    source: &LogSourceIdentity,
    reason_code: &str,
    dropped: u32,
) -> Result<(), OperationsLogError> {
    connection
        .execute(
            "INSERT INTO unified_log_index_gaps
                 (source_file_id, reason_code, dropped_count, observed_at)
             VALUES (?1, ?2, ?3, datetime('now'))",
            params![source.as_key(), reason_code, i64::from(dropped)],
        )
        .map_err(storage_error)?;
    Ok(())
}

fn save_checkpoint_in(
    connection: &Connection,
    source: &LogSourceIdentity,
    offset: u64,
) -> Result<(), OperationsLogError> {
    connection
        .execute(
            "INSERT INTO unified_log_source_checkpoints
                 (source_file_id, directory_generation, next_offset, updated_at)
             VALUES (?1, ?2, ?3, datetime('now'))
             ON CONFLICT(source_file_id) DO UPDATE SET
                 next_offset = excluded.next_offset,
                 directory_generation = excluded.directory_generation,
                 updated_at = excluded.updated_at",
            params![source.as_key(), source.directory_generation, offset as i64],
        )
        .map_err(storage_error)?;
    Ok(())
}

/// The most recent repair, whatever became of it.
///
/// Read at startup so a pass interrupted by a crash can be seen as interrupted rather than as one
/// that never happened. The checkpoints are what actually make it resumable; this is what lets the
/// resumed pass say what it is resuming.
pub(crate) fn load_repair_state(
    connection: &Connection,
) -> Result<Option<SessionLogBackfillStatus>, OperationsLogError> {
    connection
        .query_row(
            "SELECT operation_id, state, files_completed, files_total, records_indexed,
                    reason_code, started_at, updated_at
             FROM unified_log_index_repair_state
             ORDER BY updated_at DESC, rowid DESC LIMIT 1",
            [],
            |row| {
                let state: String = row.get(1)?;
                Ok(SessionLogBackfillStatus {
                    operation_id: row.get(0)?,
                    // An unrecognised state is a row this build did not write. Reading it as
                    // `Failed` rather than guessing keeps a downgrade from resuming a pass whose
                    // rules it does not know.
                    state: SessionLogBackfillState::parse(&state)
                        .unwrap_or(SessionLogBackfillState::Failed),
                    files_completed: row.get::<_, i64>(2)?.max(0) as u32,
                    files_total: row.get::<_, i64>(3)?.max(0) as u32,
                    records_indexed: row.get::<_, i64>(4)?.max(0) as u64,
                    reason_code: row.get(5)?,
                    started_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()
        .map_err(storage_error)
}

pub(crate) fn save_repair_state(
    connection: &Connection,
    status: &SessionLogBackfillStatus,
) -> Result<(), OperationsLogError> {
    connection
        .execute(
            "INSERT INTO unified_log_index_repair_state
                 (operation_id, state, files_completed, files_total, records_indexed,
                  reason_code, started_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(operation_id) DO UPDATE SET
                 state = excluded.state,
                 files_completed = excluded.files_completed,
                 files_total = excluded.files_total,
                 records_indexed = excluded.records_indexed,
                 reason_code = excluded.reason_code,
                 updated_at = excluded.updated_at",
            params![
                status.operation_id,
                status.state.token(),
                i64::from(status.files_completed),
                i64::from(status.files_total),
                status.records_indexed as i64,
                status.reason_code,
                status.started_at,
                status.updated_at,
            ],
        )
        .map_err(storage_error)?;
    Ok(())
}

pub(crate) fn gap_watermark(connection: &Connection) -> Result<i64, OperationsLogError> {
    connection
        .query_row(
            "SELECT COALESCE(MAX(id), 0) FROM unified_log_index_gaps",
            [],
            |row| row.get(0),
        )
        .map_err(storage_error)
}

/// Clears gaps for the named sources up to and including a snapshot id.
///
/// The bound is the whole point. "Delete every gap for this source" would also erase a drop that
/// happened while the repair ran — a hole in records this pass never read — and coverage would go
/// back to `complete` on the strength of work that did not cover it.
pub(crate) fn clear_gaps_through(
    connection: &Connection,
    sources: &[LogSourceIdentity],
    through_id: i64,
) -> Result<u32, OperationsLogError> {
    if sources.is_empty() {
        return Ok(0);
    }
    let keys: Vec<String> = sources.iter().map(LogSourceIdentity::as_key).collect();
    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let mut bindings: Vec<&dyn rusqlite::ToSql> =
        keys.iter().map(|key| key as &dyn rusqlite::ToSql).collect();
    bindings.push(&through_id);
    let cleared = connection
        .execute(
            &format!(
                "DELETE FROM unified_log_index_gaps
                 WHERE source_file_id IN ({placeholders}) AND id <= ?{}",
                keys.len() + 1
            ),
            bindings.as_slice(),
        )
        .map_err(storage_error)?;
    Ok(u32::try_from(cleared).unwrap_or(u32::MAX))
}

pub(crate) fn conflict_count(
    connection: &Connection,
    sources: &[LogSourceIdentity],
) -> Result<u32, OperationsLogError> {
    if sources.is_empty() {
        return Ok(0);
    }
    let keys: Vec<String> = sources.iter().map(LogSourceIdentity::as_key).collect();
    let placeholders = keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ");
    let bindings: Vec<&dyn rusqlite::ToSql> =
        keys.iter().map(|key| key as &dyn rusqlite::ToSql).collect();
    let count: i64 = connection
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM unified_log_index_gaps
                 WHERE reason_code = 'log_identity_conflict'
                   AND source_file_id IN ({placeholders})"
            ),
            bindings.as_slice(),
            |row| row.get(0),
        )
        .map_err(storage_error)?;
    Ok(u32::try_from(count).unwrap_or(u32::MAX))
}

/// Deletes one superseded generation's rows, at most `limit` at a time.
///
/// Bounded because a corpus can be larger than one transaction should hold. Holding the write lock
/// across the whole prune is how a background tidy-up becomes an application-wide stall, so the
/// caller loops on this and each call is a transaction that ends.
pub(crate) fn prune_source_generation(
    connection: &mut Connection,
    source: &LogSourceIdentity,
    limit: u32,
) -> Result<u32, OperationsLogError> {
    let transaction = connection.transaction().map_err(storage_error)?;
    let removed = transaction
        .execute(
            "DELETE FROM unified_log_query_index
             WHERE rowid IN (
                 SELECT rowid FROM unified_log_query_index
                 WHERE source_file_id = ?1 LIMIT ?2
             )",
            params![source.as_key(), i64::from(limit)],
        )
        .map_err(storage_error)?;
    // The checkpoint goes with the last batch, not the first. Removing it up front would let a
    // concurrent pass re-index the generation being pruned from offset zero.
    if removed == 0 {
        transaction
            .execute(
                "DELETE FROM unified_log_source_checkpoints WHERE source_file_id = ?1",
                params![source.as_key()],
            )
            .map_err(storage_error)?;
    }
    transaction.commit().map_err(storage_error)?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}
