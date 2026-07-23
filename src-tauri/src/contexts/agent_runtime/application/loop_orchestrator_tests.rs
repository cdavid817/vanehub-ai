use super::*;
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun, LoopRunPhase, LoopRunStatus,
    LoopVerificationCommand,
};
use serde_json::json;
use std::sync::{Arc, Mutex};

struct OrchestratorWorld {
    run: Mutex<LoopRun>,
    definition: LoopDefinition,
    iteration: Mutex<LoopIterationView>,
    terminal: Mutex<Option<LoopRoleGenerationTerminal>>,
    operations: Mutex<Vec<LoopOperationContext>>,
    logs: Mutex<Vec<LoopLog>>,
}

impl OrchestratorWorld {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            run: Mutex::new(
                LoopRun::rehydrate(
                    "run-1".to_string(),
                    "loop-1".to_string(),
                    LoopRunStatus::Running,
                    LoopRunPhase::Deciding,
                    None,
                    1,
                    0,
                    0,
                    false,
                )
                .expect("run"),
            ),
            definition: definition(),
            iteration: Mutex::new(LoopIterationView {
                id: "iteration-1".to_string(),
                run_id: "run-1".to_string(),
                sequence: 1,
                status: LoopRunStatus::Running,
                worker_session_id: Some("worker-session".to_string()),
                verifier_session_id: None,
                worker_summary: Some("Implemented the first revision.".to_string()),
                verifier_recommendation: None,
                verifier_findings: Vec::new(),
                decision_reason: None,
                diff_fingerprint: None,
                check_failure_fingerprint: None,
                user_feedback: Some("Keep the public API stable.".to_string()),
                evidence: vec![required_failure()],
                started_at: "2099-01-01T00:00:00Z".to_string(),
                completed_at: None,
            }),
            terminal: Mutex::new(None),
            operations: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
        })
    }

    fn view(&self) -> LoopRunView {
        let run = self.run.lock().expect("run").clone();
        LoopRunView {
            id: run.id().to_string(),
            definition_id: run.definition_id().to_string(),
            definition_snapshot: LoopDefinitionView::from(&self.definition),
            status: run.status(),
            phase: run.phase(),
            terminal_reason: run.terminal_reason(),
            current_iteration: run.current_iteration(),
            consecutive_runtime_errors: run.consecutive_runtime_errors(),
            consecutive_no_progress: run.consecutive_no_progress(),
            pause_requested: run.pause_requested(),
            project_path: "D:/project".to_string(),
            worktree_path: Some("D:/project-loop".to_string()),
            worktree_name: Some("loop-run-1".to_string()),
            worktree_branch: Some("loop/run-1".to_string()),
            active_operation_id: None,
            iterations: vec![self.iteration.lock().expect("iteration").clone()],
            simulated: false,
            created_at: "2099-01-01T00:00:00Z".to_string(),
            started_at: Some("2099-01-01T00:00:00Z".to_string()),
            updated_at: "2099-01-01T00:00:00Z".to_string(),
            completed_at: None,
        }
    }

    fn service(self: &Arc<Self>) -> LoopOrchestratorApplicationService {
        let observer = LoopOperationObserver::new(self.clone(), self.clone(), self.clone());
        let worker = LoopWorkerApplicationService::new(LoopWorkerApplicationPorts {
            iterations: self.clone(),
            roles: self.clone(),
            git: self.clone(),
            generations: self.clone(),
            clock: self.clone(),
        });
        let verification =
            LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
                iterations: self.clone(),
                processes: self.clone(),
                observer: observer.clone(),
                clock: self.clone(),
            });
        let verifier = LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
            iterations: self.clone(),
            roles: self.clone(),
            context: self.clone(),
            generations: self.clone(),
        });
        LoopOrchestratorApplicationService::new(LoopOrchestratorPorts {
            loops: self.clone(),
            iterations: self.clone(),
            projects: self.clone(),
            verifier_context: self.clone(),
            completions: self.clone(),
            generations: self.clone(),
            worker,
            verification,
            verifier,
            progress: LoopProgressApplicationService::new(self.clone()),
            observer,
            clock: self.clone(),
        })
    }
}

