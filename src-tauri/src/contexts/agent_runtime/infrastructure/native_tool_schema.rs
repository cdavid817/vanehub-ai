use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_native_tool_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS native_tool_operations (
            id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            session_id TEXT NOT NULL,
            generation_id TEXT NOT NULL,
            tool_name TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN (
                'queued', 'awaiting_approval', 'running', 'awaiting_human',
                'succeeded', 'failed', 'cancelled'
            )),
            progress_sequence INTEGER NOT NULL DEFAULT 0 CHECK (progress_sequence >= 0),
            progress_message TEXT,
            result_artifact_ids_json TEXT NOT NULL DEFAULT '[]',
            error_code TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_native_tool_operations_session_updated
            ON native_tool_operations(session_id, updated_at DESC, id DESC);
        CREATE INDEX IF NOT EXISTS idx_native_tool_operations_generation
            ON native_tool_operations(generation_id, id);

        CREATE TABLE IF NOT EXISTS native_tool_artifacts (
            id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            content_hash TEXT NOT NULL,
            media_type TEXT NOT NULL,
            size_bytes INTEGER NOT NULL CHECK (size_bytes >= 0),
            display_name TEXT NOT NULL,
            source_operation_id TEXT,
            created_at TEXT NOT NULL,
            expires_at TEXT,
            publication_ref TEXT,
            UNIQUE (content_hash, media_type, size_bytes),
            FOREIGN KEY (source_operation_id) REFERENCES native_tool_operations(id)
        );
        CREATE INDEX IF NOT EXISTS idx_native_tool_artifacts_expiry
            ON native_tool_artifacts(expires_at, id) WHERE expires_at IS NOT NULL;

        CREATE TABLE IF NOT EXISTS native_tool_artifact_lineage (
            artifact_id TEXT NOT NULL,
            source_artifact_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            PRIMARY KEY (artifact_id, source_artifact_id),
            UNIQUE (artifact_id, ordinal),
            CHECK (artifact_id <> source_artifact_id),
            FOREIGN KEY (artifact_id) REFERENCES native_tool_artifacts(id) ON DELETE CASCADE,
            FOREIGN KEY (source_artifact_id) REFERENCES native_tool_artifacts(id)
        );

        CREATE TABLE IF NOT EXISTS native_tool_delegations (
            id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            session_id TEXT NOT NULL,
            task_hash TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN (
                'queued', 'running', 'succeeded', 'failed', 'cancelled'
            )),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (session_id) REFERENCES sessions(id) ON DELETE CASCADE
        );
        CREATE INDEX IF NOT EXISTS idx_native_tool_delegations_session_updated
            ON native_tool_delegations(session_id, updated_at DESC, id DESC);

        CREATE TABLE IF NOT EXISTS native_tool_delegation_attempts (
            id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            delegation_id TEXT NOT NULL,
            attempt_number INTEGER NOT NULL CHECK (attempt_number BETWEEN 1 AND 3),
            target TEXT NOT NULL CHECK (target IN ('claude_code', 'codex_cli')),
            mode TEXT NOT NULL CHECK (mode IN ('analyze', 'edit')),
            status TEXT NOT NULL CHECK (status IN (
                'queued', 'running', 'succeeded', 'failed', 'cancelled'
            )),
            safe_summary TEXT,
            report_artifact_id TEXT,
            change_set_artifact_id TEXT,
            error_code TEXT,
            started_at TEXT,
            completed_at TEXT,
            UNIQUE (delegation_id, attempt_number),
            FOREIGN KEY (delegation_id) REFERENCES native_tool_delegations(id) ON DELETE CASCADE,
            FOREIGN KEY (report_artifact_id) REFERENCES native_tool_artifacts(id),
            FOREIGN KEY (change_set_artifact_id) REFERENCES native_tool_artifacts(id)
        );

        CREATE TABLE IF NOT EXISTS native_tool_change_sets (
            artifact_id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            content_hash TEXT NOT NULL,
            repository_identity TEXT NOT NULL,
            base_commit TEXT NOT NULL,
            attempt_id TEXT NOT NULL UNIQUE,
            warnings_json TEXT NOT NULL DEFAULT '[]',
            created_at TEXT NOT NULL,
            FOREIGN KEY (artifact_id) REFERENCES native_tool_artifacts(id),
            FOREIGN KEY (attempt_id) REFERENCES native_tool_delegation_attempts(id)
        );

        CREATE TABLE IF NOT EXISTS native_tool_change_set_files (
            change_set_artifact_id TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            path TEXT NOT NULL,
            change_kind TEXT NOT NULL CHECK (change_kind IN ('add', 'modify', 'delete', 'rename')),
            old_hash TEXT,
            new_hash TEXT,
            binary INTEGER NOT NULL CHECK (binary IN (0, 1)),
            mode TEXT,
            PRIMARY KEY (change_set_artifact_id, path),
            UNIQUE (change_set_artifact_id, ordinal),
            FOREIGN KEY (change_set_artifact_id) REFERENCES native_tool_change_sets(artifact_id)
                ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS native_tool_apply_attempts (
            id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            change_set_artifact_id TEXT NOT NULL,
            target_repository_identity TEXT NOT NULL,
            expected_base_commit TEXT NOT NULL,
            approval_input_hash TEXT NOT NULL,
            status TEXT NOT NULL CHECK (status IN (
                'awaiting_approval', 'preflighting', 'applying', 'verifying',
                'succeeded', 'rolled_back', 'manual_recovery_required', 'failed'
            )),
            error_code TEXT,
            consumed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (change_set_artifact_id, target_repository_identity),
            FOREIGN KEY (change_set_artifact_id) REFERENCES native_tool_change_sets(artifact_id)
        );

        CREATE TABLE IF NOT EXISTS native_tool_apply_recovery (
            apply_attempt_id TEXT PRIMARY KEY,
            contract_version INTEGER NOT NULL CHECK (contract_version = 1),
            status TEXT NOT NULL CHECK (status IN (
                'not_required', 'rolled_back', 'manual_recovery_required'
            )),
            recovery_reference TEXT,
            safe_instructions_json TEXT NOT NULL DEFAULT '[]',
            updated_at TEXT NOT NULL,
            FOREIGN KEY (apply_attempt_id) REFERENCES native_tool_apply_attempts(id)
                ON DELETE CASCADE
        );
        "#,
    )?;
    Ok(())
}
