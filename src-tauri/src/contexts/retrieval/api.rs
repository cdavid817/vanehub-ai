//! `retrieval` 的唯一跨上下文出口。
//!
//! 其他上下文与 command 层只看得到本文件：仓储、应用服务、infrastructure 一律不外露
//! （设计文档 §4.1）。

use super::application::{
    RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
    RetrievalIndexStatus, SearchOutcome, SearchService,
};
use super::domain::{RetrievalError, RetrievalQuery, SourceKind};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::Arc;

/// 后台索引 worker 的唤醒端。
///
/// 容量 1 的 bounded channel + `try_send`：唤醒由保存记忆的路径发出（Task 14），那条路径既
/// 不能被索引 worker 拖慢，也不能因为它失败。`try_send` 在两种情况下立刻返回 `Err` 且都无需
/// 处理——缓冲已满说明本轮唤醒早就排上了队，接收端消失说明 worker 线程没了、只剩定时兜底
/// 轮询，最多延迟一个周期。
#[derive(Clone)]
pub(crate) struct RetrievalWorkerSignal {
    sender: SyncSender<()>,
}

impl RetrievalWorkerSignal {
    pub(crate) fn channel() -> (Self, Receiver<()>) {
        let (sender, receiver) = sync_channel(1);
        (Self { sender }, receiver)
    }

    fn notify(&self) {
        let _ = self.sender.try_send(());
    }
}

/// 跨上下文与 command 层唯一可见的检索边界。
#[derive(Clone)]
pub(crate) struct RetrievalApi {
    search: Arc<SearchService>,
    documents: Arc<dyn RetrievalDocumentRepository>,
    configuration: Arc<dyn RetrievalConfigurationRepository>,
    worker: RetrievalWorkerSignal,
}

impl RetrievalApi {
    pub(crate) fn new(
        search: Arc<SearchService>,
        documents: Arc<dyn RetrievalDocumentRepository>,
        configuration: Arc<dyn RetrievalConfigurationRepository>,
        worker: RetrievalWorkerSignal,
    ) -> Self {
        Self {
            search,
            documents,
            configuration,
            worker,
        }
    }

