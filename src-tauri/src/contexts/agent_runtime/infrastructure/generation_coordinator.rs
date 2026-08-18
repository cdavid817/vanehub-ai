use crate::contexts::agent_runtime::application::{
    ActiveGenerationCorrelation, AgentGenerationPort, AgentRuntimeApplicationError,
    GenerationCancellation, GenerationLease, PendingPromptExecution,
};
use crate::contexts::agent_runtime::domain::GenerationAttempt;
use crate::contexts::execution_observability::api::ExecutionContext;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub(crate) struct InMemoryGenerationCoordinator {
    active: Arc<Mutex<HashMap<String, CoordinatedGeneration>>>,
    lease_ids: Arc<AtomicU64>,
    accepting: Arc<AtomicBool>,
}

impl Default for InMemoryGenerationCoordinator {
    fn default() -> Self {
        Self {
            active: Arc::new(Mutex::new(HashMap::new())),
            lease_ids: Arc::new(AtomicU64::new(0)),
            accepting: Arc::new(AtomicBool::new(true)),
        }
    }
}

struct CoordinatedGeneration {
    lease_id: String,
    attempt: GenerationAttempt,
    process_id: Option<String>,
    operation_id: Option<String>,
    execution_context: Option<ExecutionContext>,
    prompt_execution: Option<PendingPromptExecution>,
}

impl AgentGenerationPort for InMemoryGenerationCoordinator {
    fn reserve(&self, session_id: &str) -> Result<GenerationLease, AgentRuntimeApplicationError> {
        if !self.accepting.load(Ordering::Acquire) {
            return Err(AgentRuntimeApplicationError::Generation(
                "Agent runtime is shutting down.".to_string(),
            ));
        }
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
                prompt_execution: None,
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

    fn correlate_prompt(
        &self,
        lease: &GenerationLease,
        execution: &PendingPromptExecution,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut active = self.active()?;
        require_lease(&mut active, lease)?.prompt_execution = Some(execution.clone());
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
            prompt_execution: generation.prompt_execution,
        }))
    }

    fn complete(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.finish(session_id, GenerationAttempt::complete)
    }

    fn fail(&self, session_id: &str) -> Result<(), AgentRuntimeApplicationError> {
        self.finish(session_id, GenerationAttempt::fail)
    }

    fn active_process_id(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, AgentRuntimeApplicationError> {
        let active = self.active()?;
        Ok(active
            .get(session_id)
            .and_then(|generation| generation.process_id.clone()))
    }

    fn active_correlation(
        &self,
        session_id: &str,
    ) -> Result<Option<ActiveGenerationCorrelation>, AgentRuntimeApplicationError> {
        let active = self.active()?;
        Ok(active
            .get(session_id)
            .map(|generation| ActiveGenerationCorrelation {
                operation_id: generation.operation_id.clone(),
                execution_run_id: generation
                    .execution_context
                    .as_ref()
                    .map(|context| context.run_id.as_str().to_string()),
            }))
    }

    fn begin_shutdown(&self) -> Result<Vec<String>, AgentRuntimeApplicationError> {
        self.accepting.store(false, Ordering::Release);
        Ok(self.active()?.keys().cloned().collect())
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
    use crate::contexts::execution_observability::api::{
        CapturePolicy, ExecutionRunId, SpanId, TraceId,
    };

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
    fn shutdown_stops_admission_and_returns_every_owned_generation() {
        let coordinator = InMemoryGenerationCoordinator::default();
        coordinator.reserve("session-1").expect("first");
        coordinator.reserve("session-2").expect("second");
        let mut sessions = coordinator.begin_shutdown().expect("shutdown");
        sessions.sort();
        assert_eq!(sessions, ["session-1", "session-2"]);
        assert!(coordinator.reserve("session-3").is_err());
        assert!(coordinator.cancel("session-1").expect("cancel").is_some());
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
    fn active_correlation_exposes_operation_and_execution_run_without_content() {
        let coordinator = InMemoryGenerationCoordinator::default();
        let lease = coordinator.reserve("session-1").expect("reserve");
        coordinator
            .correlate(
                &lease,
                &ExecutionContext {
                    run_id: ExecutionRunId::parse("6ba7b810-9dad-41d1-80b4-00c04fd430c8")
                        .expect("run id"),
                    trace_id: TraceId::parse("4bf92f3577b34da6a3ce929d0e0e4736").expect("trace id"),
                    span_id: SpanId::parse("00f067aa0ba902b7").expect("span id"),
                    capture_policy: CapturePolicy::MetadataOnly,
                    sampling_per_million: 1_000_000,
                    mcp_relay_enabled: false,
                },
            )
            .expect("execution correlation");
        coordinator
            .attach(&lease, "message-1", "process-1", "operation-1")
            .expect("attach");

        let correlation = coordinator
            .active_correlation("session-1")
            .expect("lookup")
            .expect("active correlation");

        assert_eq!(correlation.operation_id.as_deref(), Some("operation-1"));
        assert_eq!(
            correlation.execution_run_id.as_deref(),
            Some("6ba7b810-9dad-41d1-80b4-00c04fd430c8")
        );
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
