use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// The evidence journal, its projection, and the coverage metadata that makes an empty answer
/// interpretable.
///
/// Additive and idempotent: every statement is `IF NOT EXISTS`, nothing existing is altered or
/// dropped, and no historical row is synthesised. An older database keeps every message, trace,
/// usage record, log, review, and workspace row it had.
pub(crate) fn apply_evidence_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS execution_evidence_events (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            event_id TEXT NOT NULL UNIQUE,
            source_context TEXT NOT NULL,
            source_event_id TEXT NOT NULL,
            schema_version INTEGER NOT NULL,
            content_fingerprint TEXT NOT NULL,
            session_id TEXT NOT NULL,
            run_id TEXT,
            trace_id TEXT,
            span_id TEXT,
            parent_span_id TEXT,
            operation_id TEXT,
            agent_id TEXT,
            seat_id TEXT,
            tool_call_id TEXT,
            command_id TEXT,
            file_mutation_id TEXT,
            kind TEXT NOT NULL,
            status TEXT,
            fidelity TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            safe_payload_json TEXT NOT NULL,
            redaction_applied INTEGER NOT NULL,
            redaction_rule_ids_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(source_context, source_event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_execution_evidence_session_sequence
            ON execution_evidence_events(session_id, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_execution_evidence_run_sequence
            ON execution_evidence_events(run_id, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_execution_evidence_trace_span
            ON execution_evidence_events(trace_id, span_id, sequence);
        CREATE INDEX IF NOT EXISTS idx_execution_evidence_operation
            ON execution_evidence_events(operation_id, sequence);
        CREATE INDEX IF NOT EXISTS idx_execution_evidence_retention
            ON execution_evidence_events(occurred_at, sequence);

        CREATE TABLE IF NOT EXISTS execution_evidence_records (
            record_id TEXT PRIMARY KEY,
            record_kind TEXT NOT NULL,
            session_id TEXT NOT NULL,
            run_id TEXT,
            trace_id TEXT,
            span_id TEXT,
            operation_id TEXT,
            agent_id TEXT,
            seat_id TEXT,
            started_at TEXT,
            ended_at TEXT,
            duration_ms INTEGER,
            status TEXT NOT NULL,
            fidelity TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            command_runtime_kind TEXT,
            redacted_display TEXT,
            cwd_display TEXT,
            exit_code INTEGER,
            signal TEXT,
            output_availability TEXT,
            output_truncated INTEGER,
            tool_name TEXT,
            tool_call_id TEXT,
            parent_agent_id TEXT,
            child_agent_id TEXT,
            attempt INTEGER,
            verification_name TEXT,
            verification_outcome TEXT,
            verification_passed INTEGER,
            verification_failed INTEGER,
            last_sequence INTEGER NOT NULL
        );

        -- The newest-first page query. Ordering by the same pair the cursor encodes is what makes
        -- a continuation stable while newer rows arrive at the head of the list.
        CREATE INDEX IF NOT EXISTS idx_evidence_records_session_page
            ON execution_evidence_records(session_id, occurred_at DESC, record_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evidence_records_seat_page
            ON execution_evidence_records(session_id, seat_id, occurred_at DESC, record_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evidence_records_run_page
            ON execution_evidence_records(run_id, occurred_at DESC, record_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evidence_records_retention
            ON execution_evidence_records(occurred_at, record_id);

        -- One row per session. Holds what a query cannot learn from the rows it can see: how many
        -- events were dropped before arriving, whether a conflicting source id left something
        -- unrecorded, and how far retention has trimmed the journal.
        CREATE TABLE IF NOT EXISTS execution_evidence_coverage (
            session_id TEXT PRIMARY KEY,
            dropped_count INTEGER NOT NULL DEFAULT 0,
            conflict_count INTEGER NOT NULL DEFAULT 0,
            retention_trimmed INTEGER NOT NULL DEFAULT 0,
            oldest_available_at TEXT,
            newest_available_at TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

/// Creates the evidence schema when the version gate skipped its migration.
///
/// Parallel development worktrees share one application data directory, and other open branches
/// already claim versions 81 through 84. If one of those ran first, `apply_migration` records the
/// version and this branch's statements never execute, leaving a database whose `schema_migrations`
/// looks complete while `execution_evidence_events` does not exist. The repository already handles
/// that case for version 54's collision; this is the same repair for the same reason, and it
/// rewrites no history — it only re-asserts the invariant the skipped migration was supposed to
/// establish.
pub(crate) fn repair_missing_evidence_schema(connection: &Connection) -> Result<(), DatabaseError> {
    let present: i64 = connection.query_row(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'execution_evidence_events'",
        [],
        |row| row.get(0),
    )?;
    if present > 0 {
        return Ok(());
    }
    apply_evidence_schema(connection)
}
