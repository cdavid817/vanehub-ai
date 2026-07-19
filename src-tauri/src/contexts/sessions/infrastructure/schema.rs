use rusqlite::Connection;

pub(crate) fn apply_configuration_schema(
    connection: &Connection,
) -> Result<(), crate::platform::database::DatabaseError> {
    if !crate::platform::database::table_has_column(connection, "sessions", "chat_preferences")? {
        connection.execute("ALTER TABLE sessions ADD COLUMN chat_preferences TEXT", [])?;
    }
    Ok(())
}
