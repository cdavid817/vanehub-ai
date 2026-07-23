use crate::contexts::agent_runtime::application::{
    AgentRuntimeApplicationError, LoopExecutionControlPort, LoopExecutionLeasePort,
    LoopVerificationCancellation,
};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct LoopExecutionLease {
    pub(crate) run_id: String,
    lease_id: u64,
    pub(crate) cancellation: LoopVerificationCancellation,
}

#[derive(Clone, Default)]
pub(crate) struct InMemoryLoopExecutionCoordinator {
    active: Arc<Mutex<HashMap<String, LoopExecutionLease>>>,
    next_lease_id: Arc<AtomicU64>,
}

impl InMemoryLoopExecutionCoordinator {
    pub(crate) fn reserve(
        &self,
        run_id: &str,
    ) -> Result<LoopExecutionLease, AgentRuntimeApplicationError> {
        let mut active = self.active.lock().map_err(lock_error)?;
        if active.contains_key(run_id) {
            return Err(AgentRuntimeApplicationError::Loop(format!(
                "Loop run {run_id} already has an active execution lease."
            )));
        }
        let lease = LoopExecutionLease {
            run_id: run_id.to_string(),
            lease_id: self.next_lease_id.fetch_add(1, Ordering::Relaxed) + 1,
            cancellation: LoopVerificationCancellation::default(),
        };
        active.insert(run_id.to_string(), lease.clone());
        Ok(lease)
    }

    pub(crate) fn release(
        &self,
        lease: &LoopExecutionLease,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active.lock().map_err(lock_error)?;
        let current = active
            .get(&lease.run_id)
            .ok_or_else(|| lease_error(&lease.run_id))?;
        if current.lease_id != lease.lease_id {
            return Err(lease_error(&lease.run_id));
        }
        active.remove(&lease.run_id);
        Ok(())
    }
}

impl LoopExecutionLeasePort for InMemoryLoopExecutionCoordinator {
    fn has_live_lease(&self, run_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self.active.lock().map_err(lock_error)?.contains_key(run_id))
    }
}

impl LoopExecutionControlPort for InMemoryLoopExecutionCoordinator {
    fn request_cancellation(&self, run_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        if let Some(lease) = self.active.lock().map_err(lock_error)?.get(run_id) {
            lease.cancellation.cancel();
        }
        Ok(())
    }
}

fn lock_error<T>(error: std::sync::PoisonError<T>) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(format!("Loop execution coordinator lock failed: {error}"))
}

fn lease_error(run_id: &str) -> AgentRuntimeApplicationError {
    AgentRuntimeApplicationError::Loop(format!("Loop execution lease no longer owns run {run_id}."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leases_are_exclusive_cancellable_and_ownership_checked() {
        let coordinator = InMemoryLoopExecutionCoordinator::default();
        let lease = coordinator.reserve("run-1").expect("reserve");
        assert!(coordinator.has_live_lease("run-1").expect("live"));
        assert!(coordinator.reserve("run-1").is_err());
        coordinator
            .request_cancellation("run-1")
            .expect("cancel request");
        assert!(lease.cancellation.is_cancelled());
        coordinator.release(&lease).expect("release");
        assert!(!coordinator.has_live_lease("run-1").expect("released"));
        assert!(coordinator.release(&lease).is_err());
        coordinator
            .request_cancellation("missing")
            .expect("idempotent missing cancellation");
    }
}
