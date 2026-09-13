use super::loop_test_support::{test_binding, FakeScopePlatform};
use super::*;
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopDefinitionInput, LoopLimits, LoopVerificationCommand, LoopVerificationKind,
    NATIVE_CHECK_PATCH_WHITESPACE,
};
use std::sync::{Arc, Mutex};

struct VerificationWorld {
    platform: Arc<FakeScopePlatform>,
    commands: Mutex<Vec<String>>,
    evidence: Mutex<Vec<LoopEvidenceView>>,
    operation_events: Mutex<Vec<String>>,
    logs: Mutex<Vec<LoopLog>>,
    projections: Mutex<Vec<LoopVerificationEvidenceFact>>,
}

impl Default for VerificationWorld {
    fn default() -> Self {
        Self {
            platform: FakeScopePlatform::new("tree-1"),
            commands: Mutex::new(Vec::new()),
            evidence: Mutex::new(Vec::new()),
            operation_events: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            projections: Mutex::new(Vec::new()),
        }
    }
}

impl VerificationWorld {
    fn service(self: &Arc<Self>) -> LoopVerificationApplicationService {
        LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
            iterations: self.clone(),
            processes: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
            evidence: self.clone(),
            scope_platform: self.platform.clone(),
        })
    }
}

impl LoopVerificationEvidencePort for VerificationWorld {
    fn project(&self, fact: LoopVerificationEvidenceFact) {
        self.projections.lock().expect("projections").push(fact);
    }
}

impl LoopIterationRepository for VerificationWorld {
    fn insert_iteration(&self, _: &LoopIterationView) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn attach_worker_session(&self, _: &str, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn attach_verifier_session(
        &self,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn save_verifier_result(
        &self,
        _: &SaveLoopVerifierResultRequest,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }

    fn save_iteration_fingerprints(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
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

impl LoopVerificationProcessPort for VerificationWorld {
    fn execute(
        &self,
        request: LoopVerificationProcessRequest,
    ) -> Result<LoopVerificationProcessResult, AgentRuntimeApplicationError> {
        let command_id = request.command.id.clone();
        self.commands
            .lock()
            .expect("commands")
            .push(command_id.clone());
        if command_id == "process-error" {
            return Err(AgentRuntimeApplicationError::VerificationProcess(
                "verification process rejected command".to_string(),
            ));
        }
        let status = match command_id.as_str() {
            "required-timeout" => LoopVerificationProcessStatus::TimedOut,
            "optional-fail" => LoopVerificationProcessStatus::Failed,
            "cancelled" => LoopVerificationProcessStatus::Cancelled,
            _ => LoopVerificationProcessStatus::Passed,
        };
        Ok(LoopVerificationProcessResult {
            status,
            exit_code: (status != LoopVerificationProcessStatus::TimedOut).then_some(
                if status == LoopVerificationProcessStatus::Passed {
                    0
                } else {
                    1
                },
            ),
            duration_ms: 25,
            stdout: format!("output for {command_id}"),
            stderr: String::new(),
            output_truncated: false,
        })
    }
}

impl AgentTaskPort for VerificationWorld {
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
        assert_eq!(context.kind, LoopOperationKind::Verification);
        let id = format!(
            "operation-{}",
            self.operation_events.lock().expect("events").len()
        );
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("start:{message}"));
        Ok(AgentOperation {
            id,
            related_agent_id: Some(context.run_id.clone()),
            message: Some(message.to_string()),
        })
    }

    fn append_log(
        &self,
        operation_id: &str,
        _: String,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("log:{operation_id}"));
        Ok(())
    }

    fn complete(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("complete:{operation_id}"));
        Ok(())
    }

    fn fail(&self, operation_id: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("fail:{operation_id}"));
        Ok(())
    }

    fn cancel(&self, operation_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.operation_events
            .lock()
            .expect("events")
            .push(format!("cancel:{operation_id}"));
        Ok(())
    }
}

impl AgentClockPort for VerificationWorld {
    fn now(&self) -> String {
        "2026-07-22T09:00:00Z".to_string()
    }
}

