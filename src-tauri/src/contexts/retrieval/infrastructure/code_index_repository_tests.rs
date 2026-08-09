use super::*;
use crate::contexts::retrieval::domain::code_index::CODE_INDEX_VERSION;
use crate::contexts::retrieval::domain::content_hash;
use crate::contexts::retrieval::domain::{
    CodeIndexAuditEvent, CodeIndexAuditReason, FailureCategory,
};
use crate::contexts::retrieval::domain::{CodeIndexConfigurationUpdate, CodeLanguage};
use crate::test_support::TempDirectory;

struct Fixture {
    _database_directory: TempDirectory,
    workspace_directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteCodeIndexRepository,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let database_directory = TempDirectory::new(&format!("{label}-database"));
        let workspace_directory = TempDirectory::new(&format!("{label}-workspace"));
        let database =
            NativeDatabase::new(database_directory.path().to_path_buf()).expect("database");
        Self {
            repository: SqliteCodeIndexRepository::new(database.clone()),
            database,
            workspace_directory,
            _database_directory: database_directory,
        }
    }

    fn register(&self) -> CodeWorkspace {
        self.repository
            .register_workspace(self.workspace_directory.path(), "workspace")
            .expect("register")
    }
}

fn manifest(workspace_id: &str, path: &str, hash: &str) -> CodeFileManifest {
    CodeFileManifest {
        workspace_id: workspace_id.to_string(),
        relative_path: path.to_string(),
        language: "rust".to_string(),
        byte_size: 20,
        modified_ns: 1,
        content_hash: hash.to_string(),
        index_version: CODE_INDEX_VERSION.to_string(),
    }
}

fn chunk(source_id: &str, key: &str, content: &str) -> CodeChunk {
    CodeChunk {
        source_id: source_id.to_string(),
        content: content.to_string(),
        content_hash: content_hash(content),
        language: "rust".to_string(),
        start_line: 1,
        end_line: 2,
        symbol_name: Some("main".to_string()),
        symbol_kind: Some("function".to_string()),
        ordinal: 0,
        chunk_key: key.to_string(),
        redaction_count: 0,
    }
}

fn symbol(id: &str) -> CodeSymbol {
    CodeSymbol {
        symbol_id: id.to_string(),
        normalized_name: "main".to_string(),
        display_name: "main".to_string(),
        symbol_kind: "function".to_string(),
        container_name: None,
        start_line: 1,
        end_line: 2,
    }
}

#[test]
fn registering_the_same_canonical_root_reuses_the_stable_workspace_id() {
    let fixture = Fixture::new("code index stable workspace");
    let first = fixture.register();
    let second = fixture
        .repository
        .register_workspace(fixture.workspace_directory.path(), "renamed")
        .expect("register again");

    assert_eq!(first.workspace_id, second.workspace_id);
    assert_eq!(second.display_name, "workspace");
    assert_eq!(
        fixture
            .repository
            .load_workspace(&first.workspace_id)
            .expect("load"),
        Some(first)
    );
}

#[test]
fn automatic_discovery_reuses_canonical_root_without_overwriting_explicit_configuration() {
    let fixture = Fixture::new("automatic code index reuse");
    let workspace = fixture.register();
    let explicit = fixture
        .repository
        .save_configuration(
            &workspace.workspace_id,
            CodeIndexConfigurationUpdate {
                enabled: false,
                mode: CodeIndexMode::Semantic,
                selected_roots: vec!["src".to_string()],
                languages: vec![CodeLanguage::Rust],
                exclusion_patterns: vec!["generated/**".to_string()],
                max_file_bytes: 64 * 1024,
            },
        )
        .expect("explicit configuration");

    let (reused, created) = fixture
        .repository
        .ensure_automatic_workspace(
            fixture.workspace_directory.path(),
            "automatic name",
            CodeIndexMode::Local,
        )
        .expect("automatic discovery");

    assert!(!created);
    assert_eq!(reused, explicit);
    assert_eq!(reused.origin, CodeWorkspaceOrigin::Manual);
}

