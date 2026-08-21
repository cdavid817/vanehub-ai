use super::{
    AgentClockPort, AgentLogLevel, AgentRegistryRepository, AgentRuntimeApplicationError,
    ApiAgentGateway, CanonicalLoopSignal, LoopDefinitionView, LoopOperationContext,
    LoopOperationKind, LoopOperationObserver, LoopProjectPort, LoopReadinessCheckView,
    LoopReadinessReportView, LoopRepository, LoopRunView, SaveLoopDefinitionRequest,
    StartLoopResultView,
};
use crate::contexts::agent_runtime::domain::{
    InteractionMode, LoopDefinition, LoopDefinitionInput, LoopReadinessCategory,
    LoopReadinessCheckCode, LoopRun, LoopRunStatus, LoopTerminalReason,
};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) registry: Arc<dyn AgentRegistryRepository>,
    pub(crate) api_agents: Arc<dyn ApiAgentGateway>,
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

    pub(crate) fn readiness(
        &self,
        definition_id: &str,
    ) -> Result<LoopReadinessReportView, AgentRuntimeApplicationError> {
        let definition = self
            .ports
            .loops
            .find_definition(definition_id)?
            .ok_or_else(|| loop_validation("Loop definition not found."))?;
        let values = definition.values();
        let project = self
            .ports
            .projects
            .validate_local_git_project(&values.project_path);
        let project_ready = project.is_ok();
        let branch = project.as_ref().ok().map_or(Ok(false), |canonical| {
            self.ports
                .projects
                .base_branch_available(canonical, &values.base_branch)
        });
        let worker = self.validate_agent(&values.worker_agent_id);
        let verifier = self.validate_agent(&values.verifier_agent_id);
        let commands_ready = !values.verification_commands.is_empty()
            && values.verification_commands.iter().all(|command| {
                !command.program().trim().is_empty() && command.timeout_seconds() > 0
            });
        let scope_ready = !values.allowed_paths.iter().any(|allowed| {
            values
                .protected_paths
                .iter()
                .any(|protected| normalized_scope(allowed) == normalized_scope(protected))
        });
        let no_active_run = !self.ports.loops.has_active_run(definition_id)?;
        let checks = vec![
            readiness_check(
                LoopReadinessCheckCode::DefinitionEnabled,
                LoopReadinessCategory::Definition,
                values.enabled,
                None,
                "definition",
            ),
            readiness_check(
                LoopReadinessCheckCode::ProjectAvailable,
                LoopReadinessCategory::Workspace,
                project_ready,
                project.err(),
                "project",
            ),
            readiness_check(
                LoopReadinessCheckCode::BranchAvailable,
                LoopReadinessCategory::Workspace,
                branch.as_ref().copied().unwrap_or(false),
                branch.err(),
                "branch",
            ),
            readiness_check(
                LoopReadinessCheckCode::WorkerEligible,
                LoopReadinessCategory::Agent,
                worker.is_ok(),
                worker.err(),
                "worker",
            ),
            readiness_check(
                LoopReadinessCheckCode::VerifierEligible,
                LoopReadinessCategory::Agent,
                verifier.is_ok(),
                verifier.err(),
                "verifier",
            ),
            readiness_check(
                LoopReadinessCheckCode::VerificationValid,
                LoopReadinessCategory::Verification,
                commands_ready,
                None,
                "verification",
            ),
            readiness_check(
                LoopReadinessCheckCode::PathScopeValid,
                LoopReadinessCategory::Verification,
                scope_ready,
                None,
                "verification",
            ),
            readiness_check(
                LoopReadinessCheckCode::NoActiveRun,
                LoopReadinessCategory::Runtime,
                no_active_run,
                None,
                "runs",
            ),
        ];
        let report = LoopReadinessReportView {
            definition_id: definition_id.to_string(),
            ready: checks.iter().all(|check| check.passed),
            checks,
            checked_at: self.ports.clock.now(),
        };
        if !report.ready {
            let blocked = report
                .checks
                .iter()
                .filter(|check| !check.passed)
                .map(|check| check.code.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let _ = self.ports.observer.record(
                &LoopOperationContext {
                    run_id: definition_id.to_string(),
                    iteration_id: None,
                    kind: LoopOperationKind::Readiness,
                },
                None,
                AgentLogLevel::Warn,
                &format!("Loop readiness blocked: {blocked}"),
            );
        }
        Ok(report)
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
        if let Err(error) = self
            .ports
            .observer
            .start_canonical_loop(run.id(), definition_id)
        {
            self.fail_queued_run(&mut run, &now);
            return Err(error);
        }

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
        if agent.supports(InteractionMode::Cli) {
            agent.ensure_selectable(InteractionMode::Cli)?;
            return Ok(());
        }
        agent.ensure_selectable(InteractionMode::Api)?;
        let trusted = self
            .ports
            .api_agents
            .provider_config(agent_id)?
            .map(|config| config.auto_approve_tools)
            .unwrap_or(false);
        if !trusted {
            return Err(loop_validation(
                "API agent must have tool-use trust enabled before it can be used as a Loop Worker or Verifier.",
            ));
        }
        Ok(())
    }

    fn fail_queued_run(&self, run: &mut LoopRun, now: &str) {
        if run.fail(LoopTerminalReason::RuntimeError).is_ok() {
            let _ =
                self.ports
                    .loops
                    .save_run_transition(run, LoopRunStatus::Queued, now, Some(now));
        }
        let _ = self
            .ports
            .observer
            .signal_canonical_loop(run.id(), CanonicalLoopSignal::Failed);
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

fn readiness_check(
    code: LoopReadinessCheckCode,
    category: LoopReadinessCategory,
    passed: bool,
    error: Option<AgentRuntimeApplicationError>,
    remediation_target: &'static str,
) -> LoopReadinessCheckView {
    LoopReadinessCheckView {
        code,
        category,
        passed,
        detail: error.map(|value| value.to_string()),
        remediation_target: (!passed).then_some(remediation_target),
    }
}

fn normalized_scope(value: &str) -> String {
    value
        .trim_matches(['/', '\\'])
        .replace('\\', "/")
        .to_lowercase()
}
