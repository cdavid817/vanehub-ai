use super::*;
use crate::contexts::agent_runtime::domain::{
    LoopDefinition, LoopRun, LoopRunStatus, LoopTerminalReason,
};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

struct RecoveryWorld {
    runs: Mutex<Vec<LoopRun>>,
    live_leases: BTreeSet<String>,
    evidence: Mutex<Vec<LoopEvidenceView>>,
    operations: Mutex<Vec<LoopOperationContext>>,
    logs: Mutex<Vec<LoopLog>>,
    owned_sessions: Mutex<BTreeMap<String, Vec<LoopOwnedRecoverySession>>>,
    projections: Mutex<BTreeMap<String, LoopChildRecoveryProjection>>,
    projection_reads: Mutex<Vec<String>>,
}

impl RecoveryWorld {
    fn new(runs: Vec<LoopRun>, live_leases: &[&str]) -> Arc<Self> {
        Arc::new(Self {
            runs: Mutex::new(runs),
            live_leases: live_leases.iter().map(|value| value.to_string()).collect(),
            evidence: Mutex::new(Vec::new()),
            operations: Mutex::new(Vec::new()),
            logs: Mutex::new(Vec::new()),
            owned_sessions: Mutex::new(BTreeMap::new()),
            projections: Mutex::new(BTreeMap::new()),
            projection_reads: Mutex::new(Vec::new()),
        })
    }

    fn service(self: &Arc<Self>) -> LoopRecoveryApplicationService {
        LoopRecoveryApplicationService::new(LoopRecoveryApplicationPorts {
            loops: self.clone(),
            leases: self.clone(),
            sessions: self.clone(),
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
    fn recovery_owned_sessions(
        &self,
        run_id: &str,
    ) -> Result<Vec<LoopOwnedRecoverySession>, AgentRuntimeApplicationError> {
        Ok(self
            .owned_sessions
            .lock()
            .expect("owned sessions")
            .get(run_id)
            .cloned()
            .unwrap_or_default())
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

impl LoopSessionRecoveryPort for RecoveryWorld {
    fn recovery_projection(
        &self,
        session_id: &str,
    ) -> Result<LoopChildRecoveryProjection, AgentRuntimeApplicationError> {
        self.projection_reads
            .lock()
            .expect("projection reads")
            .push(session_id.to_string());
        self.projections
            .lock()
            .expect("projections")
            .get(session_id)
            .cloned()
            .ok_or_else(|| {
                AgentRuntimeApplicationError::Loop("missing session projection".to_string())
            })
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

#[test]
fn startup_recovery_projects_conclusive_child_failure_to_owning_iteration() {
    let world = RecoveryWorld::new(vec![queued("failed-child")], &[]);
    world.owned_sessions.lock().expect("owned sessions").insert(
        "failed-child".to_string(),
        vec![LoopOwnedRecoverySession {
            iteration_id: "iteration-failed-child".to_string(),
            session_id: "worker-failed-child".to_string(),
        }],
    );
    world.projections.lock().expect("projections").insert(
        "worker-failed-child".to_string(),
        LoopChildRecoveryProjection {
            session_id: "worker-failed-child".to_string(),
            execution_run_id: Some("execution-failed-child".to_string()),
            recovery_revision: 3,
            decision: LoopChildRecoveryDecision::Failed,
        },
    );

    let recovered = world.service().reconcile_startup().expect("reconcile");
    let repeated = world
        .service()
        .reconcile_startup()
        .expect("repeat reconcile");
    let evidence = world.evidence.lock().expect("evidence");

    assert_eq!(recovered[0].status(), LoopRunStatus::Failed);
    assert_eq!(
        evidence[0].iteration_id.as_deref(),
        Some("iteration-failed-child")
    );
    assert_eq!(evidence[0].status, "failed");
    assert!(repeated.is_empty());
    assert_eq!(
        *world.projection_reads.lock().expect("projection reads"),
        vec!["worker-failed-child".to_string()]
    );
    assert_eq!(
        evidence[0].details.as_ref().expect("details")["sessions"][0]["executionRunId"],
        "execution-failed-child"
    );
}

#[test]
fn startup_recovery_keeps_conflicting_child_projections_behind_pause_gate() {
    let world = RecoveryWorld::new(vec![queued("conflicting-child")], &[]);
    world.owned_sessions.lock().expect("owned sessions").insert(
        "conflicting-child".to_string(),
        vec![
            LoopOwnedRecoverySession {
                iteration_id: "iteration-conflicting-child".to_string(),
                session_id: "worker-completed".to_string(),
            },
            LoopOwnedRecoverySession {
                iteration_id: "iteration-conflicting-child".to_string(),
                session_id: "verifier-failed".to_string(),
            },
        ],
    );
    let mut projections = world.projections.lock().expect("projections");
    projections.insert(
        "worker-completed".to_string(),
        LoopChildRecoveryProjection {
            session_id: "worker-completed".to_string(),
            execution_run_id: Some("execution-worker".to_string()),
            recovery_revision: 1,
            decision: LoopChildRecoveryDecision::Completed,
        },
    );
    projections.insert(
        "verifier-failed".to_string(),
        LoopChildRecoveryProjection {
            session_id: "verifier-failed".to_string(),
            execution_run_id: Some("execution-verifier".to_string()),
            recovery_revision: 2,
            decision: LoopChildRecoveryDecision::Failed,
        },
    );
    drop(projections);

    let recovered = world.service().reconcile_startup().expect("reconcile");
    let evidence = world.evidence.lock().expect("evidence");

    assert_eq!(recovered[0].status(), LoopRunStatus::Paused);
    assert_eq!(
        recovered[0].terminal_reason(),
        Some(LoopTerminalReason::RecoveryRequired)
    );
    assert_eq!(evidence[0].status, "blocked");
}
