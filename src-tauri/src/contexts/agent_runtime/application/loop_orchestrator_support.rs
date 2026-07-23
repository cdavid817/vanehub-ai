use super::loop_orchestrator::{current_iteration, elapsed_seconds};
use super::{
    ActiveLoopOperation, AgentLogLevel, AgentRuntimeApplicationError, LoopOperationContext,
    LoopOperationKind, LoopOrchestratorApplicationService, LoopRoleGenerationTerminal, LoopRunView,
    LoopVerificationCancellation, StartLoopWorkerRequest,
};
use crate::contexts::agent_runtime::domain::{LoopRunPhase, LoopTerminalReason};
use std::thread;
use std::time::{Duration, Instant};

impl LoopOrchestratorApplicationService {
    pub(super) fn await_terminal(
        &self,
        session_id: &str,
        timeout_seconds: u64,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopRoleGenerationTerminal, AgentRuntimeApplicationError> {
        let started = Instant::now();
        loop {
            if cancellation.is_cancelled() {
                let _ = self.ports.generations.stop_loop_generation(session_id);
                return Err(AgentRuntimeApplicationError::Loop(
                    "Loop role generation cancelled.".to_string(),
                ));
            }
            if let Some(terminal) = self.ports.completions.take_for_session(session_id)? {
                return Ok(terminal);
            }
            if started.elapsed() >= Duration::from_secs(timeout_seconds) {
                let _ = self.ports.generations.stop_loop_generation(session_id);
                return Err(AgentRuntimeApplicationError::Loop(
                    "Loop phase timed out.".to_string(),
                ));
            }
            thread::sleep(Duration::from_millis(100));
        }
    }

    pub(super) fn pause_at_boundary(
        &self,
        run_id: &str,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let mut run = self.run(run_id)?;
        if !run.pause_requested() {
            return Ok(false);
        }
        let expected = run.status();
        run.pause_at_boundary()?;
        self.save_run(&run, expected, None)?;
        Ok(true)
    }

    pub(super) fn handle_failure(
        &self,
        run_id: &str,
        error: &AgentRuntimeApplicationError,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let view = self.run_view(run_id)?;
        let mut run = self.run(run_id)?;
        if run.status().is_terminal() {
            return Ok(false);
        }
        let expected = run.status();
        let snapshot = self.snapshot(run_id)?;
        let retry = if error.to_string().contains("phase timed out") {
            run.fail(LoopTerminalReason::PhaseTimeout)?;
            false
        } else {
            !run.record_runtime_outcome(true, &snapshot.values().limits)?
        };
        let now = self.ports.clock.now();
        self.save_run(&run, expected, (!retry).then_some(now.as_str()))?;
        let context = operation_context(&view);
        self.ports.observer.record(
            &context,
            view.active_operation_id.as_deref(),
            AgentLogLevel::Error,
            &error.to_string(),
        )?;
        if !retry && context.kind == LoopOperationKind::Worktree {
            if let Some(operation_id) = &view.active_operation_id {
                self.ports.observer.fail(
                    &ActiveLoopOperation {
                        id: operation_id.clone(),
                        context,
                    },
                    &error.to_string(),
                )?;
            }
        }
        Ok(retry)
    }

    pub(crate) fn record_background_failure(
        &self,
        run_id: &str,
        error: &AgentRuntimeApplicationError,
    ) {
        let context = self
            .run_view(run_id)
            .map(|view| operation_context(&view))
            .unwrap_or(LoopOperationContext {
                run_id: run_id.to_string(),
                iteration_id: None,
                kind: LoopOperationKind::Decision,
            });
        let _ = self.ports.observer.record(
            &context,
            None,
            AgentLogLevel::Error,
            &format!("Loop runtime stopped unexpectedly: {error}"),
        );
    }

    pub(super) fn finish_role_error(
        &self,
        operation: &ActiveLoopOperation,
        error: &AgentRuntimeApplicationError,
        cancellation: &LoopVerificationCancellation,
    ) {
        if cancellation.is_cancelled() || error.to_string().contains("cancelled") {
            let _ = self
                .ports
                .observer
                .cancel(operation, "The Loop role generation was cancelled.");
        } else {
            let _ = self.ports.observer.fail(operation, &error.to_string());
        }
    }

    pub(super) fn stop_active_role(&self, view: &LoopRunView) {
        if let Ok(iteration) = current_iteration(view) {
            if let Some(session_id) = iteration
                .verifier_session_id
                .as_ref()
                .or(iteration.worker_session_id.as_ref())
            {
                let _ = self.ports.generations.stop_loop_generation(session_id);
            }
        }
    }

    pub(super) fn worker_request(&self, view: &LoopRunView) -> StartLoopWorkerRequest {
        StartLoopWorkerRequest {
            run_id: view.id.clone(),
            sequence: view.current_iteration,
            definition_snapshot: view.definition_snapshot.clone(),
            project_path: view.project_path.clone(),
            worktree_path: view.worktree_path.clone().unwrap_or_default(),
            worktree_name: view.worktree_name.clone().unwrap_or_default(),
            worktree_branch: view.worktree_branch.clone().unwrap_or_default(),
            prior_evidence: view
                .iterations
                .iter()
                .flat_map(|item| item.evidence.clone())
                .collect(),
            user_feedback: view
                .iterations
                .last()
                .and_then(|item| item.user_feedback.clone()),
            elapsed_seconds: elapsed_seconds(&view.created_at),
        }
    }
}

fn operation_context(view: &LoopRunView) -> LoopOperationContext {
    let kind = match view.phase {
        LoopRunPhase::Preparing => LoopOperationKind::Worktree,
        LoopRunPhase::Acting => LoopOperationKind::RoleGeneration,
        LoopRunPhase::Verifying => LoopOperationKind::Verification,
        LoopRunPhase::Deciding | LoopRunPhase::Finalizing => LoopOperationKind::Decision,
    };
    LoopOperationContext {
        run_id: view.id.clone(),
        iteration_id: current_iteration(view)
            .ok()
            .map(|iteration| iteration.id.clone()),
        kind,
    }
}
