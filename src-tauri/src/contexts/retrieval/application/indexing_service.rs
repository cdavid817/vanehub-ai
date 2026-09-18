use crate::contexts::retrieval::domain::{
    content_hash, document_id, FailureCategory, IndexState, RetrievalDocument, RetrievalError,
    RetrievalScope, SourceKind,
};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::ports::{EmbeddingEgressGuardPort, EmbeddingPort, RetrievalDocumentRepository};
// EmbeddingFailure 只在测试里的 FakeEmbedder::embed 构造它（Err 分支）；非测试代码只经
// EmbeddingPort::embed 的返回类型隐式用到它，从不在本文件里显式写出它的名字。同上，这个
// import 不会有"届时移除"的那一天。
#[allow(unused_imports)]
use super::ports::EmbeddingFailure;
// 同理，RetrievalIndexStatus 只出现在测试里 FakeRepository::index_status 的签名
// （unimplemented!()）。真正调用方是 api.rs 的 RetrievalApi，经仓储直接读，不经过本文件。
#[allow(unused_imports)]
use super::ports::RetrievalIndexStatus;

/// 可调常量集中在此，不散落调用点（设计文档 §5.2）。
pub(crate) const EMBEDDING_BATCH_SIZE: usize = 32;
/// 后台 worker 两轮之间的兜底轮询周期：唤醒信号丢失时，最多延迟一个周期。
pub(crate) const RECONCILE_POLL_INTERVAL_SECONDS: u64 = 300;
pub(crate) const MAX_EMBEDDING_ATTEMPTS: u32 = 5;
/// 后台 worker 两次重试之间的退避表。`process_pending_batch` 只判定单次批处理是否 give_up，
/// 不负责调度下一次尝试的时间点——那是调用方的事。
pub(crate) const RETRY_BACKOFF_SECONDS: [u64; 5] = [1, 4, 15, 60, 300];
/// 超长内容 embedding 前截断；FTS 仍索引全文，所以长记忆的尾部仍可被关键词命中。
pub(crate) const EMBEDDING_CONTENT_LIMIT: usize = 8000;

/// retrieval 从源上下文取记录的消费侧契约。第 1 期唯一实现是 bootstrap 里的记忆表适配器。
pub(crate) trait IndexSourcePort: Send + Sync {
    /// 全量快照。`reconcile` 需要全局视图才能判定孤儿行，所以这一个方法必须是全量的。
    ///
    /// Maintenance only. There is deliberately no per-id fetch here any more: a search hit is
    /// resolved through the owning context's governed read under the caller's trusted context
    /// (`AuthorizedHitResolverPort`), never through the index source's own unscoped view.
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError>;
}

/// The default egress guard: every claimed row may be dispatched. Correct for sources whose
/// bodies carry no per-record egress restriction (workspace code behind an explicit confirmation).
struct PermitAllEgress;

impl EmbeddingEgressGuardPort for PermitAllEgress {
    fn permitted(&self, documents: &[RetrievalDocument]) -> Vec<bool> {
        vec![true; documents.len()]
    }
}

pub(crate) trait IndexGenerationGuard: Send + Sync {
    fn is_current(&self, scope: &RetrievalScope) -> Result<bool, RetrievalError>;
}

struct AlwaysCurrentGeneration;

