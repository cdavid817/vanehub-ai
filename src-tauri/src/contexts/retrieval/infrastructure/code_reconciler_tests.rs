use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::contexts::retrieval::domain::code_index::DEFAULT_MAX_FILE_BYTES;
use crate::contexts::retrieval::domain::{CodeIndexConfigurationUpdate, CodeLanguage};
use crate::contexts::retrieval::infrastructure::code_index_repository::SqliteCodeIndexRepository;
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

struct Fixture {
    _database_directory: TempDirectory,
    workspace_directory: TempDirectory,
    database: NativeDatabase,
    repository: SqliteCodeIndexRepository,
    workspace: CodeWorkspace,
}

impl Fixture {
    fn new(languages: Vec<CodeLanguage>) -> Self {
        let database_directory = TempDirectory::new("code-reconcile-database");
        let workspace_directory = TempDirectory::new("code-reconcile-workspace");
        let database =
            NativeDatabase::new(database_directory.path().to_path_buf()).expect("database");
        let repository = SqliteCodeIndexRepository::new(database.clone());
        let registered = repository
            .register_workspace(workspace_directory.path(), "workspace")
            .expect("register");
        let workspace = repository
            .save_configuration(
                &registered.workspace_id,
                CodeIndexConfigurationUpdate {
                    enabled: true,
                    selected_roots: vec![String::new()],
                    languages,
                    exclusion_patterns: Vec::new(),
                    max_file_bytes: DEFAULT_MAX_FILE_BYTES,
                },
            )
            .expect("enable");
        Self {
            repository,
            database,
            workspace_directory,
            workspace,
            _database_directory: database_directory,
        }
    }

    fn reconcile(&self) -> CodeReconcileOutcome {
        reconcile_workspace(&self.repository, &self.workspace).expect("reconcile")
    }

    fn source_ids(&self) -> Vec<String> {
        let connection = self.database.connection().expect("connection");
        let source_ids = connection
            .prepare(
                "SELECT source_id FROM retrieval_documents WHERE source_kind = 'workspace_file' ORDER BY source_id",
            )
            .expect("prepare")
            .query_map([], |row| row.get(0))
            .expect("query")
            .collect::<Result<_, _>>()
            .expect("collect");
        source_ids
    }
}

#[test]
fn inventory_reconcile_reads_only_new_or_fingerprint_changed_files() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    fixture
        .workspace_directory
        .write("src/lib.rs", "fn initial() {}");

    assert_eq!(
        fixture.reconcile(),
        CodeReconcileOutcome {
            discovered: 1,
            replaced: 1,
            ..CodeReconcileOutcome::default()
        }
    );
    let initial_ids = fixture.source_ids();
    assert_eq!(
        fixture.reconcile(),
        CodeReconcileOutcome {
            discovered: 1,
            unchanged: 1,
            ..CodeReconcileOutcome::default()
        }
    );
    assert_eq!(fixture.source_ids(), initial_ids);

    let connection = fixture.database.connection().expect("connection");
    connection
        .execute(
            "UPDATE code_index_files SET modified_ns = -1 WHERE workspace_id = ?1",
            [&fixture.workspace.workspace_id],
        )
        .expect("force metadata mismatch");
    drop(connection);
    assert_eq!(
        fixture.reconcile(),
        CodeReconcileOutcome {
            discovered: 1,
            metadata_updated: 1,
            ..CodeReconcileOutcome::default()
        }
    );
    assert_eq!(fixture.source_ids(), initial_ids);
}

#[test]
fn create_change_delete_and_rename_replace_only_affected_path_sets() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    let first = fixture
        .workspace_directory
        .write("src/first.rs", "fn old_name() {}");
    fixture.reconcile();
    let old_ids = fixture.source_ids();

    std::fs::write(&first, "fn new_name_with_different_size() {}").expect("change first");
    fixture
        .workspace_directory
        .write("src/second.rs", "fn second() {}");
    assert_eq!(
        fixture.reconcile(),
        CodeReconcileOutcome {
            discovered: 2,
            replaced: 2,
            ..CodeReconcileOutcome::default()
        }
    );
    assert!(fixture
        .source_ids()
        .iter()
        .all(|source_id| !old_ids.contains(source_id)));

    std::fs::remove_file(&first).expect("delete first");
    let second = fixture.workspace_directory.path().join("src/second.rs");
    let renamed = fixture.workspace_directory.path().join("src/renamed.rs");
    std::fs::rename(second, renamed).expect("rename second");
    assert_eq!(
        fixture.reconcile(),
        CodeReconcileOutcome {
            discovered: 1,
            replaced: 1,
            deleted: 2,
            ..CodeReconcileOutcome::default()
        }
    );
    let manifests = fixture
        .repository
        .list_file_manifests(&fixture.workspace.workspace_id)
        .expect("manifests");
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].relative_path, "src/renamed.rs");
}

