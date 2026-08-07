pub(crate) mod indexing_service;
pub(crate) mod ports;
pub(crate) mod search_service;

// bootstrap 的装配根经 `application::` 路径构造 IndexingService、把记忆表适配器接成
// IndexSourcePort、并把 BatchOutcome 的计数写进批次日志；两个常量同样只在 worker 循环里
// 消费（兜底轮询周期与重试退避表），另外三个可调常量只在 indexing_service.rs 内部使用，
// 不需要经本路径导出。
pub(crate) use indexing_service::{
    BatchOutcome, IndexSourcePort, IndexSourceRecord, IndexingService,
    RECONCILE_POLL_INTERVAL_SECONDS, RETRY_BACKOFF_SECONDS,
};
// EmbeddingEndpointPort/EmbeddingFailure/EmbeddingPort 经这条 `application::` 路径被 Task 10 的
// infrastructure/openai_embedding_adapter.rs 引用，与 RetrievalDocumentRepository 等既有条目走
// 同一惯例（sqlite_repository.rs 的导入方式）。ResolvedEmbeddingEndpoint 由 bootstrap 的端点
// 适配器具名构造，因此也在此列。
pub(crate) use ports::{
    EmbeddingEndpointPort, EmbeddingFailure, EmbeddingPort, ResolvedEmbeddingEndpoint,
    RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
    RetrievalIndexStatus,
};
// SearchService 由 bootstrap 装配并持有；SearchOutcome 经 api.rs 的 search() 返回给 Task 13 的
// recall 工具（读取 hits/degraded 拼装工具结果）。
pub(crate) use search_service::{SearchOutcome, SearchService};
