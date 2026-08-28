//! The tables behind the redacted log query index.
//!
//! All of it is rebuildable. Nothing here is a second source of truth: every row can be derived
//! again from the retained redacted JSONL files, which is why the schema is free to be shaped for
//! reading rather than for durability.

use rusqlite::Connection;

/// Creates the query index, its correlation indexes, checkpoints, gaps, and repair state.
///
/// Additive and idempotent: every statement is `IF NOT EXISTS`, so running it against a database
/// that already has the tables is a no-op rather than a failure. The migration runner is
/// transactional, so a partial application is not a state this can be left in.
pub(crate) fn apply_log_query_index_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS unified_log_query_index (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            record_id TEXT NOT NULL UNIQUE,
            source_file_id TEXT NOT NULL,
            source_offset INTEGER NOT NULL,
            occurred_at TEXT NOT NULL,
            occurred_at_ms INTEGER NOT NULL,
            level TEXT NOT NULL,
            category TEXT NOT NULL,
            message TEXT NOT NULL,
            safe_context_json TEXT NOT NULL,
            session_id TEXT,
            run_id TEXT,
            trace_id TEXT,
            span_id TEXT,
            operation_id TEXT,
            agent_id TEXT,
            seat_id TEXT,
            redaction_applied INTEGER NOT NULL DEFAULT 1
        );

        -- Every interactive query is newest-first within a scope, so every correlation index is
        -- (scope, time DESC, sequence DESC). A scope index without the ordering columns would make
        -- SQLite sort the whole matching set to answer one page.
        CREATE INDEX IF NOT EXISTS idx_log_session_time
            ON unified_log_query_index(session_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_run_time
            ON unified_log_query_index(run_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_trace_time
            ON unified_log_query_index(trace_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_span_time
            ON unified_log_query_index(span_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_operation_time
            ON unified_log_query_index(operation_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_agent_time
            ON unified_log_query_index(agent_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_seat_time
            ON unified_log_query_index(seat_id, occurred_at_ms DESC, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_log_level_time
            ON unified_log_query_index(level, occurred_at_ms DESC, sequence DESC);
        -- The witness half of idempotency, and what a repair resumes against.
        CREATE INDEX IF NOT EXISTS idx_log_source_offset
            ON unified_log_query_index(source_file_id, source_offset);

        -- How far each source generation has been indexed. Keyed by generation rather than by
        -- path: a recreated file reuses the path and would otherwise resume mid-file into bytes
        -- the offset was never written for.
        CREATE TABLE IF NOT EXISTS unified_log_source_checkpoints (
            source_file_id TEXT PRIMARY KEY,
            directory_generation TEXT NOT NULL,
            next_offset INTEGER NOT NULL,
            updated_at TEXT NOT NULL
        );

        -- What is known to be missing. A count that is short without one of these reads as a
        -- smaller corpus rather than as an incomplete index.
        CREATE TABLE IF NOT EXISTS unified_log_index_gaps (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            source_file_id TEXT NOT NULL,
            reason_code TEXT NOT NULL,
            dropped_count INTEGER NOT NULL,
            observed_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_log_gap_source
            ON unified_log_index_gaps(source_file_id, observed_at DESC);

        -- One row per repair operation, so progress survives a restart and a resumed repair can
        -- say what it is resuming.
        CREATE TABLE IF NOT EXISTS unified_log_index_repair_state (
            operation_id TEXT PRIMARY KEY,
            state TEXT NOT NULL,
            files_completed INTEGER NOT NULL DEFAULT 0,
            files_total INTEGER NOT NULL DEFAULT 0,
            records_indexed INTEGER NOT NULL DEFAULT 0,
            reason_code TEXT,
            started_at TEXT,
            updated_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}

/// Creates the index schema when the version gate skipped its migration.
///
/// Every statement above is `IF NOT EXISTS`, which makes re-running the function harmless — but
/// idempotency is not the property that was missing. Parallel worktrees share one application
/// database and `main` already records 82 under a different name, so a developer whose database
/// was migrated by another branch has version 82 in `schema_migrations` and none of these tables.
/// `apply_migration` is version-gated, so it never calls the function at all; being safe to call
/// twice does not help something that is called zero times.
///
/// This rewrites no history. It re-asserts the invariant the skipped migration was supposed to
/// establish, which is the repair versions 54, 81, 83, and 84 already carry for the same reason.
/// The presence check reads the index table rather than one of the three bookkeeping tables: the
/// index is what a query fails on, and it is the one a half-created schema would still be missing.
pub(crate) fn repair_missing_log_query_index_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    let present: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master \
         WHERE type = 'table' AND name = 'unified_log_query_index'",
        [],
        |row| row.get(0),
    )?;
    if present > 0 {
        return Ok(());
    }
    apply_log_query_index_schema(connection)
}
