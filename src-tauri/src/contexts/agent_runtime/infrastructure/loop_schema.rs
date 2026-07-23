use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_loop_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS loop_definitions (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            enabled INTEGER NOT NULL DEFAULT 1,
            project_path TEXT NOT NULL,
            base_branch TEXT NOT NULL,
            goal TEXT NOT NULL,
            acceptance_criteria TEXT NOT NULL,
            allowed_paths TEXT NOT NULL,
            protected_paths TEXT NOT NULL,
            worker_agent_id TEXT NOT NULL,
            verifier_agent_id TEXT NOT NULL,
            verification_commands TEXT NOT NULL,
            limits TEXT NOT NULL,
            version INTEGER NOT NULL CHECK (version > 0),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (worker_agent_id) REFERENCES agents(id),
            FOREIGN KEY (verifier_agent_id) REFERENCES agents(id)
        );

        CREATE TABLE IF NOT EXISTS loop_runs (
            id TEXT PRIMARY KEY,
            definition_id TEXT NOT NULL,
            definition_snapshot TEXT NOT NULL,
            status TEXT NOT NULL,
            phase TEXT NOT NULL,
            terminal_reason TEXT,
            current_iteration INTEGER NOT NULL DEFAULT 1,
            consecutive_runtime_errors INTEGER NOT NULL DEFAULT 0,
            consecutive_no_progress INTEGER NOT NULL DEFAULT 0,
            pause_requested INTEGER NOT NULL DEFAULT 0,
            project_path TEXT NOT NULL,
            worktree_path TEXT,
            worktree_name TEXT,
            worktree_branch TEXT,
            active_operation_id TEXT,
            simulated INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            started_at TEXT,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (definition_id) REFERENCES loop_definitions(id)
        );

        CREATE TABLE IF NOT EXISTS loop_iterations (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            sequence INTEGER NOT NULL CHECK (sequence > 0),
            status TEXT NOT NULL,
            worker_session_id TEXT,
            verifier_session_id TEXT,
            worker_summary TEXT,
            verifier_recommendation TEXT,
            verifier_findings TEXT NOT NULL DEFAULT '[]',
            decision_reason TEXT,
            diff_fingerprint TEXT,
            check_failure_fingerprint TEXT,
            user_feedback TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            UNIQUE (run_id, sequence),
            FOREIGN KEY (run_id) REFERENCES loop_runs(id) ON DELETE CASCADE,
            FOREIGN KEY (worker_session_id) REFERENCES sessions(id),
            FOREIGN KEY (verifier_session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS loop_evidence (
            id TEXT PRIMARY KEY,
            run_id TEXT NOT NULL,
            iteration_id TEXT,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            summary TEXT NOT NULL,
            operation_id TEXT,
            command_id TEXT,
            exit_code INTEGER,
            duration_ms INTEGER,
            details TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (run_id) REFERENCES loop_runs(id) ON DELETE CASCADE,
            FOREIGN KEY (iteration_id) REFERENCES loop_iterations(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_loop_definitions_updated
            ON loop_definitions(updated_at DESC);
        CREATE INDEX IF NOT EXISTS idx_loop_runs_definition_created
            ON loop_runs(definition_id, created_at DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_loop_runs_one_active_definition
            ON loop_runs(definition_id)
            WHERE status IN ('queued', 'running', 'paused', 'awaiting-acceptance');
        CREATE INDEX IF NOT EXISTS idx_loop_iterations_run_sequence
            ON loop_iterations(run_id, sequence);
        CREATE INDEX IF NOT EXISTS idx_loop_evidence_run_created
            ON loop_evidence(run_id, created_at);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loop_schema_is_idempotent() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                "CREATE TABLE agents (id TEXT PRIMARY KEY); CREATE TABLE sessions (id TEXT PRIMARY KEY);",
            )
            .expect("dependencies");

        apply_loop_schema(&connection).expect("first apply");
        apply_loop_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'loop_%'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 4);
    }
}