#[test]
fn a_missing_registered_root_is_reported_unavailable_without_deleting_the_record() {
    let fixture = Fixture::new("code index unavailable workspace");
    let workspace = fixture.register();
    std::fs::remove_dir_all(fixture.workspace_directory.path()).expect("remove workspace root");

    let loaded = fixture
        .repository
        .load_workspace(&workspace.workspace_id)
        .expect("load")
        .expect("retained workspace");
    assert_eq!(loaded.workspace_id, workspace.workspace_id);
    assert_eq!(loaded.phase, CodeIndexPhase::Unavailable);
}

#[test]
fn replacing_a_file_atomically_removes_its_old_documents_and_symbols() {
    let fixture = Fixture::new("code index replace file");
    let workspace = fixture.register();
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src\\lib.rs", "old"),
            &[chunk("old-chunk", "old-key", "fn old() {}")],
            &[symbol("old-symbol")],
        )
        .expect("first replace");
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/lib.rs", "new"),
            &[chunk("new-chunk", "new-key", "fn main() {}")],
            &[symbol("new-symbol")],
        )
        .expect("second replace");

    let connection = fixture.database.connection().expect("connection");
    let documents: Vec<String> = connection
        .prepare("SELECT source_id FROM retrieval_documents WHERE source_kind = 'workspace_file'")
        .expect("prepare")
        .query_map([], |row| row.get(0))
        .expect("query")
        .collect::<Result<_, _>>()
        .expect("collect");
    assert_eq!(documents, vec!["new-chunk"]);
    let symbols: Vec<String> = connection
        .prepare("SELECT symbol_id FROM code_index_symbols")
        .expect("prepare symbols")
        .query_map([], |row| row.get(0))
        .expect("query symbols")
        .collect::<Result<_, _>>()
        .expect("collect symbols");
    assert_eq!(symbols, vec!["new-symbol"]);
}

#[test]
fn file_operations_reject_absolute_and_parent_traversal_paths() {
    let fixture = Fixture::new("code index path validation");
    let workspace = fixture.register();
    for path in ["../outside.rs", "C:\\outside.rs", "/outside.rs", ""] {
        let result = fixture.repository.replace_file(
            &manifest(&workspace.workspace_id, path, "hash"),
            &[],
            &[],
        );
        assert_eq!(result, Err(RetrievalError::InvalidScope), "accepted {path}");
    }
}

#[test]
fn deleting_a_file_or_workspace_removes_only_its_own_index_data() {
    let fixture = Fixture::new("code index scoped deletion");
    let first = fixture.register();
    let other_root = TempDirectory::new("code-index-other-workspace");
    let second = fixture
        .repository
        .register_workspace(other_root.path(), "other")
        .expect("register other");
    for (workspace, source) in [(&first, "first-chunk"), (&second, "second-chunk")] {
        fixture
            .repository
            .replace_file(
                &manifest(&workspace.workspace_id, "src/lib.rs", source),
                &[chunk(source, source, "fn main() {}")],
                &[],
            )
            .expect("replace");
    }

    fixture
        .repository
        .delete_file(&first.workspace_id, "src/lib.rs")
        .expect("delete file");
    fixture
        .repository
        .delete_workspace(&first.workspace_id)
        .expect("delete workspace");

    assert!(fixture
        .repository
        .load_workspace(&first.workspace_id)
        .expect("load first")
        .is_none());
    assert!(fixture
        .repository
        .load_workspace(&second.workspace_id)
        .expect("load second")
        .is_some());
    let connection = fixture.database.connection().expect("connection");
    let remaining: String = connection
        .query_row(
            "SELECT source_id FROM retrieval_documents WHERE source_kind = 'workspace_file'",
            [],
            |row| row.get(0),
        )
        .expect("remaining document");
    assert_eq!(remaining, "second-chunk");
}

