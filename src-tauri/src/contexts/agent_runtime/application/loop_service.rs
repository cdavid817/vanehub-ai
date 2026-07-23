use super::{
    AgentClockPort, AgentRegistryRepository, AgentRuntimeApplicationError, LoopDefinitionView,
    LoopOperationContext, LoopOperationKind, LoopOperationObserver, LoopProjectPort,
    LoopRepository, LoopRunView, SaveLoopDefinitionRequest, StartLoopResultView,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, LoopDefinition, LoopDefinitionInput, LoopRun, LoopRunStatus,
    LoopTerminalReason,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) projects: Arc<dyn LoopProjectPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopApplicationService {
    ports: LoopApplicationPorts,
}

impl LoopApplicationService {
    pub(crate) fn new(ports: LoopApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn list_definitions(
        &self,
    ) -> Result<Vec<LoopDefinitionView>, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .list_definitions()
            .map(|definitions| definitions.iter().map(LoopDefinitionView::from).collect())
    }

    pub(crate) fn create_definition(
        &self,
        mut request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        request.project_path = self.validate_definition_environment(&request)?;
        let now = self.ports.clock.now();
        let definition = LoopDefinition::new(definition_input(
            format!("loop-{}", Uuid::new_v4()),
            request,
            1,
            now.clone(),
            now,
        ))?;
        self.ports.loops.create_definition(&definition)?;
        Ok(LoopDefinitionView::from(&definition))
    }

    pub(crate) fn update_definition(
        &self,
        definition_id: &str,
        mut request: SaveLoopDefinitionRequest,
    ) -> Result<LoopDefinitionView, AgentRuntimeApplicationError> {
        let current = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        let expected_version = request.expected_version.unwrap_or(current.values().version);
        if expected_version != current.values().version {
            return Err(loop_validation(
                "Loop definition was updated by another operation.",
            ));
        }
        request.project_path = self.validate_definition_environment(&request)?;
        let definition = LoopDefinition::new(definition_input(
            current.values().id.clone(),
            request,
            expected_version.saturating_add(1),
            current.values().created_at.clone(),
            self.ports.clock.now(),
        ))?;
        self.ports
            .loops
            .update_definition(&definition, expected_version)?;
        Ok(LoopDefinitionView::from(&definition))
    }

    pub(crate) fn delete_definition(
        &self,
        definition_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        if self.ports.loops.has_active_run(definition_id)? {
            return Err(loop_validation(
                "Cannot delete a Loop definition with an active run.",
            ));
        }
        self.ports.loops.delete_definition(definition_id)
    }

    pub(crate) fn list_runs(
        &self,
        definition_id: Option<&str>,
    ) -> Result<Vec<LoopRunView>, AgentRuntimeApplicationError> {
        self.ports.loops.list_run_views(definition_id)
    }

    pub(crate) fn get_run(
        &self,
        run_id: &str,
    ) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run_view(run_id)?
            .ok_or_else(|| loop_validation("Loop run not found."))
    }

    pub(crate) fn start_manual(
        &self,
        definition_id: &str,
    ) -> Result<StartLoopResultView, AgentRuntimeApplicationError> {
        let definition = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        self.validate_start(&definition)?;

        if self.ports.loops.has_active_run(definition_id)? {
            return Err(loop_validation(
                "This Loop definition already has an active run.",
            ));
        }

        let now = self.ports.clock.now();
        let mut run = LoopRun::new(
            format!("loop-run-{}", Uuid::new_v4()),
            definition_id.to_string(),
        )?;
        self.ports
            .loops
            .create_run(&run, &definition, &definition.values().project_path, &now)?;

        let context = LoopOperationContext {
            run_id: run.id().to_string(),
            iteration_id: None,
            kind: LoopOperationKind::Worktree,
        };
        let operation = match self
            .ports
            .observer
            .start(context, "Preparing isolated worktree")
        {
            Ok(operation) => operation,
            Err(error) => {
                self.fail_queued_run(&mut run, &now);
                return Err(error);
            }
        };

        if let Err(error) = self.ports.loops.attach_run_operation(
            run.id(),
            &operation.id,
            LoopRunStatus::Queued,
            &now,
        ) {
            let _ = self.ports.observer.fail(
                &operation,
                "Loop run changed before preparation could be associated.",
            );
            self.fail_queued_run(&mut run, &now);
            return Err(error);
        }

        Ok(StartLoopResultView {
            run_id: run.id().to_string(),
            operation_id: operation.id,
        })
    }

    fn validate_start(
        &self,
        definition: &LoopDefinition,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let values = definition.values();
        if !values.enabled {
            return Err(loop_validation("Loop definition is disabled."));
        }
        self.ports
            .projects
            .validate_local_git_project(&values.project_path)?;
        self.validate_agent(&values.worker_agent_id)?;
        self.validate_agent(&values.verifier_agent_id)
    }

    fn validate_definition_environment(
        &self,
        request: &SaveLoopDefinitionRequest,
    ) -> Result<String, AgentRuntimeApplicationError> {
        let canonical = self
            .ports
            .projects
            .validate_local_git_project(&request.project_path)?;
        self.validate_agent(&request.worker_agent_id)?;
        self.validate_agent(&request.verifier_agent_id)?;
        Ok(canonical)
    }

    fn validate_agent(&self, agent_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let agent = self
            .ports
            .registry
            .find(agent_id)?
            .ok_or_else(|| AgentRuntimeApplicationError::AgentNotFound(agent_id.to_string()))?;
        agent.ensure_selectable(InteractionMode::Cli)?;
        Ok(())
    }

    fn fail_queued_run(&self, run: &mut LoopRun, now: &str) {
        if run.fail(LoopTerminalReason::RuntimeError).is_ok() {
            let _ =
                self.ports
                    .loops
                    .save_run_transition(run, LoopRunStatus::Queued, now, Some(now));
        }
    }
}

fn definition_input(
    id: String,
    request: SaveLoopDefinitionRequest,
    version: u64,
    created_at: String,
    updated_at: String,
) -> LoopDefinitionInput {
    LoopDefinitionInput {
        id,
        name: request.name,
        enabled: request.enabled,
        project_path: request.project_path,
        base_branch: request.base_branch,
        goal: request.goal,
        acceptance_criteria: request.acceptance_criteria,
        allowed_paths: request.allowed_paths,
        protected_paths: request.protected_paths,
        worker_agent_id: request.worker_agent_id,
        verifier_agent_id: request.verifier_agent_id,
        verification_commands: request.verification_commands,
        limits: request.limits,
        version,
        created_at,
        updated_at,
    }
}

fn loop_validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}
