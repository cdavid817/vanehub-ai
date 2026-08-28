use rusqlite::Connection;

use crate::platform::database::DatabaseError;

/// Records which personalization mode a session was created in.
///
/// `NOT NULL DEFAULT 'standard'` rather than a nullable column: every session written before this
/// existed was a standard one, and backfilling at read time would leave the answer depending on
/// which reader looked. The `CHECK` is the same closed set the domain parses, so a value this
/// build cannot interpret cannot be stored by a build that can.
pub(crate) fn apply_schema(conn: &Connection) -> Result<(), DatabaseError> {
    if !table_has_column(conn, "sessions", "personalization_mode")? {
        conn.execute(
            "ALTER TABLE sessions ADD COLUMN personalization_mode TEXT NOT NULL DEFAULT 'standard' \
             CHECK (personalization_mode IN ('standard', 'project-only', 'temporary'))",
            [],
        )?;
    }
    Ok(())
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, DatabaseError> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}
