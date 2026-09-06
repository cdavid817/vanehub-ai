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

#[cfg(test)]
mod tests {
    use super::*;

    // Startup re-asserts this schema after the version-gated migrations, because a shared local
    // database can carry version 89 from another branch while the column itself never landed.
    #[test]
    fn re_asserting_the_column_adds_it_once_and_is_a_no_op_afterwards() {
        let conn = Connection::open_in_memory().expect("in-memory database");
        conn.execute_batch("CREATE TABLE sessions (id TEXT PRIMARY KEY);")
            .expect("bare sessions table");
        assert!(!table_has_column(&conn, "sessions", "personalization_mode").expect("probe"));

        apply_schema(&conn).expect("first application adds the column");
        assert!(table_has_column(&conn, "sessions", "personalization_mode").expect("probe"));
        apply_schema(&conn).expect("second application leaves it alone");

        conn.execute("INSERT INTO sessions (id) VALUES ('s1')", [])
            .expect("insert relies on the default");
        let mode: String = conn
            .query_row(
                "SELECT personalization_mode FROM sessions WHERE id = 's1'",
                [],
                |row| row.get(0),
            )
            .expect("read default");
        assert_eq!(mode, "standard");
    }
}
