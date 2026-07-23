use super::SqliteLoopRepository;
use crate::contexts::agent_runtime::application::{
    LoopEvidenceView, LoopIterationRepository, LoopIterationView, LoopRepository,
};
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun, LoopRunPhase, LoopRunStatus,
    LoopVerificationCommand,
};
use crate::platform::database::NativeDatabase;
use crate::test_support::TempDirectory;

fn definition() -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-control".to_string(),
        name: "Control Loop".to_string(),
        enabled: true,
        project_path: "D:/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Implement controls".to_string(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec![".git".to_string()],
        worker_agent_id: "codex-cli".to_string(),
        verifier_agent_id: "claude-code".to_string(),
        verification_commands: vec![LoopVerificationCommand::new(
            "tests".to_string(),
            "npm".to_string(),
            vec!["test".to_string()],
            None,
            60,
            true,
        )
        .expect("command")],
        limits: LoopLimits::new(3, 60, 600, 2, 2).expect("limits"),
        version: 1,
        created_at: "2026-07-22T10:00:00Z".to_string(),
        updated_at: "2026-07-22T10:00:00Z".to_string(),
    })
    .expect("definition")
}

fn iteration() -> LoopIterationView {
    LoopIterationView {
        id: "iteration-control-1".to_string(),
        run_id: "run-control-1".to_string(),
        sequence: 1,
        status: LoopRunStatus::AwaitingAcceptance,
        worker_session_id: None,
        verifier_session_id: None,
        worker_summary: None,
        verifier_recommendation: None,
        verifier_findings: Vec::new(),
        decision_reason: None,
        diff_fingerprint: None,
        check_failure_fingerprint: None,
        user_feedback: None,
        evidence: Vec::new(),
        started_at: "2026-07-22T10:00:01Z".to_string(),
        completed_at: Some("2026-07-22T10:00:02Z".to_string()),
    }
}