    /// 不收 agent/folder：检索面向的就是最近记忆注入所读的那一个主机级共享池
    /// （`agent-memory-shared-pool`）。
    pub(crate) fn search(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<SearchOutcome, RetrievalError> {
        self.search.search(&RetrievalQuery {
            text: query.to_string(),
            limit,
        })
    }

    pub(crate) fn remove(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError> {
        self.documents.delete_by_source(source_kind, source_id)
    }

    /// 每次生成的工具集解析路径上都会调用（Task 13），所以只做一次单行配置读取，且**永不**
    /// 返回错误：把一个可选增强能力的读配置故障冒泡出去，会牵连用户发出的每一条消息。
    pub(crate) fn is_configured(&self) -> bool {
        self.configuration
            .load()
            .is_ok_and(|configuration| configuration.resolved_model().is_some())
    }

    pub(crate) fn configuration(&self) -> Result<RetrievalConfiguration, RetrievalError> {
        self.configuration.load()
    }

    /// 保存后立刻唤醒 worker：否则首次配置完成到第一批 embedding 之间要白等一个兜底周期。
    ///
    /// 唤醒之前先把旧模型的已索引行打回 `pending`：`vector_candidates` 按 `embedding_model`
    /// 精确过滤，而 reconcile 只认内容哈希变化，两者叠加意味着换模型后旧行既进不了向量召回、
    /// 也永远不会被重新 embedding（delta spec "Vector recall only compares same-model
    /// embeddings" 的第三条要求）。重新入队而非清空 embedding，让重建随后台 worker 逐批收敛
    /// 而不是一次性抹掉全部向量召回能力（设计文档权衡 3）。
    pub(crate) fn save_configuration(
        &self,
        profile_id: &str,
        model: &str,
    ) -> Result<(), RetrievalError> {
        self.configuration.save(profile_id, model)?;
        self.documents.requeue_stale_model(model)?;
        self.worker.notify();
        Ok(())
    }

    /// 与配置一样是全局的：检索能力对所有 agent 生效，`is_configured()` 也不分 agent，按
    /// 单个 agent 汇报状态只会让别的 agent 的索引行无处可见。
    pub(crate) fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
        self.documents.index_status()
    }

    /// 重建只把行打回 `pending` 并叫醒 worker，不在命令线程里同步跑 embedding——重建全部
    /// 记忆可能是几十次网络往返，阻塞在 command 上会让设置页整个卡住。
    pub(crate) fn rebuild(&self) -> Result<(), RetrievalError> {
        self.documents.requeue_all()?;
        self.worker.notify();
        Ok(())
    }

    /// 返回 `()` 而不是 `Result`：调用方（Task 14 的保存记忆路径）不该有机会因为索引唤醒
    /// 失败而改变自己的结果。
    pub(crate) fn wake_worker(&self) {
        self.worker.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::retrieval::application::{
        EmbeddingFailure, EmbeddingPort, IndexSourcePort, IndexSourceRecord,
        RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
        RetrievalIndexStatus, SearchService,
    };
    use crate::contexts::retrieval::domain::{
        FailureCategory, RetrievalDocument, RetrievalError, SourceKind,
    };
    use std::sync::{Arc, Mutex};

    /// 三种可编排行为：已配置 / 未配置 / 读配置本身失败。第三种是 `is_configured()` 的关键
    /// 场景——它必须把失败当作"未配置"，而不是向上冒泡。
    enum FakeConfigurationRepository {
        Configured,
        Unconfigured,
        Failing,
    }

    const PROFILE: &str = "profile-a";
    const MODEL: &str = "model-a";

    impl RetrievalConfigurationRepository for FakeConfigurationRepository {
        fn load(&self) -> Result<RetrievalConfiguration, RetrievalError> {
            match self {
                Self::Configured => Ok(RetrievalConfiguration {
                    source_profile_id: Some(PROFILE.to_string()),
                    embedding_model: Some(MODEL.to_string()),
                }),
                Self::Unconfigured => Ok(RetrievalConfiguration::default()),
                Self::Failing => Err(RetrievalError::Storage("boom".to_string())),
            }
        }

        fn save(&self, _profile_id: &str, _embedding_model: &str) -> Result<(), RetrievalError> {
            Ok(())
        }
    }

    /// 记录两路召回各被调用了几次、`delete_by_source` 收到的参数，以及 `requeue_stale_model`
    /// 收到的模型 id；其余方法在本文件的测试里不可达，走 `unimplemented!()`。
    #[derive(Default)]
    struct FakeDocumentRepository {
        recall_calls: Mutex<Vec<&'static str>>,
        deleted: Mutex<Vec<(&'static str, String)>>,
        requeued_models: Mutex<Vec<String>>,
    }

    impl RetrievalDocumentRepository for FakeDocumentRepository {
        fn upsert_pending(&self, _document: &RetrievalDocument) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn list_indexed_source_ids(
            &self,
            _source_kind: SourceKind,
        ) -> Result<Vec<(String, String)>, RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn delete_by_source(
            &self,
            source_kind: SourceKind,
            source_id: &str,
        ) -> Result<(), RetrievalError> {
            self.deleted
                .lock()
                .expect("lock")
                .push((source_kind.as_str(), source_id.to_string()));
            Ok(())
        }
        fn claim_pending_batch(
            &self,
            _source_kind: SourceKind,
            _limit: usize,
        ) -> Result<Vec<RetrievalDocument>, RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn store_embedding(
            &self,
            _id: &str,
            _model: &str,
            _embedding: &[f32],
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn record_failure(
            &self,
            _id: &str,
            _category: FailureCategory,
            _give_up: bool,
        ) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn vector_candidates(
            &self,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            self.recall_calls.lock().expect("lock").push("vector");
            Ok(Vec::new())
        }
        fn keyword_candidates(
            &self,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            self.recall_calls.lock().expect("lock").push("keyword");
            Ok(Vec::new())
        }
        fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn requeue_all(&self) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn requeue_stale_model(&self, new_model: &str) -> Result<(), RetrievalError> {
            self.requeued_models
                .lock()
                .expect("lock")
                .push(new_model.to_string());
            Ok(())
        }
    }

    struct FakeSource;

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(Vec::new())
        }
        fn fetch(&self, _source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
            Ok(Vec::new())
        }
    }

    struct FakeEmbedder;

    impl EmbeddingPort for FakeEmbedder {
        fn embed(
            &self,
            _model: &str,
            _inputs: &[String],
        ) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
            Ok(vec![vec![1.0, 0.0]])
        }
    }

    /// 装配一个只依赖 fake 的门面，返回门面、文档仓储句柄与唤醒信号的接收端。接收端必须由
    /// 调用方持有：一旦丢弃，`wake_worker()` 就落进"worker 已消失"的分支。
    fn api(
        configuration: FakeConfigurationRepository,
    ) -> (
        RetrievalApi,
        Arc<FakeDocumentRepository>,
        std::sync::mpsc::Receiver<()>,
    ) {
        let documents = Arc::new(FakeDocumentRepository::default());
        let configuration: Arc<dyn RetrievalConfigurationRepository> = Arc::new(configuration);
        let (signal, wakeups) = RetrievalWorkerSignal::channel();
        let search = SearchService::new(
            configuration.clone(),
            documents.clone(),
            Arc::new(FakeSource),
            Arc::new(FakeEmbedder),
        );
        (
            RetrievalApi::new(Arc::new(search), documents.clone(), configuration, signal),
            documents,
            wakeups,
        )
    }

    #[test]
    fn an_unconfigured_api_reports_not_configured_without_erroring() {
        let (api, _documents, _wakeups) = api(FakeConfigurationRepository::Unconfigured);

        assert!(!api.is_configured());
    }

    #[test]
    fn a_configuration_load_failure_is_treated_as_unconfigured_rather_than_propagated() {
        // 这条与上一条成对存在：`is_configured()` 在每次生成的工具集解析路径上被调用，
        // 把一次读配置失败冒泡成错误会让每条消息都受牵连。
        let (api, _documents, _wakeups) = api(FakeConfigurationRepository::Failing);

        assert!(!api.is_configured());
    }

    #[test]
    fn a_configured_api_reports_configured() {
        let (api, _documents, _wakeups) = api(FakeConfigurationRepository::Configured);

        assert!(api.is_configured());
    }

    #[test]
    fn search_reaches_both_recall_paths_without_narrowing_them_to_a_scope() {
        // 这条曾经断言门面把会话的 agent/folder 织进 scope 再传给两路召回。
        // `agent-memory-shared-pool` 之后没有 scope 可传了：门面只转发 query 与 limit，
        // 检索面向的就是注入所读的同一个共享池。方法签名本身已经让"传 scope"无从表达，
        // 这里钉死的是两路都仍然被调用到。
        let (api, documents, _wakeups) = api(FakeConfigurationRepository::Configured);

        api.search("npm", 5).expect("search");

        let mut calls = documents.recall_calls.lock().expect("lock").clone();
        calls.sort_unstable();
        assert_eq!(calls, vec!["keyword", "vector"]);
    }

    #[test]
    fn remove_delegates_to_the_repository_delete() {
        let (api, documents, _wakeups) = api(FakeConfigurationRepository::Configured);

        api.remove(SourceKind::AgentMemory, "memory-1")
            .expect("remove");

        assert_eq!(
            *documents.deleted.lock().expect("lock"),
            vec![("agent_memory", "memory-1".to_string())]
        );
    }

    #[test]
    fn saving_a_configuration_requeues_rows_embedded_under_a_different_model() {
        // delta spec "Vector recall only compares same-model embeddings" 的第三条要求。
        // `vector_candidates` 会把旧模型的行滤掉，而 reconcile 只认内容哈希变化——不在保存
        // 配置时重新入队，换模型后向量召回就永久归零，且状态页显示一切正常。
        let (api, documents, wakeups) = api(FakeConfigurationRepository::Configured);

        api.save_configuration(PROFILE, "model-b").expect("save");

        assert_eq!(
            *documents.requeued_models.lock().expect("lock"),
            vec!["model-b".to_string()]
        );
        assert!(
            wakeups.try_recv().is_ok(),
            "the worker must still be woken after the requeue"
        );
    }

    #[test]
    fn waking_the_worker_never_fails_even_after_the_worker_is_gone() {
        // 唤醒信号由保存记忆的路径调用（Task 14）：那条路径既不能被索引 worker 拖慢，
        // 也不能因为它失败——所以这里既要证明信号送得到，也要证明接收端消失后调用无害。
        let (api, _documents, wakeups) = api(FakeConfigurationRepository::Configured);

        api.wake_worker();
        assert!(wakeups.try_recv().is_ok());

        drop(wakeups);
        api.wake_worker();
    }
}
