pub(crate) mod code_chunker;
pub(crate) mod code_index_repository;
pub(crate) mod code_inventory;
pub(crate) mod code_parser;
pub(crate) mod code_reconciler;
pub(crate) mod code_symbols;
pub(crate) mod configuration_repository;
pub(crate) mod openai_embedding_adapter;
pub(crate) mod schema;
pub(crate) mod sqlite_repository;
pub(crate) mod workspace_file_index_source;

pub(crate) use configuration_repository::SqliteRetrievalConfigurationRepository;
pub(crate) use openai_embedding_adapter::HttpEmbeddingAdapter;
pub(crate) use schema::{apply_code_index_schema, apply_retrieval_schema};
pub(crate) use sqlite_repository::SqliteRetrievalDocumentRepository;
pub(crate) use workspace_file_index_source::WorkspaceFileIndexSource;
