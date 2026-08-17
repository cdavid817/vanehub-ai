use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS execution_runs (
            run_id TEXT PRIMARY KEY,
            trace_id TEXT NOT NULL,
            root_span_id TEXT NOT NULL,
            source TEXT NOT NULL,
            source_id TEXT,
            status TEXT NOT NULL,
            capture_policy TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            error_classification TEXT,
            session_id TEXT,
            user_message_id TEXT,
            assistant_message_id TEXT,
            operation_id TEXT,
            agent_id TEXT,
            provider_session_id TEXT,
            attributes_json TEXT NOT NULL DEFAULT '{}'
        );

        CREATE TABLE IF NOT EXISTS execution_spans (
            run_id TEXT NOT NULL,
            span_id TEXT NOT NULL,
            trace_id TEXT NOT NULL,
            parent_span_id TEXT,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            fidelity TEXT NOT NULL,
            started_at TEXT NOT NULL,
            ended_at TEXT,
            error_classification TEXT,
            attributes_json TEXT NOT NULL DEFAULT '{}',
            PRIMARY KEY (run_id, span_id),
            FOREIGN KEY (run_id) REFERENCES execution_runs(run_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS execution_events (
            run_id TEXT NOT NULL,
            span_id TEXT NOT NULL,
            sequence INTEGER NOT NULL,
            name TEXT NOT NULL,
            timestamp TEXT NOT NULL,
            attributes_json TEXT NOT NULL DEFAULT '{}',
            PRIMARY KEY (run_id, span_id, sequence),
            FOREIGN KEY (run_id, span_id)
                REFERENCES execution_spans(run_id, span_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS execution_links (
            run_id TEXT NOT NULL,
            span_id TEXT,
            linked_run_id TEXT NOT NULL,
            linked_trace_id TEXT NOT NULL,
            linked_span_id TEXT,
            relationship TEXT NOT NULL,
            PRIMARY KEY (
                run_id,
                span_id,
                linked_run_id,
                linked_trace_id,
                linked_span_id,
                relationship
            ),
            FOREIGN KEY (run_id) REFERENCES execution_runs(run_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS execution_observability_settings (
            singleton_id INTEGER PRIMARY KEY CHECK (singleton_id = 1),
            local_timeline_enabled INTEGER NOT NULL DEFAULT 1,
            otlp_enabled INTEGER NOT NULL DEFAULT 0,
            otlp_endpoint TEXT,
            otlp_protocol TEXT NOT NULL DEFAULT 'http_protobuf',
            sampling_ratio REAL NOT NULL DEFAULT 1.0,
            retention_days INTEGER NOT NULL DEFAULT 30,
            capture_policy TEXT NOT NULL DEFAULT 'metadata_only',
            mcp_relay_enabled INTEGER NOT NULL DEFAULT 0,
            otlp_auth_ref TEXT,
            last_retention_at TEXT,
            updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
        );

        INSERT OR IGNORE INTO execution_observability_settings (singleton_id) VALUES (1);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_execution_runs_trace
            ON execution_runs(trace_id);
        CREATE INDEX IF NOT EXISTS idx_execution_runs_session_time
            ON execution_runs(session_id, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_execution_runs_status_time
            ON execution_runs(status, started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_execution_runs_started
            ON execution_runs(started_at DESC);
        CREATE INDEX IF NOT EXISTS idx_execution_spans_run_time
            ON execution_spans(run_id, started_at, span_id);
        CREATE INDEX IF NOT EXISTS idx_execution_events_run_time
            ON execution_events(run_id, timestamp, span_id, sequence);

        CREATE TABLE IF NOT EXISTS evaluation_arenas (
            arena_id TEXT PRIMARY KEY,
            operation_id TEXT NOT NULL,
            task_id TEXT NOT NULL,
            task_version INTEGER NOT NULL,
            ranking_version TEXT NOT NULL,
            safe_snapshot_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS evaluation_attempts (
            attempt_id TEXT PRIMARY KEY,
            arena_id TEXT NOT NULL,
            canonical_run_id TEXT NOT NULL,
            outcome TEXT NOT NULL,
            safe_snapshot_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (arena_id) REFERENCES evaluation_arenas(arena_id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_evaluation_arenas_task_time ON evaluation_arenas(task_id, task_version, created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_evaluation_arenas_created ON evaluation_arenas(created_at DESC, arena_id DESC);
        CREATE INDEX IF NOT EXISTS idx_evaluation_attempts_arena ON evaluation_attempts(arena_id, attempt_id);
        CREATE TABLE IF NOT EXISTS evaluation_catalog (
            task_id TEXT NOT NULL,
            task_version INTEGER NOT NULL,
            category TEXT NOT NULL,
            safe_manifest_json TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (task_id, task_version)
        );
        CREATE TABLE IF NOT EXISTS evaluation_metrics (
            attempt_id TEXT NOT NULL,
            metric_name TEXT NOT NULL,
            value REAL,
            unit TEXT NOT NULL,
            quality TEXT NOT NULL,
            source TEXT NOT NULL,
            PRIMARY KEY (attempt_id, metric_name),
            FOREIGN KEY (attempt_id) REFERENCES evaluation_attempts(attempt_id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS evaluation_verifications (
            attempt_id TEXT NOT NULL,
            check_id TEXT NOT NULL,
            passed INTEGER NOT NULL,
            summary TEXT NOT NULL,
            PRIMARY KEY (attempt_id, check_id),
            FOREIGN KEY (attempt_id) REFERENCES evaluation_attempts(attempt_id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS evaluation_artifact_refs (
            attempt_id TEXT NOT NULL,
            artifact_id TEXT NOT NULL,
            PRIMARY KEY (attempt_id, artifact_id),
            FOREIGN KEY (attempt_id) REFERENCES evaluation_attempts(attempt_id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_evaluation_catalog_category ON evaluation_catalog(category, task_id, task_version DESC);
        "#,
    )?;
    ensure_optional_column(
        connection,
        "execution_observability_settings",
        "otlp_auth_ref",
        "TEXT",
    )?;
    Ok(())
}

fn ensure_optional_column(
    connection: &Connection,
    table: &str,
    column: &str,
    declaration: &str,
) -> Result<(), DatabaseError> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    connection.execute_batch(&format!(
        "ALTER TABLE {table} ADD COLUMN {column} {declaration}"
    ))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_is_additive_idempotent_and_uses_safe_defaults() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .unwrap();
        apply_schema(&connection).unwrap();
        apply_schema(&connection).unwrap();

        let defaults = connection
            .query_row(
                "SELECT local_timeline_enabled, otlp_enabled, retention_days, capture_policy, mcp_relay_enabled FROM execution_observability_settings WHERE singleton_id = 1",
                [],
                |row| {
                    Ok((
                        row.get::<_, bool>(0)?,
                        row.get::<_, bool>(1)?,
                        row.get::<_, u16>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, bool>(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            defaults,
            (true, false, 30, "metadata_only".to_string(), false)
        );
        let tables: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('evaluation_arenas','evaluation_attempts')", [], |row| row.get(0)).unwrap();
        assert_eq!(tables, 2);
    }
}