impl LoopRepository for OrchestratorWorld {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![self.definition.clone()])
    }

    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((definition_id == "loop-1").then(|| self.definition.clone()))
    }

    fn create_definition(&self, _: &LoopDefinition) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn update_definition(
        &self,
        _: &LoopDefinition,
        _: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn delete_definition(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn create_run(
        &self,
        _: &LoopRun,
        _: &LoopDefinition,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn has_active_run(&self, _: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(true)
    }

    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        let run = self.run.lock().expect("run");
        Ok((run.id() == run_id).then(|| run.clone()))
    }

    fn find_run_view(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopRunView>, AgentRuntimeApplicationError> {
        Ok((run_id == "run-1").then(|| self.view()))
    }

    fn attach_run_operation(
        &self,
        _: &str,
        _: &str,
        _: LoopRunStatus,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_run_worktree(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn save_run_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        assert_eq!(stored.status(), expected_status);
        *stored = run.clone();
        Ok(())
    }

    fn find_run_definition_snapshot(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((run_id == "run-1").then(|| self.definition.clone()))
    }
}

impl LoopIterationRepository for OrchestratorWorld {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn attach_verifier_session(
        &self,
        iteration_id: &str,
        session_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut iteration = self.iteration.lock().expect("iteration");
        assert_eq!(iteration.id, iteration_id);
        iteration.verifier_session_id = Some(session_id.to_string());
        Ok(())
    }

    fn save_verifier_result(
        &self,
        request: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut iteration = self.iteration.lock().expect("iteration");
        assert_eq!(iteration.id, request.iteration_id);
        assert_eq!(
            iteration.verifier_session_id.as_deref(),
            Some(request.session_id.as_str())
        );
        iteration.verifier_recommendation =
            Some(request.result.recommendation.as_str().to_string());
        iteration.verifier_findings = request.result.findings.clone();
        Ok(())
    }

    fn complete_iteration(
        &self,
        run_id: &str,
        iteration_id: &str,
        status: LoopRunStatus,
        decision_reason: &str,
        completed_at: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut iteration = self.iteration.lock().expect("iteration");
        assert_eq!(iteration.run_id, run_id);
        assert_eq!(iteration.id, iteration_id);
        iteration.status = status;
        iteration.decision_reason = Some(decision_reason.to_string());
        iteration.completed_at = Some(completed_at.to_string());
        Ok(())
    }

    fn save_iteration_fingerprints(
        &self,
        run_id: &str,
        iteration_id: &str,
        diff_fingerprint: &str,
        check_failure_fingerprint: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut iteration = self.iteration.lock().expect("iteration");
        assert_eq!(iteration.run_id, run_id);
        assert_eq!(iteration.id, iteration_id);
        iteration.diff_fingerprint = Some(diff_fingerprint.to_string());
        iteration.check_failure_fingerprint = Some(check_failure_fingerprint.to_string());
        Ok(())
    }

    fn append_evidence(&self, _: &LoopEvidenceView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopProjectPort for OrchestratorWorld {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        Ok(project_path.to_string())
    }
}

impl LoopVerifierContextPort for OrchestratorWorld {
    fn bounded_diff(&self, session_id: &str) -> Result<String, AgentRuntimeApplicationError> {
        assert_eq!(session_id, "verifier-session");
        Ok("diff --git a/src/lib.rs b/src/lib.rs\n+fixed".to_string())
    }
}

impl LoopRoleSessionPort for OrchestratorWorld {
    fn create_worker_session(
        &self,
        _: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn create_verifier_session(
        &self,
        request: LoopRoleSessionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        assert_eq!(request.run_id, "run-1");
        assert_eq!(request.iteration_id, "iteration-1");
        Ok("verifier-session".to_string())
    }
}

impl LoopVerifierGenerationPort for OrchestratorWorld {
    fn start_verifier_generation(
        &self,
        session_id: &str,
        _: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        *self.terminal.lock().expect("terminal") = Some(LoopRoleGenerationTerminal {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            role: "verifier".to_string(),
            session_id: session_id.to_string(),
            message_id: "message-1".to_string(),
            outcome: LoopRoleGenerationOutcome::Completed,
            content: Some(
                json!({"recommendation": "revise", "findings": ["Fix the required check."]})
                    .to_string(),
            ),
            error: None,
        });
        Ok("message-1".to_string())
    }
}

impl LoopRoleGenerationCompletionPort for OrchestratorWorld {
    fn deliver(
        &self,
        terminal: LoopRoleGenerationTerminal,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        *self.terminal.lock().expect("terminal") = Some(terminal);
        Ok(true)
    }

    fn take_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<LoopRoleGenerationTerminal>, AgentRuntimeApplicationError> {
        let mut terminal = self.terminal.lock().expect("terminal");
        Ok(terminal
            .take()
            .filter(|completion| completion.session_id == session_id))
    }
}

impl LoopGenerationControlPort for OrchestratorWorld {
    fn stop_loop_generation(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopGitStatePort for OrchestratorWorld {
    fn snapshot(&self, _: &str) -> Result<LoopGitStateView, AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopWorkerGenerationPort for OrchestratorWorld {
    fn start_worker_generation(
        &self,
        _: &str,
        _: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopVerificationProcessPort for OrchestratorWorld {
    fn execute(
        &self,
        _: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl AgentTaskPort for OrchestratorWorld {
    fn start_agent_launch(
        &self,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn start_agent_generation(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }

    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        message: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        let mut operations = self.operations.lock().expect("operations");
        let id = format!("operation-{}", operations.len() + 1);
        operations.push(context.clone());
        Ok(AgentOperation {
            id,
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn complete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn fail(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn cancel(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
}

impl LoopLoggingPort for OrchestratorWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for OrchestratorWorld {
    fn now(&self) -> String {
        "2099-01-01T00:01:00Z".to_string()
    }
}

fn definition() -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-1".to_string(),
        name: "Loop".to_string(),
        enabled: true,
        project_path: "D:/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Fix the project".to_string(),
        acceptance_criteria: vec!["Required tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec![".git".to_string()],
        worker_agent_id: "worker".to_string(),
        verifier_agent_id: "verifier".to_string(),
        verification_commands: vec![LoopVerificationCommand::new(
            "tests".to_string(),
            "npm".to_string(),
            vec!["test".to_string()],
            None,
            60,
            true,
        )
        .expect("command")],
        limits: LoopLimits::new(3, 60, 600, 1, 2).expect("limits"),
        version: 1,
        created_at: "2099-01-01T00:00:00Z".to_string(),
        updated_at: "2099-01-01T00:00:00Z".to_string(),
    })
    .expect("definition")
}

fn required_failure() -> LoopEvidenceView {
    LoopEvidenceView {
        id: "evidence-1".to_string(),
        run_id: "run-1".to_string(),
        iteration_id: Some("iteration-1".to_string()),
        kind: "verification-command".to_string(),
        status: "failed".to_string(),
        summary: "Required tests failed.".to_string(),
        operation_id: Some("verification-operation".to_string()),
        command_id: Some("tests".to_string()),
        exit_code: Some(1),
        duration_ms: Some(25),
        details: Some(json!({"required": true})),
        created_at: "2099-01-01T00:00:30Z".to_string(),
    }
}

#[test]
fn deciding_refreshes_verifier_state_records_fingerprints_and_starts_revision() {
    let world = OrchestratorWorld::new();
    let initial_view = world.view();

    world
        .service()
        .decide(&initial_view, &LoopVerificationCancellation::default())
        .expect("decision");

    let run = world.run.lock().expect("run");
    assert_eq!(run.status(), LoopRunStatus::Running);
    assert_eq!(run.phase(), LoopRunPhase::Acting);
    assert_eq!(run.current_iteration(), 2);
    assert_eq!(run.consecutive_runtime_errors(), 0);
    drop(run);

    let iteration = world.iteration.lock().expect("iteration");
    assert_eq!(
        iteration.verifier_session_id.as_deref(),
        Some("verifier-session")
    );
    assert_eq!(iteration.verifier_recommendation.as_deref(), Some("revise"));
    assert_eq!(iteration.status, LoopRunStatus::Failed);
    assert!(iteration.diff_fingerprint.is_some());
    assert!(iteration.check_failure_fingerprint.is_some());
    assert!(iteration
        .decision_reason
        .as_deref()
        .is_some_and(|reason| reason.contains("required deterministic checks")));
    drop(iteration);

    let operations = world.operations.lock().expect("operations");
    assert_eq!(operations.len(), 2);
    assert_eq!(operations[0].kind, LoopOperationKind::RoleGeneration);
    assert_eq!(operations[1].kind, LoopOperationKind::Decision);
    assert!(operations.iter().all(|context| {
        context.run_id == "run-1" && context.iteration_id.as_deref() == Some("iteration-1")
    }));
    drop(operations);

    let logs = world.logs.lock().expect("logs");
    assert_eq!(logs.len(), 4);
    assert!(logs.iter().all(|log| log.operation_id.is_some()));
    assert!(logs.iter().all(|log| log.context.run_id == "run-1"));
}
