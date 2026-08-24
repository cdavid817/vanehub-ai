use rusqlite::Connection;

use crate::platform::database::DatabaseError;

/// Additive. Creates the personalization tables and leaves every existing table untouched,
/// including the legacy `AppSettings` personalization fields, which stay readable for the
/// compatibility window so a rollback can still deserialize them.
///
/// Re-running is a no-op: every statement is `IF NOT EXISTS`, and the migration-state singleton is
/// seeded with `INSERT OR IGNORE`. That matters because the memory migration this schema supports
/// is itself resumable, so its bookkeeping table must survive being created twice.
pub(crate) fn apply_schema(conn: &Connection) -> Result<(), DatabaseError> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS personalization_policy_overrides (
            id TEXT PRIMARY KEY NOT NULL,
            policy_set_id TEXT NOT NULL DEFAULT 'default',
            scope_key TEXT NOT NULL UNIQUE,
            scope_kind TEXT NOT NULL,
            workspace_key TEXT,
            agent_id TEXT,
            instruction_merge_mode TEXT NOT NULL,
            about_user TEXT NOT NULL DEFAULT '',
            style_rules TEXT NOT NULL DEFAULT '',
            memory_read_mode TEXT NOT NULL,
            explicit_save_mode TEXT NOT NULL,
            automatic_extraction_mode TEXT NOT NULL,
            global_memory_access_mode TEXT NOT NULL,
            revision INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_policy_scope
            ON personalization_policy_overrides (policy_set_id, scope_kind, workspace_key, agent_id);

        CREATE TABLE IF NOT EXISTS personalization_memory_projection (
            memory_id TEXT PRIMARY KEY NOT NULL,
            file_name TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            memory_type TEXT NOT NULL,
            scope_kind TEXT NOT NULL,
            workspace_key TEXT,
            audience_json TEXT NOT NULL,
            status TEXT NOT NULL,
            source TEXT NOT NULL,
            source_agent_id TEXT,
            source_session_id TEXT,
            sensitivity TEXT NOT NULL DEFAULT 'normal',
            revision INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            verified_at TEXT,
            last_used_at TEXT,
            use_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_memory_status_updated
            ON personalization_memory_projection (status, updated_at);
        CREATE INDEX IF NOT EXISTS idx_personalization_memory_scope
            ON personalization_memory_projection (scope_kind, workspace_key, status);
        CREATE INDEX IF NOT EXISTS idx_personalization_memory_source_agent
            ON personalization_memory_projection (source_agent_id, status);
        CREATE INDEX IF NOT EXISTS idx_personalization_memory_type
            ON personalization_memory_projection (memory_type, status);
        CREATE INDEX IF NOT EXISTS idx_personalization_memory_keyset
            ON personalization_memory_projection (updated_at, memory_id);

        CREATE TABLE IF NOT EXISTS personalization_memory_candidates (
            candidate_id TEXT PRIMARY KEY NOT NULL,
            operation_kind TEXT NOT NULL,
            target_memory_id TEXT,
            expected_target_revision INTEGER,
            proposed_name TEXT,
            proposed_description TEXT,
            proposed_memory_type TEXT,
            proposed_content TEXT,
            proposed_scope_kind TEXT,
            proposed_workspace_key TEXT,
            proposed_audience_json TEXT,
            source TEXT NOT NULL,
            source_agent_id TEXT,
            source_session_id TEXT,
            source_message_id TEXT,
            review_status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            reviewed_at TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_candidate_status_created
            ON personalization_memory_candidates (review_status, created_at);
        CREATE INDEX IF NOT EXISTS idx_personalization_candidate_target
            ON personalization_memory_candidates (target_memory_id);

        -- One row per legacy source, keyed by the identity that source had *before* migration.
        -- This is both the migration journal and the alias table: it is what makes migration
        -- resumable, and it is what lets a pre-governance caller address a memory by the name that
        -- used to be its identity now that duplicate display names are legal.
        CREATE TABLE IF NOT EXISTS personalization_memory_migration_journal (
            legacy_source_id TEXT PRIMARY KEY NOT NULL,
            memory_id TEXT,
            stage TEXT NOT NULL,
            legacy_backup_path TEXT,
            legacy_content_hash TEXT,
            last_error_code TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_journal_memory
            ON personalization_memory_migration_journal (memory_id);
        CREATE INDEX IF NOT EXISTS idx_personalization_journal_stage
            ON personalization_memory_migration_journal (stage);

        CREATE TABLE IF NOT EXISTS personalization_migration_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            generation INTEGER NOT NULL DEFAULT 0,
            started_at TEXT,
            completed_at TEXT,
            last_error_code TEXT,
            repair_required INTEGER NOT NULL DEFAULT 0
        );
        INSERT OR IGNORE INTO personalization_migration_state (id, generation, repair_required)
            VALUES (1, 0, 0);
        "#,
    )
    .map_err(|error| DatabaseError::Storage(error.to_string()))
}
