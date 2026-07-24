use crate::platform::database::NativeDatabase;
use rusqlite::params;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TerminalSearchHit { pub(crate) chunk_id: i64, pub(crate) session_id: String, pub(crate) connection_id: Option<String>, pub(crate) terminal_id: Option<String>, pub(crate) run_id: Option<String>, pub(crate) content: String, pub(crate) snippet: String, pub(crate) captured_at: String }

#[derive(Clone)]
pub(crate) struct SqliteTerminalOutputSearch { database: NativeDatabase }

impl SqliteTerminalOutputSearch {
    pub(crate) fn new(database: NativeDatabase) -> Self { Self { database } }
    pub(crate) fn search(&self, query: &str, session_id: Option<&str>, connection_id: Option<&str>, terminal_id: Option<&str>, run_id: Option<&str>, limit: u32, offset: u32) -> Result<Vec<TerminalSearchHit>, String> {
        if query.trim().is_empty() || query.len() > 512 { return Ok(Vec::new()); }
        let connection = self.database.connection().map_err(|error| error.to_string())?;
        let mut statement = connection.prepare("SELECT c.id,c.session_id,c.connection_id,c.terminal_id,c.run_id,c.content,highlight(terminal_output_fts, 0, '[', ']'),c.captured_at FROM terminal_output_chunks c JOIN terminal_output_fts f ON f.rowid=c.id WHERE terminal_output_fts MATCH ?1 AND (?2 IS NULL OR c.session_id=?2) AND (?3 IS NULL OR c.connection_id=?3) AND (?4 IS NULL OR c.terminal_id=?4) AND (?5 IS NULL OR c.run_id=?5) ORDER BY c.captured_at DESC, c.id DESC LIMIT ?6 OFFSET ?7").map_err(|error| error.to_string())?;
        let rows = statement.query_map(params![query, session_id, connection_id, terminal_id, run_id, limit.clamp(1, 100), offset], |row| Ok(TerminalSearchHit { chunk_id: row.get(0)?, session_id: row.get(1)?, connection_id: row.get(2)?, terminal_id: row.get(3)?, run_id: row.get(4)?, content: row.get(5)?, snippet: row.get(6)?, captured_at: row.get(7)? })).map_err(|error| error.to_string())?;
        rows.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
    }
}