#[test]
fn saving_configuration_is_normalized_and_invalid_updates_are_atomic() {
    let fixture = Fixture::new("code index configuration");
    let workspace = fixture.register();
    let saved = fixture
        .repository
        .save_configuration(
            &workspace.workspace_id,
            CodeIndexConfigurationUpdate {
                enabled: true,
                mode: crate::contexts::retrieval::domain::CodeIndexMode::Semantic,
                selected_roots: vec!["src\\app".to_string(), "src/app".to_string()],
                languages: vec![CodeLanguage::Rust, CodeLanguage::TypeScript],
                exclusion_patterns: vec!["vendor/**".to_string()],
                max_file_bytes: 64 * 1024,
            },
        )
        .expect("save valid configuration");
    assert!(saved.enabled);
    assert_eq!(
        saved.mode,
        crate::contexts::retrieval::domain::CodeIndexMode::Semantic
    );
    assert_eq!(saved.selected_roots, vec!["src/app"]);
    assert_eq!(saved.languages, vec!["rust", "typescript"]);
    assert_eq!(saved.phase, CodeIndexPhase::Scanning);
    assert_eq!(saved.generation, workspace.generation + 1);

    let invalid = fixture.repository.save_configuration(
        &workspace.workspace_id,
        CodeIndexConfigurationUpdate {
            enabled: true,
            mode: crate::contexts::retrieval::domain::CodeIndexMode::Semantic,
            selected_roots: vec![String::new()],
            languages: vec![CodeLanguage::Rust],
            exclusion_patterns: vec!["[invalid".to_string()],
            max_file_bytes: 20,
        },
    );
    assert!(matches!(invalid, Err(RetrievalError::Validation(_))));
    assert_eq!(
        fixture
            .repository
            .load_workspace(&workspace.workspace_id)
            .expect("load after rejection"),
        Some(saved)
    );
}

#[test]
fn file_replacement_redacts_code_before_retrieval_and_embedding_persistence() {
    let fixture = Fixture::new("code index persistence redaction");
    let workspace = fixture.register();
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/lib.rs", "raw-hash-only"),
            &[chunk(
                "secret-chunk",
                "secret-key",
                "const api_key = \"SENSITIVE-PERSISTENCE-SENTINEL\";",
            )],
            &[],
        )
        .expect("replace redacted file");

    let connection = fixture.database.connection().expect("connection");
    let (content, persisted_hash): (String, String) = connection
        .query_row(
            "SELECT content, content_hash FROM retrieval_documents WHERE source_id = 'secret-chunk'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("retrieval document");
    assert!(!content.contains("SENSITIVE-PERSISTENCE-SENTINEL"));
    assert!(content.contains("[REDACTED]"));
    assert_eq!(persisted_hash, content_hash(&content));
    let redactions: u32 = connection
        .query_row(
            "SELECT redaction_count FROM code_index_chunks WHERE document_id = 'workspace_file:secret-chunk'",
            [],
            |row| row.get(0),
        )
        .expect("redaction count");
    assert_eq!(redactions, 1);
}

#[test]
fn loading_a_stale_index_version_rebuilds_only_that_workspace() {
    let fixture = Fixture::new("code index version invalidation");
    let workspace = fixture.register();
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/lib.rs", "current"),
            &[chunk("stale-code", "stale", "fn stale() {}")],
            &[],
        )
        .expect("insert code");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            r#"
            INSERT INTO retrieval_documents
              (id, source_kind, source_id, scope_agent_id, scope_folder, content, content_hash,
               created_at, updated_at)
            VALUES ('agent_memory:retained', 'agent_memory', 'retained', '', '', 'memory',
                    'memory-hash', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')
            "#,
            [],
        )
        .expect("insert memory");
    connection
        .execute(
            "UPDATE code_index_workspaces SET index_version = 'old-version' WHERE workspace_id = ?1",
            [&workspace.workspace_id],
        )
        .expect("mark stale");
    drop(connection);

    let rebuilt = fixture
        .repository
        .load_workspace(&workspace.workspace_id)
        .expect("load")
        .expect("workspace");
    assert_eq!(rebuilt.index_version, CODE_INDEX_VERSION);
    assert_eq!(rebuilt.generation, workspace.generation + 1);
    let connection = fixture.database.connection().expect("connection");
    let code_documents: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'workspace_file'",
            [],
            |row| row.get(0),
        )
        .expect("code count");
    let memory_documents: u32 = connection
        .query_row(
            "SELECT COUNT(*) FROM retrieval_documents WHERE source_kind = 'agent_memory'",
            [],
            |row| row.get(0),
        )
        .expect("memory count");
    assert_eq!(code_documents, 0);
    assert_eq!(memory_documents, 1);
}

