use super::{
    ActiveLoopOperation, AgentClockPort, AgentLogLevel, AgentRuntimeApplicationError,
    LoopGenerationControlPort, LoopIterationRepository, LoopOperationContext, LoopOperationKind,
    LoopOperationObserver, LoopProgressApplicationService, LoopProjectPort, LoopRepository,
    LoopRoleGenerationCompletionPort, LoopRunView, LoopVerificationApplicationService,
    LoopVerificationCancellation, LoopVerifierApplicationService, LoopVerifierContextPort,
    LoopWorkerApplicationService, RunLoopVerificationRequest,
};
use crate::contexts::agent_runtime::domain::{LoopRunPhase, LoopRunStatus};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

#[derive(Clone)]
pub(crate) struct LoopOrchestratorPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) iterations: Arc<dyn LoopIterationRepository>,
    pub(crate) projects: Arc<dyn LoopProjectPort>,
    pub(crate) verifier_context: Arc<dyn LoopVerifierContextPort>,
    pub(crate) completions: Arc<dyn LoopRoleGenerationCompletionPort>,
    pub(crate) generations: Arc<dyn LoopGenerationControlPort>,
    pub(crate) worker: LoopWorkerApplicationService,
    pub(crate) verification: LoopVerificationApplicationService,
    pub(crate) verifier: LoopVerifierApplicationService,
    pub(crate) progress: LoopProgressApplicationService,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopOrchestratorApplicationService {
    pub(super) ports: LoopOrchestratorPorts,
}

