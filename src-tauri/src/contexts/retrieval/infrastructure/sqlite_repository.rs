use crate::contexts::retrieval::application::{RetrievalDocumentRepository, RetrievalIndexStatus};
use crate::contexts::retrieval::domain::{
    decode_embedding, encode_embedding, FailureCategory, IndexState, RetrievalDocument,
    RetrievalError, RetrievalScope, SourceKind,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Row};

#[derive(Clone)]
pub(crate) struct SqliteRetrievalDocumentRepository {
    database: NativeDatabase,
}

impl SqliteRetrievalDocumentRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl RetrievalDocumentRepository for SqliteRetrievalDocumentRepository {
    fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        // 内容没变就保留既有 index_state/attempt_count/failure_category：否则每次 reconcile
        // 都会把已经索引好的行打回 pending，白白重烧一次 embedding 配额。
        connection
            .execute(
                r#"
                INSERT INTO retrieval_documents
                    (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
                     index_state, attempt_count, failure_category, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, NULL, ?8, ?8)
                ON CONFLICT(id) DO UPDATE SET
                    content = excluded.content,
                    content_hash = excluded.content_hash,
                    scope_agent_id = excluded.scope_agent_id,
                    scope_folder = excluded.scope_folder,
                    index_state = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                       THEN retrieval_documents.index_state ELSE 'pending' END,
                    attempt_count = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                         THEN retrieval_documents.attempt_count ELSE 0 END,
                    failure_category = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                            THEN retrieval_documents.failure_category ELSE NULL END,
                    updated_at = excluded.updated_at
                "#,
                params![
                    document.id,
                    document.source_kind.as_str(),
                    document.source_id,
                    document.scope_agent_id,
                    document.scope_folder,
                    document.content,
                    document.content_hash,
                    now,
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn reconcile_apply(
        &self,
        upserts: &[RetrievalDocument],
        orphan_source_ids: &[String],
        source_kind: SourceKind,
    ) -> Result<(), RetrievalError> {
        // One transaction for the whole reconcile instead of one autocommit per row — a full
        // workspace re-index otherwise pays one fsync per document/orphan. Same SQL as
        // upsert_pending / delete_by_source_scoped, just batched under a single BEGIN/COMMIT.
        if upserts.is_empty() && orphan_source_ids.is_empty() {
            return Ok(());
        }
        let connection = self.database.connection().map_err(database_error)?;
        let transaction = connection.unchecked_transaction().map_err(storage_error)?;
        let now = SystemClock.rfc3339();
        {
            let mut upsert_statement = transaction
                .prepare(
                    r#"
                INSERT INTO retrieval_documents
                    (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
                     index_state, attempt_count, failure_category, created_at, updated_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'pending', 0, NULL, ?8, ?8)
                ON CONFLICT(id) DO UPDATE SET
                    content = excluded.content,
                    content_hash = excluded.content_hash,
                    scope_agent_id = excluded.scope_agent_id,
                    scope_folder = excluded.scope_folder,
                    index_state = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                       THEN retrieval_documents.index_state ELSE 'pending' END,
                    attempt_count = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                         THEN retrieval_documents.attempt_count ELSE 0 END,
                    failure_category = CASE WHEN retrieval_documents.content_hash = excluded.content_hash
                                            THEN retrieval_documents.failure_category ELSE NULL END,
                    updated_at = excluded.updated_at
                "#,
                )
                .map_err(storage_error)?;
            for document in upserts {
                upsert_statement
                    .execute(params![
                        document.id,
                        document.source_kind.as_str(),
                        document.source_id,
                        document.scope_agent_id,
                        document.scope_folder,
                        document.content,
                        document.content_hash,
                        now,
                    ])
                    .map_err(storage_error)?;
            }
            let mut delete_statement = transaction
                .prepare(
                    "DELETE FROM retrieval_documents WHERE source_kind = ?1 AND source_id = ?2",
                )
                .map_err(storage_error)?;
            for source_id in orphan_source_ids {
                delete_statement
                    .execute(params![source_kind.as_str(), source_id])
                    .map_err(storage_error)?;
            }
        }
        transaction.commit().map_err(storage_error)?;
        Ok(())
    }

    fn list_indexed_source_ids(
        &self,
        source_kind: SourceKind,
    ) -> Result<Vec<(String, String)>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT source_id, content_hash FROM retrieval_documents WHERE source_kind = ?1",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![source_kind.as_str()], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn delete_by_source(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .execute(
                "DELETE FROM retrieval_documents WHERE source_kind = ?1 AND source_id = ?2",
                params![source_kind.as_str(), source_id],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn claim_pending_batch(
        &self,
        source_kind: SourceKind,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
                       index_state, attempt_count, embedding_model
                FROM retrieval_documents
                WHERE source_kind = ?1 AND index_state = 'pending'
                ORDER BY updated_at ASC
                LIMIT ?2
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![source_kind.as_str(), limit as i64],
                DocumentRow::read,
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        rows.into_iter().map(DocumentRow::into_document).collect()
    }

    fn store_embedding(
        &self,
        id: &str,
        model: &str,
        embedding: &[f32],
    ) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        let blob = encode_embedding(embedding);
        let dimensions = embedding.len() as i64;
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET embedding = ?2, embedding_model = ?3, embedding_dimensions = ?4,
                    index_state = 'indexed', failure_category = NULL, attempt_count = 0, updated_at = ?5
                WHERE id = ?1
                "#,
                params![id, blob, model, dimensions, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn record_failure(
        &self,
        id: &str,
        category: FailureCategory,
        give_up: bool,
    ) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        // give_up 决定终态；attempt_count 的自增就在这条语句里完成，调用方（后台 worker）
        // 不需要先读出旧值再写回——那样会在并发轮次之间开一个竞态窗口。
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET attempt_count = attempt_count + 1,
                    failure_category = ?2,
                    index_state = CASE WHEN ?3 THEN 'failed' ELSE 'pending' END,
                    updated_at = ?4
                WHERE id = ?1
                "#,
                params![id, category.as_str(), give_up, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    /// 不按 `scope_agent_id`/`scope_folder` 过滤：那两列在 `agent-memory-shared-pool` 之后只是
    /// 溯源信息，记忆池本身是主机级共享的。
    fn vector_candidates(
        &self,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT source_id, embedding FROM retrieval_documents
                WHERE source_kind = ?1
                  AND index_state = 'indexed' AND embedding_model = ?2 AND embedding IS NOT NULL
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![source_kind.as_str(), model], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;

        // 存进去的 BLOB 只可能来自 store_embedding 里的 encode_embedding，损坏说明数据被外部
        // 改过；让整次查询失败而不是悄悄丢一半候选，好让调用方能感知到异常。
        rows.into_iter()
            .map(|(source_id, blob)| match decode_embedding(&blob) {
                Some(vector) => Ok((source_id, vector)),
                None => Err(RetrievalError::Storage(format!(
                    "stored embedding for source '{source_id}' is not a valid f32 blob"
                ))),
            })
            .collect()
    }

    /// 与 `vector_candidates` 同样覆盖整个共享池。
    fn keyword_candidates(
        &self,
        source_kind: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT d.source_id FROM retrieval_documents d
                JOIN retrieval_documents_fts f ON f.rowid = d.rowid
                WHERE retrieval_documents_fts MATCH ?1
                  AND d.source_kind = ?2
                ORDER BY bm25(retrieval_documents_fts)
                LIMIT ?3
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![query, source_kind.as_str(), limit as i64], |row| {
                row.get::<_, String>(0)
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .query_row(
                r#"
                SELECT
                  SUM(index_state = 'indexed'), SUM(index_state = 'pending'), SUM(index_state = 'failed'),
                  (SELECT failure_category FROM retrieval_documents
                   WHERE failure_category IS NOT NULL
                   ORDER BY updated_at DESC LIMIT 1)
                FROM retrieval_documents
                "#,
                [],
                |row| {
                    // 一行索引都还没建过时，SUM() 在零行上聚合返回 NULL（SQLite 的空集合聚合
                    // 语义），不是 0——必须按 Option 读，否则会在全新安装上把这条本该是
                    // "状态全零"的查询错误地变成一个 storage 错误。
                    let indexed: Option<i64> = row.get(0)?;
                    let pending: Option<i64> = row.get(1)?;
                    let failed: Option<i64> = row.get(2)?;
                    let last_failure_category: Option<String> = row.get(3)?;
                    Ok(RetrievalIndexStatus {
                        indexed: indexed.unwrap_or(0) as u32,
                        pending: pending.unwrap_or(0) as u32,
                        failed: failed.unwrap_or(0) as u32,
                        last_failure_category,
                    })
                },
            )
            .map_err(storage_error)
    }

    fn requeue_all(&self) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?1
                "#,
                params![now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn requeue_stale_model(&self, new_model: &str) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        // 只动 `indexed` 行：`failed` 是达到重试上限的终态，换模型不该悄悄把它们复活；
        // `pending` 行本来就会被下一批认领。旧向量刻意保留不清——重新 embedding 成功时
        // `store_embedding` 会整体覆盖，在那之前保留只是多占一点空间，而清空会让本就
        // 收敛缓慢的过程连"旧模型下的召回"都一并失去（设计文档权衡 3）。
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?2
                WHERE index_state = 'indexed'
                  AND (embedding_model IS NULL OR embedding_model <> ?1)
                "#,
                params![new_model, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn list_indexed_source_ids_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<Vec<(String, String)>, RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                "SELECT source_id, content_hash FROM retrieval_documents
                 WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)",
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![source_kind.as_str(), workspace_id], |row| {
                Ok((row.get(0)?, row.get(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn claim_pending_batch_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
                       index_state, attempt_count, embedding_model
                FROM retrieval_documents
                WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                  AND index_state = 'pending'
                ORDER BY updated_at ASC
                LIMIT ?3
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![source_kind.as_str(), workspace_id, limit as i64],
                DocumentRow::read,
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        rows.into_iter().map(DocumentRow::into_document).collect()
    }

    fn vector_candidates_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT source_id, embedding FROM retrieval_documents
                WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                  AND index_state = 'indexed' AND embedding_model = ?3 AND embedding IS NOT NULL
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(params![source_kind.as_str(), workspace_id, model], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?))
            })
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        decode_candidates(rows)
    }

    fn keyword_candidates_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT d.source_id FROM retrieval_documents d
                JOIN retrieval_documents_fts f ON f.rowid = d.rowid
                WHERE retrieval_documents_fts MATCH ?1 AND d.source_kind = ?2
                  AND (?3 IS NULL OR d.scope_folder = ?3)
                ORDER BY bm25(retrieval_documents_fts)
                LIMIT ?4
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![query, source_kind.as_str(), workspace_id, limit as i64],
                |row| row.get(0),
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn index_status_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<RetrievalIndexStatus, RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .query_row(
                r#"
                SELECT
                  SUM(index_state = 'indexed'), SUM(index_state = 'pending'), SUM(index_state = 'failed'),
                  (SELECT failure_category FROM retrieval_documents
                   WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                     AND failure_category IS NOT NULL ORDER BY updated_at DESC LIMIT 1)
                FROM retrieval_documents
                WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                "#,
                params![source_kind.as_str(), workspace_id],
                read_index_status,
            )
            .map_err(storage_error)
    }

    fn requeue_all_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
    ) -> Result<(), RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?3
                WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                "#,
                params![source_kind.as_str(), workspace_id, SystemClock.rfc3339()],
            )
            .map_err(storage_error)?;
        Ok(())
    }

    fn requeue_stale_model_scoped(
        &self,
        source_kind: SourceKind,
        scope: &RetrievalScope,
        new_model: &str,
    ) -> Result<(), RetrievalError> {
        let workspace_id = validated_workspace(source_kind, scope)?;
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?4
                WHERE source_kind = ?1 AND (?2 IS NULL OR scope_folder = ?2)
                  AND index_state = 'indexed'
                  AND (embedding_model IS NULL OR embedding_model <> ?3)
                "#,
                params![
                    source_kind.as_str(),
                    workspace_id,
                    new_model,
                    SystemClock.rfc3339()
                ],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

fn validated_workspace(
    source_kind: SourceKind,
    scope: &RetrievalScope,
) -> Result<Option<&str>, RetrievalError> {
    scope.validate_for(source_kind)?;
    Ok(scope.workspace_id())
}

fn decode_candidates(
    rows: Vec<(String, Vec<u8>)>,
) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
    rows.into_iter()
        .map(|(source_id, blob)| {
            decode_embedding(&blob)
                .map(|vector| (source_id.clone(), vector))
                .ok_or_else(|| {
                    RetrievalError::Storage(format!(
                        "stored embedding for source '{source_id}' is not a valid f32 blob"
                    ))
                })
        })
        .collect()
}

fn read_index_status(row: &Row<'_>) -> Result<RetrievalIndexStatus, rusqlite::Error> {
    let indexed: Option<i64> = row.get(0)?;
    let pending: Option<i64> = row.get(1)?;
    let failed: Option<i64> = row.get(2)?;
    Ok(RetrievalIndexStatus {
        indexed: indexed.unwrap_or(0) as u32,
        pending: pending.unwrap_or(0) as u32,
        failed: failed.unwrap_or(0) as u32,
        last_failure_category: row.get(3)?,
    })
}

struct DocumentRow {
    id: String,
    source_kind: String,
    source_id: String,
    scope_agent_id: String,
    scope_folder: String,
    content: String,
    content_hash: String,
    index_state: String,
    attempt_count: u32,
    embedding_model: Option<String>,
}

impl DocumentRow {
    fn read(row: &Row<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.get(0)?,
            source_kind: row.get(1)?,
            source_id: row.get(2)?,
            scope_agent_id: row.get(3)?,
            scope_folder: row.get(4)?,
            content: row.get(5)?,
            content_hash: row.get(6)?,
            index_state: row.get(7)?,
            attempt_count: row.get(8)?,
            embedding_model: row.get(9)?,
        })
    }