impl LoopLoggingPort for VerificationWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

fn command(id: &str, required: bool) -> LoopVerificationCommandView {
    LoopVerificationCommandView {
        id: id.to_string(),
        kind: LoopVerificationKind::Process,
        program: "npm".to_string(),
        args: vec!["test".to_string()],
        working_directory: None,
        timeout_seconds: 60,
        required,
    }
}

fn native_command(id: &str) -> LoopVerificationCommandView {
    LoopVerificationCommandView {
        id: id.to_string(),
        kind: LoopVerificationKind::NativeCheck,
        program: NATIVE_CHECK_PATCH_WHITESPACE.to_string(),
        args: Vec::new(),
        working_directory: None,
        timeout_seconds: 60,
        required: true,
    }
}

fn scoped_definition(
    mode: crate::contexts::agent_runtime::domain::LoopRequestedMode,
) -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-1".to_string(),
        name: "Loop".to_string(),
        enabled: true,
        project_path: "C:/work/project".to_string(),
        base_branch: "main".to_string(),
        goal: "goal".to_string(),
        acceptance_criteria: vec!["done".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: Vec::new(),
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
        limits: LoopLimits::new(3, 60, 600, 2, 2).expect("limits"),
        version: 1,
        created_at: "t".to_string(),
        updated_at: "t".to_string(),
        scope_schema_version: Some(1),
        requested_mode: Some(mode),
    })
    .expect("definition")
}

fn verification_scope(
    mode: crate::contexts::agent_runtime::domain::LoopRequestedMode,
) -> LoopVerificationScope {
    let definition = scoped_definition(mode);
    let assessment = LoopExecutionAssessment {
        requested_mode: mode.as_str().to_string(),
        surfaces: Vec::new(),
        blockers: Vec::new(),
        limitations: Vec::new(),
        satisfies_requested_mode: true,
        acknowledgement_required: false,
        witness_digest: "sha256:witness".to_string(),
        assessed_at: "t".to_string(),
        simulated: false,
    };
    LoopVerificationScope {
        binding: test_binding(&definition, &assessment, "tree-1").expect("binding"),
        input_manifest_id: "worker-1".to_string(),
        input_digest: "tree-1".to_string(),
        assessment_digest: assessment_digest(&assessment),
    }
}

fn request(commands: Vec<LoopVerificationCommandView>) -> RunLoopVerificationRequest {
    RunLoopVerificationRequest {
        run_id: "run-1".to_string(),
        iteration_id: "iteration-1".to_string(),
        worktree_root: "C:/work/project-loop".to_string(),
        commands,
        cancellation: LoopVerificationCancellation::default(),
        scope: Some(verification_scope(
            crate::contexts::agent_runtime::domain::LoopRequestedMode::ArtifactAudited,
        )),
    }
}

#[test]
fn commands_run_in_definition_order_and_required_timeout_blocks_acceptance() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("required-pass", true),
            command("optional-fail", false),
            command("required-timeout", true),
        ]))
        .expect("verification");

    assert_eq!(
        *world.commands.lock().expect("commands"),
        ["required-pass", "optional-fail", "required-timeout"]
    );
    assert_eq!(result.evidence.len(), 3);
    assert!(!result.required_checks_passed);
    assert_eq!(result.evidence[2].status, "timed-out");
    assert_eq!(world.evidence.lock().expect("evidence").len(), 3);
    assert_eq!(
        *world.projections.lock().expect("projections"),
        [LoopVerificationEvidenceFact {
            run_id: "run-1".to_string(),
            iteration_id: "iteration-1".to_string(),
            workspace: "C:/work/project-loop".to_string(),
            occurred_at: "2026-07-22T09:00:00Z".to_string(),
            passed_count: 1,
            failed_count: 2,
            cancelled: false,
        }]
    );
}

#[test]
fn optional_failure_does_not_block_acceptance_readiness() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("required-pass", true),
            command("optional-fail", false),
        ]))
        .expect("verification");

    assert!(result.required_checks_passed);
    assert!(!result.cancelled);
}

