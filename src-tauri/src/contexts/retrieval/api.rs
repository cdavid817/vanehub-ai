//! `retrieval` 的唯一跨上下文出口。
//!
//! 其他上下文与 command 层只看得到本文件：仓储、应用服务、infrastructure 一律不外露
//! （设计文档 §4.1）。

use super::application::{
    RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
    RetrievalIndexStatus, SearchOutcome, SearchService,
};
use super::domain::{RetrievalError, RetrievalQuery, RetrievalScope, SourceKind};
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

    // Task 13 的 recall 工具执行路径调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn search(
        &self,
        agent_id: &str,
        folder: Option<&str>,
        query: &str,
        limit: usize,
    ) -> Result<SearchOutcome, RetrievalError> {
        self.search.search(&RetrievalQuery {
            text: query.to_string(),
            scope: RetrievalScope {
                agent_id: agent_id.to_string(),
                // 无工作区文件夹的会话映射到空串哨兵，与 `agent_memories.folder` 自身的约定
                // 一致；索引行的 scope_folder 也是这样写进去的，两侧不一致就永远搜不到。
                folder: folder.unwrap_or_default().to_string(),
            },
            limit,
        })
    }

    // Task 14 的删除记忆挂钩调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn remove(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError> {
        self.documents.delete_by_source(source_kind, source_id)
    }

    /// 每次生成的工具集解析路径上都会调用（Task 13），所以只做一次单行配置读取，且**永不**
    /// 返回错误：把一个可选增强能力的读配置故障冒泡出去，会牵连用户发出的每一条消息。
    // Task 13 的工具集解析调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn is_configured(&self) -> bool {
        self.configuration
            .load()
            .is_ok_and(|configuration| configuration.resolved_model().is_some())
    }

    // Task 15 的 Tauri command 调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn configuration(&self) -> Result<RetrievalConfiguration, RetrievalError> {
        self.configuration.load()
    }

    /// 保存后立刻唤醒 worker：否则首次配置完成到第一批 embedding 之间要白等一个兜底周期。
    // Task 15 的 Tauri command 调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn save_configuration(
        &self,
        profile_id: &str,
        model: &str,
    ) -> Result<(), RetrievalError> {
        self.configuration.save(profile_id, model)?;
        self.worker.notify();
        Ok(())
    }

    // Task 15 的 Tauri command 调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn index_status(
        &self,
        agent_id: &str,
    ) -> Result<RetrievalIndexStatus, RetrievalError> {
        self.documents.index_status(agent_id)
    }

    /// 重建只把行打回 `pending` 并叫醒 worker，不在命令线程里同步跑 embedding——重建一个
    /// agent 的全部记忆可能是几十次网络往返，阻塞在 command 上会让设置页整个卡住。
    // Task 15 的 Tauri command 调用它后可达；届时移除本属性。
    #[allow(dead_code)]
    pub(crate) fn rebuild(&self, agent_id: &str) -> Result<(), RetrievalError> {
        self.documents.requeue_all(agent_id)?;
        self.worker.notify();
        Ok(())
    }

    /// 返回 `()` 而不是 `Result`：调用方（Task 14 的保存记忆路径）不该有机会因为索引唤醒
    /// 失败而改变自己的结果。
    // Task 14 的保存记忆挂钩调用它后可达；届时移除本属性。
    #[allow(dead_code)]
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
        FailureCategory, RetrievalDocument, RetrievalError, RetrievalScope, SourceKind,
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

    /// 记录两路召回收到的 scope 与 `delete_by_source` 收到的参数；其余方法在本文件的测试里
    /// 不可达，走 `unimplemented!()`。
    #[derive(Default)]
    struct FakeDocumentRepository {
        scopes: Mutex<Vec<RetrievalScope>>,
        deleted: Mutex<Vec<(&'static str, String)>>,
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
            scope: &RetrievalScope,
            _source_kind: SourceKind,
            _model: &str,
        ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError> {
            self.scopes.lock().expect("lock").push(scope.clone());
            Ok(Vec::new())
        }
        fn keyword_candidates(
            &self,
            scope: &RetrievalScope,
            _source_kind: SourceKind,
            _query: &str,
            _limit: usize,
        ) -> Result<Vec<String>, RetrievalError> {
            self.scopes.lock().expect("lock").push(scope.clone());
            Ok(Vec::new())
        }
        fn index_status(&self, _agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
        fn requeue_all(&self, _agent_id: &str) -> Result<(), RetrievalError> {
            unimplemented!("not exercised by api tests")
        }
    }

    struct FakeSource;

    impl IndexSourcePort for FakeSource {
        fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
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
    fn search_scopes_a_folderless_session_to_the_empty_string_sentinel() {
        let (api, documents, _wakeups) = api(FakeConfigurationRepository::Configured);

        api.search("agent-a", None, "npm", 5).expect("search");

        let scopes = documents.scopes.lock().expect("lock");
        assert!(!scopes.is_empty(), "both recall paths must receive a scope");
        for scope in scopes.iter() {
            assert_eq!(scope.agent_id, "agent-a");
            assert_eq!(scope.folder, "");
        }
    }

    #[test]
    fn search_passes_a_present_folder_through_unchanged() {
        let (api, documents, _wakeups) = api(FakeConfigurationRepository::Configured);

        api.search("agent-a", Some("D:/project"), "npm", 5)
            .expect("search");

        let scopes = documents.scopes.lock().expect("lock");
        assert!(!scopes.is_empty(), "both recall paths must receive a scope");
        for scope in scopes.iter() {
            assert_eq!(scope.folder, "D:/project");
        }
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
