use std::sync::Mutex;

use super::*;
use crate::contexts::retrieval::application::{
    EmbeddingFailure, IndexSourceRecord, RetrievalDocumentRepository,
};
use crate::contexts::retrieval::domain::code_index::{CODE_INDEX_VERSION, DEFAULT_MAX_FILE_BYTES};
use crate::contexts::retrieval::domain::{
    content_hash, CodeChunk, CodeFileManifest, CodeIndexConfigurationUpdate, CodeIndexPhase,
    CodeLanguage,
};
use crate::contexts::retrieval::infrastructure::code_index_repository::SqliteCodeIndexRepository;
use crate::contexts::retrieval::infrastructure::SqliteRetrievalDocumentRepository;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

struct EmptySource;

impl IndexSourcePort for EmptySource {
    fn snapshot(&self) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        Ok(Vec::new())
    }

    fn fetch(&self, _source_ids: &[String]) -> Result<Vec<IndexSourceRecord>, RetrievalError> {
        Ok(Vec::new())
    }
}

#[derive(Default)]
struct CountingEmbedder {
    calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl EmbeddingPort for CountingEmbedder {
    fn embed(&self, model: &str, inputs: &[String]) -> Result<Vec<Vec<f32>>, EmbeddingFailure> {
        self.calls
            .lock()
            .expect("lock")
            .push((model.to_string(), inputs.to_vec()));
        Ok(inputs.iter().map(|_| vec![0.1, 0.2]).collect())
    }
}

struct Fixture {
    _database_directory: TempDirectory,
    _workspace_directory: TempDirectory,
    code: Arc<SqliteCodeIndexRepository>,
    documents: Arc<SqliteRetrievalDocumentRepository>,
    workspace_id: String,
    generation: u64,
}

impl Fixture {
    fn new() -> Self {
        let database_directory = TempDirectory::new("code-embedding-database");
        let workspace_directory = TempDirectory::new("code-embedding-workspace");
        let database =
            NativeDatabase::new(database_directory.path().to_path_buf()).expect("database");
        let code = Arc::new(SqliteCodeIndexRepository::new(database.clone()));
        let registered = code
            .register_workspace(workspace_directory.path(), "workspace")
            .expect("register");
        let workspace = code
            .save_configuration(
                &registered.workspace_id,
                CodeIndexConfigurationUpdate {
                    enabled: true,
                    mode: crate::contexts::retrieval::domain::CodeIndexMode::Semantic,
                    selected_roots: vec![String::new()],
                    languages: vec![CodeLanguage::Rust],
                    exclusion_patterns: Vec::new(),
                    max_file_bytes: DEFAULT_MAX_FILE_BYTES,
                },
            )
            .expect("enable");
        code.replace_file(
            &CodeFileManifest {
                workspace_id: workspace.workspace_id.clone(),
                relative_path: "src/lib.rs".to_string(),
                language: "rust".to_string(),
                byte_size: 24,
                modified_ns: 1,
                content_hash: "raw-hash".to_string(),
                index_version: CODE_INDEX_VERSION.to_string(),
            },
            &[CodeChunk {
                source_id: "local-chunk".to_string(),
                content: "fn local_only() {}".to_string(),
                content_hash: content_hash("fn local_only() {}"),
                language: "rust".to_string(),
                start_line: 1,
                end_line: 1,
                symbol_name: Some("local_only".to_string()),
                symbol_kind: Some("function".to_string()),
                ordinal: 0,
                chunk_key: "chunk-0".to_string(),
                redaction_count: 0,
            }],
            &[],
        )
        .expect("local index");
        Self {
            documents: Arc::new(SqliteRetrievalDocumentRepository::new(database)),
            code,
            workspace_id: workspace.workspace_id,
            generation: workspace.generation,
            _database_directory: database_directory,
            _workspace_directory: workspace_directory,
        }
    }

