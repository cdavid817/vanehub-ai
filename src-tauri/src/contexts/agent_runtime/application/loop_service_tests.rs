use super::loop_test_support::{assessment_service, FakeScopePlatform};
use super::*;
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AvailabilityAssessment,
    InteractionMode, LaunchMetadata, LoopDefinition, LoopDefinitionInput, LoopLimits,
    LoopReadinessCheckCode, LoopRequestedMode, LoopRun, LoopRunPhase, LoopRunStatus,
    LoopVerificationCommand, LoopVerificationKind, NATIVE_CHECK_PATCH_WHITESPACE,
};
use std::sync::{Arc, Mutex};

struct FakeWorld {
    definition: LoopDefinition,
    agents: Vec<AgentDefinition>,
    trusted_agents: Vec<String>,
    project_is_git: bool,
    branch_available: Mutex<bool>,
    run: Mutex<Option<LoopRun>>,
    snapshot: Mutex<Option<LoopDefinition>>,
    operation_id: Mutex<Option<String>>,
    operation_starts: Mutex<u16>,
    logs: Mutex<Vec<LoopLog>>,
    platform: Arc<FakeScopePlatform>,
    receipts: Mutex<Vec<LoopAuditReceipt>>,
    frozen: Mutex<Option<(LoopExecutionAssessment, Option<LoopAuditConsumption>)>>,
    control_operations: Mutex<std::collections::BTreeMap<String, String>>,
}

impl FakeWorld {
    /// Trusted API agents by default: that is the only combination strict mode admits, and the
    /// common case every launch test needs.
    fn new(enabled: bool, project_is_git: bool, agents: Vec<AgentDefinition>) -> Arc<Self> {
        Self::new_with_trust(enabled, project_is_git, agents, vec!["worker", "verifier"])
    }

    fn new_with_trust(
        enabled: bool,
        project_is_git: bool,
        agents: Vec<AgentDefinition>,
        trusted_agents: Vec<&str>,
    ) -> Arc<Self> {
        Arc::new(Self {
            definition: definition(enabled),
            agents,
            trusted_agents: trusted_agents.into_iter().map(str::to_string).collect(),
            project_is_git,
            branch_available: Mutex::new(true),
            run: Mutex::new(None),
            snapshot: Mutex::new(None),
            operation_id: Mutex::new(None),
            operation_starts: Mutex::new(0),
            logs: Mutex::new(Vec::new()),
            platform: FakeScopePlatform::new("tree-1"),
            receipts: Mutex::new(Vec::new()),
            frozen: Mutex::new(None),
            control_operations: Mutex::new(std::collections::BTreeMap::new()),
        })
    }

    fn with_definition(self: Arc<Self>, definition: LoopDefinition) -> Arc<Self> {
        let mut world = self;
        Arc::get_mut(&mut world)
            .expect("exclusive world")
            .definition = definition;
        world
    }

