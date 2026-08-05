use crate::contexts::retrieval::application::{RetrievalDocumentRepository, RetrievalIndexStatus};
use crate::contexts::retrieval::domain::{
    decode_embedding, encode_embedding, FailureCategory, IndexState, RetrievalDocument,
    RetrievalError, RetrievalScope, SourceKind,
};
use crate::platform::clock::SystemClock;
use crate::platform::database::{DatabaseError, NativeDatabase};
use rusqlite::{params, Row};

// Task 12 的 bootstrap 装配会构造它并把它交给 RetrievalApi；届时移除本属性。
#[allow(dead_code)]
#[derive(Clone)]
pub(crate) struct SqliteRetrievalDocumentRepository {
    database: NativeDatabase,
}

// 同上，随 SqliteRetrievalDocumentRepository 一起在 Task 12 移除。
#[allow(dead_code)]
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

    fn list_indexed_source_ids(
        &self,
        source_kind: SourceKind,
    ) -> Result<Vec<(String, String)>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare("SELECT source_id, content_hash FROM retrieval_documents WHERE source_kind = ?1")
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
            .query_map(params![source_kind.as_str(), limit as i64], DocumentRow::read)
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

    fn vector_candidates(
        &self,
        scope: &RetrievalScope,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let mut statement = connection
            .prepare(
                r#"
                SELECT source_id, embedding FROM retrieval_documents
                WHERE source_kind = ?1 AND scope_agent_id = ?2 AND scope_folder = ?3
                  AND index_state = 'indexed' AND embedding_model = ?4 AND embedding IS NOT NULL
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![source_kind.as_str(), scope.agent_id, scope.folder, model],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Vec<u8>>(1)?)),
            )
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

    fn keyword_candidates(
        &self,
        scope: &RetrievalScope,
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
                  AND d.source_kind = ?2 AND d.scope_agent_id = ?3 AND d.scope_folder = ?4
                ORDER BY bm25(retrieval_documents_fts)
                LIMIT ?5
                "#,
            )
            .map_err(storage_error)?;
        let rows = statement
            .query_map(
                params![query, source_kind.as_str(), scope.agent_id, scope.folder, limit as i64],
                |row| row.get::<_, String>(0),
            )
            .map_err(storage_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(storage_error)?;
        Ok(rows)
    }

    fn index_status(&self, agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        connection
            .query_row(
                r#"
                SELECT
                  SUM(index_state = 'indexed'), SUM(index_state = 'pending'), SUM(index_state = 'failed'),
                  (SELECT failure_category FROM retrieval_documents
                   WHERE scope_agent_id = ?1 AND failure_category IS NOT NULL
                   ORDER BY updated_at DESC LIMIT 1)
                FROM retrieval_documents WHERE scope_agent_id = ?1
                "#,
                params![agent_id],
                |row| {
                    // 未建过任何索引行的 agent 落进这条 SELECT 时，SUM() 在零行上聚合返回 NULL
                    // （SQLite 的空集合聚合语义），不是 0——必须按 Option 读，否则会在全新 agent
                    // 上把这条本该是"状态全零"的查询错误地变成一个 storage 错误。
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

    fn requeue_all(&self, agent_id: &str) -> Result<(), RetrievalError> {
        let connection = self.database.connection().map_err(database_error)?;
        let now = SystemClock.rfc3339();
        connection
            .execute(
                r#"
                UPDATE retrieval_documents
                SET index_state = 'pending', attempt_count = 0, failure_category = NULL, updated_at = ?2
                WHERE scope_agent_id = ?1
                "#,
                params![agent_id, now],
            )
            .map_err(storage_error)?;
        Ok(())
    }
}

// 唯一调用方是 claim_pending_batch，而它本身要到 Task 8 的索引 worker 调用批处理时才可达；
// 届时随那条调用链一起移除本属性。
#[allow(dead_code)]
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

// 同上，read()/into_document() 随 claim_pending_batch 在 Task 8 变为可达；届时移除本属性。
#[allow(dead_code)]
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

// 被本文件中的全部 10 个 trait 方法调用，但这些方法本身要到 Task 7 的差集协调服务开始调用
// upsert_pending/list_indexed_source_ids/delete_by_source 才可达；届时移除本属性。
#[allow(dead_code)]
fn database_error(error: DatabaseError) -> RetrievalError {
    match error {
        DatabaseError::Database(error) => storage_error(error),
        DatabaseError::Storage(message) => RetrievalError::Storage(message),
    }
}

// 同上，随 database_error 一起在 Task 7 移除。
#[allow(dead_code)]
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
        repository: SqliteRetrievalDocumentRepository,
    }

    impl Fixture {
        fn new(label: &str) -> Self {
            let directory = TempDirectory::new(label);
            let database =
                NativeDatabase::new(directory.path().to_path_buf()).expect("test database");
            Self {
                repository: SqliteRetrievalDocumentRepository::new(database),
                _directory: directory,
            }
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

    fn scope(agent: &str, folder: &str) -> RetrievalScope {
        RetrievalScope { agent_id: agent.to_string(), folder: folder.to_string() }
    }

    #[test]
    fn upsert_is_idempotent_and_refreshes_content_and_hash() {
        let fixture = Fixture::new("retrieval upsert idempotent");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("first");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses cargo")).expect("second");

        let indexed = fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list");
        assert_eq!(indexed.len(), 1);
        assert_eq!(indexed[0].0, "m1");
        assert_eq!(indexed[0].1, content_hash("uses cargo"));
    }

    #[test]
    fn storing_an_embedding_marks_the_row_indexed_and_clears_failure_state() {
        let fixture = Fixture::new("retrieval store embedding");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("upsert");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Network, false)
            .expect("failure");

        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "model-a", &[1.0, 0.0])
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(&scope("a", ""), SourceKind::AgentMemory, "model-a")
            .expect("candidates");
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "m1");
        assert_eq!(candidates[0].1, vec![1.0, 0.0]);
    }

    #[test]
    fn vector_candidates_exclude_rows_embedded_with_a_different_model() {
        let fixture = Fixture::new("retrieval model mismatch");
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("upsert");
        fixture
            .repository
            .store_embedding(&document_id(SourceKind::AgentMemory, "m1"), "old-model", &[1.0, 0.0])
            .expect("store");

        let candidates = fixture
            .repository
            .vector_candidates(&scope("a", ""), SourceKind::AgentMemory, "new-model")
            .expect("candidates");
        assert!(candidates.is_empty());
    }

    #[test]
    fn candidates_never_cross_agent_or_folder_boundaries() {
        let fixture = Fixture::new("retrieval scope isolation");
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
                .store_embedding(&document_id(SourceKind::AgentMemory, source_id), "m", &[1.0, 0.0])
                .expect("store");
        }

        let vectors = fixture
            .repository
            .vector_candidates(&scope("a", "D:/one"), SourceKind::AgentMemory, "m")
            .expect("vectors");
        let keywords = fixture
            .repository
            .keyword_candidates(&scope("a", "D:/one"), SourceKind::AgentMemory, "\"shared\"", 10)
            .expect("keywords");

        assert_eq!(vectors.iter().map(|(id, _)| id.as_str()).collect::<Vec<_>>(), vec!["m1"]);
        assert_eq!(keywords, vec!["m1".to_string()]);
    }

    #[test]
    fn keyword_candidates_find_pending_rows_because_fts_does_not_wait_for_the_worker() {
        let fixture = Fixture::new("retrieval keyword pending");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm not pnpm")).expect("upsert");

        let hits = fixture
            .repository
            .keyword_candidates(&scope("a", ""), SourceKind::AgentMemory, "\"pnpm\"", 10)
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
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("m1");
        fixture.repository.upsert_pending(&document("m2", "a", "", "x")).expect("m2");

        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Auth, true)
            .expect("give up");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m2"), FailureCategory::Network, false)
            .expect("retry later");

        let status = fixture.repository.index_status("a").expect("status");
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
        fixture.repository.upsert_pending(&document("m1", "a", "", "x")).expect("upsert");
        fixture
            .repository
            .record_failure(&document_id(SourceKind::AgentMemory, "m1"), FailureCategory::Auth, true)
            .expect("failure");

        fixture.repository.requeue_all("a").expect("requeue");

        let status = fixture.repository.index_status("a").expect("status");
        assert_eq!(status.failed, 0);
        assert_eq!(status.pending, 1);
        assert_eq!(status.last_failure_category, None);
    }

    #[test]
    fn index_status_spans_every_folder_of_the_agent() {
        let fixture = Fixture::new("retrieval status folders");
        fixture.repository.upsert_pending(&document("m1", "a", "D:/one", "x")).expect("m1");
        fixture.repository.upsert_pending(&document("m2", "a", "D:/two", "x")).expect("m2");
        fixture.repository.upsert_pending(&document("m3", "b", "D:/one", "x")).expect("m3");

        let status = fixture.repository.index_status("a").expect("status");
        assert_eq!(status.pending, 2);
    }

    #[test]
    fn delete_by_source_removes_the_row_and_its_fts_entry() {
        let fixture = Fixture::new("retrieval delete");
        fixture.repository.upsert_pending(&document("m1", "a", "", "uses npm")).expect("upsert");

        fixture
            .repository
            .delete_by_source(SourceKind::AgentMemory, "m1")
            .expect("delete");

        let keywords = fixture
            .repository
            .keyword_candidates(&scope("a", ""), SourceKind::AgentMemory, "\"npm\"", 10)
            .expect("keywords");
        assert!(keywords.is_empty());
        assert!(fixture
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)
            .expect("list")
            .is_empty());
    }
}
