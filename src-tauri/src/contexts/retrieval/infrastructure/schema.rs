use crate::platform::database::DatabaseError;
use rusqlite::Connection;

/// 迁移 43 `retrieval-vector-index`（`add-retrieval-vector-search`）。
///
/// `scope_agent_id`/`scope_folder` 是**溯源**，不是作用域：`agent-memory-shared-pool`（迁移 42）
/// 之后记忆是主机级共享池，两路召回都不按它们过滤，与 `agent_memories.agent_id`/`folder` 在
/// 那次变更之后的定位一致。列保留在本表，是为了让索引行仍能说清"这条记忆最初由谁、在哪个
/// 工作区存下"。`idx_retrieval_documents_scope` 因此只剩前导列 `source_kind` 还在为召回服务；
/// 不改它的定义，是因为改列不改名会让已经跑过本迁移的库与新库拿到两份同名不同义的索引。
/// FTS 建在本表而非 `agent_memories`：第 2/3 期的源表不同，统一在本表做 FTS 才能让混合检索
/// 只实现一次。
/// 不建到 `agent_memories` 的外键：跨期源表不同，靠 `source_kind + source_id` 逻辑关联，
/// 检索结果一律回查源表，源已删则跳过。
pub(crate) fn apply_retrieval_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS retrieval_documents (
            id TEXT PRIMARY KEY,
            source_kind TEXT NOT NULL,
            source_id TEXT NOT NULL,
            scope_agent_id TEXT NOT NULL,
            scope_folder TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL,
            content_hash TEXT NOT NULL,
            index_state TEXT NOT NULL DEFAULT 'pending',
            attempt_count INTEGER NOT NULL DEFAULT 0,
            failure_category TEXT,
            embedding_model TEXT,
            embedding_dimensions INTEGER,
            embedding BLOB,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE (source_kind, source_id)
        );

        CREATE INDEX IF NOT EXISTS idx_retrieval_documents_scope
            ON retrieval_documents (source_kind, scope_agent_id, scope_folder, index_state);
        CREATE INDEX IF NOT EXISTS idx_retrieval_documents_queue
            ON retrieval_documents (index_state, updated_at);

        CREATE VIRTUAL TABLE IF NOT EXISTS retrieval_documents_fts USING fts5(
            content,
            content='retrieval_documents',
            content_rowid='rowid',
            tokenize='trigram'
        );

        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_insert
        AFTER INSERT ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(rowid, content) VALUES (new.rowid, new.content);
        END;
        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_delete
        AFTER DELETE ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(retrieval_documents_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
        END;
        CREATE TRIGGER IF NOT EXISTS retrieval_documents_fts_update
        AFTER UPDATE OF content ON retrieval_documents BEGIN
            INSERT INTO retrieval_documents_fts(retrieval_documents_fts, rowid, content)
            VALUES ('delete', old.rowid, old.content);
            INSERT INTO retrieval_documents_fts(rowid, content) VALUES (new.rowid, new.content);
        END;

        -- 单例配置行。retrieval 拥有自己的配置表，而不是借用 desktop 上下文的 settings KV 表，
        -- 避免为读一条自有配置去依赖另一个上下文的 api。
        CREATE TABLE IF NOT EXISTS retrieval_configuration (
            id INTEGER PRIMARY KEY CHECK (id = 1),
            source_profile_id TEXT,
            embedding_model TEXT,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("database");
        apply_retrieval_schema(&connection).expect("first apply");
        connection
    }

    #[test]
    fn schema_is_idempotent() {
        let connection = migrated_connection();
        apply_retrieval_schema(&connection).expect("second apply");

        let table_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = 'retrieval_documents'",
                [],
                |row| row.get(0),
            )
            .expect("table count");
        assert_eq!(table_count, 1);
    }

    #[test]
    fn inserting_a_document_populates_the_fts_shadow_table() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm not pnpm', 'h', 't', 't')",
                [],
            )
            .expect("insert");

        let hits: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("fts count");
        assert_eq!(hits, 1);
    }

    #[test]
    fn updating_content_replaces_the_fts_entry() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm', 'h', 't', 't')",
                [],
            )
            .expect("insert");
        connection
            .execute(
                "UPDATE retrieval_documents SET content = 'uses cargo' WHERE id = 'agent_memory:m1'",
                [],
            )
            .expect("update");

        let stale: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("stale count");
        let fresh: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"cargo\"'",
                [],
                |row| row.get(0),
            )
            .expect("fresh count");
        assert_eq!(stale, 0);
        assert_eq!(fresh, 1);
    }

    #[test]
    fn deleting_a_document_clears_its_fts_entry() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'uses npm', 'h', 't', 't')",
                [],
            )
            .expect("insert");
        connection
            .execute(
                "DELETE FROM retrieval_documents WHERE id = 'agent_memory:m1'",
                [],
            )
            .expect("delete");

        let hits: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents_fts WHERE retrieval_documents_fts MATCH '\"npm\"'",
                [],
                |row| row.get(0),
            )
            .expect("fts count");
        assert_eq!(hits, 0);
    }

    #[test]
    fn the_same_source_cannot_be_indexed_twice() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                 VALUES ('agent_memory:m1','agent_memory','m1','a','', 'x', 'h', 't', 't')",
                [],
            )
            .expect("first insert");

        // 第二行**换一个 id**，否则 PRIMARY KEY 冲突会与 UNIQUE 冲突混在一起：
        // 那样即使 UNIQUE (source_kind, source_id) 被删掉，这条测试照样通过。
        let duplicate_source = connection.execute(
            "INSERT INTO retrieval_documents
             (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
             VALUES ('agent_memory:m1-dup','agent_memory','m1','a','', 'x', 'h', 't', 't')",
            [],
        );

        assert!(duplicate_source.is_err());
    }

    #[test]
    fn the_configuration_table_holds_at_most_one_row() {
        let connection = migrated_connection();
        connection
            .execute(
                "INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
                 VALUES (1, 'p1', 'text-embedding-3-small', 't')",
                [],
            )
            .expect("singleton insert");
        assert!(connection
            .execute(
                "INSERT INTO retrieval_configuration (id, source_profile_id, embedding_model, updated_at)
                 VALUES (2, 'p2', 'other', 't')",
                [],
            )
            .is_err());
    }
}