impl LoopOrchestratorApplicationService {
    pub(crate) fn new(ports: LoopOrchestratorPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn execute(
        &self,
        run_id: &str,
        cancellation: LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        loop {
            match self.execute_inner(run_id, &cancellation) {
                Ok(()) => return Ok(()),
                Err(error) if self.handle_failure(run_id, &error)? => {
                    thread::sleep(Duration::from_millis(250));
                }
                Err(error) => return Err(error),
            }
        }
    }

    fn execute_inner(
        &self,
        run_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        loop {
            let view = self.run_view(run_id)?;
            if view.status.is_terminal() || view.status == LoopRunStatus::AwaitingAcceptance {
                return Ok(());
            }
            if cancellation.is_cancelled() {
                self.stop_active_role(&view);
                return Ok(());
            }
            let mut limited_run = self.run(run_id)?;
            let snapshot = self.snapshot(run_id)?;
            let expected = limited_run.status();
            if limited_run
                .enforce_elapsed_limits(
                    elapsed_seconds(&view.created_at),
                    0,
                    &snapshot.values().limits,
                )?
                .is_some()
            {
                let now = self.ports.clock.now();
                self.save_run(&limited_run, expected, Some(&now))?;
                return Ok(());
            }
            match (view.status, view.phase) {
                (LoopRunStatus::Queued, LoopRunPhase::Preparing) => self.prepare(&view)?,
                (LoopRunStatus::Running, LoopRunPhase::Acting) => self.act(&view, cancellation)?,
                (LoopRunStatus::Running, LoopRunPhase::Verifying) => {
                    self.verify(&view, cancellation)?
                }
                (LoopRunStatus::Running, LoopRunPhase::Deciding) => {
                    self.decide(&view, cancellation)?
                }
                _ => return Ok(()),
            }
            if self.pause_at_boundary(run_id)? {
                return Ok(());
            }
        }
    }

    fn prepare(&self, view: &LoopRunView) -> Result<(), AgentRuntimeApplicationError> {
        let context = LoopOperationContext {
            run_id: view.id.clone(),
            iteration_id: None,
            kind: LoopOperationKind::Worktree,
        };
        let operation = ActiveLoopOperation {
            id: required(&view.active_operation_id, "Loop preparation operation")?,
            context,
        };
        self.ports.observer.record(
            &operation.context,
            Some(&operation.id),
            AgentLogLevel::Info,
            "Preparing the isolated Loop worktree.",
        )?;
        if view.worktree_path.is_none() {
            let created = self.ports.projects.prepare_loop_worktree(
                &view.project_path,
                &view.id,
                &view.definition_snapshot.base_branch,
            )?;
            self.ports.loops.attach_run_worktree(
                &view.id,
                &created.path,
                &created.name,
                &created.branch,
                LoopRunStatus::Queued,
            )?;
        }
        self.ports
            .observer
            .complete(&operation, "The isolated Loop worktree is ready.")?;
        let mut run = self.run(&view.id)?;
        run.begin()?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        self.save_run(&run, LoopRunStatus::Queued, None)
    }

    fn act(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let current = view
            .iterations
            .iter()
            .find(|item| item.sequence == view.current_iteration);
        if current
            .and_then(|item| item.worker_summary.as_ref())
            .is_none()
        {
            let request = self.worker_request(view);
            let started = match current {
                Some(item) => self.ports.worker.resume_iteration(&item.id, request)?,
                None => self.ports.worker.start_iteration(request)?,
            };
            let operation = match self.ports.observer.start(
                LoopOperationContext {
                    run_id: view.id.clone(),
                    iteration_id: Some(started.iteration_id.clone()),
                    kind: LoopOperationKind::RoleGeneration,
                },
                "Running the Loop Worker role",
            ) {
                Ok(operation) => operation,
                Err(error) => {
                    let _ = self
                        .ports
                        .generations
                        .stop_loop_generation(&started.session_id);
                    return Err(error);
                }
            };
            let result = (|| {
                let terminal = self.await_terminal(
                    &started.session_id,
                    view.definition_snapshot.limits.step_timeout_seconds,
                    cancellation,
                )?;
                self.ports.worker.complete(terminal)
            })();
            match result {
                Ok(_) => self
                    .ports
                    .observer
                    .complete(&operation, "The Loop Worker role completed.")?,
                Err(error) => {
                    self.finish_role_error(&operation, &error, cancellation);
                    return Err(error);
                }
            }
        }
        let mut run = self.run(&view.id)?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        run.move_to(LoopRunPhase::Verifying)?;
        self.save_run(&run, LoopRunStatus::Running, None)
    }

    fn verify(
        &self,
        view: &LoopRunView,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let iteration = current_iteration(view)?;
        let existing = view
            .definition_snapshot
            .verification_commands
            .iter()
            .all(|command| {
                iteration.evidence.iter().any(|item| {
                    item.kind == "verification-command"
                        && item.command_id.as_deref() == Some(command.id.as_str())
                })
            });
        if !existing {
            let result = self
                .ports
                .verification
                .run_commands(RunLoopVerificationRequest {
                    run_id: view.id.clone(),
                    iteration_id: iteration.id.clone(),
                    worktree_root: required(&view.worktree_path, "Loop worktree path")?,
                    commands: view.definition_snapshot.verification_commands.clone(),
                    cancellation: cancellation.clone(),
                })?;
            if result.cancelled {
                return Ok(());
            }
        }
        let mut run = self.run(&view.id)?;
        let snapshot = self.snapshot(&view.id)?;
        run.record_runtime_outcome(false, &snapshot.values().limits)?;
        run.move_to(LoopRunPhase::Deciding)?;
        self.save_run(&run, LoopRunStatus::Running, None)
    }

    pub(super) fn run_view(&self, id: &str) -> Result<LoopRunView, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run_view(id)?
            .ok_or_else(|| missing("Loop run"))
    }
    pub(super) fn run(
        &self,
        id: &str,
    ) -> Result<crate::contexts::agent_runtime::domain::LoopRun, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run(id)?
            .ok_or_else(|| missing("Loop run"))
    }
    pub(super) fn snapshot(
        &self,
        id: &str,
    ) -> Result<crate::contexts::agent_runtime::domain::LoopDefinition, AgentRuntimeApplicationError>
    {
        self.ports
            .loops
            .find_run_definition_snapshot(id)?
            .ok_or_else(|| missing("Loop definition snapshot"))
    }
    pub(super) fn save_run(
        &self,
        run: &crate::contexts::agent_runtime::domain::LoopRun,
        expected: LoopRunStatus,
        completed: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports
            .loops
            .save_run_transition(run, expected, &self.ports.clock.now(), completed)
    }
}

pub(super) fn current_iteration(
    view: &LoopRunView,
) -> Result<&super::LoopIterationView, AgentRuntimeApplicationError> {
    view.iterations
        .iter()
        .find(|item| item.sequence == view.current_iteration)
        .ok_or_else(|| missing("Current Loop iteration"))
}
pub(super) fn required(
    value: &Option<String>,
    label: &str,
) -> Result<String, AgentRuntimeApplicationError> {
    value.clone().ok_or_else(|| missing(label))
}
pub(super) fn elapsed_seconds(created_at: &str) -> u64 {
    chrono::DateTime::parse_from_rfc3339(created_at)
        .ok()
        .and_then(|created| {
            u64::try_from(
                chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(created.timestamp()),
            )
            .ok()
        })
        .unwrap_or(0)
}
pub(super) fn missing(label: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(format!("{label} is unavailable."))
}