#[test]
fn workspace_status_reports_one_consistent_file_and_chunk_snapshot() {
    let fixture = Fixture::new("code index status");
    let workspace = fixture.register();
    let workspace = fixture
        .repository
        .save_configuration(
            &workspace.workspace_id,
            CodeIndexConfigurationUpdate {
                enabled: true,
                mode: crate::contexts::retrieval::domain::CodeIndexMode::Semantic,
                selected_roots: vec![String::new()],
                languages: vec![CodeLanguage::Rust],
                exclusion_patterns: Vec::new(),
                max_file_bytes: 100 * 1024,
            },
        )
        .expect("semantic configuration");
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/lib.rs", "status"),
            &[
                chunk("indexed-chunk", "indexed", "fn indexed() {}"),
                chunk("failed-chunk", "failed", "fn failed() {}"),
            ],
            &[],
        )
        .expect("replace");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE retrieval_documents SET index_state = 'indexed' WHERE source_id = 'indexed-chunk'",
            [],
        )
        .expect("mark indexed");
    connection
        .execute(
            "UPDATE retrieval_documents SET index_state = 'failed', failure_category = 'network' WHERE source_id = 'failed-chunk'",
            [],
        )
        .expect("mark failed");
    drop(connection);

    let status = fixture
        .repository
        .workspace_status(&workspace.workspace_id)
        .expect("status");
    assert_eq!(status.phase, CodeIndexPhase::Scanning);
    assert_eq!((status.processed_files, status.total_files), (1, 1));
    assert_eq!((status.processed_chunks, status.total_chunks), (2, 2));
    assert_eq!(status.pending_chunks, 0);
    assert_eq!((status.indexed_chunks, status.failed_chunks), (1, 1));
    assert_eq!(status.estimated_embedding_requests, 1);
    assert_eq!(status.last_failure_category, Some(FailureCategory::Network));
}

#[test]
fn workspace_statuses_batch_matches_per_workspace_status() {
    // The batched workspace_statuses query must return the same shape as calling
    // workspace_status once per workspace — the command-level N+1 it replaces relied on
    // per-workspace status, so any drift in the aggregated COUNT/window-function SQL would
    // silently change the workspace list page.
    let fixture = Fixture::new("code index status batch");
    let workspace = fixture
        .repository
        .save_configuration(
            &fixture.register().workspace_id,
            CodeIndexConfigurationUpdate {
                enabled: true,
                mode: crate::contexts::retrieval::domain::CodeIndexMode::Semantic,
                selected_roots: vec![String::new()],
                languages: vec![CodeLanguage::Rust],
                exclusion_patterns: Vec::new(),
                max_file_bytes: 100 * 1024,
            },
        )
        .expect("semantic configuration");
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/a.rs", "a"),
            &[
                chunk("a-indexed", "indexed", "fn a() {}"),
                chunk("a-failed", "failed", "fn af() {}"),
            ],
            &[],
        )
        .expect("replace a");
    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE retrieval_documents SET index_state = 'indexed' WHERE source_id = 'a-indexed'",
            [],
        )
        .expect("mark a indexed");
    connection
        .execute(
            "UPDATE retrieval_documents SET index_state = 'failed', failure_category = 'network' WHERE source_id = 'a-failed'",
            [],
        )
        .expect("mark a failed");
    drop(connection);

    let ids = vec![workspace.workspace_id.clone()];
    let batched = fixture
        .repository
        .workspace_statuses(&ids)
        .expect("batched statuses");
    let single = fixture
        .repository
        .workspace_status(&workspace.workspace_id)
        .expect("single status");

    assert_eq!(batched.len(), 1);
    assert_eq!(
        batched.get(&workspace.workspace_id),
        Some(&single),
        "batched status must match per-workspace status"
    );
    // The latest-failure window picks the same row as the per-workspace LIMIT-1 subquery.
    assert_eq!(
        batched[&workspace.workspace_id].last_failure_category,
        Some(FailureCategory::Network)
    );
}

