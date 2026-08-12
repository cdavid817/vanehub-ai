use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Replays the retired Plan schema so existing migration history remains valid.
///
/// No runtime service reads or writes these tables after Plan execution removal.
pub(crate) fn apply_legacy_plan_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS plans (
            id TEXT PRIMARY KEY,
            status TEXT NOT NULL CHECK (status IN ('draft', 'approved', 'archived')),
            current_version INTEGER NOT NULL CHECK (current_version > 0),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS plan_versions (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            version INTEGER NOT NULL CHECK (version > 0),
            goal TEXT NOT NULL,
            project_path TEXT NOT NULL,
            base_ref TEXT NOT NULL,
            planner_profile_id TEXT,
            created_at TEXT NOT NULL,
            approved_at TEXT,
            UNIQUE (plan_id, version),
            FOREIGN KEY (plan_id) REFERENCES plans(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_subtasks (
            id TEXT NOT NULL,
            plan_version_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            title TEXT NOT NULL,
            description TEXT NOT NULL,
            acceptance_criteria TEXT NOT NULL,
            assigned_role TEXT NOT NULL,
            token_budget INTEGER,
            tool_call_limit INTEGER,
            timeout_seconds INTEGER,
            validation_commands TEXT NOT NULL DEFAULT '[]',
            PRIMARY KEY (plan_version_id, id),
            UNIQUE (plan_version_id, ordinal),
            FOREIGN KEY (plan_version_id) REFERENCES plan_versions(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_subtask_dependencies (
            plan_version_id TEXT NOT NULL,
            predecessor_id TEXT NOT NULL,
            successor_id TEXT NOT NULL,
            PRIMARY KEY (plan_version_id, predecessor_id, successor_id),
            CHECK (predecessor_id <> successor_id),
            FOREIGN KEY (plan_version_id, predecessor_id)
                REFERENCES plan_subtasks(plan_version_id, id) ON DELETE CASCADE,
            FOREIGN KEY (plan_version_id, successor_id)
                REFERENCES plan_subtasks(plan_version_id, id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_runs (
            id TEXT PRIMARY KEY,
            plan_id TEXT NOT NULL,
            plan_version_id TEXT NOT NULL,
            status TEXT NOT NULL,
            project_path TEXT NOT NULL,
            base_ref TEXT NOT NULL,
            base_oid TEXT,
            worktree_path TEXT,
            worktree_name TEXT,
            worktree_branch TEXT,
            simulated INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            started_at TEXT,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            FOREIGN KEY (plan_id) REFERENCES plans(id),
            FOREIGN KEY (plan_version_id) REFERENCES plan_versions(id)
        );

        CREATE TABLE IF NOT EXISTS plan_subtask_runs (
            id TEXT PRIMARY KEY,
            plan_run_id TEXT NOT NULL,
            subtask_id TEXT NOT NULL,
            status TEXT NOT NULL,
            topological_rank INTEGER NOT NULL CHECK (topological_rank >= 0),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            result_summary TEXT,
            changed_files TEXT NOT NULL DEFAULT '[]',
            verification_summary TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT,
            UNIQUE (plan_run_id, subtask_id),
            FOREIGN KEY (plan_run_id) REFERENCES plan_runs(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_subtask_attempts (
            id TEXT PRIMARY KEY,
            subtask_run_id TEXT NOT NULL,
            sequence INTEGER NOT NULL CHECK (sequence > 0),
            status TEXT NOT NULL,
            session_id TEXT,
            profile_id TEXT,
            execution_run_id TEXT,
            operation_id TEXT,
            token_usage INTEGER NOT NULL DEFAULT 0,
            tool_call_count INTEGER NOT NULL DEFAULT 0,
            error_class TEXT,
            started_at TEXT NOT NULL,
            completed_at TEXT,
            UNIQUE (subtask_run_id, sequence),
            FOREIGN KEY (subtask_run_id) REFERENCES plan_subtask_runs(id) ON DELETE CASCADE,
            FOREIGN KEY (session_id) REFERENCES sessions(id)
        );

        CREATE TABLE IF NOT EXISTS plan_verification_evidence (
            id TEXT PRIMARY KEY,
            attempt_id TEXT NOT NULL,
            command_id TEXT NOT NULL,
            status TEXT NOT NULL,
            exit_code INTEGER,
            duration_ms INTEGER,
            output_summary TEXT,
            created_at TEXT NOT NULL,
            FOREIGN KEY (attempt_id) REFERENCES plan_subtask_attempts(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_control_requests (
            id TEXT PRIMARY KEY,
            plan_run_id TEXT NOT NULL,
            kind TEXT NOT NULL,
            status TEXT NOT NULL,
            requested_at TEXT NOT NULL,
            resolved_at TEXT,
            FOREIGN KEY (plan_run_id) REFERENCES plan_runs(id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS plan_generation_failures (
            id TEXT PRIMARY KEY,
            plan_id TEXT,
            requested_version INTEGER NOT NULL CHECK (requested_version > 0),
            failure_class TEXT NOT NULL,
            safe_action TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_plan_versions_plan_version
            ON plan_versions(plan_id, version DESC);
        CREATE INDEX IF NOT EXISTS idx_plan_dependencies_successor
            ON plan_subtask_dependencies(plan_version_id, successor_id);
        CREATE INDEX IF NOT EXISTS idx_plan_runs_status_created
            ON plan_runs(status, created_at, id);
        CREATE INDEX IF NOT EXISTS idx_plan_subtask_runs_schedule
            ON plan_subtask_runs(plan_run_id, status, topological_rank, ordinal, subtask_id);
        CREATE INDEX IF NOT EXISTS idx_plan_attempts_task_sequence
            ON plan_subtask_attempts(subtask_run_id, sequence DESC);
        CREATE INDEX IF NOT EXISTS idx_plan_evidence_attempt_created
            ON plan_verification_evidence(attempt_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_plan_controls_run_requested
            ON plan_control_requests(plan_run_id, requested_at DESC);
        CREATE INDEX IF NOT EXISTS idx_plan_generation_failures_created
            ON plan_generation_failures(plan_id, created_at DESC);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_schema_remains_additive_idempotent_and_indexed() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch("PRAGMA foreign_keys = ON; CREATE TABLE sessions (id TEXT PRIMARY KEY);")
            .expect("dependencies");
        apply_legacy_plan_schema(&connection).expect("first apply");
        apply_legacy_plan_schema(&connection).expect("second apply");

        let tables: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name LIKE 'plan%'",
                [],
                |row| row.get(0),
            )
            .expect("tables");
        let indexes: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name LIKE 'idx_plan_%'",
                [],
                |row| row.get(0),
            )
            .expect("indexes");
        assert_eq!(tables, 10);
        assert_eq!(indexes, 8);
        assert_eq!(
            connection
                .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
                .expect("foreign keys"),
            1
        );
    }
}