#[test]
fn cancellation_records_evidence_and_stops_scheduling_commands() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![
            command("cancelled", true),
            command("never-runs", true),
        ]))
        .expect("verification");

    assert!(result.cancelled);
    assert!(!result.required_checks_passed);
    assert_eq!(*world.commands.lock().expect("commands"), ["cancelled"]);
    assert_eq!(result.evidence[0].status, "cancelled");
}

#[test]
fn optional_process_error_records_evidence_without_blocking_acceptance() {
    let world = Arc::new(VerificationWorld::default());
    let result = world
        .service()
        .run_commands(request(vec![command("process-error", false)]))
        .expect("verification result");

    assert!(result.required_checks_passed);
    assert_eq!(result.evidence[0].status, "error");
    assert_eq!(world.evidence.lock().expect("evidence").len(), 1);
    assert!(world
        .operation_events
        .lock()
        .expect("events")
        .iter()
        .any(|event| event.starts_with("fail:")));
    let logs = world.logs.lock().expect("logs");
    assert!(logs.iter().all(|log| log.context.run_id == "run-1"));
    assert!(logs
        .iter()
        .all(|log| { log.context.iteration_id.as_deref() == Some("iteration-1") }));
    assert!(logs
        .iter()
        .all(|log| log.context.kind == LoopOperationKind::Verification));
    assert!(logs.iter().all(|log| log.operation_id.is_some()));
}

#[test]
fn strict_mode_refuses_process_checks_and_runs_native_checks_in_process() {
    let world = Arc::new(VerificationWorld::default());
    let mut strict = request(vec![command("tests", true), native_command("whitespace")]);
    strict.scope = Some(verification_scope(
        crate::contexts::agent_runtime::domain::LoopRequestedMode::PreventiveRequired,
    ));
    let result = world.service().run_commands(strict).expect("verification");

    assert!(
        world.commands.lock().expect("commands").is_empty(),
        "no process was launched"
    );
    assert_eq!(result.evidence[0].status, "error");
    assert_eq!(result.evidence[1].status, "passed");
    assert_eq!(
        result.evidence[1].details.as_ref().expect("details")["verificationKind"],
        "native-check"
    );
    assert!(!result.required_checks_passed);

    *world.platform.native_check.lock().expect("native") = Some(LoopNativeCheckView {
        status: "failed".to_string(),
        findings: vec!["src/a.ts:3: trailing whitespace".to_string()],
        inspected_files: 1,
        binary_excluded: 0,
        detail: None,
    });
    let mut failing = request(vec![native_command("whitespace")]);
    failing.scope = Some(verification_scope(
        crate::contexts::agent_runtime::domain::LoopRequestedMode::PreventiveRequired,
    ));
    let result = world.service().run_commands(failing).expect("verification");
    assert_eq!(result.evidence[0].status, "failed");
    assert!(!result.required_checks_passed);
}

#[test]
fn verification_fingerprints_change_with_inputs_scope_and_command_content() {
    let scope = verification_scope(
        crate::contexts::agent_runtime::domain::LoopRequestedMode::PreventiveRequired,
    );
    let base = verification_fingerprint(&command("tests", true), &scope);
    let mut other_input = scope.clone();
    other_input.input_digest = "tree-2".to_string();
    assert_ne!(
        base,
        verification_fingerprint(&command("tests", true), &other_input)
    );
    let mut other_args = command("tests", true);
    other_args.args = vec!["lint".to_string()];
    assert_ne!(base, verification_fingerprint(&other_args, &scope));
    let mut other_scope = scope.clone();
    other_scope.binding.scope_digest = "sha256:other".to_string();
    assert_ne!(
        base,
        verification_fingerprint(&command("tests", true), &other_scope)
    );
    let mut other_policy = scope.clone();
    other_policy.assessment_digest = "sha256:policy".to_string();
    assert_ne!(
        base,
        verification_fingerprint(&command("tests", true), &other_policy)
    );
    assert_eq!(
        base,
        verification_fingerprint(&command("tests", true), &scope)
    );
}
