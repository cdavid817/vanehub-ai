//! The SQLite half of the redacted log query index.
//!
//! Reading and parsing source files happens in the reader, not here, and never inside a
//! transaction: a transaction held across file IO holds the write lock for as long as the disk
//! takes, which turns one slow read into an application-wide stall.

use crate::contexts::operations::application::{
    filter_fingerprint, IndexedLogLevel, IndexedSessionLogPage, IndexedSessionLogQuery,
    IndexedSessionLogRecord, LogCorrelation, LogIndexInsertOutcome, LogPageCursor,
    LogSortDirection, LogSourceIdentity, OperationsLogError, RedactedLogRecord, SessionLogCoverage,
    SessionLogCoverageState, SessionLogIndexRepository, DEFAULT_LOG_PAGE_SIZE, MAX_LOG_PAGE_SIZE,
    MAX_LOG_SEARCH_CANDIDATES,
};
use crate::platform::database::{NativeDatabase, PooledSqlite};
use rusqlite::{params, Connection, OptionalExtension, Row};
use std::collections::BTreeMap;

#[derive(Clone)]
pub(crate) struct SqliteLogIndexRepository {
    database: NativeDatabase,
}

impl SqliteLogIndexRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    /// A connection, or the one error a caller can act on.
    ///
    /// The underlying message names a file path, so it is dropped rather than forwarded: the index
    /// being unavailable is the fact a caller needs, and where it lives is not.
    fn connection(&self) -> Result<PooledSqlite, OperationsLogError> {
        self.database
            .connection()
            .map_err(|_| OperationsLogError::IndexUnavailable("log_index_connection_unavailable"))
    }
}

fn storage_error(_error: rusqlite::Error) -> OperationsLogError {
    OperationsLogError::IndexUnavailable("log_index_storage_failed")
}

fn context_json(context: &BTreeMap<String, String>) -> String {
    serde_json::to_string(context).unwrap_or_else(|_| "{}".to_string())
}

fn parse_context(raw: &str) -> BTreeMap<String, String> {
    serde_json::from_str(raw).unwrap_or_default()
}

/// Case-insensitive containment over the already-redacted fields only.
///
/// Message, category, and safe context values — nothing else exists to search. The needle is
/// treated as literal text rather than as a pattern, so a user typing `%` or `_` searches for those
/// characters instead of accidentally writing a wildcard that matches everything.
fn matches_search(record: &IndexedSessionLogRecord, needle: &str) -> bool {
    let needle = needle.to_lowercase();
    if needle.is_empty() {
        return true;
    }
    record.message.to_lowercase().contains(&needle)
        || record.category.to_lowercase().contains(&needle)
        || record
            .context
            .values()
            .any(|value| value.to_lowercase().contains(&needle))
}

impl IndexedSessionLogRecord {
    /// The millisecond key this row sorts on, recovered from its own timestamp.
    fn sequence_time(&self) -> i64 {
        chrono::DateTime::parse_from_rfc3339(&self.occurred_at)
            .map(|value| value.timestamp_millis())
            .unwrap_or_default()
    }
}

fn read_record(row: &Row<'_>) -> rusqlite::Result<IndexedSessionLogRecord> {
    let context: String = row.get("safe_context_json")?;
    let level: String = row.get("level")?;
    Ok(IndexedSessionLogRecord {
        record_id: row.get("record_id")?,
        sequence: row.get("sequence")?,
        occurred_at: row.get("occurred_at")?,
        // An unrecognised level is read as `info` rather than dropping the row: the row is real
        // and its text is what the reader came for, and refusing it would turn one bad enum into
        // a hole in the transcript.
        level: IndexedLogLevel::parse(&level).unwrap_or(IndexedLogLevel::Info),
        category: row.get("category")?,
        message: row.get("message")?,
        context: parse_context(&context),
        correlation: LogCorrelation {
            session_id: row.get("session_id")?,
            run_id: row.get("run_id")?,
            trace_id: row.get("trace_id")?,
            span_id: row.get("span_id")?,
            operation_id: row.get("operation_id")?,
            agent_id: row.get("agent_id")?,
            seat_id: row.get("seat_id")?,
        },
    })
}

/// Whether a stored row and an incoming record are the same record.
///
/// Same id and same witness is a retry; same id and a different witness is a conflict, and the
/// stored row wins. Overwriting would let a later, differently-derived record silently replace
/// the one a reader may already have cited.
fn same_witness(
    connection: &Connection,
    record: &RedactedLogRecord,
) -> rusqlite::Result<Option<bool>> {
    connection
        .query_row(
            "SELECT source_file_id, source_offset FROM unified_log_query_index WHERE record_id = ?1",
            params![record.record_id],
            |row| {
                let file_id: String = row.get(0)?;
                let offset: i64 = row.get(1)?;
                Ok(file_id == record.source.as_key() && offset == record.source_offset as i64)
            },
        )
        .optional()
}

