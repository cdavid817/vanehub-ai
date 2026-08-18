use super::*;
use crate::contexts::agent_runtime::application::{
    PreparedRunnerLaunch, RunnerCapabilities, RunnerError, RunnerEvent, RunnerHandle,
    RunnerLaunchSpec, RunnerSelection,
};
use crate::contexts::operations::api::{RunOwner, RunRecoveryPolicy, RunRunner, RunState};
use std::sync::atomic::{AtomicUsize, Ordering};

struct InspectRunner {
    inspection: RunnerInspection,
    recoveries: AtomicUsize,
    side_effects: AtomicUsize,
}

impl AgentRunner for InspectRunner {
    fn kind(&self) -> RunnerKind {
        RunnerKind::Ssh
    }

    fn capabilities(&self) -> RunnerCapabilities {
        unreachable!()
    }

    fn prepare(
        &self,
        _selection: &RunnerSelection,
        _spec: RunnerLaunchSpec,
    ) -> Result<PreparedRunnerLaunch, RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn spawn(&self, _prepared: PreparedRunnerLaunch) -> Result<RunnerHandle, RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn send_input(&self, _handle: &RunnerHandle, _content: &[u8]) -> Result<(), RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn next_event(&self, _handle: &RunnerHandle) -> Result<Option<RunnerEvent>, RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn cancel(&self, _handle: &RunnerHandle) -> Result<bool, RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn inspect(&self, _handle: &RunnerHandle) -> Result<RunnerInspection, RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn cleanup(&self, _handle: &RunnerHandle) -> Result<(), RunnerError> {
        self.side_effects.fetch_add(1, Ordering::SeqCst);
        unreachable!()
    }

    fn recover(
        &self,
        _reference: &RunnerReference,
        _process_reference: Option<&str>,
    ) -> Result<RunnerInspection, RunnerError> {
        self.recoveries.fetch_add(1, Ordering::SeqCst);
        Ok(self.inspection.clone())
    }
}

fn run(recovery: RunRunnerRecovery) -> AgentRun {
    AgentRun {
        id: "run-1".into(),
        owner: RunOwner {
            owner_type: "session_generation".into(),
            owner_id: "message-1".into(),
        },
        links: Vec::new(),
        parent_run_id: None,
        state: RunState::Running,
        recovery_policy: if recovery == RunRunnerRecovery::None {
            RunRecoveryPolicy::NotRecoverable
        } else {
            RunRecoveryPolicy::OwnerReconciles
        },
        runner: Some(RunRunner {
            kind: if recovery == RunRunnerRecovery::None {
                RunRunnerKind::Local
            } else {
                RunRunnerKind::Ssh
            },
            target_id: if recovery == RunRunnerRecovery::None {
                "local".into()
            } else {
                "ssh-1".into()
            },
            target_revision: (recovery != RunRunnerRecovery::None).then_some(7),
            label: "Runner".into(),
            host_label: None,
            recovery,
            capability_witness: "capability-v1".into(),
            authority_witness: "authority-v1".into(),
            recovery_reference: Some("opaque-reference".into()),
        }),
        retry_count: 0,
        max_retries: 1,
        reason_code: None,
        created_at: "2026-08-18T00:00:00Z".into(),
        updated_at: "2026-08-18T00:00:00Z".into(),
        version: 2,
        last_witness: "running-v1".into(),
    }
}

#[test]
fn local_none_is_interrupted_without_runtime_side_effects() {
    let runner = Arc::new(InspectRunner {
        inspection: RunnerInspection::Running,
        recoveries: AtomicUsize::new(0),
        side_effects: AtomicUsize::new(0),
    });
    let recovery = RunnerRunRecoveryAdapter::new(runner.clone());
    assert_eq!(
        recovery
            .reconcile(&run(RunRunnerRecovery::None))
            .expect("decision"),
        RunRecoveryDecision::Interrupted
    );
    assert_eq!(runner.recoveries.load(Ordering::SeqCst), 0);
    assert_eq!(runner.side_effects.load(Ordering::SeqCst), 0);
}

#[test]
fn ssh_inspect_only_never_replays_and_reports_truthful_outcomes() {
    for (inspection, expected) in [
        (RunnerInspection::Running, RunRecoveryDecision::Blocked),
        (
            RunnerInspection::Exited(Some(0)),
            RunRecoveryDecision::Interrupted,
        ),
    ] {
        let runner = Arc::new(InspectRunner {
            inspection,
            recoveries: AtomicUsize::new(0),
            side_effects: AtomicUsize::new(0),
        });
        let recovery = RunnerRunRecoveryAdapter::new(runner.clone());
        assert_eq!(
            recovery
                .reconcile(&run(RunRunnerRecovery::InspectOnly))
                .expect("decision"),
            expected
        );
        assert_eq!(runner.recoveries.load(Ordering::SeqCst), 1);
        assert_eq!(runner.side_effects.load(Ordering::SeqCst), 0);
    }
}
