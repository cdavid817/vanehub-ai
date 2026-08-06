pub(crate) mod document;
pub(crate) mod error;
pub(crate) mod fusion;
pub(crate) mod query;
pub(crate) mod vector;

pub(crate) use document::{
    content_hash, document_id, FailureCategory, IndexState, RetrievalDocument, SourceKind,
};
pub(crate) use error::RetrievalError;

pub(crate) use fusion::fuse_with_rrf;
pub(crate) use query::{escape_fts_query, Degradation, MatchedVia, RetrievalQuery, ScoredHit};
pub(crate) use vector::{cosine_similarity, decode_embedding, encode_embedding};
