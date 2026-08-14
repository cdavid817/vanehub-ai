use super::*;
use crate::contexts::agent_runtime::application::{
    ArtifactRecord, ChangeSetApplyRecord, ChangeSetFileRecord, ChangeSetRecord, ChangeSetStatus,
    DelegationAttemptRecord, DelegationMode, DelegationRecord, DelegationStatus, DelegationTarget,
    FileChangeKind, RecoveryRecord, RecoveryStatus, StoredToolOperation, StoredToolOperationStatus,
};
use crate::test_support::TempDirectory;
use std::sync::{Arc, Barrier};

fn fixture() -> (TempDirectory, NativeDatabase, SqliteNativeToolRepository) {
    let directory = TempDirectory::new("native-tool-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    database
        .connection()
        .expect("connection")
        .execute(
            r#"
            INSERT INTO sessions (
                id, title, agent_id, interaction_mode, lifecycle_state, created_at, updated_at
            ) VALUES ('session-1', 'Tools', 'onepiece', 'api', 'idle', '100', '100')
            "#,
            [],
        )
        .expect("session");
    let repository = SqliteNativeToolRepository::new(database.clone());
    (directory, database, repository)
}

fn artifact(id: &str, hash: &str, sources: Vec<String>) -> ArtifactRecord {
    ArtifactRecord {
        contract_version: 1,
        id: id.to_owned(),
        content_hash: hash.to_owned(),
        media_type: "application/json".to_owned(),
        size_bytes: 42,
        display_name: format!("{id}.json"),
        source_operation_id: Some("operation-1".to_owned()),
        source_artifact_ids: sources,
        created_at: "101".to_owned(),
        expires_at: None,
        publication_ref: None,
    }
}

fn operation(status: StoredToolOperationStatus, sequence: u32) -> StoredToolOperation {
    StoredToolOperation {
        contract_version: 1,
        id: "operation-state".to_owned(),
        session_id: "session-1".to_owned(),
        generation_id: "generation-1".to_owned(),
        tool_name: "browser".to_owned(),
        status,
        progress_sequence: sequence,
        progress_message: None,
        result_artifact_ids: Vec::new(),
        error_code: None,
        created_at: "100".to_owned(),
        updated_at: sequence.to_string(),
    }
}

#[test]
fn operation_activity_progress_is_monotonic_and_terminal_states_are_sticky() {
    let (_directory, database, repository) = fixture();
    repository
        .save_operation(&operation(StoredToolOperationStatus::Running, 2))
        .expect("running");
    repository
        .save_operation(&operation(StoredToolOperationStatus::Queued, 1))
        .expect("stale progress ignored");
    repository
        .save_operation(&operation(StoredToolOperationStatus::Succeeded, 3))
        .expect("terminal");
    repository
        .save_operation(&operation(StoredToolOperationStatus::Running, 4))
        .expect("terminal regression ignored");

    let (status, sequence): (String, u32) = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status, progress_sequence FROM native_tool_operations WHERE id = 'operation-state'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .expect("activity");
    assert_eq!(status, "succeeded");
    assert_eq!(sequence, 3);
}

#[test]
fn all_native_tool_records_survive_database_reopen() {
    let (directory, database, repository) = fixture();
    repository
        .save_operation(&StoredToolOperation {
            contract_version: 1,
            id: "operation-1".to_owned(),
            session_id: "session-1".to_owned(),
            generation_id: "generation-1".to_owned(),
            tool_name: "delegate_cli".to_owned(),
            status: StoredToolOperationStatus::Running,
            progress_sequence: 1,
            progress_message: Some("Preparing".to_owned()),
            result_artifact_ids: Vec::new(),
            error_code: None,
            created_at: "100".to_owned(),
            updated_at: "101".to_owned(),
        })
        .expect("operation");
    repository
        .insert_artifact(&artifact("artifact-source", "sha256-source", Vec::new()))
        .expect("source artifact");
    repository
        .insert_artifact(&artifact("artifact-report", "sha256-report", Vec::new()))
        .expect("report artifact");
    repository
        .insert_artifact(&artifact(
            "artifact-change-set",
            "sha256-change-set",
            vec!["artifact-source".to_owned()],
        ))
        .expect("change set artifact");
    repository
        .save_delegation(&DelegationRecord {
            contract_version: 1,
            id: "delegation-1".to_owned(),
            session_id: "session-1".to_owned(),
            task_hash: "sha256-task".to_owned(),
            status: DelegationStatus::Running,
            created_at: "102".to_owned(),
            updated_at: "102".to_owned(),
        })
        .expect("delegation");
    repository
        .save_delegation_attempt(&DelegationAttemptRecord {
            contract_version: 1,
            id: "attempt-1".to_owned(),
            delegation_id: "delegation-1".to_owned(),
            attempt_number: 1,
            target: DelegationTarget::CodexCli,
            mode: DelegationMode::Edit,
            status: DelegationStatus::Succeeded,
            safe_summary: Some("Changed one file".to_owned()),
            report_artifact_id: Some("artifact-report".to_owned()),
            change_set_artifact_id: Some("artifact-change-set".to_owned()),
            error_code: None,
            started_at: Some("103".to_owned()),
            completed_at: Some("104".to_owned()),
        })
        .expect("attempt");
    repository
        .insert_change_set(&ChangeSetRecord {
            contract_version: 1,
            artifact_id: "artifact-change-set".to_owned(),
            content_hash: "sha256-change-set".to_owned(),
            repository_identity: "repo-1".to_owned(),
            base_commit: "abc123".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            files: vec![ChangeSetFileRecord {
                path: "src/main.rs".to_owned(),
                change_kind: FileChangeKind::Modify,
                old_hash: Some("old".to_owned()),
                new_hash: Some("new".to_owned()),
                binary: false,
                mode: Some("100644".to_owned()),
            }],
            warnings: vec!["report differs from host evidence".to_owned()],
            created_at: "104".to_owned(),
        })
        .expect("change set");
    repository
        .save_apply_attempt(&ChangeSetApplyRecord {
            contract_version: 1,
            id: "apply-1".to_owned(),
            change_set_artifact_id: "artifact-change-set".to_owned(),
            target_repository_identity: "repo-1".to_owned(),
            expected_base_commit: "abc123".to_owned(),
            approval_input_hash: "sha256-approval".to_owned(),
            status: ChangeSetStatus::RolledBack,
            error_code: Some("verification_failed".to_owned()),
            consumed_at: None,
            created_at: "105".to_owned(),
            updated_at: "106".to_owned(),
        })
        .expect("apply attempt");
    repository
        .save_recovery(&RecoveryRecord {
            contract_version: 1,
            apply_attempt_id: "apply-1".to_owned(),
            status: RecoveryStatus::RolledBack,
            recovery_reference: Some("recovery-1".to_owned()),
            safe_instructions: vec!["Review the target worktree.".to_owned()],
            updated_at: "106".to_owned(),
        })
        .expect("recovery");
    drop(repository);
    drop(database);

    let reopened = NativeDatabase::new(directory.path().to_path_buf()).expect("reopen");
    let connection = reopened.connection().expect("connection");
    for (table, expected) in [
        ("native_tool_operations", 1_i64),
        ("native_tool_artifacts", 3),
        ("native_tool_artifact_lineage", 1),
        ("native_tool_delegations", 1),
        ("native_tool_delegation_attempts", 1),
        ("native_tool_change_sets", 1),
        ("native_tool_change_set_files", 1),
        ("native_tool_apply_attempts", 1),
        ("native_tool_apply_recovery", 1),
    ] {
        let count: i64 = connection
            .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .expect("row count");
        assert_eq!(count, expected, "unexpected row count for {table}");
    }
    let statuses: (String, String, String) = connection
        .query_row(
            "SELECT d.status, a.status, r.status FROM native_tool_delegations d \
             JOIN native_tool_apply_attempts a ON a.id = 'apply-1' \
             JOIN native_tool_apply_recovery r ON r.apply_attempt_id = a.id \
             WHERE d.id = 'delegation-1'",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("stored statuses");
    assert_eq!(
        statuses,
        (
            "running".to_owned(),
            "rolled_back".to_owned(),
            "rolled_back".to_owned()
        )
    );
}

#[test]
fn concurrent_operation_writers_remain_bounded_and_durable() {
    const WRITERS: usize = 8;
    const OPERATIONS_PER_WRITER: usize = 25;
    let (_directory, database, repository) = fixture();
    let barrier = Arc::new(Barrier::new(WRITERS));
    let mut threads = Vec::new();
    for writer in 0..WRITERS {
        let repository = repository.clone();
        let barrier = barrier.clone();
        threads.push(std::thread::spawn(move || {
            barrier.wait();
            for item in 0..OPERATIONS_PER_WRITER {
                let id = format!("stress-{writer}-{item}");
                let mut record = operation(StoredToolOperationStatus::Running, 1);
                record.id = id;
                repository
                    .save_operation(&record)
                    .expect("concurrent write");
            }
        }));
    }
    for thread in threads {
        thread.join().expect("writer thread");
    }

    let count: i64 = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT COUNT(*) FROM native_tool_operations WHERE id LIKE 'stress-%'",
            [],
            |row| row.get(0),
        )
        .expect("operation count");
    assert_eq!(count, (WRITERS * OPERATIONS_PER_WRITER) as i64);
}

#[test]
fn schema_rejects_invalid_attempt_number() {
    let (_directory, database, _repository) = fixture();
    let connection = database.connection().expect("connection");
    connection
        .execute(
            "INSERT INTO native_tool_delegations \
             (id, contract_version, session_id, task_hash, status, created_at, updated_at) \
             VALUES ('delegation-guard', 1, 'session-1', 'hash', 'queued', '1', '1')",
            [],
        )
        .expect("delegation");
    let result = connection.execute(
        "INSERT INTO native_tool_delegation_attempts \
         (id, contract_version, delegation_id, attempt_number, target, mode, status) \
         VALUES ('attempt-invalid', 1, 'delegation-guard', 4, 'codex_cli', 'edit', 'queued')",
        [],
    );
    assert!(result.is_err());
}

#[test]
fn delegation_persistence_does_not_regress_terminal_state() {
    let (_directory, database, repository) = fixture();
    let mut delegation = DelegationRecord {
        contract_version: 1,
        id: "delegation-terminal".to_owned(),
        session_id: "session-1".to_owned(),
        task_hash: "sha256:task".to_owned(),
        status: DelegationStatus::Succeeded,
        created_at: "1".to_owned(),
        updated_at: "2".to_owned(),
    };
    repository.save_delegation(&delegation).expect("terminal");
    delegation.status = DelegationStatus::Running;
    delegation.updated_at = "3".to_owned();
    repository
        .save_delegation(&delegation)
        .expect("stale update");

    let status: String = database
        .connection()
        .expect("connection")
        .query_row(
            "SELECT status FROM native_tool_delegations WHERE id = ?1",
            ["delegation-terminal"],
            |row| row.get(0),
        )
        .expect("status");
    assert_eq!(status, "succeeded");
}
