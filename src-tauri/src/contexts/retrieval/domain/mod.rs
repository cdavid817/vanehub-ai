pub(crate) mod document;
pub(crate) mod error;
pub(crate) mod fusion;
pub(crate) mod query;
pub(crate) mod vector;

// Task 5 的仓储层（sqlite_repository.rs）与 Task 7 的索引服务（indexing_service.rs）已经在
// 非测试代码里经 `domain::` 路径引用这两条重新导出里的名字（如 SourceKind、RetrievalError）；
// 已用 cargo check 实测确认：现在单独移除这两个 allow 不会再触发 unused_imports 告警，本属性
// 已不是必需项，只是审计范围限定为"仅改注释"而保留原样，未做属性层面的改动。
#[allow(unused_imports)]
pub(crate) use document::{
    content_hash, document_id, FailureCategory, IndexState, RetrievalDocument, SourceKind,
};
#[allow(unused_imports)]
pub(crate) use error::RetrievalError;

// 以下三条重新导出要到 Task 9 的检索服务经 `domain::` 路径引用时才会被使用；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use fusion::fuse_with_rrf;
#[allow(unused_imports)]
pub(crate) use query::{
    escape_fts_query, Degradation, MatchedVia, RetrievalQuery, RetrievalScope, ScoredHit,
};
#[allow(unused_imports)]
pub(crate) use vector::{cosine_similarity, decode_embedding, encode_embedding};
