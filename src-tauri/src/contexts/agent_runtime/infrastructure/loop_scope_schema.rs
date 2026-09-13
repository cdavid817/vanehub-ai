use crate::platform::database::{table_has_column, DatabaseError};
use rusqlite::Connection;

/// Additive columns and tables for versioned Loop execution scope. Existing rows keep `NULL`
/// scope columns, which the runtime reads as legacy-unverified; nothing here invents a binding
/// for a run that started before the schema existed.
pub(crate) fn apply_loop_scope_schema(connection: &Connection) -> Result<(), DatabaseError> {
    for (table, column, definition) in [
        ("loop_definitions", "scope_schema_version", "INTEGER"),
        ("loop_definitions", "requested_mode", "TEXT"),
        ("loop_runs", "revision", "INTEGER NOT NULL DEFAULT 1"),
        ("loop_runs", "scope_binding", "TEXT"),
        ("loop_runs", "execution_assessment", "TEXT"),
        ("loop_runs", "sealed_evidence_id", "TEXT"),
        ("loop_runs", "acceptance_operation_id", "TEXT"),
    ] {
        if !table_has_column(connection, table, column)? {
            connection.execute(
                &format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"),
                [],
            )?;
        }
    }
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS loop_audit_receipts (
            id TEXT PRIMARY KEY,
            challenge_id TEXT NOT NULL UNIQUE,
            action TEXT NOT NULL,
            target_id TEXT NOT NULL,
            expected_revision INTEGER NOT NULL,
            scope_digest TEXT NOT NULL,
            assessment_digest TEXT NOT NULL,
            client_context TEXT NOT NULL,
            app_epoch TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            acknowledged_at TEXT,
            consumed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_loop_audit_receipts_target
            ON loop_audit_receipts(target_id, action);

        CREATE TABLE IF NOT EXISTS loop_control_operations (
            idempotency_key TEXT PRIMARY KEY,
            action TEXT NOT NULL,
            target_id TEXT NOT NULL,
            outcome TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_scope_schema_is_additive_and_idempotent() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "CREATE TABLE agents (id TEXT PRIMARY KEY); CREATE TABLE sessions (id TEXT PRIMARY KEY);
                 INSERT INTO agents (id) VALUES ('w'), ('v');",
            )
            .expect("dependencies");
        super::super::apply_loop_schema(&connection).expect("loop schema");
        connection
            .execute(
                r#"INSERT INTO loop_definitions (id, name, enabled, project_path, base_branch, goal,
                    acceptance_criteria, allowed_paths, protected_paths, worker_agent_id,
                    verifier_agent_id, verification_commands, limits, version, created_at, updated_at)
                   VALUES ('loop-old', 'old', 1, '/p', 'main', 'g', '["a"]', '[]', '[]', 'w', 'v',
                    '[{"id":"t","program":"npm","args":[],"working_directory":null,"timeout_seconds":1,"required":true}]',
                    '{"max_iterations":1,"step_timeout_seconds":1,"total_timeout_seconds":1,"max_consecutive_runtime_errors":1,"max_consecutive_no_progress":1}',
                    1, 't', 't')"#,
                [],
            )
            .expect("legacy row");

        apply_loop_scope_schema(&connection).expect("first apply");
        apply_loop_scope_schema(&connection).expect("second apply");

        let (version, mode): (Option<i64>, Option<String>) = connection
            .query_row(
                "SELECT scope_schema_version, requested_mode FROM loop_definitions WHERE id = 'loop-old'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("legacy columns");
        assert_eq!(version, None);
        assert_eq!(mode, None);
        let revision: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(revision), 1) FROM loop_runs",
                [],
                |row| row.get(0),
            )
            .expect("revision default");
        assert_eq!(revision, 1);
        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name IN ('loop_audit_receipts', 'loop_control_operations')",
                [],
                |row| row.get(0),
            )
            .expect("tables");
        assert_eq!(tables, 2);
    }
}