#[test]
fn control_updates_use_pause_cas_and_atomic_feedback_transition() {
    let directory = TempDirectory::new("loop-control-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteLoopRepository::new(database.clone());
    let definition = definition();
    repository
        .create_definition(&definition)
        .expect("definition");
    let mut run =
        LoopRun::new("run-control-1".to_string(), "loop-control".to_string()).expect("run");
    repository
        .create_run(&run, &definition, "D:/project", "2026-07-22T10:00:00Z")
        .expect("create run");

    run.request_pause().expect("pause request");
    repository
        .save_pause_request(&run, LoopRunStatus::Queued, false, "2026-07-22T10:00:01Z")
        .expect("save pause");
    assert!(repository
        .save_pause_request(&run, LoopRunStatus::Queued, false, "2026-07-22T10:00:01Z",)
        .is_err());
    assert_eq!(
        repository
            .find_run_definition_snapshot("run-control-1")
            .expect("snapshot")
            .expect("definition")
            .values()
            .version,
        1
    );

    run.pause_at_boundary().expect("pause boundary");
    repository
        .save_run_transition(&run, LoopRunStatus::Queued, "2026-07-22T10:00:02Z", None)
        .expect("save paused");
    run.resume().expect("resume");
    repository
        .save_run_transition(&run, LoopRunStatus::Paused, "2026-07-22T10:00:03Z", None)
        .expect("save resumed");
    run.begin().expect("begin");
    repository
        .save_run_transition(&run, LoopRunStatus::Queued, "2026-07-22T10:00:03Z", None)
        .expect("save begin");
    run.move_to(LoopRunPhase::Verifying).expect("verifying");
    run.move_to(LoopRunPhase::Deciding).expect("deciding");
    run.await_acceptance(true).expect("await acceptance");
    repository
        .save_run_transition(&run, LoopRunStatus::Running, "2026-07-22T10:00:04Z", None)
        .expect("save acceptance");
    repository
        .insert_iteration(&iteration())
        .expect("iteration");
    repository
        .append_evidence(&LoopEvidenceView {
            id: "evidence-control-1".to_string(),
            run_id: run.id().to_string(),
            iteration_id: Some("iteration-control-1".to_string()),
            kind: "verification".to_string(),
            status: "passed".to_string(),
            summary: "Tests passed".to_string(),
            operation_id: Some("operation-control-1".to_string()),
            command_id: Some("tests".to_string()),
            exit_code: Some(0),
            duration_ms: Some(42),
            details: Some(serde_json::json!({ "required": true })),
            created_at: "2026-07-22T10:00:04Z".to_string(),
        })
        .expect("evidence");

    run.continue_iteration(&definition.values().limits)
        .expect("continue");
    repository
        .save_continue_transition(
            &run,
            LoopRunStatus::AwaitingAcceptance,
            "Add a regression test",
            "2026-07-22T10:00:05Z",
        )
        .expect("save continue");
    assert!(repository
        .save_continue_transition(
            &run,
            LoopRunStatus::AwaitingAcceptance,
            "Duplicate",
            "2026-07-22T10:00:06Z",
        )
        .is_err());

    let connection = database.connection().expect("connection");
    let stored: (String, i64, String) = connection
        .query_row(
            r#"SELECT r.status, r.current_iteration, i.user_feedback
               FROM loop_runs r JOIN loop_iterations i ON i.run_id = r.id
               WHERE r.id = 'run-control-1' AND i.sequence = 1"#,
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .expect("stored control state");
    assert_eq!(
        stored,
        (
            "running".to_string(),
            2,
            "Add a regression test".to_string()
        )
    );

    let views = repository
        .list_run_views(Some("loop-control"))
        .expect("run views");
    assert_eq!(views.len(), 1);
    assert_eq!(views[0].definition_snapshot.version, 1);
    assert_eq!(
        views[0].iterations[0].user_feedback.as_deref(),
        Some("Add a regression test")
    );
    assert_eq!(
        views[0].iterations[0].evidence[0].command_id.as_deref(),
        Some("tests")
    );
    assert_eq!(
        repository
            .find_run_view("run-control-1")
            .expect("run view")
            .expect("stored run view"),
        views[0]
    );
}

#[test]
fn recovery_transition_is_atomic_and_resumable_after_confirmation() {
    let directory = TempDirectory::new("loop-recovery-repository");
    let database = NativeDatabase::new(directory.path().to_path_buf()).expect("database");
    let repository = SqliteLoopRepository::new(database.clone());
    let definition = definition();
    repository
        .create_definition(&definition)
        .expect("definition");
    let mut run =
        LoopRun::new("run-recovery-1".to_string(), "loop-control".to_string()).expect("run");
    repository
        .create_run(&run, &definition, "D:/project", "2026-07-22T10:00:00Z")
        .expect("create run");
    assert_eq!(repository.list_recoverable_runs().expect("runs").len(), 1);

    run.recover_orphaned().expect("recover");
    let evidence = LoopEvidenceView {
        id: "recovery-evidence-1".to_string(),
        run_id: run.id().to_string(),
        iteration_id: None,
        kind: "recovery".to_string(),
        status: "blocked".to_string(),
        summary: "Explicit confirmation is required.".to_string(),
        operation_id: Some("recovery-operation-1".to_string()),
        command_id: None,
        exit_code: None,
        duration_ms: None,
        details: Some(serde_json::json!({ "reason": "recovery-required" })),
        created_at: "2026-07-22T10:00:01Z".to_string(),
    };
    repository
        .save_recovery_transition(
            &run,
            LoopRunStatus::Queued,
            &evidence,
            "2026-07-22T10:00:01Z",
        )
        .expect("save recovery");
    assert!(repository
        .save_recovery_transition(
            &run,
            LoopRunStatus::Queued,
            &evidence,
            "2026-07-22T10:00:01Z",
        )
        .is_err());
    assert!(repository.list_recoverable_runs().expect("runs").is_empty());
    assert_eq!(
        database
            .connection()
            .expect("connection")
            .query_row(
                "SELECT COUNT(*) FROM loop_evidence WHERE run_id = 'run-recovery-1'",
                [],
                |row| row.get::<_, i64>(0),
            )
            .expect("evidence count"),
        1
    );

    run.resume().expect("confirmed resume");
    repository
        .save_run_transition(&run, LoopRunStatus::Paused, "2026-07-22T10:00:02Z", None)
        .expect("save resume");
    assert_eq!(
        repository
            .find_run("run-recovery-1")
            .expect("find")
            .expect("run")
            .status(),
        LoopRunStatus::Queued
    );
}
