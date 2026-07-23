use super::InMemoryLoopExecutionCoordinator;
use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopOrchestratorApplicationService,
};

#[derive(Clone)]
pub(crate) struct NativeLoopScheduler {
    coordinator: InMemoryLoopExecutionCoordinator,
    orchestrator: LoopOrchestratorApplicationService,
}

impl NativeLoopScheduler {
    pub(crate) fn new(
        coordinator: InMemoryLoopExecutionCoordinator,
        orchestrator: LoopOrchestratorApplicationService,
    ) -> Self {
        Self {
            coordinator,
            orchestrator,
        }
    }

    pub(crate) fn schedule(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        let lease = self.coordinator.reserve(run_id)?;
        let thread_lease = lease.clone();
        let coordinator = self.coordinator.clone();
        let orchestrator = self.orchestrator.clone();
        std::thread::Builder::new()
            .name(format!("loop-runtime-{run_id}"))
            .spawn(move || {
                if let Err(error) =
                    orchestrator.execute(&thread_lease.run_id, thread_lease.cancellation.clone())
                {
                    orchestrator.record_background_failure(&thread_lease.run_id, &error);
                }
                let _ = coordinator.release(&thread_lease);
            })
            .map_err(|error| {
                let _ = self.coordinator.release(&lease);
                AgentRuntimeApplicationError::Loop(format!(
                    "Could not start Loop runtime thread: {error}"
                ))
            })?;
        Ok(())
    }
}
