use crate::contexts::retrieval::domain::{
    content_hash, document_id, IndexState, RetrievalDocument, RetrievalError, SourceKind,
};
// FailureCategory 目前只出现在测试里 FakeRepository::record_failure 的签名（该方法本身
// unimplemented!()）——reconcile 不构造失败分类。Task 8 给本文件加 process_pending_batch 后
// 会直接构造 FailureCategory::InvalidRequest 等变体，届时移除本属性。
#[allow(unused_imports)]
use crate::contexts::retrieval::domain::FailureCategory;
// RetrievalScope 同样只出现在测试里 FakeRepository::vector_candidates/keyword_candidates 的
// 签名（两者都 unimplemented!()），纯粹是为了让 trait 实现完整。真正调用这两个方法的是
// Task 9 的 search_service.rs，不在本文件——这个 import 不会有"届时移除"的那一天。
#[allow(unused_imports)]
use crate::contexts::retrieval::domain::RetrievalScope;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::ports::RetrievalDocumentRepository;
// 同理，RetrievalIndexStatus 只出现在测试里 FakeRepository::index_status 的签名
// （unimplemented!()）。真正调用方是 Task 12 的 RetrievalApi，经仓储直接读，不经过本文件。
#[allow(unused_imports)]
use super::ports::RetrievalIndexStatus;

/// retrieval 从源上下文取快照的消费侧契约。第 1 期唯一实现是 agent_runtime 的记忆表适配器。
// 唯一实现要到 Task 12 的 bootstrap 装配把 agent_runtime 的记忆表适配器构造出来、
// 注入 IndexingService 才会存在；届时移除本属性。
#[allow(dead_code)]
pub(crate) trait IndexSourcePort: Send + Sync {
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError>;
}

// 同上，随 IndexSourcePort 一起在 Task 12 移除。
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct IndexSourceRecord {
    pub(crate) source_id: String,
    pub(crate) agent_id: String,
    pub(crate) folder: String,
    pub(crate) content: String,
    /// 检索结果要带上它（Task 9 的 `ScoredHit.created_at`），且它只存在于源表——
    /// 索引行刻意不复制这个字段，避免又多一处会陈旧的副本。
    pub(crate) created_at: String,
}

// 唯一构造点是 Task 12 的 bootstrap 装配；届时移除本属性。
#[allow(dead_code)]
pub(crate) struct IndexingService {
    repository: Arc<dyn RetrievalDocumentRepository>,
    source: Arc<dyn IndexSourcePort>,
}

// 同上，随 IndexingService 一起在 Task 12 移除。
#[allow(dead_code)]
impl IndexingService {
    pub(crate) fn new(
        repository: Arc<dyn RetrievalDocumentRepository>,
        source: Arc<dyn IndexSourcePort>,
    ) -> Self {
        Self { repository, source }
    }

    /// 索引的真源是这一次差集协调，而**不是**保存路径上的双写。
    ///
    /// 在 `SqliteAgentMemoryRepository::save` 里顺手插一条索引行看似更简单，但那会引入
    /// "入队写失败 → 该记忆永远搜不到"的静默漏洞：保存成功了，用户以为记住了，检索却永远看不见。
    /// 协调式的代价只是最多延迟一个周期，而且顺带把历史存量记忆回填掉，不需要单独的数据迁移脚本。
    pub(crate) fn reconcile(&self) -> Result<ReconcileOutcome, RetrievalError> {
        let records = self.source.snapshot()?;
        let existing: HashMap<String, String> = self
            .repository
            .list_indexed_source_ids(SourceKind::AgentMemory)?
            .into_iter()
            .collect();

        let mut outcome = ReconcileOutcome::default();
        let mut live: HashSet<&str> = HashSet::new();
        for record in &records {
            live.insert(record.source_id.as_str());
            let hash = content_hash(&record.content);
            match existing.get(&record.source_id) {
                Some(existing_hash) if existing_hash == &hash => continue,
                Some(_) => outcome.invalidated += 1,
                None => outcome.added += 1,
            }
            self.repository.upsert_pending(&RetrievalDocument {
                id: document_id(SourceKind::AgentMemory, &record.source_id),
                source_kind: SourceKind::AgentMemory,
                source_id: record.source_id.clone(),
                scope_agent_id: record.agent_id.clone(),
                scope_folder: record.folder.clone(),
                content: record.content.clone(),
                content_hash: hash,
                index_state: IndexState::Pending,
                attempt_count: 0,
                embedding_model: None,
            })?;
        }

        // 孤儿清理是 §5.3 显式撤销失败时的兜底。少了它，一次失败的撤销调用会让索引行永久残留。
        for source_id in existing.keys() {
            if !live.contains(source_id.as_str()) {
                self.repository
                    .delete_by_source(SourceKind::AgentMemory, source_id)?;
                outcome.orphans_removed += 1;
            }
        }
        Ok(outcome)
    }
}

// 唯一构造点是 reconcile 内部；reconcile 要到 Task 12 才被真正调用，届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReconcileOutcome {
    pub(crate) added: usize,
    pub(crate) invalidated: usize,
    pub(crate) orphans_removed: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct FakeSource {
        records: Vec<IndexSourceRecord>,
    }

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(self.records.clone())
        }
    }

    #[derive(Default)]
    struct FakeRepository {
        /// 已存在的索引行：(source_id, content_hash)
        rows: Vec<(String, String)>,
        upserted: Mutex<Vec<String>>,
        deleted: Mutex<Vec<String>>,
    }

    impl RetrievalDocumentRepository for FakeRepository {
        fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError> {
            self.upserted
                .lock()
                .expect("lock")
                .push(document.source_id.clone());
            Ok(())
        }

        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            Ok(self.rows.clone())
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

        // reconcile 只用到上面三个方法。其余方法在本套测试中不可达，走 unimplemented!()。
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn vector_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn keyword_candidates(
            &self,
            _scope: &RetrievalScope,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn index_status(&self, _agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
        fn requeue_all(&self, _agent_id: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by reconcile tests")
        }
    }

    fn record(source_id: &str, content: &str) -> IndexSourceRecord {
        IndexSourceRecord {
            source_id: source_id.to_string(),
            agent_id: "a".to_string(),
            folder: String::new(),
            content: content.to_string(),
            created_at: "2026-08-05T00:00:00Z".to_string(),
        }
    }

    fn indexed(source_id: &str, content: &str) -> (String, String) {
        (source_id.to_string(), content_hash(content))
    }

    /// 装配一个只依赖两个 fake 的服务，返回服务与仓储句柄以便事后断言调用记录。
    fn service(
        records: Vec<IndexSourceRecord>,
        rows: Vec<(String, String)>,
    ) -> (IndexingService, Arc<FakeRepository>) {
        let repository = Arc::new(FakeRepository {
            rows,
            ..FakeRepository::default()
        });
        let service = IndexingService::new(repository.clone(), Arc::new(FakeSource { records }));
        (service, repository)
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
}
