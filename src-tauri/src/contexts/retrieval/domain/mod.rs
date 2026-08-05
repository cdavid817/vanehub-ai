pub(crate) mod document;
pub(crate) mod error;

// 这两条重新导出要到 Task 5 的仓储层经 `domain::` 路径引用时才会被使用；届时移除本属性。
#[allow(unused_imports)]
pub(crate) use document::{
    content_hash, document_id, FailureCategory, IndexState, RetrievalDocument, SourceKind,
};
#[allow(unused_imports)]
pub(crate) use error::RetrievalError;
