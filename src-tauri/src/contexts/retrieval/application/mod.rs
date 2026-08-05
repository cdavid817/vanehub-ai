pub(crate) mod indexing_service;
pub(crate) mod ports;

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
pub(crate) use ports::{
    RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
    RetrievalIndexStatus,
};
