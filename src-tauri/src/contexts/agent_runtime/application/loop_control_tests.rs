use super::loop_test_support::{
    assessment_service, scope_evidence, test_binding, FakeScopePlatform,
};
use super::*;
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AvailabilityAssessment,
    InteractionMode, LaunchMetadata, LoopDefinition, LoopDefinitionInput, LoopLimits,
    LoopRequestedMode, LoopRun, LoopRunPhase, LoopRunStatus, LoopTerminalReason,
    LoopVerificationCommand, LoopVerificationKind, NATIVE_CHECK_PATCH_WHITESPACE,
};
use std::sync::{Arc, Mutex};

struct ControlWorld {
    run: Mutex<LoopRun>,
    snapshot: LoopDefinition,
    feedback: Mutex<Option<String>>,
    cancellation_requests: Mutex<u16>,
    operations: Mutex<Vec<LoopOperationContext>>,
    logs: Mutex<Vec<LoopLog>>,
    platform: Arc<FakeScopePlatform>,
    bound: Mutex<bool>,
    revision: Mutex<u64>,
    acceptance_operation: Mutex<Option<String>>,
    sealed: Mutex<Option<String>>,
    evidence: Mutex<Vec<LoopEvidenceView>>,
    stopped_sessions: Mutex<Vec<String>>,
}

impl ControlWorld {
    fn new(run: LoopRun, max_iterations: u16) -> Arc<Self> {
        Arc::new(Self {
            run: Mutex::new(run),
            snapshot: definition(max_iterations),
            feedback: Mutex::new(None),
            cancellation_requests: Mutex::new(0),
            operations: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            platform: FakeScopePlatform::new("tree-verified"),
            bound: Mutex::new(true),
            revision: Mutex::new(3),
            acceptance_operation: Mutex::new(None),
            sealed: Mutex::new(None),
            evidence: Mutex::new(vec![
                scope_evidence("worker", "passed", "tree-worker"),
                scope_evidence("verification", "passed", "tree-verification"),
                scope_evidence("verifier", "passed", "tree-verified"),
            ]),
            stopped_sessions: Mutex::new(Vec::new()),
        })
    }
    fn admission(self: &Arc<Self>) -> LoopApplicationService {
        LoopApplicationService::new(LoopApplicationPorts {
            loops: self.clone(),
            registry: self.clone(),
            api_agents: self.clone(),
            projects: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
            assessment: assessment_service(
                self.clone(),
                self.clone(),
                self.platform.clone(),
                self.clone(),
            ),
            app_epoch: "epoch-test".to_string(),
        })
    }
    fn service(self: &Arc<Self>) -> LoopControlApplicationService {
        LoopControlApplicationService::new(LoopControlApplicationPorts {
            loops: self.clone(),
            execution: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
            admission: self.admission(),
        })
    }
    fn acceptance(self: &Arc<Self>) -> LoopAcceptanceApplicationService {
        LoopAcceptanceApplicationService::new(LoopAcceptanceApplicationPorts {
            loops: self.clone(),
            iterations: self.clone(),
            generations: self.clone(),
            scope_platform: self.platform.clone(),
            background: Arc::new(InlineBackground),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
        })
    }
    fn binding(&self) -> LoopScopeBinding {
        let service = assessment_service(
            Arc::new(RegistryOnly),
            Arc::new(RegistryOnly),
            self.platform.clone(),
            Arc::new(RegistryOnly),
        );
        let assessment = service.assess(&self.snapshot).expect("assessment");
        test_binding(&self.snapshot, &assessment, "tree-baseline").expect("binding")
    }
    fn view(&self) -> LoopRunView {
        let run = self.run.lock().expect("run").clone();
        LoopRunView {
            id: run.id().to_string(),
            definition_id: run.definition_id().to_string(),
            definition_snapshot: LoopDefinitionView::from(&self.snapshot),
            status: run.status(),
            phase: run.phase(),
            terminal_reason: run.terminal_reason(),
            current_iteration: run.current_iteration(),
            consecutive_runtime_errors: run.consecutive_runtime_errors(),
            consecutive_no_progress: run.consecutive_no_progress(),
            pause_requested: run.pause_requested(),
            project_path: "D:/project".to_string(),
            worktree_path: Some("D:/project-loop".to_string()),
            worktree_name: Some("loop".to_string()),
            worktree_branch: Some("vanehub/loop".to_string()),
            active_operation_id: None,
            iterations: vec![LoopIterationView {
                id: "iteration-1".to_string(),
                run_id: run.id().to_string(),
                sequence: run.current_iteration(),
                status: run.status(),
                worker_session_id: Some("worker-session".to_string()),
                verifier_session_id: Some("verifier-session".to_string()),
                worker_summary: Some("done".to_string()),
                verifier_recommendation: Some("pass".to_string()),
                verifier_findings: Vec::new(),
                decision_reason: None,
                diff_fingerprint: None,
                check_failure_fingerprint: None,
                user_feedback: None,
                evidence: self.evidence.lock().expect("evidence").clone(),
                started_at: "t".to_string(),
                completed_at: None,
            }],
            simulated: false,
            created_at: "2026-07-22T11:00:00Z".to_string(),
            started_at: None,
            updated_at: "2026-07-22T11:00:00Z".to_string(),
            completed_at: None,
            revision: *self.revision.lock().expect("revision"),
            scope: None,
        }
    }
}