    fn service(self: &Arc<Self>) -> LoopApplicationService {
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
}

impl LoopRepository for FakeWorld {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok(vec![self.definition.clone()])
    }
    fn find_definition(
        &self,
        definition_id: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        Ok((definition_id == self.definition.values().id).then(|| self.definition.clone()))
    }
    fn create_definition(&self, _: &LoopDefinition) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn update_definition(
        &self,
        _: &LoopDefinition,
        _: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn delete_definition(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn create_run(
        &self,
        run: &LoopRun,
        snapshot: &LoopDefinition,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut stored = self.run.lock().expect("run");
        if stored.as_ref().is_some_and(|run| run.status().is_active()) {
            return Err(AgentRuntimeApplicationError::Loop(
                "active run conflict".to_string(),
            ));
        }
        *stored = Some(run.clone());
        *self.snapshot.lock().expect("snapshot") = Some(snapshot.clone());
        Ok(())
    }
    fn has_active_run(&self, _: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self
            .run
            .lock()
            .expect("run")
            .as_ref()
            .is_some_and(|run| run.status().is_active()))
    }
    fn find_run(&self, _: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        Ok(self.run.lock().expect("run").clone())
    }
    fn attach_run_operation(
        &self,
        _: &str,
        operation_id: &str,
        _: LoopRunStatus,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        *self.operation_id.lock().expect("operation") = Some(operation_id.to_string());
        Ok(())
    }
    fn attach_run_worktree(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn save_run_transition(
        &self,
        run: &LoopRun,
        _: LoopRunStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        *self.run.lock().expect("run") = Some(run.clone());
        Ok(())
    }
    fn create_run_with_scope(
        &self,
        run: &LoopRun,
        snapshot: &LoopDefinition,
        project_path: &str,
        created_at: &str,
        assessment: &LoopExecutionAssessment,
        audit: Option<&LoopAuditConsumption>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if let Some(audit) = audit {
            let mut receipts = self.receipts.lock().expect("receipts");
            let receipt = receipts
                .iter_mut()
                .find(|receipt| receipt.id == audit.receipt_id)
                .filter(|receipt| {
                    receipt.acknowledged_at.is_some() && receipt.consumed_at.is_none()
                })
                .ok_or_else(|| {
                    AgentRuntimeApplicationError::Validation("receipt not consumable".to_string())
                })?;
            receipt.consumed_at = Some(audit.now.clone());
        }
        self.create_run(run, snapshot, project_path, created_at)?;
        *self.frozen.lock().expect("frozen") = Some((assessment.clone(), audit.cloned()));
        Ok(())
    }
    fn create_audit_challenge(
        &self,
        receipt: &LoopAuditReceipt,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.receipts
            .lock()
            .expect("receipts")
            .push(receipt.clone());
        Ok(())
    }
    fn acknowledge_audit_challenge(
        &self,
        challenge_id: &str,
        acknowledged_at: &str,
    ) -> Result<LoopAuditReceipt, AgentRuntimeApplicationError> {
        let mut receipts = self.receipts.lock().expect("receipts");
        let receipt = receipts
            .iter_mut()
            .find(|receipt| {
                receipt.challenge_id == challenge_id && receipt.acknowledged_at.is_none()
            })
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Validation("unknown challenge".to_string())
            })?;
        receipt.acknowledged_at = Some(acknowledged_at.to_string());
        Ok(receipt.clone())
    }
    fn find_audit_receipt(
        &self,
        receipt_id: &str,
    ) -> Result<Option<LoopAuditReceipt>, AgentRuntimeApplicationError> {
        Ok(self
            .receipts
            .lock()
            .expect("receipts")
            .iter()
            .find(|receipt| receipt.id == receipt_id)
            .cloned())
    }
    fn claim_control_operation(
        &self,
        key: &str,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<LoopControlOperationClaim, AgentRuntimeApplicationError> {
        let mut operations = self.control_operations.lock().expect("operations");
        Ok(match operations.get(key) {
            Some(outcome) if outcome.is_empty() => LoopControlOperationClaim::InFlight,
            Some(outcome) => LoopControlOperationClaim::Existing(outcome.clone()),
            None => {
                operations.insert(key.to_string(), String::new());
                LoopControlOperationClaim::New
            }
        })
    }
    fn record_control_operation(
        &self,
        key: &str,
        outcome: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.control_operations
            .lock()
            .expect("operations")
            .insert(key.to_string(), outcome.to_string());
        Ok(())
    }
    fn release_control_operation(&self, key: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.control_operations
            .lock()
            .expect("operations")
            .remove(key);
        Ok(())
    }
}

impl AgentRegistryRepository for FakeWorld {
    fn list(&self) -> Result<Vec<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(self.agents.clone())
    }
    fn find(
        &self,
        agent_id: &str,
    ) -> Result<Option<AgentDefinition>, AgentRuntimeApplicationError> {
        Ok(self
            .agents
            .iter()
            .find(|agent| agent.id().as_str() == agent_id)
            .cloned())
    }
}

