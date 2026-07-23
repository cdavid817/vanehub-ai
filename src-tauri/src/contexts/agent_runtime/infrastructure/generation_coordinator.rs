use crate::contexts::agent_runtime::application::{
    AgentGenerationPort, AgentRuntimeApplicationError, GenerationCancellation, GenerationLease,
};
use crate::contexts::agent_runtime::domain::GenerationAttempt;
use crate::contexts::execution_observability::api::ExecutionContext;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub(crate) struct InMemoryGenerationCoordinator {
    active: Arc<Mutex<HashMap<String, CoordinatedGeneration>>>,
    lease_ids: Arc<AtomicU64>,
}

struct CoordinatedGeneration {
    lease_id: String,
    attempt: GenerationAttempt,
    process_id: Option<String>,
    operation_id: Option<String>,
    execution_context: Option<ExecutionContext>,
}

impl AgentGenerationPort for InMemoryGenerationCoordinator {
    fn reserve(&self, session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError> {
        let attempt = GenerationAttempt::reserve(session_id)?;
        let mut active = self.active()?;
        if active.contains_key(session_id) {
            return Err(AgentRuntimeApplicationError::GenerationConflict(
                session_id.to_string(),
            ));
        }
        let lease_id = format!(
            "generation-lease-{}",
            self.lease_ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        active.insert(
            session_id.to_string(),
            CoordinatedGeneration {
                lease_id: lease_id.clone(),
                attempt,
                process_id: None,
                operation_id: None,
                execution_context: None,
            },
        );
        Ok(GenerationLease {
            session_id: session_id.to_string(),
            lease_id,
        })
    }

    fn correlate(
        &self,
        lease: &GenerationLease,
        execution_context: &ExecutionContext,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        require_lease(&mut active, lease)?.execution_context = Some(execution_context.clone());
        Ok(())
    }

    fn attach(
        &self,
        lease: &GenerationLease,
        message_id: &str,
        process_id: &str,
        operation_id: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        let generation = require_lease(&mut active, lease)?;
        generation.attempt.attach(message_id, true)?;
        generation.process_id = Some(process_id.to_string());
        generation.operation_id = Some(operation_id.to_string());
        Ok(())
    }

    fn release(&self, lease: &GenerationLease) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        require_lease(&mut active, lease)?;
        active.remove(&lease.session_id);
        Ok(())
    }

    fn cancel(
        &self,
        session_id: &str,
    ) -> Result<Option<GenerationCancellation>, AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        let Some(mut generation) = active.remove(session_id) else {
            return Ok(None);
        };
        let cancellation = generation.attempt.cancel()?;
        Ok(Some(GenerationCancellation {
            message_id: cancellation.message_id,
            process_id: generation.process_id,
            operation_id: generation.operation_id,
            execution_context: generation.execution_context,
        }))
    }

    fn complete(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.finish(session_id, GenerationAttempt::complete)
    }

    fn fail(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.finish(session_id, GenerationAttempt::fail)
    }
}

impl InMemoryGenerationCoordinator {
    fn finish(
        &self,
        session_id: &str,
        transition: fn(
            &mut GenerationAttempt,
        ) -> Result<
            (),
            crate::contexts::agent_runtime::domain::AgentRuntimeDomainError,
        >,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        let generation = active.get_mut(session_id).ok_or_else(|| {
            AgentRuntimeApplicationError::Generation(format!(
                "Generation for session {session_id} is not active."
            ))
        })?;
        transition(&mut generation.attempt)?;
        active.remove(session_id);
        Ok(())
    }

    fn active(
        &self,
    ) -> Result<
        std::sync::MutexGuard<'_, HashMap<String, CoordinatedGeneration>>,
        AgentRuntimeApplicationError,
    > {
        self.active
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Generation(error.to_string()))
    }
}

fn require_lease<'a>(
    active: &'a mut HashMap<String, CoordinatedGeneration>,
    lease: &GenerationLease,
) -> Result<&'a mut CoordinatedGeneration, AgentRuntimeApplicationError> {
    let generation = active.get_mut(&lease.session_id).ok_or_else(|| {
        AgentRuntimeApplicationError::Generation(format!(
            "Generation for session {} was cancelled before startup completed.",
            lease.session_id
        ))
    })?;
    if generation.lease_id != lease.lease_id {
        return Err(AgentRuntimeApplicationError::Generation(
            "Generation lease does not own the active reservation.".to_string(),
        ));
    }
    Ok(generation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reservation_is_exclusive_and_release_allows_retry() {
        let coordinator = InMemoryGenerationCoordinator::default();
        let lease = coordinator.reserve("session-1").expect("reserve");

        assert!(matches!(
            coordinator.reserve("session-1"),
            Err(AgentRuntimeApplicationError::GenerationConflict(session_id))
                if session_id == "session-1"
        ));

        coordinator.release(&lease).expect("release");
        assert!(coordinator.reserve("session-1").is_ok());
    }

    #[test]
    fn cancellation_returns_attached_message_process_and_operation() {
        let coordinator = InMemoryGenerationCoordinator::default();
        let lease = coordinator.reserve("session-1").expect("reserve");
        coordinator
            .attach(&lease, "message-1", "process-1", "operation-1")
            .expect("attach");

        let cancellation = coordinator
            .cancel("session-1")
            .expect("cancel")
            .expect("active");

        assert_eq!(cancellation.message_id.as_deref(), Some("message-1"));
        assert_eq!(cancellation.process_id.as_deref(), Some("process-1"));
        assert_eq!(cancellation.operation_id.as_deref(), Some("operation-1"));
        assert_eq!(coordinator.cancel("session-1").expect("again"), None);
    }

    #[test]
    fn completion_and_failure_are_terminal_and_remove_the_claim() {
        let coordinator = InMemoryGenerationCoordinator::default();
        let complete = coordinator.reserve("complete").expect("reserve complete");
        coordinator
            .attach(&complete, "message-1", "process-1", "operation-1")
            .expect("attach complete");
        coordinator.complete("complete").expect("complete");

        let failed = coordinator.reserve("failed").expect("reserve failed");
        coordinator
            .attach(&failed, "message-2", "process-2", "operation-2")
            .expect("attach failed");
        coordinator.fail("failed").expect("fail");

        assert!(coordinator.reserve("complete").is_ok());
        assert!(coordinator.reserve("failed").is_ok());
    }
}