/// Runs the acceptance task on the calling thread so tests observe its outcome directly.
struct InlineBackground;

impl LoopBackgroundPort for InlineBackground {
    fn spawn(
        &self,
        _: &str,
        task: Box<dyn FnOnce() + Send + 'static>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        task();
        Ok(())
    }
}

/// Trusted API agents for both roles, the combination the strict mode admits.
struct RegistryOnly;

impl AgentRegistryRepository for RegistryOnly {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(Some(api_agent(agent_id)))
    }
}

impl ApiAgentGateway for RegistryOnly {
    fn register(
        &self,
        _: &str,
        _: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn provider_config(
        &self,
        _: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        Ok(Some(trusted_config()))
    }
    fn update(
        &self,
        _: &str,
        _: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn delete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl AgentClockPort for RegistryOnly {
    fn now(&self) -> String {
        "2026-07-22T12:00:00Z".to_string()
    }
}

impl AgentRegistryRepository for ControlWorld {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(Some(api_agent(agent_id)))
    }
}

impl ApiAgentGateway for ControlWorld {
    fn register(
        &self,
        _: &str,
        _: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn provider_config(
        &self,
        _: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        Ok(Some(trusted_config()))
    }
    fn update(
        &self,
        _: &str,
        _: &UpdateApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn delete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl LoopProjectPort for ControlWorld {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        Ok(project_path.to_string())
    }
}

impl LoopGenerationControlPort for ControlWorld {
    fn stop_loop_generation(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.stopped_sessions
            .lock()
            .expect("stopped")
            .push(session_id.to_string());
        Ok(())
    }
}

impl LoopIterationRepository for ControlWorld {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn attach_verifier_session(
        &self,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn save_verifier_result(
        &self,
        _: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn save_iteration_fingerprints(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn append_evidence(
        &self,
        evidence: &LoopEvidenceView,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.evidence
            .lock()
            .expect("evidence")
            .push(evidence.clone());
        Ok(())
    }
}

fn trusted_config() -> ApiProviderConfig {
    ApiProviderConfig {
        source_provider_id: None,
        model_id: "model".to_string(),
        interface_format: "anthropic".to_string(),
        base_url: None,
        auto_approve_tools: true,
    }
}

fn api_agent(id: &str) -> AgentDefinition {
    AgentDefinition::new(AgentDefinitionInput {
        id: id.to_string(),
        display_name: id.to_string(),
        provider: "test".to_string(),
        managed_sdk_dependency_id: None,
        launch: LaunchMetadata::new("api".to_string(), None, None, None).expect("launch"),
        supported_interaction_modes: vec![InteractionMode::Api],
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: Vec::new(),
    })
    .expect("agent")
}

impl LoopRepository for ControlWorld {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![self.snapshot.clone()])
    }
    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((definition_id == self.snapshot.values().id).then(|| self.snapshot.clone()))
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
        Ok(self.run.lock().expect("run").status().is_active())
    }
    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        let run = self.run.lock().expect("run");
        Ok((run.id() == run_id).then(|| run.clone()))
    }
    fn find_run_view(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopRunView>, AgentRuntimeApplicationError> {
        let matches = self.run.lock().expect("run").id() == run_id;
        Ok(matches.then(|| self.view()))
    }
    fn find_run_scope(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopRunScopeRecord>, AgentRuntimeApplicationError> {
        if self.run.lock().expect("run").id() != run_id {
            return Ok(None);
        }
        let bound = *self.bound.lock().expect("bound");
        Ok(Some(LoopRunScopeRecord {
            revision: *self.revision.lock().expect("revision"),
            binding: bound.then(|| self.binding()),
            assessment: None,
            sealed_evidence_id: self.sealed.lock().expect("sealed").clone(),
            acceptance_operation_id: self
                .acceptance_operation
                .lock()
                .expect("acceptance")
                .clone(),
        }))
    }
    fn attach_acceptance_operation(
        &self,
        _: &str,
        operation_id: &str,
        expected_revision: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut current = self.acceptance_operation.lock().expect("acceptance");
        if current.is_some() || *self.revision.lock().expect("revision") != expected_revision {
            return Err(loop_conflict());
        }
        *current = Some(operation_id.to_string());
        Ok(())
    }
    fn release_acceptance_operation(
        &self,
        _: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut current = self.acceptance_operation.lock().expect("acceptance");
        if current.as_deref() == Some(operation_id) {
            *current = None;
        }
        Ok(())
    }
    fn seal_acceptance(
        &self,
        run: &LoopRun,
        expected_revision: u64,
        operation_id: &str,
        evidence_id: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut revision = self.revision.lock().expect("revision");
        if *revision != expected_revision
            || self
                .acceptance_operation
                .lock()
                .expect("acceptance")
                .as_deref()
                != Some(operation_id)
            || self.sealed.lock().expect("sealed").is_some()
        {
            return Err(loop_conflict());
        }
        *revision += 1;
        *self.sealed.lock().expect("sealed") = Some(evidence_id.to_string());
        *self.acceptance_operation.lock().expect("acceptance") = None;
        *self.run.lock().expect("run") = run.clone();
        Ok(())
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
        if stored.status() != expected_status {
            return Err(loop_conflict());
        }
        *stored = run.clone();
        *self.revision.lock().expect("revision") += 1;
        Ok(())
    }

    fn save_pause_request(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        expected_pause_requested: bool,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        if stored.status() != expected_status
            || stored.pause_requested() != expected_pause_requested
        {
            return Err(loop_conflict());
        }
        *stored = run.clone();
        Ok(())
    }

    fn find_run_definition_snapshot(
        &self,
        run_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((self.run.lock().expect("run").id() == run_id).then(|| self.snapshot.clone()))
    }

    fn save_continue_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        feedback: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        let mut saved_feedback = self.feedback.lock().expect("feedback");
        if stored.status() != expected_status || saved_feedback.is_some() {
            return Err(loop_conflict());
        }
        *saved_feedback = Some(feedback.to_string());
        *stored = run.clone();
        Ok(())
    }
}

impl LoopExecutionControlPort for ControlWorld {
    fn request_cancellation(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        *self
            .cancellation_requests
            .lock()
            .expect("cancellation requests") += 1;
        Ok(())
    }
}

impl AgentTaskPort for ControlWorld {
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
        self.operations
            .lock()
            .expect("operations")
            .push(context.clone());
        Ok(AgentOperation {
            id: "cancellation-operation".to_string(),
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

impl LoopLoggingPort for ControlWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

impl AgentClockPort for ControlWorld {
    fn now(&self) -> String {
        "2026-07-22T12:00:00Z".to_string()
    }
}

fn running_run() -> LoopRun {
    let mut run = LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run");
    run.begin().expect("begin");
    run
}

fn acceptance_run() -> LoopRun {
    let mut run = running_run();
    run.move_to(LoopRunPhase::Verifying).expect("verifying");
    run.move_to(LoopRunPhase::Deciding).expect("deciding");
    run.await_acceptance(true).expect("acceptance");
    run
}

fn definition(max_iterations: u16) -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-1".to_string(),
        name: "Loop".to_string(),
        enabled: true,
        project_path: "D:/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Implement the goal".to_string(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec!["src/generated".to_string()],
        worker_agent_id: "worker".to_string(),
        verifier_agent_id: "verifier".to_string(),
        verification_commands: vec![LoopVerificationCommand::new_with_kind(
            "whitespace".to_string(),
            LoopVerificationKind::NativeCheck,
            NATIVE_CHECK_PATCH_WHITESPACE.to_string(),
            Vec::new(),
            None,
            60,
            true,
        )
        .expect("command")],
        limits: LoopLimits::new(max_iterations, 60, 600, 2, 2).expect("limits"),
        version: 1,
        created_at: "2026-07-22T11:00:00Z".to_string(),
        updated_at: "2026-07-22T11:00:00Z".to_string(),
        scope_schema_version: Some(1),
        requested_mode: Some(LoopRequestedMode::PreventiveRequired),
    })
    .expect("definition")
}

fn loop_conflict() -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop("concurrent action".to_string())
}

#[test]
fn pause_is_requested_once_and_applied_only_at_a_boundary() {
    let world = ControlWorld::new(running_run(), 3);
    let service = world.service();

    let requested = service.request_pause("run-1").expect("request pause");
    assert_eq!(requested.status(), LoopRunStatus::Running);
    assert!(requested.pause_requested());
    assert!(service.request_pause("run-1").is_err());

    let paused = service.pause_at_boundary("run-1").expect("pause boundary");
    assert_eq!(paused.status(), LoopRunStatus::Paused);
    let resumed = service
        .resume("run-1", LoopControlEnvelope::legacy())
        .expect("resume");
    assert_eq!(resumed.status(), LoopRunStatus::Running);
    assert!(service
        .resume("run-1", LoopControlEnvelope::legacy())
        .is_err());
}

#[test]
fn resume_refuses_a_run_without_a_trustworthy_binding_or_a_stale_revision() {
    let world = ControlWorld::new(running_run(), 3);
    let service = world.service();
    service.request_pause("run-1").expect("request");
    service.pause_at_boundary("run-1").expect("pause");

    let stale = service.resume(
        "run-1",
        LoopControlEnvelope {
            expected_revision: Some(1),
            idempotency_key: None,
            audit_acknowledgement_id: None,
        },
    );
    assert!(stale.is_err());

    *world.bound.lock().expect("bound") = false;
    let error = service
        .resume("run-1", LoopControlEnvelope::legacy())
        .expect_err("binding missing");
    assert!(error.to_string().contains("scope-binding-missing"));
    assert_eq!(
        world.run.lock().expect("run").status(),
        LoopRunStatus::Paused,
        "the blocker is not cleared"
    );
}

#[test]
fn cancellation_requests_child_stop_and_rejects_duplicates() {
    let world = ControlWorld::new(running_run(), 3);
    let service = world.service();

    let cancelled = service.cancel("run-1").expect("cancel");
    assert_eq!(cancelled.status(), LoopRunStatus::Cancelled);
    assert_eq!(
        cancelled.terminal_reason(),
        Some(LoopTerminalReason::UserStopped)
    );
    assert!(service.cancel("run-1").is_err());
    assert_eq!(*world.cancellation_requests.lock().expect("requests"), 1);
    let operations = world.operations.lock().expect("operations");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].kind, LoopOperationKind::Cancellation);
    let logs = world.logs.lock().expect("logs");
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|log| log.context.run_id == "run-1"));
    assert!(logs
        .iter()
        .all(|log| log.operation_id.as_deref() == Some("cancellation-operation")));
}

