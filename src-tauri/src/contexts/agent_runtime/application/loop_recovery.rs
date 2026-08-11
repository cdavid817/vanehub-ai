use super::{
    AgentClockPort, AgentRuntimeApplicationError, LoopChildRecoveryDecision,
    LoopChildRecoveryProjection, LoopEvidenceView, LoopExecutionLeasePort, LoopOperationContext,
    LoopOperationKind, LoopOperationObserver, LoopRepository, LoopSessionRecoveryPort,
};
use crate::contexts::agent_runtime::domain::{LoopRun, LoopTerminalReason};
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopRecoveryApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) leases: Arc<dyn LoopExecutionLeasePort>,
    pub(crate) sessions: Arc<dyn LoopSessionRecoveryPort>,
    pub(crate) observer: LoopOperationObserver,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct LoopRecoveryApplicationService {
    ports: LoopRecoveryApplicationPorts,
}

impl LoopRecoveryApplicationService {
    pub(crate) fn new(ports: LoopRecoveryApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn reconcile_startup(&self) -> Result<Vec<LoopRun>, AgentRuntimeApplicationError> {
        let mut recovered = Vec::new();
        for mut run in self.ports.loops.list_recoverable_runs()? {
            if self.ports.leases.has_live_lease(run.id())? {
                continue;
            }
            let expected_status = run.status();
            let operation = self.ports.observer.start(
                LoopOperationContext {
                    run_id: run.id().to_string(),
                    iteration_id: None,
                    kind: LoopOperationKind::Recovery,
                },
                "Reconciling interrupted Loop run",
            )?;
            let projection = self.child_projection(run.id())?;
            match projection.decision {
                LoopChildRecoveryDecision::Failed => {
                    run.fail(LoopTerminalReason::RuntimeError)?;
                }
                LoopChildRecoveryDecision::Cancelled => {
                    run.cancel(LoopTerminalReason::UserStopped)?;
                }
                LoopChildRecoveryDecision::Completed | LoopChildRecoveryDecision::Ambiguous => {
                    run.recover_orphaned()?;
                }
            }
            let now = self.ports.clock.now();
            let evidence = LoopEvidenceView {
                id: format!("loop-evidence-{}", Uuid::new_v4()),
                run_id: run.id().to_string(),
                iteration_id: projection.iteration_id,
                kind: "recovery".to_string(),
                status: projection_status(projection.decision).to_string(),
                summary: projection_summary(projection.decision).to_string(),
                operation_id: Some(operation.id.clone()),
                command_id: None,
                exit_code: None,
                duration_ms: None,
                details: Some(serde_json::json!({
                    "reason": "session-recovery-projection",
                    "sessions": projection.sessions.iter().map(|session| serde_json::json!({
                        "sessionId": session.session_id,
                        "executionRunId": session.execution_run_id,
                        "recoveryRevision": session.recovery_revision,
                        "decision": projection_status(session.decision),
                    })).collect::<Vec<_>>(),
                })),
                created_at: now.clone(),
            };
            if let Err(error) =
                self.ports
                    .loops
                    .save_recovery_transition(&run, expected_status, &evidence, &now)
            {
                let _ = self.ports.observer.fail(
                    &operation,
                    "Loop recovery state changed before reconciliation completed.",
                );
                return Err(error);
            }
            self.ports.observer.complete(
                &operation,
                "Interrupted Loop run paused at its last durable phase boundary.",
            )?;
            recovered.push(run);
        }
        Ok(recovered)
    }

    fn child_projection(
        &self,
        run_id: &str,
    ) -> Result<LoopRecoveryAggregate, AgentRuntimeApplicationError> {
        let owned_sessions = self.ports.loops.recovery_owned_sessions(run_id)?;
        let Some(iteration_id) = owned_sessions
            .first()
            .map(|owned| owned.iteration_id.clone())
        else {
            return Ok(LoopRecoveryAggregate {
                iteration_id: None,
                sessions: Vec::new(),
                decision: LoopChildRecoveryDecision::Ambiguous,
            });
        };
        let mut sessions = Vec::new();
        for owned in owned_sessions {
            sessions.push(self.ports.sessions.recovery_projection(&owned.session_id)?);
        }
        let decision = aggregate_decision(&sessions);
        Ok(LoopRecoveryAggregate {
            iteration_id: Some(iteration_id),
            sessions,
            decision,
        })
    }
}

struct LoopRecoveryAggregate {
    iteration_id: Option<String>,
    sessions: Vec<LoopChildRecoveryProjection>,
    decision: LoopChildRecoveryDecision,
}

fn aggregate_decision(sessions: &[LoopChildRecoveryProjection]) -> LoopChildRecoveryDecision {
    let Some(first) = sessions.first().map(|projection| projection.decision) else {
        return LoopChildRecoveryDecision::Ambiguous;
    };
    if sessions
        .iter()
        .any(|projection| projection.decision == LoopChildRecoveryDecision::Ambiguous)
        || sessions
            .iter()
            .any(|projection| projection.decision != first)
    {
        LoopChildRecoveryDecision::Ambiguous
    } else {
        first
    }
}

fn projection_status(decision: LoopChildRecoveryDecision) -> &'static str {
    match decision {
        LoopChildRecoveryDecision::Completed => "completed",
        LoopChildRecoveryDecision::Failed => "failed",
        LoopChildRecoveryDecision::Cancelled => "cancelled",
        LoopChildRecoveryDecision::Ambiguous => "blocked",
    }
}

fn projection_summary(decision: LoopChildRecoveryDecision) -> &'static str {
    match decision {
        LoopChildRecoveryDecision::Completed => {
            "The owned role session completed before restart; the result was projected to the iteration without redispatch."
        }
        LoopChildRecoveryDecision::Failed => {
            "The owned role session failed before restart; the shared failure was projected to the iteration."
        }
        LoopChildRecoveryDecision::Cancelled => {
            "The owned role session was cancelled before restart; the shared cancellation was projected to the iteration."
        }
        LoopChildRecoveryDecision::Ambiguous => {
            "Owned role-session recovery is inconclusive; explicit resume or cancellation is required."
        }
    }
}
