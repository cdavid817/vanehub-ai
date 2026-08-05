pub(crate) mod document;
pub(crate) mod error;
pub(crate) mod fusion;
pub(crate) mod query;
pub(crate) mod vector;

// 这两条重新导出要到 Task 5 的仓储层经 `domain::` 路径引用时才会被使用；届时移除本属性。
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