impl IndexGenerationGuard for AlwaysCurrentGeneration {
    fn is_current(&self, _scope: &RetrievalScope) -> Result<bool, RetrievalError> {
        Ok(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexSourceRecord {
    pub(crate) source_id: String,
    pub(crate) agent_id: String,
    pub(crate) folder: String,
    pub(crate) content: String,
    /// 检索结果要带上它（Task 9 的 `ScoredHit.created_at`），且它只存在于源表——
    /// 索引行刻意不复制这个字段，避免又多一处会陈旧的副本。
    pub(crate) created_at: String,
    /// Whether the body may be sent to a remote embedder. Decided by the owning source; a
    /// restricted record enters the local keyword index and nothing else.
    pub(crate) egress_restricted: bool,
}

pub(crate) struct IndexingService {
    repository: Arc<dyn RetrievalDocumentRepository>,
    source: Arc<dyn IndexSourcePort>,
    embeddings: Arc<dyn EmbeddingPort>,
    source_kind: SourceKind,
    scope: RetrievalScope,
    generation_guard: Arc<dyn IndexGenerationGuard>,
    egress: Arc<dyn EmbeddingEgressGuardPort>,
}

impl IndexingService {
    pub(crate) fn new(
        repository: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
        embeddings: Arc<dyn EmbeddingPort>,
    ) -> Self {
        // This constructor fixes the scope/kind pair rather than accepting one, and
        // `validate_for` accepts exactly this combination, so there is no input to reject and
        // no error a caller could act on. `new_scoped` stays the checked entry point for
        // callers that choose the pair themselves.
        debug_assert!(RetrievalScope::GlobalMemory
            .validate_for(SourceKind::AgentMemory)
            .is_ok());
        Self {
            repository,
            source,
            embeddings,
            source_kind: SourceKind::AgentMemory,
            scope: RetrievalScope::GlobalMemory,
            generation_guard: Arc::new(AlwaysCurrentGeneration),
            egress: Arc::new(PermitAllEgress),
        }
    }

    /// Installs the dispatch-time egress check. Production agent-memory indexing always sets
    /// one; the permissive default exists for sources without per-record restrictions.
    pub(crate) fn with_egress_guard(mut self, egress: Arc<dyn EmbeddingEgressGuardPort>) -> Self {
        self.egress = egress;
        self
    }

    /// The checked constructor for a caller-chosen scope/kind pair with the default generation
    /// guard. Production reaches for `new_scoped_with_guard` directly; this wrapper was
    /// previously kept alive only by `new` delegating to it, which is also where the panic
    /// shortcut lived.
    #[allow(dead_code)]
    pub(crate) fn new_scoped(
        repository: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
        embeddings: Arc<dyn EmbeddingPort>,
        source_kind: SourceKind,
        scope: RetrievalScope,
    ) -> Result<Self, RetrievalError> {
        Self::new_scoped_with_guard(
            repository,
            source,
            embeddings,
            source_kind,
            scope,
            Arc::new(AlwaysCurrentGeneration),
        )
    }

    pub(crate) fn new_scoped_with_guard(
        repository: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
        embeddings: Arc<dyn EmbeddingPort>,
        source_kind: SourceKind,
        scope: RetrievalScope,
        generation_guard: Arc<dyn IndexGenerationGuard>,
    ) -> Result<Self, RetrievalError> {
        scope.validate_for(source_kind)?;
        Ok(Self {
            repository,
            source,
            embeddings,
            source_kind,
            scope,
            generation_guard,
            egress: Arc::new(PermitAllEgress),
        })
    }

    /// 索引的真源是这一次差集协调，而**不是**保存路径上的双写。
    ///
    /// 在 `SqliteAgentMemoryRepository::save` 里顺手插一条索引行看似更简单，但那会引入
    /// "入队写失败 → 该记忆永远搜不到"的静默漏洞：保存成功了，用户以为记住了，检索却永远看不见。
    /// 协调式的代价只是最多延迟一个周期，而且顺带把历史存量记忆回填掉，不需要单独的数据迁移脚本。
    pub(crate) fn reconcile(&self) -> Result<ReconcileOutcome, RetrievalError> {
        let records = self.source.snapshot()?;
        let existing: HashMap<String, (String, bool)> = match self.scope {
            // The scoped listing predates the restriction column and answers for one workspace;
            // the host-wide memory pool reads the richer form so a restriction change alone is
            // enough to re-queue or retire a row.
            RetrievalScope::GlobalMemory => self
                .repository
                .list_indexed_source_states(self.source_kind)?
                .into_iter()
                .map(|(source_id, hash, restricted)| (source_id, (hash, restricted)))
                .collect(),
            RetrievalScope::Workspace(_) => self
                .repository
                .list_indexed_source_ids_scoped(self.source_kind, &self.scope)?
                .into_iter()
                .map(|(source_id, hash)| (source_id, (hash, false)))
                .collect(),
        };

        let mut outcome = ReconcileOutcome::default();
        let mut live: HashSet<&str> = HashSet::new();
        let mut upserts: Vec<RetrievalDocument> = Vec::new();
        for record in &records {
            live.insert(record.source_id.as_str());
            let hash = content_hash(&record.content);
            match existing.get(&record.source_id) {
                Some((existing_hash, restricted))
                    if existing_hash == &hash && *restricted == record.egress_restricted =>
                {
                    continue
                }
                Some(_) => outcome.invalidated += 1,
                None => outcome.added += 1,
            }
            upserts.push(RetrievalDocument {
                id: document_id(self.source_kind, &record.source_id),
                source_kind: self.source_kind,
                source_id: record.source_id.clone(),
                scope_agent_id: record.agent_id.clone(),
                scope_folder: record.folder.clone(),
                content: record.content.clone(),
                content_hash: hash,
                index_state: if record.egress_restricted {
                    IndexState::KeywordOnly
                } else {
                    IndexState::Pending
                },
                attempt_count: 0,
                embedding_model: None,
                egress_restricted: record.egress_restricted,
            });
        }

        // 孤儿清理是 §5.3 显式撤销失败时的兜底。少了它，一次失败的撤销调用会让索引行永久残留。
        let orphans: Vec<String> = existing
            .keys()
            .filter(|source_id| !live.contains(source_id.as_str()))
            .cloned()
            .collect();
        outcome.orphans_removed += orphans.len();
        // Apply the whole diff (upserts + orphan deletes) in one repository transaction so a
        // full reconcile pays one fsync instead of one per row. `list_indexed_source_ids_scoped`
        // already filtered `existing` to this scope, so the unscoped `reconcile_apply` operates
        // on exactly the rows we compared against.
        self.repository
            .reconcile_apply(&upserts, &orphans, self.source_kind)?;
        Ok(outcome)
    }

    /// 一次只处理一批（`EMBEDDING_BATCH_SIZE` 条），成功与失败都是终态——调用方（后台 worker
    /// 循环）负责按 `RECONCILE_POLL_INTERVAL_SECONDS` 与 `RETRY_BACKOFF_SECONDS` 的节奏反复调用
    /// 它，不在这里睡眠等待。
    pub(crate) fn process_pending_batch(
        &self,
        model: &str,
    ) -> Result<BatchOutcome, RetrievalError> {
        self.process_pending_batch_with_identity(model, model)
    }

    pub(crate) fn process_pending_batch_with_identity(
        &self,
        request_model: &str,
        embedding_identity: &str,
    ) -> Result<BatchOutcome, RetrievalError> {
        if !self.generation_guard.is_current(&self.scope)? {
            return Ok(BatchOutcome::default());
        }
        let batch = self.repository.claim_pending_batch_scoped(
            self.source_kind,
            &self.scope,
            EMBEDDING_BATCH_SIZE,
        )?;
        if batch.is_empty() {
            return Ok(BatchOutcome::default());
        }
        // Re-decided from the authoritative record at dispatch time, not from the row: a public
        // record that was restricted after it queued must not leave the machine because an older
        // reconcile once said it could. Blocked rows retire to keyword-only rather than counting
        // as failures, so they neither retry forever nor read as completed vectors.
        let permitted = self.egress.permitted(&batch);
        let mut blocked = 0usize;
        let mut dispatch = Vec::with_capacity(batch.len());
        for (index, document) in batch.into_iter().enumerate() {
            let allowed =
                permitted.get(index).copied().unwrap_or(false) && !document.egress_restricted;
            if allowed {
                dispatch.push(document);
            } else {
                blocked += 1;
                self.repository.mark_keyword_only(&document.id)?;
            }
        }
        let batch = dispatch;
        if batch.is_empty() {
            return Ok(BatchOutcome {
                keyword_only: blocked,
                ..BatchOutcome::default()
            });
        }
        let inputs: Vec<String> = batch
            .iter()
            .map(|document| truncate_for_embedding(&document.content))
            .collect();

        let embedding_result = self.embeddings.embed(request_model, &inputs);
        if !self.generation_guard.is_current(&self.scope)? {
            return Ok(BatchOutcome::default());
        }
        match embedding_result {
            // 数量对不上说明 provider 的响应与请求不成对，不能靠位置把向量配给文档——
            // 错配的向量比没有向量更糟：它会安静地污染检索结果。
            Ok(vectors) if vectors.len() != batch.len() => {
                for document in &batch {
                    self.repository.record_failure(
                        &document.id,
                        FailureCategory::InvalidRequest,
                        true,
                    )?;
                }
                Ok(BatchOutcome {
                    succeeded: 0,
                    failed: batch.len(),
                    last_failure_category: Some(FailureCategory::InvalidRequest),
                    retry_after: None,
                    keyword_only: blocked,
                })
            }
            Ok(vectors) => {
                for (document, vector) in batch.iter().zip(vectors.iter()) {
                    self.repository
                        .store_embedding(&document.id, embedding_identity, vector)?;
                }
                Ok(BatchOutcome {
                    succeeded: batch.len(),
                    failed: 0,
                    last_failure_category: None,
                    retry_after: None,
                    keyword_only: blocked,
                })
            }
            Err(failure) => {
                for document in &batch {
                    let give_up = !failure.category.is_retryable()
                        || document.attempt_count + 1 >= MAX_EMBEDDING_ATTEMPTS;
                    self.repository
                        .record_failure(&document.id, failure.category, give_up)?;
                }
                Ok(BatchOutcome {
                    succeeded: 0,
                    failed: batch.len(),
                    last_failure_category: Some(failure.category),
                    retry_after: failure.retry_after,
                    keyword_only: blocked,
                })
            }
        }
    }
}

/// 按字符而非字节截断——按字节切会把多字节 UTF-8 字符劈成两半并 panic。
///
/// 检索路（`search_service.rs`）也用它截断 query：query 由模型自撰、没有天然长度约束，
/// 两侧用同一个上限才不会出现"能被索引、却无法被查询"的长度带。
pub(super) fn truncate_for_embedding(content: &str) -> String {
    content.chars().take(EMBEDDING_CONTENT_LIMIT).collect()
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReconcileOutcome {
    pub(crate) added: usize,
    pub(crate) invalidated: usize,
    pub(crate) orphans_removed: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct BatchOutcome {
    pub(crate) succeeded: usize,
    pub(crate) failed: usize,
    /// 只给类别，不带原始错误文本——设计文档 §8.2 只允许索引失败日志落盘错误类别，不允许
    /// provider 响应体或凭据经这条路径渗出。`None` 表示本批没有失败（全部成功，或本来就是空批）。
    pub(crate) last_failure_category: Option<FailureCategory>,
    pub(crate) retry_after: Option<std::time::Duration>,
    /// Rows this batch retired to the keyword-only index instead of dispatching. Neither a
    /// success nor a failure: the worker moves on without backing off.
    pub(crate) keyword_only: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    struct FakeSource {
        records: Vec<IndexSourceRecord>,
    }

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(self.records.clone())
        }
    }

    /// embed() 的两种可编排行为：测试只需要"这一批调用要么全成功要么按某个分类全失败"，
    /// 不需要逐条区分。
    enum EmbedBehavior {
        Succeed(Vec<Vec<f32>>),
        Fail(FailureCategory),
    }

    /// 记录每次 embed() 收到的 (model, inputs)，供批大小/截断相关断言使用。
    struct FakeEmbedder {
        behavior: EmbedBehavior,
        calls: Mutex<Vec<(String, Vec<String>)>>,
    }

    impl FakeEmbedder {
        fn succeeding(vectors: Vec<Vec<f32>>) -> Self {
            Self {
                behavior: EmbedBehavior::Succeed(vectors),
                calls: Mutex::new(Vec::new()),
            }
        }

        fn failing(category: FailureCategory) -> Self {
            Self {
                behavior: EmbedBehavior::Fail(category),
                calls: Mutex::new(Vec::new()),
            }
        }
    }

    impl EmbeddingPort for FakeEmbedder {
        fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
            self.calls
                .lock()
                .expect("lock")
                .push((model.to_string(), inputs.to_vec()));
            match &self.behavior {
                EmbedBehavior::Succeed(vectors) => Ok(vectors.clone()),
                EmbedBehavior::Fail(category) => Err(EmbeddingFailure {
                    category: *category,
                    retry_after: None,
                    message: "fake embedding failure".to_string(),
                }),
            }
        }
    }

    #[derive(Default)]
    struct FakeRepository {
        /// 已存在的索引行：(source_id, content_hash)
        rows: Vec<(String, String)>,
        /// Existing rows with their restriction flag; used instead of `rows` when non-empty.
        states: Vec<(String, String, bool)>,
        upserted: Mutex<Vec<String>>,
        upserted_documents: Mutex<Vec<RetrievalDocument>>,
        keyword_only: Mutex<Vec<String>>,
        deleted: Mutex<Vec<String>>,
        /// claim_pending_batch 要返回的待索引文档，测试按场景摆放。
        pending: Vec<RetrievalDocument>,
        /// 记录每次 claim_pending_batch 被调用时收到的 limit 参数。
        claim_limits: Mutex<Vec<usize>>,
        stored: Mutex<Vec<(String, String, Vec<f32>)>>,
        failures: Mutex<Vec<(String, FailureCategory, bool)>>,
        /// How many `reconcile_apply` calls fail before committing anything, standing in for a
        /// crash or a lost connection between computing the diff and writing it.
        apply_failures: AtomicUsize,
    }

    impl RetrievalDocumentRepository for FakeRepository {
        fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError> {
            self.upserted
                .lock()
                .expect("lock")
                .push(document.source_id.clone());
            self.upserted_documents
                .lock()
                .expect("lock")
                .push(document.clone());
            Ok(())
        }

        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            Ok(self.rows.clone())
        }

        fn list_indexed_source_states(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String, bool)>, RetrievalError> {
            if self.states.is_empty() {
                return Ok(self
                    .rows
                    .iter()
                    .map(|(id, hash)| (id.clone(), hash.clone(), false))
                    .collect());
            }
            Ok(self.states.clone())
        }

        fn mark_keyword_only(&self, id: &str) -> Result<(), RetrievalError> {
            self.keyword_only.lock().expect("lock").push(id.to_string());
            Ok(())
        }

        fn reconcile_apply(
            &self,
            upserts: &[RetrievalDocument],
            orphan_source_ids: &[String],
            source_kind: SourceKind,
        ) -> Result<(), RetrievalError> {
            if self.apply_failures.load(Ordering::SeqCst) > 0 {
                self.apply_failures.fetch_sub(1, Ordering::SeqCst);
                return Err(RetrievalError::Storage(
                    "interrupted before the diff committed".to_string(),
                ));
            }
            for document in upserts {
                self.upsert_pending(document)?;
            }
            for source_id in orphan_source_ids {
                self.delete_by_source(source_kind, source_id)?;
            }
            Ok(())
        }

        fn delete_by_source(
            &self,
            _source_kind: SourceKind,
            source_id: &str,
        ) -> Result<(), RetrievalError> {
            self.deleted
                .lock()
                .expect("lock")
                .push(source_id.to_string());
            Ok(())
        }

        // reconcile 用上面三个方法；process_pending_batch 用下面这三个。其余五个
        // （vector_candidates/keyword_candidates/index_status/requeue_all/requeue_stale_model）
        // 在本文件的测试里仍不可达，走 unimplemented!()。
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            self.claim_limits.lock().expect("lock").push(limit);
            Ok(self.pending.iter().take(limit).cloned().collect())
        }
        fn store_embedding(
            &self,
            id: &str,
            model: &str,
            embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            self.stored.lock().expect("lock").push((
                id.to_string(),
                model.to_string(),
                embedding.to_vec(),
            ));
            Ok(())
        }
        fn record_failure(
            &self,
            id: &str,
            category: FailureCategory,
            give_up: bool,
        ) -> Result<(), RetrievalError> {
            self.failures
                .lock()
                .expect("lock")
                .push((id.to_string(), category, give_up));
            Ok(())
        }
        fn vector_candidates(
            &self,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by indexing_service tests")
        }
        fn keyword_candidates(
            &self,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by indexing_service tests")
        }
        fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by indexing_service tests")
        }
        fn requeue_all(&self) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by indexing_service tests")
        }
        fn requeue_stale_model(&self, _new_model: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by indexing_service tests")
        }
    }

    const MODEL: &str = "test-embedding-model";

    fn record(source_id: &str, content: &str) -> IndexSourceRecord {
        IndexSourceRecord {
            source_id: source_id.to_string(),
            agent_id: "a".to_string(),
            folder: String::new(),
            content: content.to_string(),
            created_at: "2026-08-05T00:00:00Z".to_string(),
            egress_restricted: false,
        }
    }

    fn indexed(source_id: &str, content: &str) -> (String, String) {
        (source_id.to_string(), content_hash(content))
    }

    /// process_pending_batch 测试用的最小 RetrievalDocument：id/content/attempt_count 是唯一
    /// 被服务读取的字段，其余字段填占位值即可。
    fn pending_document(id: &str, content: &str, attempt_count: u32) -> RetrievalDocument {
        RetrievalDocument {
            id: id.to_string(),
            source_kind: SourceKind::AgentMemory,
            source_id: id.to_string(),
            scope_agent_id: "a".to_string(),
            scope_folder: String::new(),
            content: content.to_string(),
            content_hash: content_hash(content),
            index_state: IndexState::Pending,
            attempt_count,
            embedding_model: None,
            egress_restricted: false,
        }
    }

    /// 装配一个只依赖两个 fake 的服务，返回服务与仓储句柄以便事后断言调用记录。
    /// reconcile 从不触碰 embeddings，给一个成功但永不会被调用的 embedder 占位即可。
    fn service(
        records: Vec<IndexSourceRecord>,
        rows: Vec<(String, String)>,
    ) -> (IndexingService, Arc<FakeRepository>) {
        let repository = Arc::new(FakeRepository {
            rows,
            ..FakeRepository::default()
        });
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource { records }),
            Arc::new(FakeEmbedder::succeeding(Vec::new())),
        );
        (service, repository)
    }

    /// 装配一个用于 process_pending_batch 测试的服务；reconcile 侧的 source 用不到，
    /// 给个空快照的 FakeSource 占位。返回三个句柄，方便测试分别断言仓储调用记录与
    /// embedder 收到的 (model, inputs)。
    fn batch_service(
        pending: Vec<RetrievalDocument>,
        embedder: FakeEmbedder,
    ) -> (IndexingService, Arc<FakeRepository>, Arc<FakeEmbedder>) {
        let repository = Arc::new(FakeRepository {
            pending,
            ..FakeRepository::default()
        });
        let embedder = Arc::new(embedder);
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource {
                records: Vec::new(),
            }),
            embedder.clone(),
        );
        (service, repository, embedder)
    }

    /// Blocks every document whose source id is in its list; records what it was asked.
    struct DenyingEgress {
        blocked: Vec<&'static str>,
        asked: Mutex<Vec<Vec<String>>>,
    }

    impl EmbeddingEgressGuardPort for DenyingEgress {
        fn permitted(&self, documents: &[RetrievalDocument]) -> Vec<bool> {
            self.asked.lock().expect("lock").push(
                documents
                    .iter()
                    .map(|document| document.source_id.clone())
                    .collect(),
            );
            documents
                .iter()
                .map(|document| !self.blocked.contains(&document.source_id.as_str()))
                .collect()
        }
    }

    #[test]
    fn a_body_the_authoritative_record_no_longer_permits_is_retired_before_dispatch_not_embedded() {
        // MR-23: the queue row still says public; the guard, answering from the authoritative
        // record at dispatch time, says restricted. The body must not reach the embedder, and the
        // row must retire to keyword-only rather than count as a failure to retry.
        let repository = Arc::new(FakeRepository {
            pending: vec![
                pending_document("public", "uses npm", 0),
                pending_document("restricted", "workspace secret", 0),
            ],
            ..FakeRepository::default()
        });
        let embedder = Arc::new(FakeEmbedder::succeeding(vec![vec![0.1]]));
        let guard = Arc::new(DenyingEgress {
            blocked: vec!["restricted"],
            asked: Mutex::new(Vec::new()),
        });
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource {
                records: Vec::new(),
            }),
            embedder.clone(),
        )
        .with_egress_guard(guard.clone());

        let outcome = service.process_pending_batch(MODEL).expect("batch");

        assert_eq!(outcome.succeeded, 1);
        assert_eq!(outcome.failed, 0);
        assert_eq!(outcome.keyword_only, 1);
        assert_eq!(
            *repository.keyword_only.lock().expect("lock"),
            vec!["restricted".to_string()]
        );
        let calls = embedder.calls.lock().expect("lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1, vec!["uses npm".to_string()]);
        assert!(repository.failures.lock().expect("lock").is_empty());
    }

    #[test]
    fn a_row_already_flagged_restricted_is_never_dispatched_even_when_the_guard_would_permit_it() {
        let repository = Arc::new(FakeRepository {
            pending: vec![RetrievalDocument {
                egress_restricted: true,
                ..pending_document("scoped", "workspace fact", 0)
            }],
            ..FakeRepository::default()
        });
        let embedder = Arc::new(FakeEmbedder::succeeding(vec![vec![0.1]]));
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource {
                records: Vec::new(),
            }),
            embedder.clone(),
        );

        let outcome = service.process_pending_batch(MODEL).expect("batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                keyword_only: 1,
                ..BatchOutcome::default()
            }
        );
        assert!(embedder.calls.lock().expect("lock").is_empty());
    }

    #[test]
    fn reconcile_queues_a_restricted_record_as_keyword_only_and_notices_a_restriction_change() {
        let mut restricted = record("scoped", "workspace fact");
        restricted.egress_restricted = true;
        let (service, repository) = service(
            vec![restricted, record("public", "shared fact")],
            vec![indexed("public", "shared fact")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 0);
        let upserted = repository.upserted_documents.lock().expect("lock");
        assert_eq!(upserted.len(), 1);
        assert_eq!(upserted[0].source_id, "scoped");
        assert_eq!(upserted[0].index_state, IndexState::KeywordOnly);
        assert!(upserted[0].egress_restricted);
        drop(upserted);

        // Same content, restriction lifted: the row is re-queued even though the hash is equal.
        let repository_rows = vec![("scoped".to_string(), content_hash("workspace fact"), true)];
        let repository = Arc::new(FakeRepository {
            states: repository_rows,
            ..FakeRepository::default()
        });
        let service = IndexingService::new(
            repository.clone(),
            Arc::new(FakeSource {
                records: vec![record("scoped", "workspace fact")],
            }),
            Arc::new(FakeEmbedder::succeeding(Vec::new())),
        );
        let outcome = service.reconcile().expect("reconcile");
        assert_eq!(outcome.invalidated, 1);
        let upserted = repository.upserted_documents.lock().expect("lock");
        assert_eq!(upserted[0].index_state, IndexState::Pending);
        assert!(!upserted[0].egress_restricted);
    }

    /// MR-24 / 7.2: a reconcile interrupted before its diff commits leaves nothing half-applied
    /// -- no orphan deleted early, no upsert lost -- and the next run recomputes the whole diff
    /// from the sources rather than resuming a partial one.
    #[test]
    fn an_interrupted_reconcile_applies_nothing_and_the_next_run_recomputes_the_whole_diff() {
        let (service, repository) = service(
            vec![
                record("kept", "same text"),
                record("changed", "new text"),
                record("added", "fresh text"),
            ],
            vec![
                indexed("kept", "same text"),
                indexed("changed", "old text"),
                indexed("orphan", "gone text"),
            ],
        );
        repository.apply_failures.store(1, Ordering::SeqCst);

        assert!(matches!(
            service.reconcile(),
            Err(RetrievalError::Storage(_))
        ));
        assert!(repository.upserted.lock().expect("lock").is_empty());
        assert!(repository.deleted.lock().expect("lock").is_empty());

        let outcome = service.reconcile().expect("the retried reconcile");
        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 1);
        assert_eq!(outcome.orphans_removed, 1);
        let mut upserted = repository.upserted.lock().expect("lock").clone();
        upserted.sort();
        assert_eq!(upserted, vec!["added".to_string(), "changed".to_string()]);
        assert_eq!(
            repository.deleted.lock().expect("lock").as_slice(),
            ["orphan".to_string()]
        );
    }

    struct FirstCheckOnlyGuard {
        checks: AtomicUsize,
    }

    impl IndexGenerationGuard for FirstCheckOnlyGuard {
        fn is_current(&self, _scope: &RetrievalScope) -> Result<bool, RetrievalError> {
            Ok(self.checks.fetch_add(1, Ordering::SeqCst) == 0)
        }
    }

    #[test]
    fn stale_generation_discards_an_in_flight_embedding_response() {
        let repository = Arc::new(FakeRepository {
            pending: vec![pending_document("d1", "code", 0)],
            ..FakeRepository::default()
        });
        let embedder = Arc::new(FakeEmbedder::succeeding(vec![vec![1.0, 2.0]]));
        let service = IndexingService::new_scoped_with_guard(
            repository.clone(),
            Arc::new(FakeSource {
                records: Vec::new(),
            }),
            embedder.clone(),
            SourceKind::AgentMemory,
            RetrievalScope::GlobalMemory,
            Arc::new(FirstCheckOnlyGuard {
                checks: AtomicUsize::new(0),
            }),
        )
        .expect("guarded service");

        assert_eq!(
            service.process_pending_batch(MODEL).expect("batch"),
            BatchOutcome::default()
        );
        assert_eq!(embedder.calls.lock().expect("calls").len(), 1);
        assert!(repository.stored.lock().expect("stored").is_empty());
        assert!(repository.failures.lock().expect("failures").is_empty());
    }

    #[test]
    fn a_source_record_with_no_index_row_is_added() {
        let (service, repository) = service(vec![record("m1", "uses npm")], Vec::new());

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 0);
        assert_eq!(outcome.orphans_removed, 0);
        assert_eq!(
            *repository.upserted.lock().expect("lock"),
            vec!["m1".to_string()]
        );
    }

    #[test]
    fn a_content_change_invalidates_the_existing_index_row() {
        let (service, repository) = service(
            vec![record("m1", "uses cargo")],
            vec![indexed("m1", "uses npm")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.invalidated, 1);
        assert_eq!(outcome.added, 0);
        assert_eq!(
            *repository.upserted.lock().expect("lock"),
            vec!["m1".to_string()]
        );
    }

    #[test]
    fn an_index_row_whose_source_disappeared_is_removed() {
        let (service, repository) = service(Vec::new(), vec![indexed("m1", "uses npm")]);

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.orphans_removed, 1);
        assert_eq!(
            *repository.deleted.lock().expect("lock"),
            vec!["m1".to_string()]
        );
    }

    #[test]
    fn an_unchanged_record_is_left_alone() {
        let (service, repository) = service(
            vec![record("m1", "uses npm")],
            vec![indexed("m1", "uses npm")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome, ReconcileOutcome::default());
        assert!(repository.upserted.lock().expect("lock").is_empty());
        assert!(repository.deleted.lock().expect("lock").is_empty());
    }

    #[test]
    fn all_three_kinds_of_work_are_handled_in_one_pass() {
        let (service, repository) = service(
            vec![record("m1", "new"), record("m2", "changed")],
            vec![indexed("m2", "original"), indexed("m3", "orphan")],
        );

        let outcome = service.reconcile().expect("reconcile");

        assert_eq!(outcome.added, 1);
        assert_eq!(outcome.invalidated, 1);
        assert_eq!(outcome.orphans_removed, 1);
        assert_eq!(
            *repository.deleted.lock().expect("lock"),
            vec!["m3".to_string()]
        );
    }

    #[test]
    fn a_successful_batch_stores_one_embedding_per_document() {
        let pending = vec![
            pending_document("m1", "uses npm", 0),
            pending_document("m2", "uses cargo", 0),
            pending_document("m3", "uses rustup", 0),
        ];
        let vectors = vec![vec![0.1, 0.2], vec![0.3, 0.4], vec![0.5, 0.6]];
        let (service, repository, _embedder) =
            batch_service(pending, FakeEmbedder::succeeding(vectors.clone()));

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 3,
                failed: 0,
                ..BatchOutcome::default()
            }
        );
        assert_eq!(
            *repository.stored.lock().expect("lock"),
            vec![
                ("m1".to_string(), MODEL.to_string(), vectors[0].clone()),
                ("m2".to_string(), MODEL.to_string(), vectors[1].clone()),
                ("m3".to_string(), MODEL.to_string(), vectors[2].clone()),
            ]
        );
        assert!(repository.failures.lock().expect("lock").is_empty());
    }

    #[test]
    fn an_auth_failure_gives_up_immediately_without_burning_quota() {
        // attempt_count 是 0——离 MAX_EMBEDDING_ATTEMPTS 还差得远，纯按次数算不该 give up。
        // Auth 是确定性失败：重试只会烧配额，所以无论次数都必须立刻放弃。
        let pending = vec![pending_document("m1", "uses npm", 0)];
        let (service, repository, _embedder) =
            batch_service(pending, FakeEmbedder::failing(FailureCategory::Auth));

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 0,
                failed: 1,
                last_failure_category: Some(FailureCategory::Auth),
                ..BatchOutcome::default()
            }
        );
        assert_eq!(
            *repository.failures.lock().expect("lock"),
            vec![("m1".to_string(), FailureCategory::Auth, true)]
        );
        assert!(repository.stored.lock().expect("lock").is_empty());
    }

    #[test]
    fn an_invalid_request_failure_gives_up_immediately() {
        let pending = vec![pending_document("m1", "uses npm", 0)];
        let (service, repository, _embedder) = batch_service(
            pending,
            FakeEmbedder::failing(FailureCategory::InvalidRequest),
        );

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 0,
                failed: 1,
                last_failure_category: Some(FailureCategory::InvalidRequest),
                ..BatchOutcome::default()
            }
        );
        assert_eq!(
            *repository.failures.lock().expect("lock"),
            vec![("m1".to_string(), FailureCategory::InvalidRequest, true)]
        );
    }

    #[test]
    fn a_network_failure_below_the_attempt_ceiling_stays_retryable() {
        // attempt_count 1 → 下一次是第 2 次尝试，远低于 MAX_EMBEDDING_ATTEMPTS，
        // 且 Network 是瞬时性失败，应保持可重试（give_up = false）。
        let pending = vec![pending_document("m1", "uses npm", 1)];
        let (service, repository, _embedder) =
            batch_service(pending, FakeEmbedder::failing(FailureCategory::Network));

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 0,
                failed: 1,
                last_failure_category: Some(FailureCategory::Network),
                ..BatchOutcome::default()
            }
        );
        assert_eq!(
            *repository.failures.lock().expect("lock"),
            vec![("m1".to_string(), FailureCategory::Network, false)]
        );
    }

    #[test]
    fn a_network_failure_at_the_attempt_ceiling_gives_up() {
        // attempt_count = MAX_EMBEDDING_ATTEMPTS - 1 → 这次失败后尝试数达到上限，
        // 即使 Network 本身可重试，也必须放弃，否则会无限重试下去。
        let pending = vec![pending_document(
            "m1",
            "uses npm",
            MAX_EMBEDDING_ATTEMPTS - 1,
        )];
        let (service, repository, _embedder) =
            batch_service(pending, FakeEmbedder::failing(FailureCategory::Network));

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 0,
                failed: 1,
                last_failure_category: Some(FailureCategory::Network),
                ..BatchOutcome::default()
            }
        );
        assert_eq!(
            *repository.failures.lock().expect("lock"),
            vec![("m1".to_string(), FailureCategory::Network, true)]
        );
    }

    #[test]
    fn content_longer_than_the_limit_is_truncated_before_embedding() {
        // 用多字节字符（每个 3 字节）构造超长内容：如果实现按字节而非字符切片，
        // EMBEDDING_CONTENT_LIMIT（8000）不是 3 的倍数，切点会落在字符中间——要么直接
        // panic，要么切出来的字符数就不会恰好等于 EMBEDDING_CONTENT_LIMIT。
        let content: String = "记".repeat(EMBEDDING_CONTENT_LIMIT + 1);
        assert_eq!(content.chars().count(), EMBEDDING_CONTENT_LIMIT + 1);
        let pending = vec![pending_document("m1", &content, 0)];
        let (service, _repository, embedder) =
            batch_service(pending, FakeEmbedder::succeeding(vec![vec![0.1]]));

        service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        let calls = embedder.calls.lock().expect("lock");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].1.len(), 1);
        assert_eq!(calls[0].1[0].chars().count(), EMBEDDING_CONTENT_LIMIT);
    }

    #[test]
    fn a_batch_never_exceeds_its_size_limit() {
        let pending: Vec<RetrievalDocument> = (0..40)
            .map(|i| pending_document(&format!("m{i}"), "uses npm", 0))
            .collect();
        let vectors = vec![vec![0.1]; EMBEDDING_BATCH_SIZE];
        let (service, repository, _embedder) =
            batch_service(pending, FakeEmbedder::succeeding(vectors));

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(outcome.succeeded, EMBEDDING_BATCH_SIZE);
        assert_eq!(
            *repository.claim_limits.lock().expect("lock"),
            vec![EMBEDDING_BATCH_SIZE]
        );
    }

    #[test]
    fn a_provider_returning_the_wrong_number_of_vectors_fails_the_batch_without_storing_anything() {
        let pending = vec![
            pending_document("m1", "uses npm", 0),
            pending_document("m2", "uses cargo", 0),
            pending_document("m3", "uses rustup", 0),
        ];
        // provider 只回了 2 个向量，但批里有 3 条——数量对不上，不能靠位置瞎配对。
        let (service, repository, _embedder) = batch_service(
            pending,
            FakeEmbedder::succeeding(vec![vec![0.1], vec![0.2]]),
        );

        let outcome = service
            .process_pending_batch(MODEL)
            .expect("process_pending_batch");

        assert_eq!(
            outcome,
            BatchOutcome {
                succeeded: 0,
                failed: 3,
                last_failure_category: Some(FailureCategory::InvalidRequest),
                ..BatchOutcome::default()
            }
        );
        assert!(repository.stored.lock().expect("lock").is_empty());
        assert_eq!(
            *repository.failures.lock().expect("lock"),
            vec![
                ("m1".to_string(), FailureCategory::InvalidRequest, true),
                ("m2".to_string(), FailureCategory::InvalidRequest, true),
                ("m3".to_string(), FailureCategory::InvalidRequest, true),
            ]
        );
    }
}
