pub(crate) mod configuration_repository;
pub(crate) mod schema;
pub(crate) mod sqlite_repository;

pub(crate) use schema::apply_retrieval_schema;
// Task 12 的 bootstrap 装配会经 `infrastructure::` 路径引用它来构造仓储实例；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use configuration_repository::SqliteRetrievalConfigurationRepository;
#[allow(unused_imports)]
pub(crate) use sqlite_repository::SqliteRetrievalDocumentRepository;
