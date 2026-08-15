use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Revision-bound Skill tool state.
///
/// Deliberately not foreign-keyed to `skills`: tools are discovered from the *effective* Skill
/// package, which for System and Registry layers has no registry row. A foreign key would make
/// those Skills' tools unrecordable, and cascade-delete would erase quarantine and trust evidence
/// the moment a Skill row was rewritten.
///
/// The primary key is the immutable revision witness, so replacing content creates a new row
/// instead of mutating the record an earlier operator decision was made against.
pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS skill_tool_revisions (
            revision_witness TEXT PRIMARY KEY,
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL CHECK (scope IN ('global', 'workspace')),
            workspace_path TEXT NOT NULL DEFAULT '',
            tool_id TEXT NOT NULL,
            implementation_kind TEXT NOT NULL CHECK (implementation_kind IN ('declarative', 'wasm')),
            base_revision TEXT NOT NULL,
            manifest_hash TEXT NOT NULL,
            implementation_hash TEXT NOT NULL,
            capability_digest TEXT NOT NULL,
            validation_state TEXT NOT NULL DEFAULT 'pending'
                CHECK (validation_state IN ('pending', 'valid', 'invalid')),
            validation_code TEXT,
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            quarantined INTEGER NOT NULL DEFAULT 0 CHECK (quarantined IN (0, 1)),
            quarantine_reason TEXT,
            consecutive_failures INTEGER NOT NULL DEFAULT 0 CHECK (consecutive_failures >= 0),
            diagnostic_summary TEXT NOT NULL DEFAULT '',
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_skill_tool_revisions_owner
            ON skill_tool_revisions(skill_id, scope, workspace_path, tool_id);

        CREATE TABLE IF NOT EXISTS skill_tool_trust (
            revision_witness TEXT PRIMARY KEY,
            skill_id TEXT NOT NULL,
            scope TEXT NOT NULL CHECK (scope IN ('global', 'workspace')),
            workspace_path TEXT NOT NULL DEFAULT '',
            tool_id TEXT NOT NULL,
            base_revision TEXT NOT NULL,
            manifest_hash TEXT NOT NULL,
            implementation_hash TEXT NOT NULL,
            capability_digest TEXT NOT NULL,
            decision TEXT NOT NULL CHECK (decision IN ('trusted', 'revoked')),
            actor TEXT NOT NULL,
            decided_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_skill_tool_trust_owner
            ON skill_tool_trust(skill_id, scope, workspace_path, tool_id);
        "#,
    )?;
    Ok(())
}