    fn service(
        &self,
        embedder: Arc<CountingEmbedder>,
        profile: &str,
        model: &str,
    ) -> WorkspaceCodeEmbeddingService {
        WorkspaceCodeEmbeddingService::new(
            self.documents.clone(),
            Arc::new(EmptySource),
            embedder,
            self.code.clone(),
            self.workspace_id.clone(),
            self.generation,
            profile.to_string(),
            model.to_string(),
        )
        .expect("service")
    }
}

#[test]
fn local_fts_is_available_while_external_embedding_waits_for_confirmation() {
    let fixture = Fixture::new();
    let scope = RetrievalScope::Workspace(fixture.workspace_id.clone());
    let keywords = fixture
        .documents
        .keyword_candidates_scoped(SourceKind::WorkspaceFile, &scope, "\"local\"", 5)
        .expect("keyword search");
    assert_eq!(keywords, vec!["local-chunk"]);

    assert!(!prepare_code_embedding(
        fixture.code.as_ref(),
        &fixture.workspace_id,
        "profile-a",
        "model-a",
        fixture.generation,
    )
    .expect("prepare"));
    assert_eq!(
        fixture
            .code
            .workspace_status(&fixture.workspace_id)
            .expect("status")
            .phase,
        CodeIndexPhase::AwaitingEmbeddingConfirmation
    );
    let embedder = Arc::new(CountingEmbedder::default());
    assert_eq!(
        fixture
            .service(embedder.clone(), "profile-a", "model-a")
            .process_pending_batch()
            .expect("gated batch"),
        BatchOutcome::default()
    );
    assert!(embedder.calls.lock().expect("lock").is_empty());
}

#[test]
fn provider_changes_require_confirmation_and_exclude_the_previous_vector_identity() {
    let fixture = Fixture::new();
    let scope = RetrievalScope::Workspace(fixture.workspace_id.clone());
    confirm_code_embedding(
        fixture.code.as_ref(),
        &fixture.workspace_id,
        "profile-a",
        "shared-model",
        fixture.generation,
    )
    .expect("confirm first profile");
    let first_embedder = Arc::new(CountingEmbedder::default());
    assert_eq!(
        fixture
            .service(first_embedder.clone(), "profile-a", "shared-model")
            .process_pending_batch()
            .expect("first batch")
            .succeeded,
        1
    );
    let first_identity = code_embedding_identity("profile-a", "shared-model");
    assert_eq!(
        fixture
            .documents
            .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &first_identity)
            .expect("first vectors")
            .len(),
        1
    );

    let second_embedder = Arc::new(CountingEmbedder::default());
    assert_eq!(
        fixture
            .service(second_embedder.clone(), "profile-b", "shared-model")
            .process_pending_batch()
            .expect("unconfirmed second profile"),
        BatchOutcome::default()
    );
    assert!(second_embedder.calls.lock().expect("lock").is_empty());
    let second_identity = code_embedding_identity("profile-b", "shared-model");
    assert!(fixture
        .documents
        .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &second_identity)
        .expect("new identity excludes old vectors")
        .is_empty());

    confirm_code_embedding(
        fixture.code.as_ref(),
        &fixture.workspace_id,
        "profile-b",
        "shared-model",
        fixture.generation,
    )
    .expect("confirm second profile");
    assert_eq!(
        fixture
            .service(second_embedder, "profile-b", "shared-model")
            .process_pending_batch()
            .expect("second batch")
            .succeeded,
        1
    );
    assert!(fixture
        .documents
        .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &first_identity)
        .expect("old identity")
        .is_empty());
    assert_eq!(
        fixture
            .documents
            .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &second_identity)
            .expect("second vectors")
            .len(),
        1
    );

    let changed_model_embedder = Arc::new(CountingEmbedder::default());
    assert_eq!(
        fixture
            .service(changed_model_embedder.clone(), "profile-b", "changed-model")
            .process_pending_batch()
            .expect("unconfirmed changed model"),
        BatchOutcome::default()
    );
    assert!(changed_model_embedder
        .calls
        .lock()
        .expect("lock")
        .is_empty());
    let changed_model_identity = code_embedding_identity("profile-b", "changed-model");
    assert!(fixture
        .documents
        .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &changed_model_identity)
        .expect("changed model excludes old vectors")
        .is_empty());
    confirm_code_embedding(
        fixture.code.as_ref(),
        &fixture.workspace_id,
        "profile-b",
        "changed-model",
        fixture.generation,
    )
    .expect("confirm changed model");
    assert_eq!(
        fixture
            .service(changed_model_embedder, "profile-b", "changed-model")
            .process_pending_batch()
            .expect("changed model batch")
            .succeeded,
        1
    );
    assert!(fixture
        .documents
        .vector_candidates_scoped(SourceKind::WorkspaceFile, &scope, &second_identity)
        .expect("previous model identity")
        .is_empty());
}
