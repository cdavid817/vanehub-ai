pub(crate) mod configuration_repository;
pub(crate) mod openai_embedding_adapter;
pub(crate) mod schema;
pub(crate) mod sqlite_repository;

pub(crate) use schema::apply_retrieval_schema;
// Task 12 的 bootstrap 装配会经 `infrastructure::` 路径引用它来构造仓储实例；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use configuration_repository::SqliteRetrievalConfigurationRepository;
// 同上，Task 12 的 bootstrap 装配会经 `infrastructure::` 路径引用它来构造 EmbeddingPort 实例；
// 届时移除本属性。
#[allow(unused_imports)]
pub(crate) use openai_embedding_adapter::HttpEmbeddingAdapter;
#[allow(unused_imports)]
pub(crate) use sqlite_repository::SqliteRetrievalDocumentRepository;
