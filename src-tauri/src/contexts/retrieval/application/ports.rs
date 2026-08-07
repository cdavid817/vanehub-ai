use crate::contexts::retrieval::domain::{
    FailureCategory, RetrievalDocument, RetrievalError, SourceKind,
};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalIndexStatus {
    pub(crate) indexed: u32,
    pub(crate) pending: u32,
    pub(crate) failed: u32,
    /// 只给类别，不带原始错误文本——错误体可能含凭据或 provider 响应内容（设计文档 §8.2）。
    pub(crate) last_failure_category: Option<String>,
}

pub(crate) trait RetrievalDocumentRepository: Send + Sync {
    fn upsert_pending(&self, document: &RetrievalDocument) -> Result<(), RetrievalError>;
    fn list_indexed_source_ids(
        &self,
        source_kind: SourceKind,
    ) -> Result<Vec<(String, String)>, RetrievalError>;
    fn delete_by_source(
        &self,
        source_kind: SourceKind,
        source_id: &str,
    ) -> Result<(), RetrievalError>;
    fn claim_pending_batch(
        &self,
        source_kind: SourceKind,
        limit: usize,
    ) -> Result<Vec<RetrievalDocument>, RetrievalError>;
    fn store_embedding(
        &self,
        id: &str,
        model: &str,
        embedding: &[f32],
    ) -> Result<(), RetrievalError>;
    fn record_failure(
        &self,
        id: &str,
        category: FailureCategory,
        give_up: bool,
    ) -> Result<(), RetrievalError>;
    /// 覆盖整个共享池，不按 agent/folder 过滤：`agent-memory-shared-pool` 之后每条记忆对每个
    /// Agent 都可见，过滤只会让 `recall` 搜不到已经注入进系统提示词的记忆。
    fn vector_candidates(
        &self,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError>;
    /// 与 `vector_candidates` 同样覆盖整个共享池。
    fn keyword_candidates(
        &self,
        source_kind: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError>;
    /// 全量聚合，不按 agent 分组：检索配置本身是全局单例，`is_configured()` 是全局的，索引源
    /// 快照也覆盖全部 agent，所以"这套索引现在什么状态"只有一个答案。按 agent 过滤会让
    /// 非 OnePiece agent 的行既不出现在状态里、也无法被重建。
    fn index_status(&self) -> Result<RetrievalIndexStatus, RetrievalError>;
    fn requeue_all(&self) -> Result<(), RetrievalError>;
    /// 把 embedding 模型与 `new_model` 不一致的已索引行打回 `pending`。
    ///
    /// 换模型后 `vector_candidates` 会按 `embedding_model = ?` 把旧模型的行全部滤掉，而
    /// reconcile 只在内容哈希变化时重新入队——换模型不改内容，所以没有这个方法，那些行会
    /// 永远停在 `indexed` 却永远进不了向量召回，每次检索静默降级成关键词单路，状态页还显示
    /// 一切正常。
    fn requeue_stale_model(&self, new_model: &str) -> Result<(), RetrievalError>;
}

/// 算向量的消费侧契约。唯一实现是 infrastructure 的 `HttpEmbeddingAdapter`。
pub(crate) trait EmbeddingPort: Send + Sync {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure>;
}

/// `HttpEmbeddingAdapter` 把 provider 的 HTTP 状态码映射成它；`process_pending_batch` 读
/// `category` 判定是否放弃重试。
#[derive(Debug)]
pub(crate) struct EmbeddingFailure {
    pub(crate) category: FailureCategory,
    /// 非测试代码里没有读者，也**不该**有：后台 worker 的失败日志按设计文档 §8.2 只落盘错误
    /// 类别，而这段文本来自与 provider 的交互，是最可能夹带响应体片段的一处。字段保留是因为
    /// 它属于既定的接口形状，且 openai_embedding_adapter.rs 的哨兵测试正是拿它做断言对象。
    /// 没有计划中的任务会给它添加非测试读者，故不写"届时移除"。
    #[allow(dead_code)]
    pub(crate) message: String,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedEmbeddingEndpoint {
    pub(crate) base_url: String,
    pub(crate) credential: String,
}

/// 消费侧契约：retrieval 只声明"给我一个可用的 embedding 端点"，不知道 Profile、凭据存储、
/// provider 目录的存在（设计文档 §4.3）。唯一实现写在 bootstrap 的装配根里，封装
/// `agent_runtime::api::resolve_embedding_endpoint`。
pub(crate) trait EmbeddingEndpointPort: Send + Sync {
    fn resolve(&self, profile_id: &str) -> Result<ResolvedEmbeddingEndpoint, RetrievalError>;
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalConfiguration {
    pub(crate) source_profile_id: Option<String>,
    pub(crate) embedding_model: Option<String>,
}

impl RetrievalConfiguration {
    /// 两者齐备才算"已配置"——缺任一个都无法发起一次 embedding 调用。
    pub(crate) fn resolved_model(&self) -> Option<(&str, &str)> {
        let profile = self.source_profile_id.as_deref()?;
        let model = self.embedding_model.as_deref()?;
        (!profile.is_empty() && !model.is_empty()).then_some((profile, model))
    }
}

pub(crate) trait RetrievalConfigurationRepository: Send + Sync {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError>;
    fn save(&self, profile_id: &str, embedding_model: &str) -> Result<(), RetrievalError>;
}
