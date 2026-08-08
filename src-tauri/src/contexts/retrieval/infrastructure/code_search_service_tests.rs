use std::sync::Mutex;

use super::*;
use crate::contexts::retrieval::application::{EmbeddingFailure, RetrievalConfiguration};
use crate::contexts::retrieval::domain::code_index::{CODE_INDEX_VERSION, DEFAULT_MAX_FILE_BYTES};
use crate::contexts::retrieval::domain::{
    content_hash, document_id, CodeChunk, CodeFileManifest, CodeIndexConfigurationUpdate,
    CodeLanguage,
};
use crate::contexts::retrieval::infrastructure::code_index_repository::SqliteCodeIndexRepository;
use crate::contexts::retrieval::infrastructure::SqliteRetrievalDocumentRepository;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

const PROFILE: &str = "profile-a";
const MODEL: &str = "model-a";

struct Configured;

impl RetrievalConfigurationRepository for Configured {
    fn load(&self) -> Result<RetrievalConfiguration, RetrievalError> {
        Ok(RetrievalConfiguration {
            source_profile_id: Some(PROFILE.to_string()),
            embedding_model: Some(MODEL.to_string()),
        })
    }

    fn save(&self, _profile_id: &str, _embedding_model: &str) -> Result<(), RetrievalError> {
        Ok(())
    }
}

#[derive(Default)]
struct QueryEmbedder {
    calls: Mutex<usize>,
}

impl EmbeddingPort for QueryEmbedder {
    fn embed(&self, _model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
        *self.calls.lock().expect("lock") += 1;
        Ok(inputs.iter().map(|_| vec![1.0, 0.0]).collect())
    }
}

struct Fixture {
    _database_directory: TempDirectory,
    _first_root: TempDirectory,
    _second_root: TempDirectory,
    code: Arc<SqliteCodeIndexRepository>,
    documents: Arc<SqliteRetrievalDocumentRepository>,
    first_id: String,
    first_generation: u64,
    second_id: String,
}

impl Fixture {
    fn new() -> Self {
        let database_directory = TempDirectory::new("code-search-database");
        let first_root = TempDirectory::new("code-search-first");
        let second_root = TempDirectory::new("code-search-second");
        let database =
            NativeDatabase::new(database_directory.path().to_path_buf()).expect("database");
        let code = Arc::new(SqliteCodeIndexRepository::new(database.clone()));
        let first = register_enabled(&code, &first_root, "first");
        let second = register_enabled(&code, &second_root, "second");
        insert_chunk(
            &code,
            &first.workspace_id,
            "src/auth.rs",
            "first-hit",
            "fn handle_login() { let api_key = \"SENSITIVE-CODE-SEARCH\"; }",
        );
        insert_chunk(
            &code,
            &second.workspace_id,
            "src/stronger.rs",
            "second-hit",
            "fn handle_login() { login login login }",
        );
        Self {
            documents: Arc::new(SqliteRetrievalDocumentRepository::new(database)),
            code,
            first_id: first.workspace_id,
            first_generation: first.generation,
            second_id: second.workspace_id,
            _database_directory: database_directory,
            _first_root: first_root,
            _second_root: second_root,
        }
    }

    fn service(&self, embedder: Arc<QueryEmbedder>) -> CodeSearchService {
        CodeSearchService::new(
            self.first_id.clone(),
            Arc::new(Configured),
            self.documents.clone(),
            self.code.clone(),
            embedder,
        )
        .expect("service")
    }
}

fn register_enabled(
    code: &SqliteCodeIndexRepository,
    root: &TempDirectory,
    name: &str,
) -> crate::contexts::retrieval::domain::CodeWorkspace {
    let registered = code
        .register_workspace(root.path(), name)
        .expect("register");
    code.save_configuration(
        &registered.workspace_id,
        CodeIndexConfigurationUpdate {
            enabled: true,
            selected_roots: vec![String::new()],
            languages: vec![CodeLanguage::Rust],
            exclusion_patterns: Vec::new(),
            max_file_bytes: DEFAULT_MAX_FILE_BYTES,
        },
    )
    .expect("enable")
}

fn insert_chunk(
    code: &SqliteCodeIndexRepository,
    workspace_id: &str,
    path: &str,
    source_id: &str,
    content: &str,
) {
    code.replace_file(
        &CodeFileManifest {
            workspace_id: workspace_id.to_string(),
            relative_path: path.to_string(),
            language: "rust".to_string(),
            byte_size: content.len() as u64,
            modified_ns: 1,
            content_hash: content_hash(content),
            index_version: CODE_INDEX_VERSION.to_string(),
        },
        &[CodeChunk {
            source_id: source_id.to_string(),
            content: content.to_string(),
            content_hash: content_hash(content),
            language: "rust".to_string(),
            start_line: 10,
            end_line: 12,
            symbol_name: Some("handle_login".to_string()),
            symbol_kind: Some("function".to_string()),
            ordinal: 0,
            chunk_key: "chunk-0".to_string(),
            redaction_count: 0,
        }],
        &[],
    )
    .expect("replace");
}

#[test]
fn unconfirmed_search_is_keyword_only_typed_redacted_and_workspace_scoped() {
    let fixture = Fixture::new();
    let embedder = Arc::new(QueryEmbedder::default());
    let outcome = fixture
        .service(embedder.clone())
        .search_code(&CodeSearchQuery {
            text: "handle_login".to_string(),
            limit: 5,
        })
        .expect("search");

    assert_eq!(outcome.degraded, Some(Degradation::KeywordOnly));
    assert_eq!(outcome.hits.len(), 1);
    let hit = &outcome.hits[0];
    assert_eq!(hit.file_path, "src/auth.rs");
    assert_eq!((hit.start_line, hit.end_line), (10, 12));
    assert_eq!(hit.language, "rust");
    assert_eq!(hit.symbol_name.as_deref(), Some("handle_login"));
    assert_eq!(hit.symbol_kind.as_deref(), Some("function"));
    assert_eq!(hit.matched_via, MatchedVia::Keyword);
    assert!(hit.snippet.contains("[REDACTED]"));
    assert!(!hit.snippet.contains("SENSITIVE-CODE-SEARCH"));
    assert_eq!(*embedder.calls.lock().expect("lock"), 0);
    assert_ne!(fixture.first_id, fixture.second_id);
}

#[test]
fn confirmed_search_fuses_only_vectors_from_the_bound_workspace() {
    let fixture = Fixture::new();
    fixture
        .code
        .confirm_embedding(&fixture.first_id, PROFILE, MODEL, fixture.first_generation)
        .expect("confirm");
    let identity = code_embedding_identity(PROFILE, MODEL);
    fixture
        .documents
        .store_embedding(
            &document_id(SourceKind::WorkspaceFile, "first-hit"),
            &identity,
            &[1.0, 0.0],
        )
        .expect("first vector");
    fixture
        .documents
        .store_embedding(
            &document_id(SourceKind::WorkspaceFile, "second-hit"),
            &identity,
            &[1.0, 0.0],
        )
        .expect("second vector");

    let outcome = fixture
        .service(Arc::new(QueryEmbedder::default()))
        .search_code(&CodeSearchQuery {
            text: "handle_login".to_string(),
            limit: 5,
        })
        .expect("search");
    assert_eq!(outcome.degraded, None);
    assert_eq!(outcome.hits.len(), 1);
    assert_eq!(outcome.hits[0].file_path, "src/auth.rs");
    assert_eq!(outcome.hits[0].matched_via, MatchedVia::Both);
}
