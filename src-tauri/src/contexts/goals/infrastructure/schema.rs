use rusqlite::Connection;

use crate::platform::database::DatabaseError;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS goals (
            id TEXT PRIMARY KEY,
            title TEXT NOT NULL,
            description TEXT NOT NULL DEFAULT '',
            acceptance_notes TEXT NOT NULL DEFAULT '',
            status TEXT NOT NULL CHECK (status IN ('draft', 'active', 'achieved', 'abandoned')),
            project_path TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS goal_links (
            goal_id TEXT NOT NULL,
            target_kind TEXT NOT NULL CHECK (target_kind IN ('plan', 'loop', 'work_item', 'session')),
            target_id TEXT NOT NULL,
            linked_at TEXT NOT NULL,
            PRIMARY KEY (goal_id, target_kind, target_id),
            FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS goal_links_by_target
            ON goal_links (target_kind, target_id);
        "#,
    )?;
    Ok(())
}
