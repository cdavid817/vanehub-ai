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
            source_workspace_key TEXT,
            -- Provenance carried over from the pre-governance store. `legacy_folder` is the raw
            -- value that file recorded and `source_workspace_key` is what could be derived from it;
            -- both are projected so "recorded an origin but no key could be derived" is a query
            -- rather than a re-read of every file.
            legacy_save_source TEXT,
            legacy_folder TEXT,
            legacy_source_path TEXT,
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

        -- Compatibility addressing: which v2 record does an old display-name-derived address point
        -- at. Separate from the migration journal because they answer different questions — this
        -- one exists for as long as a pre-governance caller does, and is keyed by something a
        -- caller supplies rather than by something a scan discovered.
        CREATE TABLE IF NOT EXISTS personalization_legacy_memory_alias (
            legacy_address_key TEXT PRIMARY KEY NOT NULL,
            target_memory_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_alias_target
            ON personalization_legacy_memory_alias (target_memory_id);

        -- Migration journal: which stage has each actually-discovered source reached. Keyed by the
        -- source's own location, never by a display name — a file's frontmatter name can disagree
        -- with its filename, two files can share a name, and a malformed file has none.
        CREATE TABLE IF NOT EXISTS personalization_memory_migration_journal (
            source_id TEXT PRIMARY KEY NOT NULL,
            locator_kind TEXT NOT NULL,
            locator_path TEXT,
            locator_table TEXT,
            locator_row_id TEXT,
            target_memory_id TEXT,
            stage TEXT NOT NULL,
            backup_relative_path TEXT,
            source_raw_sha256 TEXT,
            source_byte_length INTEGER,
            last_error_code TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_personalization_journal_memory
            ON personalization_memory_migration_journal (target_memory_id);
        CREATE INDEX IF NOT EXISTS idx_personalization_journal_stage
            ON personalization_memory_migration_journal (stage);
        -- One source per location. A second row for the same file would mean the same bytes
        -- migrated twice.
        CREATE UNIQUE INDEX IF NOT EXISTS idx_personalization_journal_locator
            ON personalization_memory_migration_journal (locator_kind, locator_path, locator_table,
                                                          locator_row_id);

        CREATE TABLE IF NOT EXISTS personalization_migration_state (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            generation INTEGER NOT NULL DEFAULT 0,
            -- How far startup maintenance got. Persisted rather than inferred from the timestamps:
            -- "started but not completed" cannot distinguish converting files from rebuilding the
            -- views they feed, and only the second is safe to resume without re-reading every file.
            phase TEXT NOT NULL DEFAULT 'not_started',
            started_at TEXT,
            completed_at TEXT,
            -- The pre-file row conversion's own marker. Deliberately not the existence of
            -- `MEMORY.md`: that file is rebuilt from v2 records too, so its presence says nothing
            -- about whether the rows were ever converted.
            legacy_rows_migrated_at TEXT,
            last_error_code TEXT,
            repair_required INTEGER NOT NULL DEFAULT 0
        );
        INSERT OR IGNORE INTO personalization_migration_state (id, generation, repair_required)
            VALUES (1, 0, 0);
        "#,
    )
    .map_err(|error| DatabaseError::Storage(error.to_string()))
}