impl ApiAgentGateway for FakeWorld {
    fn register(
        &self,
        _: &str,
        _: &RegisterApiAgentInput,
    ) -> Result<AgentDefinition, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn provider_config(
        &self,
        agent_id: &str,
    ) -> Result<Option<ApiProviderConfig>, AgentRuntimeApplicationError> {
        if !self
            .agents
            .iter()
            .any(|agent| agent.id().as_str() == agent_id)
        {
            return Ok(None);
        }
        Ok(Some(ApiProviderConfig {
            source_provider_id: None,
            model_id: "test-model".to_string(),
            interface_format: "anthropic".to_string(),
            base_url: None,
            auto_approve_tools: self
                .trusted_agents
                .iter()
                .any(|trusted_id| trusted_id == agent_id),
        }))
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

impl LoopProjectPort for FakeWorld {
    fn validate_local_git_project(
        &self,
        project_path: &str,
    ) -> Result<String, AgentRuntimeApplicationError> {
        if self.project_is_git {
            Ok(project_path.to_string())
        } else {
            Err(AgentRuntimeApplicationError::Validation(
                "Loop project must be a local Git repository.".to_string(),
            ))
        }
    }
    fn base_branch_available(
        &self,
        _: &str,
        _: &str,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(*self.branch_available.lock().expect("branch"))
    }
}

impl AgentTaskPort for FakeWorld {
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
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        *self.operation_starts.lock().expect("starts") += 1;
        assert_eq!(context.kind, LoopOperationKind::Worktree);
        Ok(AgentOperation {
            id: "operation-1".to_string(),
            related_agent_id: Some(context.run_id.clone()),
            message: Some("preparing".to_string()),
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

impl AgentClockPort for FakeWorld {
    fn now(&self) -> String {
        "2026-07-21T08:00:00Z".to_string()
    }
}

impl LoopLoggingPort for FakeWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

fn definition(enabled: bool) -> LoopDefinition {
    LoopDefinition::new(LoopDefinitionInput {
        id: "loop-1".to_string(),
        name: "Improve tests".to_string(),
        enabled,
        project_path: "C:/work/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Improve test coverage".to_string(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec!["src/generated".to_string()],
        worker_agent_id: "worker".to_string(),
        verifier_agent_id: "verifier".to_string(),
        verification_commands: vec![native_check()],
        limits: LoopLimits::new(3, 60, 600, 2, 2).expect("limits"),
        version: 7,
        created_at: "2026-07-21T07:00:00Z".to_string(),
        updated_at: "2026-07-21T07:30:00Z".to_string(),
        scope_schema_version: Some(1),
        requested_mode: Some(LoopRequestedMode::PreventiveRequired),
    })
    .expect("definition")
}

fn native_check() -> LoopVerificationCommand {
    LoopVerificationCommand::new_with_kind(
        "whitespace".to_string(),
        LoopVerificationKind::NativeCheck,
        NATIVE_CHECK_PATCH_WHITESPACE.to_string(),
        Vec::new(),
        None,
        60,
        true,
    )
    .expect("native check")
}

fn process_check() -> LoopVerificationCommand {
    LoopVerificationCommand::new(
        "tests".to_string(),
        "npm".to_string(),
        vec!["test".to_string()],
        None,
        60,
        true,
    )
    .expect("command")
}

fn definition_with(
    mode: Option<LoopRequestedMode>,
    scope_version: Option<u32>,
    commands: Vec<LoopVerificationCommand>,
) -> LoopDefinition {
    let mut input = definition(true).values().clone();
    input.requested_mode = mode;
    input.scope_schema_version = scope_version;
    input.verification_commands = commands;
    LoopDefinition::new(input).expect("definition")
}

fn start(world: &Arc<FakeWorld>) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
    world
        .service()
        .start_manual("loop-1", LoopControlEnvelope::legacy())
}

fn agent(id: &str) -> AgentDefinition {
    AgentDefinition::new(AgentDefinitionInput {
        id: id.to_string(),
        display_name: id.to_string(),
        provider: "test".to_string(),
        managed_sdk_dependency_id: None,
        launch: LaunchMetadata::new(
            "cli".to_string(),
            Some(id.to_string()),
            None,
            Some(id.to_string()),
        )
        .expect("launch"),
        supported_interaction_modes: vec![InteractionMode::Cli],
        availability: AvailabilityAssessment::new(AgentAvailability::Available, None),
        capability_tags: vec!["coding".to_string()],
    })
    .expect("agent")
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
        capability_tags: vec!["coding".to_string()],
    })
    .expect("agent")
}

fn save_request(worker_agent_id: &str, verifier_agent_id: &str) -> SaveLoopDefinitionRequest {
    SaveLoopDefinitionRequest {
        name: "Improve tests".to_string(),
        enabled: true,
        project_path: "C:/work/project".to_string(),
        base_branch: "main".to_string(),
        goal: "Improve test coverage".to_string(),
        acceptance_criteria: vec!["Tests pass".to_string()],
        allowed_paths: vec!["src".to_string()],
        protected_paths: vec!["src/generated".to_string()],
        worker_agent_id: worker_agent_id.to_string(),
        verifier_agent_id: verifier_agent_id.to_string(),
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
        expected_version: None,
        scope_schema_version: Some(1),
        requested_mode: Some(LoopRequestedMode::PreventiveRequired),
    }
}

#[test]
fn create_definition_accepts_a_trusted_api_agent_as_worker_or_verifier() {
    let world = FakeWorld::new_with_trust(
        true,
        true,
        vec![api_agent("worker"), agent("verifier")],
        vec!["worker"],
    );

    let result = world
        .service()
        .create_definition(save_request("worker", "verifier"));

    assert!(result.is_ok());
}

#[test]
fn create_definition_rejects_an_untrusted_api_agent_as_worker_or_verifier() {
    let world = FakeWorld::new_with_trust(
        true,
        true,
        vec![api_agent("worker"), agent("verifier")],
        Vec::new(),
    );

    let error = world
        .service()
        .create_definition(save_request("worker", "verifier"))
        .expect_err("untrusted API agent should be rejected");

    let message = error.to_string();
    assert!(message.contains("tool-use trust"), "{message}");
}

#[test]
fn manual_start_accepts_a_trusted_api_agent_as_worker_or_verifier() {
    let world = FakeWorld::new_with_trust(
        true,
        true,
        vec![api_agent("worker"), api_agent("verifier")],
        vec!["worker", "verifier"],
    );

    let result = start(&world);

    assert!(result.is_ok(), "{result:?}");
    let (assessment, audit) = world
        .frozen
        .lock()
        .expect("frozen")
        .clone()
        .expect("frozen");
    assert!(assessment.satisfies_requested_mode);
    assert!(audit.is_none());
}

#[test]
fn manual_start_rejects_an_untrusted_api_agent_as_worker_or_verifier() {
    let world = FakeWorld::new_with_trust(
        true,
        true,
        vec![api_agent("worker"), api_agent("verifier")],
        vec!["worker"],
    );

    let error = start(&world).expect_err("untrusted verifier should be rejected");

    let message = error.to_string();
    assert!(message.contains("tool-use trust"), "{message}");
    assert!(world.run.lock().expect("run").is_none());
}

#[test]
fn manual_start_persists_snapshot_before_preparation_operation() {
    let world = FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")]);
    let result = start(&world).expect("start");

    assert!(result.run_id.starts_with("loop-run-"));
    assert_eq!(result.operation_id, "operation-1");
    let run = world.run.lock().expect("run").clone().expect("stored run");
    assert_eq!(run.status(), LoopRunStatus::Queued);
    assert_eq!(run.phase(), LoopRunPhase::Preparing);
    assert_eq!(
        world.snapshot.lock().expect("snapshot").as_ref(),
        Some(&world.definition)
    );
    let logs = world.logs.lock().expect("logs");
    assert_eq!(logs.len(), 1);
    assert_eq!(logs[0].context.run_id, result.run_id);
    assert_eq!(logs[0].context.kind, LoopOperationKind::Worktree);
    assert_eq!(logs[0].operation_id.as_deref(), Some("operation-1"));
}

#[test]
fn manual_start_validates_before_creating_run_or_operation() {
    for world in [
        FakeWorld::new(
            false,
            true,
            vec![api_agent("worker"), api_agent("verifier")],
        ),
        FakeWorld::new(
            true,
            false,
            vec![api_agent("worker"), api_agent("verifier")],
        ),
        FakeWorld::new(true, true, vec![api_agent("worker")]),
    ] {
        assert!(start(&world).is_err());
        assert!(world.run.lock().expect("run").is_none());
        assert_eq!(*world.operation_starts.lock().expect("starts"), 0);
    }
}

#[test]
fn manual_start_rejects_a_second_active_run() {
    let world = FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")]);
    start(&world).expect("first run");
    assert!(start(&world).is_err());
    assert_eq!(*world.operation_starts.lock().expect("starts"), 1);
}

#[test]
fn readiness_reports_all_checks_without_starting_a_run() {
    let world = FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")]);

    let report = world.service().readiness("loop-1").expect("readiness");

    assert!(report.ready, "{:?}", report.checks);
    assert_eq!(report.definition_id, "loop-1");
    assert_eq!(report.checked_at, "2026-07-21T08:00:00Z");
    assert_eq!(report.checks.len(), 10);
    assert_eq!(
        report.requested_mode,
        Some(LoopRequestedMode::PreventiveRequired)
    );
    assert!(!report.acknowledgement_required);
    assert!(report
        .assessment
        .as_ref()
        .is_some_and(|assessment| assessment.satisfies_requested_mode));
    assert!(report.checks.iter().all(|check| check.passed));
    assert!(world.run.lock().expect("run").is_none());
    assert!(world.snapshot.lock().expect("snapshot").is_none());
    assert_eq!(*world.operation_starts.lock().expect("starts"), 0);
    assert!(world.logs.lock().expect("logs").is_empty());
}

#[test]
fn readiness_identifies_disabled_project_branch_and_agent_blockers() {
    let disabled = FakeWorld::new(
        false,
        true,
        vec![api_agent("worker"), api_agent("verifier")],
    );
    let missing_project = FakeWorld::new(
        true,
        false,
        vec![api_agent("worker"), api_agent("verifier")],
    );
    let missing_verifier = FakeWorld::new(true, true, vec![api_agent("worker")]);
    let missing_branch =
        FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")]);
    *missing_branch.branch_available.lock().expect("branch") = false;

    for (world, expected) in [
        (disabled, LoopReadinessCheckCode::DefinitionEnabled),
        (missing_project, LoopReadinessCheckCode::ProjectAvailable),
        (missing_branch, LoopReadinessCheckCode::BranchAvailable),
        (missing_verifier, LoopReadinessCheckCode::VerifierEligible),
    ] {
        let report = world.service().readiness("loop-1").expect("readiness");
        assert!(!report.ready);
        let check = report
            .checks
            .iter()
            .find(|check| check.code == expected)
            .expect("expected readiness check");
        assert!(!check.passed);
        assert!(check.remediation_target.is_some());
        let logs = world.logs.lock().expect("logs");
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].category, "loop.readiness");
        assert_eq!(logs[0].level, AgentLogLevel::Warn);
        assert!(!logs[0].message.contains("C:/work/project"));
    }
}

#[test]
fn readiness_blocks_when_the_definition_has_an_active_run() {
    let world = FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")]);
    *world.run.lock().expect("run") =
        Some(LoopRun::new("run-1".to_string(), "loop-1".to_string()).expect("run"));

    let report = world.service().readiness("loop-1").expect("readiness");
    let check = report
        .checks
        .iter()
        .find(|check| check.code == LoopReadinessCheckCode::NoActiveRun)
        .expect("active run check");

    assert!(!report.ready);
    assert!(!check.passed);
    assert_eq!(*world.operation_starts.lock().expect("starts"), 0);
}

#[test]
fn readiness_blocks_conflicting_normalized_path_scopes() {
    // A covered allowed path is rejected at save time under the versioned scope; a legacy row
    // that still carries one is reported as an unverified scope rather than silently widened.
    let mut input = definition(true).values().clone();
    input.protected_paths = vec!["src/".to_string()];
    assert!(LoopDefinition::new(input.clone()).is_err());
    input.scope_schema_version = None;
    let world = FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")])
        .with_definition(LoopDefinition::new(input).expect("legacy definition"));

    let report = world.service().readiness("loop-1").expect("readiness");
    let scope = report
        .checks
        .iter()
        .find(|check| check.code == LoopReadinessCheckCode::PathScopeValid)
        .expect("scope check");
    let version = report
        .checks
        .iter()
        .find(|check| check.code == LoopReadinessCheckCode::ScopeVersionSupported)
        .expect("version check");

    assert!(!report.ready);
    assert!(!scope.passed);
    assert!(!version.passed);
    assert_eq!(scope.remediation_target, Some("definition"));
    assert!(start(&world).is_err());
    assert!(world.run.lock().expect("run").is_none());
}

#[test]
fn strict_mode_rejects_uncovered_surfaces_before_queuing() {
    // A process check has no containment; a CLI Verifier cannot be proven read-only; an ACP
    // CLI Worker leaves internal tools unmediated. None of them may start under
    // preventive-required, and none creates a run or a preparation operation.
    let cases = vec![
        (
            "process check",
            FakeWorld::new(true, true, vec![api_agent("worker"), api_agent("verifier")])
                .with_definition(definition_with(
                    Some(LoopRequestedMode::PreventiveRequired),
                    Some(1),
                    vec![process_check()],
                )),
        ),
        (
            "cli verifier",
            FakeWorld::new(true, true, vec![api_agent("worker"), agent("verifier")]),
        ),
        (
            "acp worker",
            FakeWorld::new(true, true, vec![agent("acp-worker"), api_agent("verifier")])
                .with_definition({
                    let mut input = definition(true).values().clone();
                    input.worker_agent_id = "acp-worker".to_string();
                    LoopDefinition::new(input).expect("definition")
                }),
        ),
    ];
    for (label, world) in cases {
        let report = world.service().readiness("loop-1").expect("readiness");
        let coverage = report
            .checks
            .iter()
            .find(|check| check.code == LoopReadinessCheckCode::ExecutionCoverage)
            .expect("coverage check");
        assert!(!coverage.passed, "{label}");
        let error = start(&world).expect_err(label);
        assert!(
            error.to_string().contains("preventive-required"),
            "{label}: {error}"
        );
        assert!(world.run.lock().expect("run").is_none(), "{label}");
        assert_eq!(
            *world.operation_starts.lock().expect("starts"),
            0,
            "{label}"
        );
    }
}

#[test]
fn artifact_audited_start_requires_a_fresh_operation_bound_acknowledgement() {
    let world = FakeWorld::new(
        true,
        true,
        vec![agent("plain-worker"), api_agent("verifier")],
    )
    .with_definition({
        let mut input = definition(true).values().clone();
        input.worker_agent_id = "plain-worker".to_string();
        input.requested_mode = Some(LoopRequestedMode::ArtifactAudited);
        input.verification_commands = vec![process_check()];
        LoopDefinition::new(input).expect("definition")
    });
    let service = world.service();

    // Readiness reports the requirement without any receipt existing.
    let report = service.readiness("loop-1").expect("readiness");
    assert!(report.ready, "{:?}", report.checks);
    assert!(report.acknowledgement_required);
    let assessment = report.assessment.expect("assessment");
    assert!(assessment
        .limitations
        .iter()
        .any(|limitation| limitation.contains("cooperative CLI")));

    // Start without a receipt is refused and creates nothing.
    assert!(start(&world).is_err());
    assert!(world.run.lock().expect("run").is_none());

    let admission = service
        .prepare_admission(PrepareLoopAdmissionRequest {
            action: LoopControlAction::Start,
            definition_id: Some("loop-1".to_string()),
            run_id: None,
            expected_revision: 7,
            client_context: "test".to_string(),
        })
        .expect("admission");
    assert_eq!(admission.requested_mode, LoopRequestedMode::ArtifactAudited);
    let challenge = admission.challenge_id.expect("challenge");
    assert!(
        world.run.lock().expect("run").is_none(),
        "admission creates no run"
    );

    // A challenge that was never acknowledged is not a receipt.
    let unacknowledged = service.start_manual(
        "loop-1",
        LoopControlEnvelope {
            expected_revision: Some(7),
            idempotency_key: None,
            audit_acknowledgement_id: Some("loop-audit-unknown".to_string()),
        },
    );
    assert!(unacknowledged.is_err());

    let receipt = service.acknowledge_audit(&challenge).expect("acknowledge");
    // Wrong action: a start receipt never authorizes a resume.
    let stale_action = service.audit_consumption(
        &LoopControlEnvelope {
            expected_revision: Some(7),
            idempotency_key: None,
            audit_acknowledgement_id: Some(receipt.id.clone()),
        },
        LoopControlAction::Resume,
        "loop-1",
        7,
        &admission.scope_digest,
        &assessment,
    );
    assert!(stale_action.is_err());

    let started = service
        .start_manual(
            "loop-1",
            LoopControlEnvelope {
                expected_revision: Some(7),
                idempotency_key: Some("key-1".to_string()),
                audit_acknowledgement_id: Some(receipt.id.clone()),
            },
        )
        .expect("audited start");
    let (_, audit) = world
        .frozen
        .lock()
        .expect("frozen")
        .clone()
        .expect("frozen");
    assert_eq!(audit.expect("consumed").receipt_id, receipt.id);

    // The same idempotency key returns the recorded outcome instead of a second run.
    let replay = service
        .start_manual(
            "loop-1",
            LoopControlEnvelope {
                expected_revision: Some(7),
                idempotency_key: Some("key-1".to_string()),
                audit_acknowledgement_id: Some(receipt.id.clone()),
            },
        )
        .expect("replay");
    assert_eq!(replay, started);
    assert_eq!(*world.operation_starts.lock().expect("starts"), 1);

    // The consumed receipt cannot authorize anything else.
    *world.run.lock().expect("run") = None;
    assert!(service
        .start_manual(
            "loop-1",
            LoopControlEnvelope {
                expected_revision: Some(7),
                idempotency_key: None,
                audit_acknowledgement_id: Some(receipt.id),
            },
        )
        .is_err());
}

#[test]
fn a_receipt_bound_to_another_revision_scope_or_process_is_refused() {
    let world = FakeWorld::new(
        true,
        true,
        vec![agent("plain-worker"), api_agent("verifier")],
    )
    .with_definition({
        let mut input = definition(true).values().clone();
        input.worker_agent_id = "plain-worker".to_string();
        input.requested_mode = Some(LoopRequestedMode::ArtifactAudited);
        LoopDefinition::new(input).expect("definition")
    });
    let service = world.service();
    let admission = service
        .prepare_admission(PrepareLoopAdmissionRequest {
            action: LoopControlAction::Start,
            definition_id: Some("loop-1".to_string()),
            run_id: None,
            expected_revision: 7,
            client_context: "test".to_string(),
        })
        .expect("admission");
    let receipt = service
        .acknowledge_audit(&admission.challenge_id.expect("challenge"))
        .expect("acknowledge");
    let assessment = service
        .assessment()
        .assess(&world.definition)
        .expect("assessment");
    let envelope = LoopControlEnvelope {
        expected_revision: Some(7),
        idempotency_key: None,
        audit_acknowledgement_id: Some(receipt.id.clone()),
    };
    assert!(service
        .audit_consumption(
            &envelope,
            LoopControlAction::Start,
            "loop-1",
            8,
            &admission.scope_digest,
            &assessment
        )
        .is_err());
    assert!(service
        .audit_consumption(
            &envelope,
            LoopControlAction::Start,
            "loop-1",
            7,
            "sha256:other",
            &assessment
        )
        .is_err());
    assert!(service
        .audit_consumption(
            &envelope,
            LoopControlAction::Start,
            "loop-2",
            7,
            &admission.scope_digest,
            &assessment
        )
        .is_err());
    let mut expired = receipt.clone();
    expired.expires_at = "2026-07-21T07:59:00Z".to_string();
    *world.receipts.lock().expect("receipts") = vec![expired];
    assert!(service
        .audit_consumption(
            &envelope,
            LoopControlAction::Start,
            "loop-1",
            7,
            &admission.scope_digest,
            &assessment
        )
        .is_err());
    // A stale definition revision is refused before any receipt is looked at.
    assert!(service
        .prepare_admission(PrepareLoopAdmissionRequest {
            action: LoopControlAction::Start,
            definition_id: Some("loop-1".to_string()),
            run_id: None,
            expected_revision: 6,
            client_context: "test".to_string(),
        })
        .is_err());
}

#[test]
fn invalid_verification_commands_are_rejected_before_readiness_or_launch() {
    let world = FakeWorld::new(true, true, vec![agent("worker"), agent("verifier")]);

    assert!(LoopVerificationCommand::new(
        "tests".to_string(),
        " ".to_string(),
        Vec::new(),
        None,
        0,
        true,
    )
    .is_err());
    assert!(world.run.lock().expect("run").is_none());
    assert_eq!(*world.operation_starts.lock().expect("starts"), 0);
}