impl SessionLogIndexRepository for SqliteLogIndexRepository {
    /// Writes one record, or recognises that it is already written.
    ///
    /// The whole thing is one transaction, and the read that decides which case this is happens
    /// inside it. Reading first and inserting after would leave a window where two callers both see
    /// "absent" and both insert; the unique constraint would catch the second, but as a storage
    /// failure rather than as the retry it actually is.
    ///
    /// A conflicting record — same id, different witness — leaves the stored row alone and records
    /// a gap, so coverage stops claiming to be complete. Overwriting would let a later,
    /// differently-derived record silently replace one a reader may already have cited.
    fn insert(
        &self,
        record: &RedactedLogRecord,
    ) -> Result<LogIndexInsertOutcome, OperationsLogError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction().map_err(storage_error)?;
        if let Some(matches) = same_witness(&transaction, record).map_err(storage_error)? {
            if !matches {
                // Recorded inside the same transaction as the decision, so coverage cannot report
                // complete between noticing the conflict and writing it down.
                transaction
                    .execute(
                        "INSERT INTO unified_log_index_gaps
                             (source_file_id, reason_code, dropped_count, observed_at)
                         VALUES (?1, 'log_identity_conflict', 1, datetime('now'))",
                        params![record.source.as_key()],
                    )
                    .map_err(storage_error)?;
                transaction.commit().map_err(storage_error)?;
                return Ok(LogIndexInsertOutcome::Conflicted);
            }
            // Nothing to write, so nothing to commit. A retry of an already-indexed record must
            // leave the store exactly as it was, including its coverage.
            return Ok(LogIndexInsertOutcome::AlreadyIndexed);
        }
        transaction
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
        let sequence = transaction.last_insert_rowid();
        // The sequence is read before the commit and returned after it. A notice carrying a
        // sequence whose row is not committed would name a record a reader cannot find.
        transaction.commit().map_err(storage_error)?;
        Ok(LogIndexInsertOutcome::Inserted { sequence })
    }

    /// Newest first, bounded, within one scope.
    ///
    /// The full keyset cursor, structured filters, and bounded search land with the query service
    /// that owns those semantics; this is the storage read it is built on.
    fn query(
        &self,
        query: &IndexedSessionLogQuery,
    ) -> Result<IndexedSessionLogPage, OperationsLogError> {
        let connection = self.connection()?;
        let limit = query
            .limit
            .unwrap_or(DEFAULT_LOG_PAGE_SIZE)
            .clamp(1, MAX_LOG_PAGE_SIZE);
        let fingerprint = filter_fingerprint(&query.scope, &query.filters);
        let cursor = query
            .cursor
            .as_deref()
            .map(|raw| LogPageCursor::decode(raw, &fingerprint))
            .transpose()?;

        let mut sql = String::from("SELECT * FROM unified_log_query_index WHERE 1 = 1");
        let mut binds: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        for (column, value) in [
            ("session_id", query.scope.session_id.as_deref()),
            ("seat_id", query.scope.seat_id.as_deref()),
            ("run_id", query.scope.run_id.as_deref()),
            ("trace_id", query.scope.trace_id.as_deref()),
            ("span_id", query.scope.span_id.as_deref()),
            ("operation_id", query.scope.operation_id.as_deref()),
            ("agent_id", query.scope.agent_id.as_deref()),
        ] {
            if let Some(value) = value {
                sql.push_str(&format!(" AND {column} = ?"));
                binds.push(Box::new(value.to_string()));
            }
        }
        if !query.filters.levels.is_empty() {
            let placeholders = query
                .filters
                .levels
                .iter()
                .map(|_| "?")
                .collect::<Vec<_>>()
                .join(", ");
            sql.push_str(&format!(" AND level IN ({placeholders})"));
            for level in &query.filters.levels {
                binds.push(Box::new(level.token().to_string()));
            }
        }
        for (comparison, value) in [(">=", &query.filters.from), ("<=", &query.filters.to)] {
            if let Some(value) = value.as_deref() {
                sql.push_str(&format!(" AND occurred_at {comparison} ?"));
                binds.push(Box::new(value.to_string()));
            }
        }
        // The keyset itself: strictly after the last row in page order, on the same three columns
        // the `ORDER BY` uses. Same columns, same direction, same tie-breaks — a boundary built
        // from fewer columns than the ordering would leave rows on both sides of it.
        //
        // `sequence` is the table's `INTEGER PRIMARY KEY`, so it is already unique and the pair
        // above it is a strict total order. `record_id` is carried anyway: it is `UNIQUE`, it makes
        // the ordering total on its own terms rather than on an implementation detail of the
        // primary key, and it is what the cursor names when a reader has to be told which row.
        let boundary = match query.filters.sort {
            LogSortDirection::NewestFirst => "<",
            LogSortDirection::OldestFirst => ">",
        };
        if let Some(cursor) = &cursor {
            sql.push_str(&format!(
                " AND (occurred_at_ms {boundary} ? \
                   OR (occurred_at_ms = ? AND sequence {boundary} ?) \
                   OR (occurred_at_ms = ? AND sequence = ? AND record_id {boundary} ?))"
            ));
            binds.push(Box::new(cursor.occurred_at_ms));
            binds.push(Box::new(cursor.occurred_at_ms));
            binds.push(Box::new(cursor.sequence));
            binds.push(Box::new(cursor.occurred_at_ms));
            binds.push(Box::new(cursor.sequence));
            binds.push(Box::new(cursor.record_id.clone()));
        }
        let order = match query.filters.sort {
            LogSortDirection::NewestFirst => "DESC",
            LogSortDirection::OldestFirst => "ASC",
        };
        sql.push_str(&format!(
            " ORDER BY occurred_at_ms {order}, sequence {order}, record_id {order} LIMIT ?"
        ));
        // One more than the page, so "is there another page" is answered by what came back rather
        // than by a second count query that could disagree with it. A text search reads a bounded
        // candidate window instead, because the match is applied after the rows are read.
        let scan_limit = if query.filters.search.is_some() {
            MAX_LOG_SEARCH_CANDIDATES
        } else {
            limit + 1
        };
        binds.push(Box::new(scan_limit as i64));

        let mut statement = connection.prepare(&sql).map_err(storage_error)?;
        let bindings: Vec<&dyn rusqlite::ToSql> =
            binds.iter().map(|value| value.as_ref()).collect();
        let candidates = statement
            .query_map(bindings.as_slice(), read_record)
            .map_err(storage_error)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(storage_error)?;

        let examined = candidates.len();
        let mut matched: Vec<IndexedSessionLogRecord> = match query.filters.search.as_deref() {
            Some(search) => candidates
                .into_iter()
                .filter(|record| matches_search(record, search))
                .collect(),
            None => candidates,
        };
        let has_more = matched.len() > limit;
        matched.truncate(limit);

        let mut coverage = self.coverage(query.scope.session_id.as_deref())?;
        // A search that stopped at its candidate bound has not established that nothing more
        // matches. Reporting it as complete would be the same false claim as a coverage zero.
        if query.filters.search.is_some() && examined >= MAX_LOG_SEARCH_CANDIDATES {
            coverage = coverage.mark_truncated("log_search_candidates_exhausted");
        }
        let next_cursor = matched.last().filter(|_| has_more).map(|record| {
            LogPageCursor {
                occurred_at_ms: record.sequence_time(),
                sequence: record.sequence,
                record_id: record.record_id.clone(),
                filter_fingerprint: fingerprint.clone(),
            }
            .encode()
        });
        Ok(IndexedSessionLogPage {
            truncated: has_more || coverage.truncated,
            next_cursor,
            items: matched,
            coverage,
        })
    }

    fn record(&self, record_id: &str) -> Result<IndexedSessionLogRecord, OperationsLogError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT * FROM unified_log_query_index WHERE record_id = ?1",
                params![record_id],
                read_record,
            )
            .optional()
            .map_err(storage_error)?
            .ok_or(OperationsLogError::RecordNotFound)
    }

    /// What the index can honestly claim about the scope it was asked about.
    ///
    /// The three states below `complete` exist because "I have no rows" has three different causes
    /// and a reader acts differently on each: nothing has happened, nothing has been read yet, or
    /// something was read and lost. Only the first is a definitive zero, so `complete` is the state
    /// that has to be earned — every other path degrades to something weaker.
    fn coverage(&self, session_id: Option<&str>) -> Result<SessionLogCoverage, OperationsLogError> {
        let connection = self.connection()?;
        let (oldest, newest, rows): (Option<String>, Option<String>, i64) = connection
            .query_row(
                "SELECT MIN(occurred_at), MAX(occurred_at), COUNT(*)
                 FROM unified_log_query_index
                 WHERE (?1 IS NULL OR session_id = ?1)",
                params![session_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(storage_error)?;
        // Reason codes come from the gap rows themselves rather than from a single flag, so a
        // reader is told *which* kind of loss applies — retention, a conflict, a dropped receipt
        // and a rejected line are four different things to do something about.
        let mut reasons = Vec::new();
        let mut gap_statement = connection
            .prepare(
                "SELECT reason_code, COALESCE(SUM(dropped_count), 0)
                 FROM unified_log_index_gaps GROUP BY reason_code ORDER BY reason_code",
            )
            .map_err(storage_error)?;
        let mut dropped: i64 = 0;
        for entry in gap_statement
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
            })
            .map_err(storage_error)?
        {
            let (code, count) = entry.map_err(storage_error)?;
            dropped += count;
            reasons.push(code);
        }
        let repairing: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM unified_log_index_repair_state WHERE state = 'running'",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        let checkpoints: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM unified_log_source_checkpoints",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)?;

        // A corpus that has never been read is not a corpus that was read and found empty. Both
        // render as zero unless the coverage says which one happened, which is the single most
        // consequential distinction this type exists to carry.
        let never_read = checkpoints == 0;
        let state = if repairing > 0 || never_read {
            SessionLogCoverageState::Indexing
        } else if !reasons.is_empty() {
            SessionLogCoverageState::Partial
        } else {
            SessionLogCoverageState::Complete
        };
        let mut coverage = SessionLogCoverage::with_state(state);
        coverage.oldest_available_at = oldest;
        // `indexed_through` is what the index holds; `newest_available_at` is the newest it can
        // *claim* to hold. They are the same value only once the sources are caught up — while a
        // repair is running the index is behind whatever the files already contain, and a reader
        // comparing the two is how a stale page explains itself.
        coverage.indexed_through = newest.clone();
        coverage.newest_available_at = newest;
        coverage.dropped_count = u32::try_from(dropped).unwrap_or(u32::MAX);
        if repairing > 0 {
            coverage.reason_codes.push("log_repair_active".to_string());
        }
        if never_read && rows == 0 {
            coverage
                .reason_codes
                .push("log_index_not_backfilled".to_string());
        }
        coverage.reason_codes.extend(reasons);
        Ok(coverage)
    }

    fn watermark(&self) -> Result<i64, OperationsLogError> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT COALESCE(MAX(sequence), 0) FROM unified_log_query_index",
                [],
                |row| row.get(0),
            )
            .map_err(storage_error)
    }

    fn error_count(&self, session_id: &str) -> Result<u32, OperationsLogError> {
        let connection = self.connection()?;
        let count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM unified_log_query_index
                 WHERE session_id = ?1 AND level = 'error'",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(storage_error)?;
        Ok(u32::try_from(count).unwrap_or(u32::MAX))
    }

    fn checkpoint(&self, source: &LogSourceIdentity) -> Result<Option<u64>, OperationsLogError> {
        let connection = self.connection()?;
        let offset: Option<i64> = connection
            .query_row(
                "SELECT next_offset FROM unified_log_source_checkpoints WHERE source_file_id = ?1",
                params![source.as_key()],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error)?;
        Ok(offset.map(|value| value.max(0) as u64))
    }

    fn save_checkpoint(
        &self,
        source: &LogSourceIdentity,
        offset: u64,
    ) -> Result<(), OperationsLogError> {
        let connection = self.connection()?;
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

    fn record_gap(
        &self,
        source: &LogSourceIdentity,
        reason_code: &str,
        dropped: u32,
    ) -> Result<(), OperationsLogError> {
        let connection = self.connection()?;
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

    /// Forgets rows and checkpoints whose source is gone.
    ///
    /// Called with what is still retained rather than with what disappeared: a source that vanished
    /// leaves nothing to name it by, and a caller listing absences would have to remember what used
    /// to exist.
    fn forget_sources(&self, retained: &[LogSourceIdentity]) -> Result<u32, OperationsLogError> {
        let connection = self.connection()?;
        let keys: Vec<String> = retained.iter().map(LogSourceIdentity::as_key).collect();
        let placeholders = if keys.is_empty() {
            "''".to_string()
        } else {
            keys.iter().map(|_| "?").collect::<Vec<_>>().join(", ")
        };
        let bindings: Vec<&dyn rusqlite::ToSql> =
            keys.iter().map(|key| key as &dyn rusqlite::ToSql).collect();
        let removed = connection
            .execute(
                &format!(
                    "DELETE FROM unified_log_query_index WHERE source_file_id NOT IN ({placeholders})"
                ),
                bindings.as_slice(),
            )
            .map_err(storage_error)?;
        connection
            .execute(
                &format!(
                    "DELETE FROM unified_log_source_checkpoints \
                     WHERE source_file_id NOT IN ({placeholders})"
                ),
                bindings.as_slice(),
            )
            .map_err(storage_error)?;
        Ok(u32::try_from(removed).unwrap_or(u32::MAX))
    }
}
