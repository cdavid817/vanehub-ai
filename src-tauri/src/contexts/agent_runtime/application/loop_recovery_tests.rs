use super::*;
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopRun, LoopRunStatus, LoopTerminalReason,
};
use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

struct RecoveryWorld {
    runs: Mutex<Vec<LoopRun>>,
    live_leases: BTreeSet<String>,
    evidence: Mutex<Vec<LoopEvidenceView>>,
    operations: Mutex<Vec<LoopOperationContext>>,
    logs: Mutex<Vec<LoopLog>>,
}

impl RecoveryWorld {
    fn new(runs: Vec<LoopRun>, live_leases: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            runs: Mutex::new(runs),
            live_leases: live_leases.iter().map(|value| value.to_string()).collect(),
            evidence: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
        })
    }

    fn service(self: &Arc<Self>) -> LoopRecoveryApplicationService {
        LoopRecoveryApplicationService::new(LoopRecoveryApplicationPorts {
            loops: self.clone(),
            leases: self.clone(),
            observer: LoopOperationObserver::new(self.clone(), self.clone(), self.clone()),
            clock: self.clone(),
        })
    }
}

impl LoopRepository for RecoveryWorld {
    fn list_definitions(&self) -> Result<Vec<LoopDefinition>, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn find_definition(
        &self,
        _: &str,
    ) -> Result<Option<LoopDefinition>, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn create_definition(&self, _: &LoopDefinition) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn update_definition(
        &self,
        _: &LoopDefinition,
        _: u64,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn delete_definition(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn create_run(
        &self,
        _: &LoopRun,
        _: &LoopDefinition,
        _: &str,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn has_active_run(&self, _: &str) -> Result<bool, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn find_run(&self, run_id: &str) -> Result<Option<LoopRun>, AgentRuntimeApplicationError> {
        Ok(self
            .runs
            .lock()
            .expect("runs")
            .iter()
            .find(|run| run.id() == run_id)
            .cloned())
    }
    fn attach_run_operation(
        &self,
        _: &str,
        _: &str,
        _: LoopRunStatus,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn attach_run_worktree(
        &self,
        _: &str,
        _: &str,
        _: &str,
        _: &str,
        _: LoopRunStatus,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn save_run_transition(
        &self,
        _: &LoopRun,
        _: LoopRunStatus,
        _: &str,
        _: Option<&str>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn list_recoverable_runs(&self) -> Result<Vec<LoopRun>, AgentRuntimeApplicationError> {
        Ok(self
            .runs
            .lock()
            .expect("runs")
            .iter()
            .filter(|run| {
                matches!(
                    run.status(),
                    LoopRunStatus::Queued
                        | LoopRunStatus::Running
                        | LoopRunStatus::AwaitingAcceptance
                )
            })
            .cloned()
            .collect())
    }
    fn save_recovery_transition(
        &self,
        run: &LoopRun,
        expected_status: LoopRunStatus,
        evidence: &LoopEvidenceView,
        _: &str,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let mut runs = self.runs.lock().expect("runs");
        let stored = runs
            .iter_mut()
            .find(|stored| stored.id() == run.id())
            .ok_or_else(|| AgentRuntimeApplicationError::Loop("missing run".to_string()))?;
        if stored.status() != expected_status {
            return Err(AgentRuntimeApplicationError::Loop(
                "run changed".to_string(),
            ));
        }
        *stored = run.clone();
        self.evidence
            .lock()
            .expect("evidence")
            .push(evidence.clone());
        Ok(())
    }
}

impl LoopExecutionLeasePort for RecoveryWorld {
    fn has_live_lease(&self, run_id: &str) -> Result<bool, AgentRuntimeApplicationError> {
        Ok(self.live_leases.contains(run_id))
    }
}

impl AgentTaskPort for RecoveryWorld {
    fn start_agent_launch(
        &self,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn start_agent_generation(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        unreachable!()
    }
    fn start_loop_operation(
        &self,
        context: &LoopOperationContext,
        _: &str,
    ) -> Result<AgentOperation, AgentRuntimeApplicationError> {
        self.operations
            .lock()
            .expect("operations")
            .push(context.clone());
        Ok(AgentOperation {
            id: format!("recovery-operation-{}", context.run_id),
            related_agent_id: Some(context.run_id.clone()),
            message: None,
        })
    }
    fn append_log(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn complete(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn fail(&self, _: &str, _: String) -> Result<(), AgentRuntimeApplicationError> {
        Ok(())
    }
    fn cancel(&self, _: &str) -> Result<(), AgentRuntimeApplicationError> {
        unreachable!()
    }
}

impl AgentClockPort for RecoveryWorld {
    fn now(&self) -> String {
        "2026-07-22T13:00:00Z".to_string()
    }
}

impl LoopLoggingPort for RecoveryWorld {
    fn record_loop(&self, log: LoopLog) -> Result<(), AgentRuntimeApplicationError> {
        self.logs.lock().expect("logs").push(log);
        Ok(())
    }
}

fn queued(id: &str) -> LoopRun {
    LoopRun::new(id.to_string(), "loop-1".to_string()).expect("run")
}

#[test]
fn startup_recovery_pauses_only_runs_without_live_leases() {
    let mut already_paused = queued("paused");
    already_paused.request_pause().expect("pause request");
    already_paused.pause_at_boundary().expect("paused");
    let world = RecoveryWorld::new(
        vec![queued("orphan"), queued("leased"), already_paused],
        &["leased"],
    );

    let recovered = world.service().reconcile_startup().expect("reconcile");
    assert_eq!(recovered.len(), 1);
    assert_eq!(recovered[0].id(), "orphan");
    assert_eq!(recovered[0].status(), LoopRunStatus::Paused);
    assert_eq!(
        recovered[0].terminal_reason(),
        Some(LoopTerminalReason::RecoveryRequired)
    );
    assert_eq!(world.evidence.lock().expect("evidence").len(), 1);
    let operations = world.operations.lock().expect("operations");
    assert_eq!(operations.len(), 1);
    assert_eq!(operations[0].kind, LoopOperationKind::Recovery);
    let logs = world.logs.lock().expect("logs");
    assert_eq!(logs.len(), 2);
    assert!(logs.iter().all(|log| log.context.run_id == "orphan"));
    assert!(logs
        .iter()
        .all(|log| log.context.kind == LoopOperationKind::Recovery));
    assert!(logs
        .iter()
        .all(|log| { log.operation_id.as_deref() == Some("recovery-operation-orphan") }));
    assert_eq!(
        world
            .find_run("leased")
            .expect("find")
            .expect("run")
            .status(),
        LoopRunStatus::Queued
    );
}