#[test]
fn acceptance_seals_contents_matching_the_verified_evidence_exactly_once() {
    let world = ControlWorld::new(acceptance_run(), 3);
    let accepted = world
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: Some(3),
            expected_scope_digest: Some(world.binding().scope_digest),
            expected_evidence_id: None,
            idempotency_key: Some("accept-1".to_string()),
        })
        .expect("acceptance");
    assert_eq!(accepted.run_id, "run-1");
    let run = world.run.lock().expect("run").clone();
    assert_eq!(run.status(), LoopRunStatus::Succeeded);
    assert_eq!(run.terminal_reason(), Some(LoopTerminalReason::GoalMet));
    assert!(world.sealed.lock().expect("sealed").is_some());
    assert_eq!(
        *world.stopped_sessions.lock().expect("stopped"),
        vec!["worker-session".to_string(), "verifier-session".to_string()],
        "owned writers are reconciled before the tree is read"
    );
    assert!(world
        .evidence
        .lock()
        .expect("evidence")
        .iter()
        .any(|item| item.kind == "acceptance" && item.status == "passed"));
    assert!(world
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: None,
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
}

#[test]
fn acceptance_refuses_stale_revision_changed_trees_and_incomplete_evidence() {
    let stale = ControlWorld::new(acceptance_run(), 3);
    assert!(stale
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: Some(2),
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
    assert_eq!(
        stale.run.lock().expect("run").status(),
        LoopRunStatus::AwaitingAcceptance
    );

    // The tree changed after the Verifier phase: sealing finds a different digest, the run is
    // paused as unverifiable and nothing is marked succeeded.
    let changed = ControlWorld::new(acceptance_run(), 3);
    changed
        .platform
        .mutate_tree("tree-edited-after-verification");
    changed
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: Some(3),
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .expect("request accepted for processing");
    let run = changed.run.lock().expect("run").clone();
    assert_eq!(run.status(), LoopRunStatus::Paused);
    assert_eq!(
        run.terminal_reason(),
        Some(LoopTerminalReason::ScopeUnverifiable)
    );
    assert!(changed.sealed.lock().expect("sealed").is_none());
    assert!(changed
        .acceptance_operation
        .lock()
        .expect("acceptance")
        .is_none());
    assert!(changed
        .evidence
        .lock()
        .expect("evidence")
        .iter()
        .any(|item| item.kind == "acceptance" && item.status == "unverifiable"));

    // A recorded violation is sticky: even with checks and advice passing the gate refuses.
    let violated = ControlWorld::new(acceptance_run(), 3);
    violated
        .evidence
        .lock()
        .expect("evidence")
        .push(scope_evidence("worker", "violation", "x"));
    assert!(violated
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: Some(3),
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());

    // The legacy accept entry point routes through the same gate.
    let unbound = ControlWorld::new(acceptance_run(), 3);
    *unbound.bound.lock().expect("bound") = false;
    assert!(unbound
        .acceptance()
        .request(RequestLoopAcceptanceRequest {
            run_id: "run-1".to_string(),
            expected_revision: None,
            expected_scope_digest: None,
            expected_evidence_id: None,
            idempotency_key: None,
        })
        .is_err());
}

#[test]
fn human_reject_is_a_terminal_one_time_action() {
    let rejected_world = ControlWorld::new(acceptance_run(), 3);
    let rejected = rejected_world.service().reject("run-1").expect("reject");
    assert_eq!(rejected.status(), LoopRunStatus::Cancelled);
    assert_eq!(
        rejected.terminal_reason(),
        Some(LoopTerminalReason::UserRejected)
    );
    assert!(rejected_world.service().reject("run-1").is_err());
}

#[test]
fn continuation_requires_feedback_and_atomically_advances_once() {
    let world = ControlWorld::new(acceptance_run(), 3);
    let service = world.service();
    assert!(service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "   ".to_string(),
            envelope: LoopControlEnvelope::legacy(),
        })
        .is_err());

    let continued = service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: " Add the missing regression test. ".to_string(),
            envelope: LoopControlEnvelope::legacy(),
        })
        .expect("continue");
    assert_eq!(continued.current_iteration(), 2);
    assert_eq!(continued.phase(), LoopRunPhase::Acting);
    assert_eq!(
        world.feedback.lock().expect("feedback").as_deref(),
        Some("Add the missing regression test.")
    );
    assert!(service
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "Again".to_string(),
            envelope: LoopControlEnvelope::legacy(),
        })
        .is_err());

    let limited = ControlWorld::new(acceptance_run(), 1);
    assert!(limited
        .service()
        .continue_with_feedback(ContinueLoopRequest {
            run_id: "run-1".to_string(),
            feedback: "More".to_string(),
            envelope: LoopControlEnvelope::legacy(),
        })
        .is_err());
}
