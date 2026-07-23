use super::*;
use crate::contexts::agent_runtime::domain::{
    AgentAvailability, AgentDefinition, AgentDefinitionInput, AvailabilityAssessment,
    InteractionMode, LaunchMetadata, LoopDefinition, LoopDefinitionInput, LoopLimits, LoopRun,
    LoopRunPhase, LoopRunStatus, LoopVerificationCommand,
};
use std::sync::{Arc, Mutex};

struct FakeWorld {
    definition: LoopDefinition,
    agents: Vec<AgentDefinition>,
    project_is_git: bool,
    run: Mutex<Option<LoopRun>>,
    snapshot: Mutex<Option<LoopDefinition>>,
    operation_id: Mutex<Option<String>>,
    operation_starts: Mutex<u16>,
    logs: Mutex<Vec<LoopLog>>,
}

impl FakeWorld {
    fn new(enabled: bool, project_is_git: bool, agents: Vec<AgentDefinition>) -> Arc<Self> {
        Arc::new(Self {
            definition: definition(enabled),
            agents,
            project_is_git,
            run: Mutex::new(None),
            snapshot: Mutex::new(None),
            operation_id: Mutex::new(None),
            operation_starts: Mutex::new(0),
            logs: Mutex::new(Vec::new()),
        })
    }

    fn service(self: &Arc<Self>) -> LoopApplicationService {
        LoopApplicationService::new(LoopApplicationPorts {
            loops: self.clone(),
            registry: self.clone(),
            projects: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
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
        limits: LoopLimits::new(3, 60, 600, 2, 2).expect("limits"),
        version: 7,
        created_at: "2026-07-21T07:00:00Z".to_string(),
        updated_at: "2026-07-21T07:30:00Z".to_string(),
    })
    .expect("definition")
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

#[test]
fn manual_start_persists_snapshot_before_preparation_operation() {
    let world = FakeWorld::new(true, true, vec![agent("worker"), agent("verifier")]);
    let result = world.service().start_manual("loop-1").expect("start");

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
        FakeWorld::new(false, true, vec![agent("worker"), agent("verifier")]),
        FakeWorld::new(true, false, vec![agent("worker"), agent("verifier")]),
        FakeWorld::new(true, true, vec![agent("worker")]),
    ] {
        assert!(world.service().start_manual("loop-1").is_err());
        assert!(world.run.lock().expect("run").is_none());
        assert_eq!(*world.operation_starts.lock().expect("starts"), 0);
    }
}

#[test]
fn manual_start_rejects_a_second_active_run() {
    let world = FakeWorld::new(true, true, vec![agent("worker"), agent("verifier")]);
    world.service().start_manual("loop-1").expect("first run");
    assert!(world.service().start_manual("loop-1").is_err());
    assert_eq!(*world.operation_starts.lock().expect("starts"), 1);
}
