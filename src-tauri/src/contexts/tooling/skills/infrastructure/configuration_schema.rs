use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Additive only: existing databases gain empty tables, and no Skill row, overlay, or binding is
/// read or rewritten. A Skill without stored configuration is indistinguishable from one that
/// never had any, which is what keeps this safe to apply before the write paths exist.
pub(crate) fn apply_skill_configuration_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skill_configuration_records (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL CHECK (scope IN ('user', 'project')),
            workspace_identity TEXT NOT NULL,
            schema_hash TEXT NOT NULL,
            base_revision TEXT NOT NULL,
            stored_revision INTEGER NOT NULL DEFAULT 0,
            validation_state TEXT NOT NULL DEFAULT 'compatible'
                CHECK (validation_state IN ('compatible', 'migration-required', 'invalid')),
            values_json TEXT NOT NULL DEFAULT '{}',
            secret_keys_json TEXT NOT NULL DEFAULT '[]',
            orphaned_at TEXT,
            cleanup_state TEXT NOT NULL DEFAULT 'none'
                CHECK (cleanup_state IN ('none', 'pending', 'failed')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            -- User scope stores the empty string rather than NULL: SQLite treats NULLs as
            -- distinct in a UNIQUE index, so a nullable column would let one Skill accumulate
            -- duplicate User records that the compare-and-swap save could not arbitrate.
            UNIQUE (skill_id, scope, workspace_identity)
        );

        CREATE INDEX IF NOT EXISTS idx_skill_configuration_skill
            ON skill_configuration_records(skill_id);

        CREATE INDEX IF NOT EXISTS idx_skill_configuration_workspace
            ON skill_configuration_records(workspace_identity, skill_id);

        CREATE INDEX IF NOT EXISTS idx_skill_configuration_cleanup
            ON skill_configuration_records(cleanup_state)
            WHERE cleanup_state <> 'none';
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "configuration_schema_tests.rs"]
mod tests;