#[test]
fn local_workspace_reports_searchable_chunks_without_embedding_work() {
    let fixture = Fixture::new("local code index status");
    let workspace = fixture.register();
    fixture
        .repository
        .replace_file(
            &manifest(&workspace.workspace_id, "src/lib.rs", "local"),
            &[chunk("local-chunk", "local", "fn local() {}")],
            &[],
        )
        .expect("replace");

    let status = fixture
        .repository
        .workspace_status(&workspace.workspace_id)
        .expect("status");
    assert_eq!((status.processed_chunks, status.total_chunks), (1, 1));
    assert_eq!(status.pending_chunks, 0);
    assert_eq!((status.indexed_chunks, status.failed_chunks), (1, 0));
    assert_eq!(status.estimated_embedding_requests, 0);

    let connection = fixture.database.connection().expect("connection");
    let state: String = connection
        .query_row(
            "SELECT index_state FROM retrieval_documents WHERE source_id = 'local-chunk'",
            [],
            |row| row.get(0),
        )
        .expect("document state");
    assert_eq!(state, "indexed");
}

#[test]
fn local_audit_is_path_safe_workspace_scoped_and_bounded() {
    let fixture = Fixture::new("code index audit");
    let workspace = fixture.register();
    let other_root = TempDirectory::new("code-index-audit-other");
    let other = fixture
        .repository
        .register_workspace(other_root.path(), "other")
        .expect("other workspace");

    assert_eq!(
        fixture.repository.record_audit(
            &workspace.workspace_id,
            Some("../secret.rs"),
            CodeIndexAuditEvent::Skipped,
            Some(CodeIndexAuditReason::SensitiveFile),
            1,
        ),
        Err(RetrievalError::InvalidScope)
    );
    fixture
        .repository
        .record_audit(
            &other.workspace_id,
            Some("src/other.rs"),
            CodeIndexAuditEvent::Indexed,
            None,
            1,
        )
        .expect("other audit");
    for item in 0..205 {
        fixture
            .repository
            .record_audit(
                &workspace.workspace_id,
                Some(&format!("src/{item}.rs")),
                CodeIndexAuditEvent::Skipped,
                Some(CodeIndexAuditReason::UserExcluded),
                1,
            )
            .expect("audit");
    }

    let connection = fixture.database.connection().expect("connection");
    let retained: i64 = connection
        .query_row(
            "SELECT COUNT(*) FROM code_index_audit WHERE workspace_id = ?1",
            [&workspace.workspace_id],
            |row| row.get(0),
        )
        .expect("retained count");
    assert_eq!(retained, MAX_AUDIT_ROWS_PER_WORKSPACE as i64);
    drop(connection);

    let records = fixture
        .repository
        .list_audit(&workspace.workspace_id, usize::MAX)
        .expect("audit list");
    assert_eq!(records.len(), MAX_AUDIT_QUERY_ROWS);
    assert!(records
        .iter()
        .all(|record| record.workspace_id == workspace.workspace_id));
    assert!(records.iter().all(|record| record
        .relative_path
        .as_deref()
        .is_some_and(|path| path.starts_with("src/"))));
}
