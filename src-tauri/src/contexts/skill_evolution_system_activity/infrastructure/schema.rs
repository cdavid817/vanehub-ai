use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("schema.sql"))?;
    connection.execute_batch(include_str!("query_schema.sql"))?;
    Ok(())
}

pub(crate) fn apply_query_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("query_schema.sql"))?;
    Ok(())
}

pub(crate) fn apply_source_outbox_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(include_str!("source_outbox.sql"))?;
    Ok(())
}
