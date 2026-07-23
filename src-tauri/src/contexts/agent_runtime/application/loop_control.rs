use super::{
    AgentClockPort, AgentRuntimeApplicationError, ContinueLoopRequest, LoopExecutionControlPort,
    LoopOperationContext, LoopOperationKind, LoopOperationObserver, LoopRepository,
};
use crate::contexts::agent_runtime::domain::{LoopRun, LoopRunStatus, LoopTerminalReason};
use std::sync::Arc;

const MAX_FEEDBACK_BYTES: usize = 16 * 1024;

#[derive(Clone)]
pub(crate) struct LoopControlApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) execution: Arc<dyn LoopExecutionControlPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopControlApplicationService {
    ports: LoopControlApplicationPorts,
}

impl LoopControlApplicationService {
    pub(crate) fn new(ports: LoopControlApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn request_pause(
        &self,
        run_id: &str,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        let expected_pause_requested = run.pause_requested();
        run.request_pause()?;
        self.ports.loops.save_pause_request(
            &run,
            expected_status,
            expected_pause_requested,
            &self.ports.clock.now(),
        )?;
        Ok(run)
    }

    pub(crate) fn pause_at_boundary(
        &self,
        run_id: &str,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.pause_at_boundary()?;
        self.save_transition(&run, expected_status, None)?;
        Ok(run)
    }

    pub(crate) fn resume(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.resume()?;
        self.save_transition(&run, expected_status, None)?;
        Ok(run)
    }

    pub(crate) fn cancel(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.cancel(LoopTerminalReason::UserStopped)?;
        let operation = self.ports.observer.start(
            LoopOperationContext {
                run_id: run_id.to_string(),
                iteration_id: None,
                kind: LoopOperationKind::Cancellation,
            },
            "Requesting immediate Loop cancellation",
        )?;
        let completed_at = self.ports.clock.now();
        if let Err(error) = self.ports.loops.save_run_transition(
            &run,
            expected_status,
            &completed_at,
            Some(&completed_at),
        ) {
            let _ = self.ports.observer.fail(&operation, &error.to_string());
            return Err(error);
        }
        if let Err(error) = self.ports.execution.request_cancellation(run_id) {
            let _ = self.ports.observer.fail(&operation, &error.to_string());
            return Err(error);
        }
        self.ports
            .observer
            .complete(&operation, "Loop cancellation was requested.")?;
        Ok(run)
    }

    pub(crate) fn accept(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.accept()?;
        let completed_at = self.ports.clock.now();
        self.ports.loops.save_run_transition(
            &run,
            expected_status,
            &completed_at,
            Some(&completed_at),
        )?;
        Ok(run)
    }

    pub(crate) fn continue_with_feedback(
        &self,
        request: ContinueLoopRequest,
    ) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let feedback = request.feedback.trim();
        if feedback.is_empty() {
            return Err(validation("Continuation feedback is required."));
        }
        if feedback.len() > MAX_FEEDBACK_BYTES {
            return Err(validation("Continuation feedback is too large."));
        }

        let mut run = self.find_run(&request.run_id)?;
        let snapshot = self
            .ports
            .loops
            .find_run_definition_snapshot(&request.run_id)?
            .ok_or_else(|| loop_error("Loop definition snapshot not found."))?;
        let expected_status = run.status();
        run.continue_iteration(&snapshot.values().limits)?;
        self.ports.loops.save_continue_transition(
            &run,
            expected_status,
            feedback,
            &self.ports.clock.now(),
        )?;
        Ok(run)
    }

    pub(crate) fn reject(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        let mut run = self.find_run(run_id)?;
        let expected_status = run.status();
        run.cancel(LoopTerminalReason::UserRejected)?;
        let completed_at = self.ports.clock.now();
        self.ports.loops.save_run_transition(
            &run,
            expected_status,
            &completed_at,
            Some(&completed_at),
        )?;
        Ok(run)
    }

    fn find_run(&self, run_id: &str) -> Result<LoopRun, AgentRuntimeApplicationError> {
        self.ports
            .loops
            .find_run(run_id)?
            .ok_or_else(|| loop_error("Loop run not found."))
    }

    fn save_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        completed_at: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        self.ports.loops.save_run_transition(
            run,
            expected_status,
            &self.ports.clock.now(),
            completed_at,
        )
    }
}

fn validation(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Validation(message.to_string())
}

fn loop_error(message: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(message.to_string())
}
