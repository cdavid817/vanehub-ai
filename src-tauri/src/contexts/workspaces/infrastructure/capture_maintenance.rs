use crate::platform::database::NativeDatabase;
use rusqlite::params;

#[derive(Clone)]
pub(crate) struct TerminalCaptureMaintenance { database: NativeDatabase }

impl TerminalCaptureMaintenance {
    pub(crate) fn new(database: NativeDatabase) -> Self { Self { database } }

    pub(crate) fn purge_session(&self, session_id: &str) -> Result<usize, String> {
        let mut connection = self.database.connection().map_err(|error| error.to_string())?;
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        let count = transaction.execute("DELETE FROM terminal_output_chunks WHERE session_id=?1", params![session_id]).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(count)
    }

    pub(crate) fn purge_before(&self, captured_before: &str) -> Result<usize, String> {
        let mut connection = self.database.connection().map_err(|error| error.to_string())?;
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        let count = transaction.execute("DELETE FROM terminal_output_chunks WHERE captured_at < ?1", params![captured_before]).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(count)
    }

    pub(crate) fn enforce_capacity(&self, capacity_bytes: i64) -> Result<usize, String> {
        let mut connection = self.database.connection().map_err(|error| error.to_string())?;
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        let mut removed = 0usize;
        loop {
            let total: i64 = transaction.query_row("SELECT COALESCE(SUM(content_bytes),0) FROM terminal_output_chunks", [], |row| row.get(0)).map_err(|error| error.to_string())?;
            if total <= capacity_bytes { break; }
            let count = transaction.execute("DELETE FROM terminal_output_chunks WHERE id = (SELECT id FROM terminal_output_chunks ORDER BY captured_at ASC, id ASC LIMIT 1)", []).map_err(|error| error.to_string())?;
            if count == 0 { break; }
            removed += count;
        }
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(removed)
    }
}
