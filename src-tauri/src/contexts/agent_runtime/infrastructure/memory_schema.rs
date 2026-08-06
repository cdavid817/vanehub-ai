use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// Cross-session agent memory (`add-agent-cross-session-memory`) — `folder` uses an empty-string
/// sentinel for "no workspace folder" (matching `skill-management`'s own global-scope convention)
/// rather than a nullable column, so `WHERE folder = ?` works uniformly without `IS NULL` branches.
pub(crate) fn apply_memory_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS agent_memories (
            id TEXT PRIMARY KEY,
            agent_id TEXT NOT NULL,
            folder TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            source TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            FOREIGN KEY (agent_id) REFERENCES agents(id)
        );

        CREATE INDEX IF NOT EXISTS idx_agent_memories_scope
            ON agent_memories(agent_id, folder, created_at DESC);
        "#,
    )?;
    Ok(())
}

/// `add-cli-memory-support`: memories became a shared host-level pool — reads no longer filter by
/// `agent_id`/`folder`, so the composite index from `apply_memory_schema` no longer serves the
/// query pattern. Runs as its own versioned migration (not a retroactive edit to
/// `apply_memory_schema`, which has already executed on existing databases) so already-migrated
/// installs pick up the replacement index too.
pub(crate) fn apply_memory_shared_pool_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        DROP INDEX IF EXISTS idx_agent_memories_scope;

        CREATE INDEX IF NOT EXISTS idx_agent_memories_recency
            ON agent_memories(created_at DESC);
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_schema_is_idempotent() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch("CREATE TABLE agents (id TEXT PRIMARY KEY);")
            .expect("dependencies");

        apply_memory_schema(&connection).expect("first apply");
        apply_memory_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'agent_memories'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 1);
    }

    #[test]
    fn shared_pool_schema_is_idempotent_and_replaces_the_scoped_index() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch("CREATE TABLE agents (id TEXT PRIMARY KEY);")
            .expect("dependencies");
        apply_memory_schema(&connection).expect("base schema");

        apply_memory_shared_pool_schema(&connection).expect("first apply");
        apply_memory_shared_pool_schema(&connection).expect("second apply");

        let scoped_index_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_agent_memories_scope'",
                [],
                |row| row.get(0),
            )
            .expect("scoped index count");
        assert_eq!(scoped_index_count, 0);

        let recency_index_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'index' AND name = 'idx_agent_memories_recency'",
                [],
                |row| row.get(0),
            )
            .expect("recency index count");
        assert_eq!(recency_index_count, 1);
    }
}
