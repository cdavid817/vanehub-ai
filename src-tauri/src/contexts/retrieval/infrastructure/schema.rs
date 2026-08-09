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
            automatic_code_index_mode TEXT NOT NULL DEFAULT 'disabled'
                CHECK (automatic_code_index_mode IN ('disabled', 'local', 'semantic')),
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_code_index_automatic_mode_schema(
    connection: &Connection,
) -> Result<(), DatabaseError> {
    let has_column = connection
        .prepare("PRAGMA table_info(retrieval_configuration)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == "automatic_code_index_mode");
    if !has_column {
        connection.execute_batch(
            "ALTER TABLE retrieval_configuration ADD COLUMN automatic_code_index_mode TEXT \
             NOT NULL DEFAULT 'disabled' CHECK (automatic_code_index_mode IN \
             ('disabled', 'local', 'semantic'));",
        )?;
    }
    let has_origin = connection
        .prepare("PRAGMA table_info(code_index_workspaces)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .filter_map(Result::ok)
        .any(|name| name == "origin");
    if !has_origin {
        connection.execute_batch(
            "ALTER TABLE code_index_workspaces ADD COLUMN origin TEXT NOT NULL DEFAULT 'manual' \
             CHECK (origin IN ('manual', 'automatic'));",
        )?;
    }
    Ok(())
}

pub(crate) fn apply_code_index_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS code_index_workspaces (
            workspace_id TEXT PRIMARY KEY,
            canonical_root TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            origin TEXT NOT NULL DEFAULT 'manual' CHECK (origin IN ('manual', 'automatic')),
            enabled INTEGER NOT NULL DEFAULT 0 CHECK (enabled IN (0, 1)),
            index_mode TEXT NOT NULL DEFAULT 'semantic' CHECK (index_mode IN ('local', 'semantic')),
            selected_roots_json TEXT NOT NULL DEFAULT '[""]',
            languages_json TEXT NOT NULL DEFAULT '[]',
            exclusion_patterns_json TEXT NOT NULL DEFAULT '[]',
            max_file_bytes INTEGER NOT NULL DEFAULT 102400 CHECK (max_file_bytes > 0),
            index_version TEXT NOT NULL,
            phase TEXT NOT NULL DEFAULT 'disabled',
            generation INTEGER NOT NULL DEFAULT 0,
            embedding_confirmed_profile TEXT,
            embedding_confirmed_model TEXT,
            embedding_confirmed_generation INTEGER,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_code_index_workspaces_root
            ON code_index_workspaces(canonical_root COLLATE NOCASE);

        CREATE TABLE IF NOT EXISTS code_index_files (
            workspace_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            language TEXT NOT NULL,
            byte_size INTEGER NOT NULL,
            modified_ns INTEGER NOT NULL,
            content_hash TEXT NOT NULL,
            index_version TEXT NOT NULL,
            state TEXT NOT NULL DEFAULT 'pending',
            failure_category TEXT,
            chunk_count INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY (workspace_id, relative_path),
            FOREIGN KEY (workspace_id) REFERENCES code_index_workspaces(workspace_id) ON DELETE CASCADE
        );

        CREATE TABLE IF NOT EXISTS code_index_chunks (
            document_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            language TEXT NOT NULL,
            start_line INTEGER NOT NULL CHECK (start_line >= 1),
            end_line INTEGER NOT NULL CHECK (end_line >= start_line),
            symbol_name TEXT,
            symbol_kind TEXT,
            chunk_ordinal INTEGER NOT NULL,
            chunk_key TEXT NOT NULL,
            redaction_count INTEGER NOT NULL DEFAULT 0,
            index_version TEXT NOT NULL,
            FOREIGN KEY (document_id) REFERENCES retrieval_documents(id) ON DELETE CASCADE,
            FOREIGN KEY (workspace_id, relative_path)
                REFERENCES code_index_files(workspace_id, relative_path) ON DELETE CASCADE,
            UNIQUE (workspace_id, relative_path, chunk_key)
        );

        CREATE INDEX IF NOT EXISTS idx_code_index_chunks_workspace
            ON code_index_chunks(workspace_id, relative_path, start_line);

        CREATE TRIGGER IF NOT EXISTS code_index_chunks_delete_document
        AFTER DELETE ON code_index_chunks BEGIN
            DELETE FROM retrieval_documents WHERE id = old.document_id;
        END;

        CREATE TABLE IF NOT EXISTS code_index_symbols (
            symbol_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            relative_path TEXT NOT NULL,
            normalized_name TEXT NOT NULL,
            display_name TEXT NOT NULL,
            symbol_kind TEXT NOT NULL,
            container_name TEXT,
            start_line INTEGER NOT NULL CHECK (start_line >= 1),
            end_line INTEGER NOT NULL CHECK (end_line >= start_line),
            FOREIGN KEY (workspace_id, relative_path)
                REFERENCES code_index_files(workspace_id, relative_path) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_code_index_symbols_lookup
            ON code_index_symbols(workspace_id, normalized_name, symbol_kind);

        CREATE TABLE IF NOT EXISTS code_index_audit (
            audit_id INTEGER PRIMARY KEY AUTOINCREMENT,
            workspace_id TEXT NOT NULL,
            relative_path TEXT,
            event_kind TEXT NOT NULL,
            reason_category TEXT,
            item_count INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            FOREIGN KEY (workspace_id) REFERENCES code_index_workspaces(workspace_id) ON DELETE CASCADE
        );

        CREATE INDEX IF NOT EXISTS idx_code_index_audit_workspace_time
            ON code_index_audit(workspace_id, created_at DESC, audit_id DESC);
        "#,
    )?;
    Ok(())
}

pub(crate) fn apply_code_index_mode_schema(connection: &Connection) -> Result<(), DatabaseError> {
    if !crate::platform::database::table_has_column(
        connection,
        "code_index_workspaces",
        "index_mode",
    )? {
        connection.execute_batch(
            "ALTER TABLE code_index_workspaces ADD COLUMN index_mode TEXT NOT NULL DEFAULT 'semantic' CHECK (index_mode IN ('local', 'semantic'));",
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn migrated_connection() -> Connection {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .pragma_update(None, "foreign_keys", "ON")
            .expect("foreign keys");
        apply_retrieval_schema(&connection).expect("first apply");
        connection
    }

    fn code_index_connection() -> Connection {
        let connection = migrated_connection();
        apply_code_index_schema(&connection).expect("code index schema");
        connection
    }

    #[test]
    fn code_index_schema_is_idempotent_and_preserves_agent_memory() {
        let connection = code_index_connection();
        connection
            .execute(
                "INSERT INTO retrieval_documents
                 (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
                  created_at, updated_at)
                 VALUES ('agent_memory:m1', 'agent_memory', 'm1', 'agent', '', 'memory', 'hash', 't', 't')",
                [],
            )
            .expect("memory fixture");

        apply_code_index_schema(&connection).expect("second apply");

        let memory_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'agent_memory'",
                [],
                |row| row.get(0),
            )
            .expect("memory count");
        assert_eq!(memory_count, 1);
        let code_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'workspace_file'",
                [],
                |row| row.get(0),
            )
            .expect("code count");
        assert_eq!(code_count, 0);
        for table in [
            "code_index_workspaces",
            "code_index_files",
            "code_index_chunks",
            "code_index_symbols",
            "code_index_audit",
        ] {
            let count: i64 = connection
                .query_row(
                    "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = ?1",
                    [table],
                    |row| row.get(0),
                )
                .expect("table count");
            assert_eq!(count, 1, "missing table {table}");
        }
    }

    #[test]
    fn mode_migration_preserves_existing_workspaces_as_semantic() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE code_index_workspaces (
                    workspace_id TEXT PRIMARY KEY,
                    canonical_root TEXT NOT NULL,
                    display_name TEXT NOT NULL
                );
                INSERT INTO code_index_workspaces VALUES ('existing', 'C:/repo', 'repo');
                "#,
            )
            .expect("legacy fixture");

        apply_code_index_mode_schema(&connection).expect("mode migration");
        apply_code_index_mode_schema(&connection).expect("idempotent migration");

        let mode: String = connection
            .query_row(
                "SELECT index_mode FROM code_index_workspaces WHERE workspace_id = 'existing'",
                [],
                |row| row.get(0),
            )
            .expect("migrated mode");
        assert_eq!(mode, "semantic");
    }

    #[test]
    fn automatic_mode_migration_defaults_disabled_and_marks_existing_workspaces_manual() {
        let connection = Connection::open_in_memory().expect("database");
        connection
            .execute_batch(
                r#"
                CREATE TABLE retrieval_configuration (
                    id INTEGER PRIMARY KEY,
                    source_profile_id TEXT,
                    embedding_model TEXT,
                    updated_at TEXT NOT NULL
                );
                INSERT INTO retrieval_configuration VALUES (1, 'profile', 'model', 't');
                CREATE TABLE code_index_workspaces (
                    workspace_id TEXT PRIMARY KEY,
                    canonical_root TEXT NOT NULL,
                    display_name TEXT NOT NULL
                );
                INSERT INTO code_index_workspaces VALUES ('existing', 'C:/repo', 'repo');
                "#,
            )
            .expect("legacy fixture");

        apply_code_index_automatic_mode_schema(&connection).expect("migration");
        apply_code_index_automatic_mode_schema(&connection).expect("idempotent migration");

        let values: (String, String, String) = connection
            .query_row(
                "SELECT automatic_code_index_mode, source_profile_id, embedding_model \
                 FROM retrieval_configuration WHERE id = 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .expect("configuration");
        let origin: String = connection
            .query_row(
                "SELECT origin FROM code_index_workspaces WHERE workspace_id = 'existing'",
                [],
                |row| row.get(0),
            )
            .expect("origin");
        assert_eq!(
            values,
            (
                "disabled".to_string(),
                "profile".to_string(),
                "model".to_string()
            )
        );
        assert_eq!(origin, "manual");
    }

    #[test]
    fn deleting_a_workspace_cascades_code_rows_without_touching_memory() {
        let connection = code_index_connection();
        connection
            .execute_batch(
                r#"
                INSERT INTO retrieval_documents
                  (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash, created_at, updated_at)
                VALUES
                  ('agent_memory:m1', 'agent_memory', 'm1', 'agent', '', 'memory', 'hm', 't', 't'),
                  ('workspace_file:c1', 'workspace_file', 'c1', '', 'workspace-1', 'code', 'hc', 't', 't');
                INSERT INTO code_index_workspaces
                  (workspace_id, canonical_root, display_name, index_version, created_at, updated_at)
                VALUES ('workspace-1', 'C:/repo', 'repo', 'v1', 't', 't');
                INSERT INTO code_index_files
                  (workspace_id, relative_path, language, byte_size, modified_ns, content_hash,
                   index_version, created_at, updated_at)
                VALUES ('workspace-1', 'src/lib.rs', 'rust', 10, 1, 'hc', 'v1', 't', 't');
                INSERT INTO code_index_chunks
                  (document_id, workspace_id, relative_path, language, start_line, end_line,
                   chunk_ordinal, chunk_key, index_version)
                VALUES ('workspace_file:c1', 'workspace-1', 'src/lib.rs', 'rust', 1, 2, 0, 'k1', 'v1');
                INSERT INTO code_index_symbols
                  (symbol_id, workspace_id, relative_path, normalized_name, display_name,
                   symbol_kind, start_line, end_line)
                VALUES ('s1', 'workspace-1', 'src/lib.rs', 'main', 'main', 'function', 1, 2);
                INSERT INTO code_index_audit
                  (workspace_id, relative_path, event_kind, created_at)
                VALUES ('workspace-1', 'src/lib.rs', 'indexed', 't');
                DELETE FROM code_index_workspaces WHERE workspace_id = 'workspace-1';
                "#,
            )
            .expect("fixture and delete");

        for table in [
            "code_index_files",
            "code_index_chunks",
            "code_index_symbols",
            "code_index_audit",
        ] {
            let count: i64 = connection
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                    row.get(0)
                })
                .expect("row count");
            assert_eq!(count, 0, "orphaned rows in {table}");
        }
        let memory_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'agent_memory'",
                [],
                |row| row.get(0),
            )
            .expect("memory count");
        assert_eq!(memory_count, 1);
        let code_count: i64 = connection
            .query_row(
                "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'workspace_file'",
                [],
                |row| row.get(0),
            )
            .expect("code count");
        assert_eq!(code_count, 0);
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
