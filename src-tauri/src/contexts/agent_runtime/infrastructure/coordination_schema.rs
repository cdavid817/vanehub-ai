use crate::platform::database::DatabaseError;
use rusqlite::Connection;

pub(crate) fn apply_coordination_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS coordination_runs (
            id TEXT PRIMARY KEY,
            operation_id TEXT NOT NULL,
            status TEXT NOT NULL,
            cancel_requested INTEGER NOT NULL DEFAULT 0,
            run_snapshot TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            completed_at TEXT
        );

        CREATE INDEX IF NOT EXISTS idx_coordination_runs_created
            ON coordination_runs(created_at DESC);
        CREATE INDEX IF NOT EXISTS idx_coordination_runs_recovery
            ON coordination_runs(status, cancel_requested)
            WHERE status IN ('queued', 'running');
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordination_schema_is_idempotent() {
        let connection = Connection::open_in_memory().expect("database");
        apply_coordination_schema(&connection).expect("first apply");
        apply_coordination_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'coordination_runs'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 1);
    }
}
