use rusqlite::Connection;

pub(crate) fn apply_configuration_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !crate::platform::database::table_has_column(connection, "sessions", "chat_preferences")? {
        connection.execute("ALTER TABLE sessions ADD COLUMN chat_preferences TEXT", [])?;
    }
    Ok(())
}

pub(crate) fn apply_loop_ownership_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    for column in ["loop_run_id", "loop_iteration_id", "loop_role"] {
        if !crate::platform::database::table_has_column(connection, "sessions", column)? {
            connection.execute(
                &format!("ALTER TABLE sessions ADD COLUMN {column} TEXT"),
                [],
            )?;
        }
    }
    connection.execute(
        "CREATE INDEX IF NOT EXISTS idx_sessions_loop_ownership ON sessions(loop_run_id, loop_iteration_id, loop_role)",
        [],
    )?;
    Ok(())
}

/// Seats are stored as a JSON list on the session rather than in a joined table: `SESSION_SELECT`
/// is the hot path for list, search, and get, and a join there would cost every read for a feature
/// most sessions do not use. Existing rows default to `[]`, which readers present as the one-seat
/// case built from `agent_id`.
pub(crate) fn apply_session_seat_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    let mut statement = connection.prepare("PRAGMA table_info(sessions)")?;
    let existing: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .collect();
    if !existing.iter().any(|column| column == "seats") {
        connection.execute(
            "ALTER TABLE sessions ADD COLUMN seats TEXT NOT NULL DEFAULT '[]'",
            [],
        )?;
    }
    Ok(())
}
