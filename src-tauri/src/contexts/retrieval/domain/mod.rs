pub(crate) mod code_admission;
pub(crate) mod code_index;
pub(crate) mod code_index_status;
pub(crate) mod code_redaction;
pub(crate) mod code_search;
pub(crate) mod document;
pub(crate) mod error;
pub(crate) mod fusion;
pub(crate) mod query;
pub(crate) mod vector;

pub(crate) use document::{
    content_hash, content_hash_bytes, document_id, FailureCategory, IndexState, RetrievalDocument,
    RetrievalScope, SourceKind,
};
pub(crate) use error::RetrievalError;

pub(crate) use code_index::{
    CodeChunk, CodeFileManifest, CodeIndexConfigurationUpdate, CodeIndexPhase, CodeLanguage,
    CodeSymbol, CodeWorkspace,
};
pub(crate) use code_index_status::{
    code_embedding_identity, CodeEmbeddingConfirmation, CodeIndexAuditEntry, CodeIndexAuditEvent,
    CodeIndexAuditReason, CodeIndexStatus,
};
pub(crate) use code_redaction::redact_code;
pub(crate) use code_search::{
    CodeSearchCandidate, CodeSearchHit, CodeSearchOutcome, CodeSearchQuery,
};
pub(crate) use fusion::fuse_with_rrf;
pub(crate) use query::{escape_fts_query, Degradation, MatchedVia, RetrievalQuery, ScoredHit};
pub(crate) use vector::{cosine_similarity, decode_embedding, encode_embedding};
