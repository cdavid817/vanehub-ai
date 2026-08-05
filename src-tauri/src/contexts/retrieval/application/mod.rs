pub(crate) mod indexing_service;
pub(crate) mod ports;
pub(crate) mod search_service;

// 这四个类型都是索引侧概念（IndexingService 本身、它读取源快照用的 IndexSourcePort/
// IndexSourceRecord、reconcile() 的返回值 ReconcileOutcome），Task 9 的检索服务只消费
// escape_fts_query/cosine_similarity/decode_embedding/fuse_with_rrf 等查询侧类型，不涉及
// 这四个——真正会经 `application::` 路径引用它们的是 Task 12 的 bootstrap 装配（构造/持有
// IndexingService、把 agent_runtime 适配器接成 IndexSourcePort、记 ReconcileOutcome 的日志
// 字段）；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use indexing_service::{
    IndexSourcePort, IndexSourceRecord, IndexingService, ReconcileOutcome,
};
// EmbeddingEndpointPort/EmbeddingFailure/EmbeddingPort 经这条 `application::` 路径被 Task 10 的
// infrastructure/openai_embedding_adapter.rs 引用，与 RetrievalDocumentRepository 等既有条目走
// 同一惯例（sqlite_repository.rs 的导入方式）。ResolvedEmbeddingEndpoint 不在此列——本任务只经
// 类型推断读它的字段，从不在代码里写它的名字；它仍在 ports.rs 里定义并带自己的
// #[allow(dead_code)]，需要具名引用时（大概率是 Task 12）再补这条重新导出。
pub(crate) use ports::{
    EmbeddingEndpointPort, EmbeddingFailure, EmbeddingPort, RetrievalConfiguration,
    RetrievalConfigurationRepository, RetrievalDocumentRepository, RetrievalIndexStatus,
};
// SearchService 本身与它的返回值 SearchOutcome：真正会经 `application::` 路径引用它们的是
// Task 12 的 bootstrap 装配（构造并持有 SearchService）与 Task 13 的 recall 工具（读取
// SearchOutcome 的 hits/degraded 拼装工具结果）；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use search_service::{SearchOutcome, SearchService};