    fn into_document(self) -> Result<RetrievalDocument, RetrievalError> {
        let source_kind = SourceKind::parse(&self.source_kind).ok_or_else(|| {
            RetrievalError::Storage(format!(
                "invalid persisted source kind: {}",
                self.source_kind
            ))
        })?;
        let index_state = IndexState::parse(&self.index_state).ok_or_else(|| {
            RetrievalError::Storage(format!(
                "invalid persisted index state: {}",
                self.index_state
            ))
        })?;
        Ok(RetrievalDocument {
            id: self.id,
            source_kind,
            source_id: self.source_id,
            scope_agent_id: self.scope_agent_id,
            scope_folder: self.scope_folder,
            content: self.content,
            content_hash: self.content_hash,
            index_state,
            attempt_count: self.attempt_count,
            embedding_model: self.embedding_model,
        })
    }
}

fn database_error(error: DatabaseError) -> RetrievalError {
    match error {
        DatabaseError::Database(error) => storage_error(error),
        DatabaseError::Storage(message) => RetrievalError::Storage(message),
    }
}

fn storage_error(error: rusqlite::Error) -> RetrievalError {
    RetrievalError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::domain::{content_hash, document_id};
    use crate::test_support::TempDirectory;

    struct Fixture {
        _directory: TempDirectory,
        database: NativeDatabase,
        repository: SqliteRetrievalDocumentRepository,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let database =
                NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
            Self {
                repository: SqliteRetrievalDocumentRepository::new(database.clone()),
                database,
                _directory: directory,
            }
        }

        /// `attempt_count` 不进 `RetrievalIndexStatus`——它是 worker 的内部记账，不该出现在 UI 契约里。
        /// 但重试与重建的正确性正取决于它，所以测试直接读列。
        fn attempt_count(&self, source_id: &str) -> i64 {
            self.database
                .connection()
                .expect("connection")
                .query_row(
                    "SELECT attempt_count FROM retrieval_documents WHERE source_id = ?1",
                    params![source_id],
                    |row| row.get(0),
                )
                .expect("attempt count")
        }
    }

    fn document(source_id: &str, agent: &str, folder: &str, content: &str) -> RetrievalDocument {
        RetrievalDocument {
            id: document_id(SourceKind::AgentMemory, source_id),
            source_kind: SourceKind::AgentMemory,
            source_id: source_id.to_string(),
            scope_agent_id: agent.to_string(),
            scope_folder: folder.to_string(),
            content: content.to_string(),
            content_hash: content_hash(content),
            index_state: IndexState::Pending,
            attempt_count: 0,
            embedding_model: None,
        }
    }

    fn workspace_document(workspace: &str, source_id: &str, content: &str) -> RetrievalDocument {
        RetrievalDocument {
            id: document_id(SourceKind::WorkspaceFile, source_id),
            source_kind: SourceKind::WorkspaceFile,
            source_id: source_id.to_string(),
            scope_agent_id: String::new(),
            scope_folder: workspace.to_string(),
            content: content.to_string(),
            content_hash: content_hash(content),
            index_state: IndexState::Pending,
            attempt_count: 0,
            embedding_model: None,
        }
    }

    fn workspace_scope(id: &str) -> RetrievalScope {
        RetrievalScope::Workspace(id.to_string())
    }

    #[test]
    fn workspace_scoped_operations_never_cross_workspace_or_memory_boundaries() {
        let fixture = Fixture::new("retrieval workspace scope isolation");
        for entry in [
            document("memory", "agent", "workspace-a", "shared memory"),
            workspace_document("workspace-a", "a-indexed", "shared alpha"),
            workspace_document("workspace-a", "a-pending", "pending alpha"),
            workspace_document("workspace-b", "b-indexed", "shared beta"),
        ] {
            fixture.repository.upsert_pending(&entry).expect("upsert");
        }
        for id in ["a-indexed", "b-indexed"] {
            fixture
                .repository
                .store_embedding(
                    &document_id(SourceKind::WorkspaceFile, id),
                    "model-a",
                    &[1.0, 0.0],
                )
                .expect("store embedding");
        }

        let scope_a = workspace_scope("workspace-a");
        let scope_b = workspace_scope("workspace-b");
        assert_eq!(
            fixture
                .repository
                .list_indexed_source_ids_scoped(SourceKind::WorkspaceFile, &scope_a)
                .expect("list a")
                .len(),
            2
        );
        assert_eq!(
            fixture
                .repository
                .claim_pending_batch_scoped(SourceKind::WorkspaceFile, &scope_a, 10)
                .expect("claim a")
                .into_iter()
                .map(|row| row.source_id)
                .collect::<Vec<_>>(),
            vec!["a-pending"]
        );
        assert_eq!(
            fixture
                .repository
                .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope_a, "model-a")
                .expect("vectors a")[0]
                .0,
            "a-indexed"
        );
        assert_eq!(
            fixture
                .repository
                .keyword_candidates_scoped(SourceKind::WorkspaceFile, &scope_b, "\"shared\"", 10,)
                .expect("keywords b"),
            vec!["b-indexed"]
        );

        let status_a = fixture
            .repository
            .index_status_scoped(SourceKind::WorkspaceFile, &scope_a)
            .expect("status a");
        assert_eq!((status_a.indexed, status_a.pending), (1, 1));
        fixture
            .repository
            .requeue_stale_model_scoped(SourceKind::WorkspaceFile, &scope_a, "model-b")
            .expect("requeue stale a");
        assert_eq!(
            fixture
                .repository
                .index_status_scoped(SourceKind::WorkspaceFile, &scope_b)
                .expect("status b")
                .indexed,
            1
        );
        assert_eq!(
            fixture
                .repository
                .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope_b, "model-a")
                .expect("vectors b")
                .len(),
            1
        );
        assert_eq!(
            fixture
                .repository
                .index_status_scoped(SourceKind::AgentMemory, &RetrievalScope::GlobalMemory)
                .expect("memory status")
                .pending,
            1
        );
    }

    #[test]
    fn upsert_is_idempotent_and_refreshes_content_and_hash() {
        let fixture = Fixture::new("retrieval upsert idempotent");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm"))
            .expect("first");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses cargo"))
            .expect("second");

        let indexed = fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list");
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].0, "m1");
        assert_eq!(indexed[0].1, content_hash("uses cargo"));
    }

    #[test]
    fn upserting_unchanged_content_preserves_index_state_and_failure_bookkeeping() {
        let fixture = Fixture::new("retrieval upsert preserves state");
        let unchanged = document("m1", "a", "", "uses npm");
        fixture
            .repository
            .upsert_pending(&unchanged)
            .expect("first upsert");
        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "model-a",
                &[1.0, 0.0],
            )
            .expect("store");

        // 内容没变的重复 reconcile 不能把已索引行打回 pending——否则每一轮轮询都会重烧
        // 一次 embedding 配额。
        fixture
            .repository
            .upsert_pending(&unchanged)
            .expect("second upsert");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(
            status.indexed, 1,
            "an unchanged upsert must not reset index_state"
        );
        assert_eq!(status.pending, 0);
        assert_eq!(
            fixture
                .repository
                .vector_candidates(SourceKind::AgentMemory, "model-a")
                .expect("candidates")
                .len(),
            1,
            "the stored embedding must survive an unchanged upsert"
        );
    }

    #[test]
    fn storing_an_embedding_marks_the_row_indexed_and_clears_failure_state() {
        let fixture = Fixture::new("retrieval store embedding");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm"))
            .expect("upsert");
        fixture
            .repository
            .record_failure(
                &document_id(SourceKind::AgentMemory, "m1"),
                FailureCategory::Network,
                false,
            )
            .expect("failure");

        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "model-a",
                &[1.0, 0.0],
            )
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(SourceKind::AgentMemory, "model-a")
            .expect("candidates");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "m1");
        assert_eq!(candidates[0].1, vec![1.0, 0.0]);

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.indexed, 1);
        assert_eq!(
            status.last_failure_category, None,
            "store_embedding must clear the failure category"
        );
        assert_eq!(
            fixture.attempt_count("m1"),
            0,
            "store_embedding must reset the attempt count"
        );
    }

    #[test]
    fn vector_candidates_exclude_rows_embedded_with_a_different_model() {
        let fixture = Fixture::new("retrieval model mismatch");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "x"))
            .expect("upsert");
        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "old-model",
                &[1.0, 0.0],
            )
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(SourceKind::AgentMemory, "new-model")
            .expect("candidates");
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidates_span_every_agent_and_every_folder() {
        // 这条测试原先断言的是相反的事（按 agent + folder 隔离）。`agent-memory-shared-pool`
        // （迁移 42）把记忆改成主机级共享池：每条记忆都会被注入进**每个** Agent 的系统提示词。
        // 再按 scope 过滤召回，只会让 `recall` 返回模型已经看得到的内容的一个真子集——
        // 换个 Agent 存下的记忆能被注入却搜不到，这不是安全边界，是自相矛盾。
        let fixture = Fixture::new("retrieval shared pool");
        for (source_id, agent, folder) in [
            ("m1", "a", "D:/one"),
            ("m2", "a", "D:/two"),
            ("m3", "b", "D:/one"),
        ] {
            fixture
                .repository
                .upsert_pending(&document(source_id, agent, folder, "shared content"))
                .expect("upsert");
            fixture
                .repository
                .store_embedding(
                    &document_id(SourceKind::AgentMemory, source_id),
                    "m",
                    &[1.0, 0.0],
                )
                .expect("store");
        }

        let mut vectors = fixture
            .repository
            .vector_candidates(SourceKind::AgentMemory, "m")
            .expect("vectors")
            .into_iter()
            .map(|(id, _)| id)
            .collect::<Vec<_>>();
        vectors.sort();
        let mut keywords = fixture
            .repository
            .keyword_candidates(SourceKind::AgentMemory, "\"shared\"", 10)
            .expect("keywords");
        keywords.sort();

        assert_eq!(vectors, vec!["m1", "m2", "m3"]);
        assert_eq!(keywords, vec!["m1", "m2", "m3"]);
    }

    #[test]
    fn keyword_candidates_find_pending_rows_because_fts_does_not_wait_for_the_worker() {
        let fixture = Fixture::new("retrieval keyword pending");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm not pnpm"))
            .expect("upsert");

        let hits = fixture
            .repository
            .keyword_candidates(SourceKind::AgentMemory, "\"pnpm\"", 10)
            .expect("keywords");
        assert_eq!(hits, vec!["m1".to_string()]);
    }

    #[test]
    fn claim_pending_batch_respects_its_limit_and_skips_indexed_rows() {
        let fixture = Fixture::new("retrieval claim batch");
        for source_id in ["m1", "m2", "m3"] {
            fixture
                .repository
                .upsert_pending(&document(source_id, "a", "", "content"))
                .expect("upsert");
        }
        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "m", &[1.0])
            .expect("store");

        let batch = fixture
            .repository
            .claim_pending_batch(SourceKind::AgentMemory, 2)
            .expect("batch");
        assert_eq!(batch.len(), 2);
        assert!(batch.iter().all(|document| document.source_id != "m1"));
    }

    #[test]
    fn giving_up_marks_failed_while_a_retryable_failure_stays_pending() {
        let fixture = Fixture::new("retrieval failure states");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "x"))
            .expect("m1");
        fixture
            .repository
            .upsert_pending(&document("m2", "a", "", "x"))
            .expect("m2");

        fixture
            .repository
            .record_failure(
                &document_id(SourceKind::AgentMemory, "m1"),
                FailureCategory::Auth,
                true,
            )
            .expect("give up");
        fixture
            .repository
            .record_failure(
                &document_id(SourceKind::AgentMemory, "m2"),
                FailureCategory::Network,
                false,
            )
            .expect("retry later");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.failed, 1);
        assert_eq!(status.pending, 1);
        assert_eq!(status.indexed, 0);
        // m2 的 record_failure 严格晚于 m1 执行，updated_at 更新——"most recently updated"
        // 语义下胜出的确实是 m2 的 "network"，不是 m1 的 "auth"。
        assert_eq!(status.last_failure_category.as_deref(), Some("network"));
    }

    #[test]
    fn requeue_all_resets_failures_and_attempt_counts() {
        let fixture = Fixture::new("retrieval requeue");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "x"))
            .expect("upsert");
        fixture
            .repository
            .record_failure(
                &document_id(SourceKind::AgentMemory, "m1"),
                FailureCategory::Auth,
                true,
            )
            .expect("failure");
        assert_eq!(
            fixture.attempt_count("m1"),
            1,
            "the failure must have counted an attempt"
        );

        fixture.repository.requeue_all().expect("requeue");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.failed, 0);
        assert_eq!(status.pending, 1);
        assert_eq!(status.last_failure_category, None);
        assert_eq!(
            fixture.attempt_count("m1"),
            0,
            "requeue_all must reset the attempt count"
        );
    }

    #[test]
    fn requeue_stale_model_returns_rows_to_pending_until_they_are_re_embedded() {
        // delta spec "Vector recall only compares same-model embeddings" 的第三条要求。
        // 没有这一步，换模型后旧行既被 `vector_candidates` 的 `embedding_model = ?` 滤掉，
        // 又因为 reconcile 只认内容哈希变化而永远不会被重新 embedding——向量召回永久归零，
        // 状态页却报告 indexed=N / pending=0 / failed=0，看不出任何异常。
        let fixture = Fixture::new("retrieval requeue stale model");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm"))
            .expect("upsert");
        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "model-a",
                &[1.0, 0.0],
            )
            .expect("store under model-a");

        fixture
            .repository
            .requeue_stale_model("model-b")
            .expect("requeue");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.pending, 1, "the stale row must be queued again");
        assert_eq!(status.indexed, 0);
        assert!(
            fixture
                .repository
                .vector_candidates(SourceKind::AgentMemory, "model-b")
                .expect("candidates")
                .is_empty(),
            "a requeued row must not be recallable under the new model until it is re-embedded"
        );
        assert!(
            fixture
                .repository
                .claim_pending_batch(SourceKind::AgentMemory, 10)
                .expect("batch")
                .iter()
                .any(|document| document.source_id == "m1"),
            "the worker must be able to claim the requeued row"
        );

        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "model-b",
                &[0.0, 1.0],
            )
            .expect("store under model-b");

        assert_eq!(
            fixture
                .repository
                .vector_candidates(SourceKind::AgentMemory, "model-b")
                .expect("candidates")
                .len(),
            1,
            "re-embedding under the new model must restore vector recall"
        );
    }

    #[test]
    fn requeue_stale_model_leaves_rows_already_on_the_configured_model_alone() {
        // 与上一条成对：保存配置时模型没变（换的是来源 Profile，或只是重复保存同一份配置）
        // 就不能把整张索引打回 pending——那等于每次点保存都重烧一遍全部 embedding 配额。
        let fixture = Fixture::new("retrieval requeue stale model noop");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm"))
            .expect("upsert");
        fixture
            .repository
            .store_embedding(
                &document_id(SourceKind::AgentMemory, "m1"),
                "model-a",
                &[1.0, 0.0],
            )
            .expect("store");

        fixture
            .repository
            .requeue_stale_model("model-a")
            .expect("requeue");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.indexed, 1);
        assert_eq!(status.pending, 0);
    }

    #[test]
    fn requeue_stale_model_does_not_revive_rows_that_gave_up() {
        // `failed` 是达到重试上限后的终态，只有用户显式重建才该复活它；换模型顺手把它们
        // 拉回 pending 会让一批确定性失败（例如 auth）的行在每次换模型时重新烧一轮重试。
        let fixture = Fixture::new("retrieval requeue stale model failed");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "x"))
            .expect("upsert");
        fixture
            .repository
            .record_failure(
                &document_id(SourceKind::AgentMemory, "m1"),
                FailureCategory::Auth,
                true,
            )
            .expect("give up");

        fixture
            .repository
            .requeue_stale_model("model-b")
            .expect("requeue");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.failed, 1);
        assert_eq!(status.pending, 0);
    }

    #[test]
    fn index_status_and_requeue_all_span_every_agent_and_folder() {
        // 检索配置是全局单例、`is_configured()` 不分 agent、索引源快照也覆盖全部 agent，
        // 所以状态与重建同样是全局的。按 agent 过滤会让非 OnePiece agent 的行既不出现在
        // 唯一的状态面板里，也无法被唯一的重建按钮救回来。
        let fixture = Fixture::new("retrieval status all agents");
        for (source_id, agent, folder) in [
            ("m1", "a", "D:/one"),
            ("m2", "a", "D:/two"),
            ("m3", "b", "D:/one"),
        ] {
            fixture
                .repository
                .upsert_pending(&document(source_id, agent, folder, "x"))
                .expect("upsert");
            fixture
                .repository
                .store_embedding(
                    &document_id(SourceKind::AgentMemory, source_id),
                    "model-a",
                    &[1.0, 0.0],
                )
                .expect("store");
        }

        assert_eq!(
            fixture.repository.index_status().expect("status").indexed,
            3
        );

        fixture.repository.requeue_all().expect("requeue");

        let status = fixture.repository.index_status().expect("status");
        assert_eq!(status.pending, 3);
        assert_eq!(status.indexed, 0);
    }

    #[test]
    fn delete_by_source_removes_the_row_and_its_fts_entry() {
        let fixture = Fixture::new("retrieval delete");
        fixture
            .repository
            .upsert_pending(&document("m1", "a", "", "uses npm"))
            .expect("upsert");

        fixture
            .repository
            .delete_by_source(SourceKind::AgentMemory, "m1")
            .expect("delete");

        let keywords = fixture
            .repository
            .keyword_candidates(SourceKind::AgentMemory, "\"npm\"", 10)
            .expect("keywords");
        assert!(keywords.is_empty());
        assert!(fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list")
            .is_empty());
    }
}
