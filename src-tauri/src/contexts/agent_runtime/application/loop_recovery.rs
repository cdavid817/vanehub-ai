use super::{
    AgentClockPort, AgentRuntimeApplicationError, LoopEvidenceView, LoopExecutionLeasePort,
    LoopOperationContext, LoopOperationKind, LoopOperationObserver, LoopRepository,
};
use crate::contexts::agent_runtime::domain::LoopRun;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct LoopRecoveryApplicationPorts {
    pub(crate) loops: Arc<dyn LoopRepository>,
    pub(crate) leases: Arc<dyn LoopExecutionLeasePort>,
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
            run.recover_orphaned()?;
            let now = self.ports.clock.now();
            let evidence = LoopEvidenceView {
                id: format!("loop-evidence-{}", Uuid::new_v4()),
                run_id: run.id().to_string(),
                iteration_id: None,
                kind: "recovery".to_string(),
                status: "blocked".to_string(),
                summary: "Execution lease was not present after startup; explicit resume or cancellation is required.".to_string(),
                operation_id: Some(operation.id.clone()),
                command_id: None,
                exit_code: None,
                duration_ms: None,
                details: Some(serde_json::json!({ "reason": "recovery-required" })),
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
}