#[test]
fn disabling_a_language_removes_its_existing_manifests_and_chunks() {
    let mut fixture = Fixture::new(vec![CodeLanguage::Rust, CodeLanguage::TypeScript]);
    fixture
        .workspace_directory
        .write("src/lib.rs", "fn kept() {}");
    fixture
        .workspace_directory
        .write("src/app.ts", "function removed() {}");
    fixture.reconcile();

    fixture.workspace = fixture
        .repository
        .save_configuration(
            &fixture.workspace.workspace_id,
            CodeIndexConfigurationUpdate {
                enabled: true,
                selected_roots: vec![String::new()],
                languages: vec![CodeLanguage::Rust],
                exclusion_patterns: Vec::new(),
                max_file_bytes: DEFAULT_MAX_FILE_BYTES,
            },
        )
        .expect("disable typescript");
    let outcome = fixture.reconcile();
    assert_eq!(outcome.unchanged, 1);
    assert_eq!(outcome.deleted, 1);
    let manifests = fixture
        .repository
        .list_file_manifests(&fixture.workspace.workspace_id)
        .expect("manifests");
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].language, "rust");
}

#[test]
fn targeted_reconcile_handles_bounded_create_change_delete_and_rename_sets() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    fixture
        .workspace_directory
        .write("src/change.rs", "fn old() {}");
    fixture
        .workspace_directory
        .write("src/delete.rs", "fn deleted() {}");
    fixture
        .workspace_directory
        .write("src/rename.rs", "fn renamed() {}");
    fixture
        .workspace_directory
        .write("src/untouched.rs", "fn untouched() {}");
    fixture.reconcile();

    std::fs::write(
        fixture.workspace_directory.path().join("src/change.rs"),
        "fn changed_with_new_size() {}",
    )
    .expect("change");
    std::fs::remove_file(fixture.workspace_directory.path().join("src/delete.rs")).expect("delete");
    std::fs::rename(
        fixture.workspace_directory.path().join("src/rename.rs"),
        fixture.workspace_directory.path().join("src/renamed.rs"),
    )
    .expect("rename");
    fixture
        .workspace_directory
        .write("src/create.rs", "fn created() {}");

    let outcome = reconcile_paths(
        &fixture.repository,
        &fixture.workspace,
        &[
            CodePathChange::Upsert("src/change.rs".to_string()),
            CodePathChange::Delete("src/delete.rs".to_string()),
            CodePathChange::Rename {
                from: "src/rename.rs".to_string(),
                to: "src/renamed.rs".to_string(),
            },
            CodePathChange::Upsert("src/create.rs".to_string()),
        ],
    )
    .expect("targeted reconcile");
    assert_eq!(
        outcome,
        CodeReconcileOutcome {
            discovered: 3,
            replaced: 3,
            deleted: 2,
            ..CodeReconcileOutcome::default()
        }
    );
    let paths = fixture
        .repository
        .list_file_manifests(&fixture.workspace.workspace_id)
        .expect("manifests")
        .into_iter()
        .map(|manifest| manifest.relative_path)
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            "src/change.rs",
            "src/create.rs",
            "src/renamed.rs",
            "src/untouched.rs",
        ]
    );
}

#[test]
fn targeted_reconcile_hashes_explicit_paths_and_rejects_escape_sets() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    fixture
        .workspace_directory
        .write("src/lib.rs", "fn stable() {}");
    fixture.reconcile();
    let outcome = reconcile_paths(
        &fixture.repository,
        &fixture.workspace,
        &[CodePathChange::Upsert("src/lib.rs".to_string())],
    )
    .expect("explicit hash");
    assert_eq!(outcome.metadata_updated, 1);
    assert_eq!(outcome.unchanged, 0);

    assert_eq!(
        reconcile_paths(
            &fixture.repository,
            &fixture.workspace,
            &[CodePathChange::Delete("../outside.rs".to_string())],
        ),
        Err(RetrievalError::InvalidScope)
    );
    assert!(matches!(
        reconcile_paths(
            &fixture.repository,
            &fixture.workspace,
            &vec![CodePathChange::Upsert("src/lib.rs".to_string()); 513],
        ),
        Err(RetrievalError::Validation(_))
    ));
}

struct CancelAfterChecks {
    checks: AtomicUsize,
    allowed_checks: usize,
}

impl CodeIndexCancellation for CancelAfterChecks {
    fn is_cancelled(&self) -> bool {
        self.checks.fetch_add(1, Ordering::SeqCst) >= self.allowed_checks
    }
}

#[test]
fn reconciliation_stops_between_files_without_deleting_remaining_manifests() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    fixture.workspace_directory.write("a.rs", "fn a() {}");
    fixture.workspace_directory.write("b.rs", "fn b() {}");
    let cancellation = CancelAfterChecks {
        checks: AtomicUsize::new(0),
        allowed_checks: 3,
    };
    let outcome =
        reconcile_workspace_cancellable(&fixture.repository, &fixture.workspace, &cancellation)
            .expect("cancelled reconcile");
    assert!(outcome.cancelled);
    assert_eq!(outcome.replaced, 1);
    assert_eq!(
        fixture
            .repository
            .list_file_manifests(&fixture.workspace.workspace_id)
            .expect("manifests")
            .len(),
        1
    );
}

#[test]
fn a_stale_workspace_generation_discards_work_before_persistence() {
    let fixture = Fixture::new(vec![CodeLanguage::Rust]);
    fixture
        .workspace_directory
        .write("src/lib.rs", "fn stale() {}");
    fixture
        .repository
        .save_configuration(
            &fixture.workspace.workspace_id,
            fixture.workspace.configuration().expect("configuration"),
        )
        .expect("increment generation");

    let outcome = fixture.reconcile();
    assert!(outcome.cancelled);
    assert_eq!(outcome.replaced, 0);
    assert!(fixture.source_ids().is_empty());
}
