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
    }
}
