pub(crate) mod indexing_service;
pub(crate) mod ports;

// Task 9 的检索服务与 Task 12 的 bootstrap 装配都会经 `application::` 路径引用这四个类型
// （构造/持有 IndexingService、读 IndexSourcePort 快照、记 ReconcileOutcome 的日志字段）；
// 届时移除本属性。
#[allow(unused_imports)]
pub(crate) use indexing_service::{
    IndexSourcePort, IndexSourceRecord, IndexingService, ReconcileOutcome,
};
pub(crate) use ports::{
    RetrievalConfiguration, RetrievalConfigurationRepository, RetrievalDocumentRepository,
    RetrievalIndexStatus,
};
