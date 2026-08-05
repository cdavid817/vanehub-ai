use crate::contexts::retrieval::domain::{
    FailureCategory, RetrievalDocument, RetrievalError, RetrievalScope, SourceKind,
};

// Task 12 的 RetrievalApi::index_status 会经仓储构造并返回它；届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalIndexStatus {
    pub(crate) indexed: u32,
    pub(crate) pending: u32,
    pub(crate) failed: u32,
    /// 只给类别，不带原始错误文本——错误体可能含凭据或 provider 响应内容（设计文档 §8.2）。
    pub(crate) last_failure_category: Option<String>,
}

// 本 trait 唯一的实现（SqliteRetrievalDocumentRepository）要到 Task 12 的 bootstrap 装配把它
// 构造出来并注入 IndexingService 才会被真正调用；届时移除本属性。Task 7 的 IndexingService
// 已经把它用作字段类型，但只要 IndexingService 自身还没被真实构造过，光靠字段类型引用
// 不足以让这个 trait 被判定为"已使用"（已用 cargo check 实测确认）。
#[allow(dead_code)]
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
    fn vector_candidates(
        &self,
        scope: &RetrievalScope,
        source_kind: SourceKind,
        model: &str,
    ) -> Result<Vec<(String, Vec<f32>)>, RetrievalError>;
    fn keyword_candidates(
        &self,
        scope: &RetrievalScope,
        source_kind: SourceKind,
        query: &str,
        limit: usize,
    ) -> Result<Vec<String>, RetrievalError>;
    fn index_status(&self, agent_id: &str) -> Result<RetrievalIndexStatus, RetrievalError>;
    fn requeue_all(&self, agent_id: &str) -> Result<(), RetrievalError>;
}

// 本 trait 唯一的实现要到 Task 10 的 openai-compatible 适配器才会写出来;即使写出来,也要到
// Task 12 的 bootstrap 装配把它注入 IndexingService、并让 process_pending_batch 从活根被调用
// 才会被真正调用。Task 8 的 IndexingService 已经把它用作字段类型,但参见上面
// RetrievalDocumentRepository 的同类结论(已用 cargo check 实测确认)——光靠字段类型引用不足以
// 让 trait 被判定为"已使用"。届时移除本属性。
#[allow(dead_code)]
pub(crate) trait EmbeddingPort: Send + Sync {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure>;
}

// EmbeddingFailure 目前只在测试用的 FakeEmbedder 里构造;非测试代码里唯一读它字段的是
// process_pending_batch 的 Err(failure) 分支,且只读 category(用于 give_up 判定)——message
// 至今没有任何读者:worker 的失败日志按设计文档 §8.2 只记错误类别与 attempt_count,不落盘
// message 这种可能带凭据/provider 响应体的原始文本,所以 Task 12 大概率也不会读它。保留这个
// 字段只是因为它是本任务给定的接口形状。process_pending_batch 本身要到 Task 12 才可达。
// 第一处非测试构造点是 Task 10 的适配器把 provider 的 HTTP 状态码/错误体映射成这个结构体时。
// 届时（Task 12 令整条链路可达）移除本属性。
#[allow(dead_code)]
pub(crate) struct EmbeddingFailure {
    pub(crate) category: FailureCategory,
    pub(crate) message: String,
}

// Task 6 的仓储已经在 load() 里构造它了（含 unwrap_or_default 的默认值路径），但 load() 本身是
// SqliteRetrievalConfigurationRepository 的 trait 方法，要到 Task 12 的 bootstrap 装配把仓储从
// 活根构造出来才可达（已用 cargo check 实测确认：仅移除本属性、保留仓储 struct 与它实现的
// trait 自身的 allow 时不会触发告警，必须连同它们一起摘掉 allow 才会看到本 struct 被判定为
// 未构造）。Task 9 的检索服务会读它来判断向量路是否可用，但那同样要等 Task 12 把仓储接到活
// 入口之后。真正的移除点是 Task 12。届时移除本属性。
#[allow(dead_code)]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct RetrievalConfiguration {
    pub(crate) source_profile_id: Option<String>,
    pub(crate) embedding_model: Option<String>,
}

// resolved_model 目前没有任何调用方（仅测试里直接构造 RetrievalConfiguration 后调用）；
// 唯一预期的调用方是 Task 9 的检索服务，用它判断"是否已配置"。届时移除本属性。
#[allow(dead_code)]
impl RetrievalConfiguration {
    /// 两者齐备才算"已配置"——缺任一个都无法发起一次 embedding 调用。
    pub(crate) fn resolved_model(&self) -> Option<(&str, &str)> {
        let profile = self.source_profile_id.as_deref()?;
        let model = self.embedding_model.as_deref()?;
        (!profile.is_empty() && !model.is_empty()).then_some((profile, model))
    }
}

// 本 trait 唯一的实现（SqliteRetrievalConfigurationRepository）要到 Task 12 的 bootstrap 装配
// 把它构造出来并注入检索/索引服务才会被真正调用；届时移除本属性。Task 9 的检索服务会把它
// 当依赖注入，但那只是把它记成字段类型/trait 对象——参见 RetrievalDocumentRepository 上的
// 同类结论，光靠依赖注入不足以让 trait 被判定为"已使用"。
#[allow(dead_code)]
pub(crate) trait RetrievalConfigurationRepository: Send + Sync {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError>;
    fn save(&self, profile_id: &str, embedding_model: &str) -> Result<(), RetrievalError>;
}
