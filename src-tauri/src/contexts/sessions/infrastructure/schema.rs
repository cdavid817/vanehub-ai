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
