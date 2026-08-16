use super::*;
use crate::contexts::operations::domain::RunRecoveryPolicy;
use std::sync::Mutex;

#[derive(Default)]
struct Memory {
    runs: Mutex<Vec<AgentRun>>,
    events: Mutex<Vec<(String, RunEvent)>>,
}
impl AgentRunRepository for Memory {
    fn insert(&self, run: &AgentRun, event: &RunEvent) -> Result<(), ApplicationError> {
        self.runs.lock().expect("runs").push(run.clone());
        self.events
            .lock()
            .expect("events")
            .push((run.id.clone(), event.clone()));
        Ok(())
    }
    fn get(&self, id: &str) -> Result<AgentRun, ApplicationError> {
        self.runs
            .lock()
            .expect("runs")
            .iter()
            .find(|r| r.id == id)
            .cloned()
            .ok_or_else(|| ApplicationError::NotFound(id.into()))
    }
    fn save(
        &self,
        version: u64,
        run: &AgentRun,
        event: Option<&RunEvent>,
    ) -> Result<(), ApplicationError> {
        let mut runs = self.runs.lock().expect("runs");
        let current = runs
            .iter_mut()
            .find(|r| r.id == run.id)
            .ok_or_else(|| ApplicationError::NotFound(run.id.clone()))?;
        if current.version != version {
            return Err(ApplicationError::Conflict);
        }
        *current = run.clone();
        if let Some(event) = event {
            self.events
                .lock()
                .expect("events")
                .push((run.id.clone(), event.clone()));
        }
        Ok(())
    }
    fn list(
        &self,
        filter: &RunListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<AgentRun>, ApplicationError> {
        Ok(self
            .runs
            .lock()
            .expect("runs")
            .iter()
            .filter(|run| {
                filter
                    .owner_type
                    .as_ref()
                    .is_none_or(|value| &run.owner.owner_type == value)
                    && filter
                        .owner_id
                        .as_ref()
                        .is_none_or(|value| &run.owner.owner_id == value)
                    && filter
                        .parent_run_id
                        .as_ref()
                        .is_none_or(|value| run.parent_run_id.as_ref() == Some(value))
                    && filter.state.is_none_or(|value| run.state == value)
            })
            .skip(offset)
            .take(limit)
            .cloned()
            .collect())
    }
    fn children(&self, parent: &str) -> Result<Vec<AgentRun>, ApplicationError> {
        Ok(self
            .runs
            .lock()
            .expect("runs")
            .iter()
            .filter(|r| r.parent_run_id.as_deref() == Some(parent))
            .cloned()
            .collect())
    }
    fn events(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ApplicationError> {
        Ok(self
            .events
            .lock()
            .expect("events")
            .iter()
            .filter(|(run, _)| run == id)
            .skip(offset)
            .take(limit)
            .map(|(_, event)| event.clone())
            .collect())
    }
}
struct Clock;
impl RunClockPort for Clock {
    fn now(&self) -> String {
        "2026-08-16T00:00:00Z".into()
    }
}

#[derive(Default)]
struct RuntimePorts {
    cancellations: Mutex<Vec<String>>,
    lifecycle: Mutex<Vec<(String, RunState)>>,
    recovery: Mutex<Option<RunRecoveryDecision>>,
}
impl RunCancellationPort for RuntimePorts {
    fn request_cancel(&self, run: &AgentRun) -> Result<(), ApplicationError> {
        self.cancellations
            .lock()
            .expect("cancellations")
            .push(run.id.clone());
        Ok(())
    }
}
impl RunLifecycleEventPort for RuntimePorts {
    fn record(&self, run_id: &str, event: &RunEvent) -> Result<(), ApplicationError> {
        self.lifecycle
            .lock()
            .expect("lifecycle")
            .push((run_id.into(), event.state));
        Ok(())
    }
}
impl RunOwnerRecoveryPort for RuntimePorts {
    fn reconcile(&self, _run: &AgentRun) -> Result<RunRecoveryDecision, ApplicationError> {
        Ok(self
            .recovery
            .lock()
            .expect("recovery")
            .unwrap_or(RunRecoveryDecision::Interrupted))
    }
}
struct Ids(Mutex<u32>);
impl OperationIdGenerator for Ids {
    fn next_id(&self, _: &str) -> String {
        let mut n = self.0.lock().expect("ids");
        *n += 1;
        format!("018f0f17-4d6a-7e20-b41d-66c5271a{:04x}", *n)
    }
}
fn service(repo: Arc<Memory>) -> AgentRunService {
    AgentRunService::new(repo, Arc::new(Clock), Arc::new(Ids(Mutex::new(0))))
}
fn input(parent: Option<String>) -> CreateAgentRun {
    CreateAgentRun {
        id: None,
        owner: RunOwner {
            owner_type: "generation".into(),
            owner_id: "owner".into(),
        },
        links: vec![],
        parent_run_id: parent,
        recovery_policy: RunRecoveryPolicy::NotRecoverable,
        max_retries: 1,
        witness: "create".into(),
    }
}

#[test]
fn cancellation_propagates_and_is_version_guarded() {
    let repo = Arc::new(Memory::default());
    let service = service(repo);
    let parent = service.create(input(None)).expect("parent");
    let child = service
        .create(input(Some(parent.id.clone())))
        .expect("child");
    service
        .cancel_tree(
            &parent.id,
            parent.version,
            RunTrigger::CancelUser,
            "cancel".into(),
        )
        .expect("cancel tree");
    assert_eq!(
        service.get(&child.id).expect("child").state,
        RunState::Cancelled
    );
    assert_eq!(
        service.transition(
            &parent.id,
            parent.version,
            RunTrigger::Complete,
            None,
            "late".into()
        ),
        Err(ApplicationError::Conflict)
    );
}

#[test]
fn pages_are_bounded_and_resume_is_guarded() {
    let repo = Arc::new(Memory::default());
    let service = service(repo);
    let run = service.create(input(None)).expect("run");
    assert_eq!(
        service
            .list(RunListFilter::default(), 0, 500)
            .expect("page")
            .limit,
        100
    );
    assert!(matches!(
        service.resume(&run.id, run.version, "resume".into()),
        Err(ApplicationError::Invalid(_))
    ));
}

#[test]
fn runtime_ports_observe_durable_cancellation_and_verified_recovery() {
    let repo = Arc::new(Memory::default());
    let ports = Arc::new(RuntimePorts::default());
    *ports.recovery.lock().expect("recovery") = Some(RunRecoveryDecision::Blocked);
    let service = service(repo).with_runtime_ports(ports.clone(), ports.clone(), ports.clone());
    let mut recoverable = input(None);
    recoverable.recovery_policy = RunRecoveryPolicy::OwnerReconciles;
    let run = service.create(recoverable).expect("run");
    service.reconcile_after_restart().expect("reconcile");
    assert_eq!(service.get(&run.id).expect("run").state, RunState::Blocked);
    let blocked = service.get(&run.id).expect("run");
    service
        .resume(&blocked.id, blocked.version, "verified-resume".into())
        .expect("owner-approved resume");
    let cancelled = service
        .cancel_tree(
            &run.id,
            service.get(&run.id).expect("run").version,
            RunTrigger::CancelUser,
            "cancel".into(),
        )
        .expect("cancel");
    assert_eq!(cancelled.state, RunState::Cancelled);
    assert_eq!(ports.cancellations.lock().expect("cancellations").len(), 1);
    assert!(ports
        .lifecycle
        .lock()
        .expect("lifecycle")
        .iter()
        .any(|(_, state)| *state == RunState::Cancelled));
}

#[test]
fn forged_parent_is_rejected_without_writing_a_run() {
    let repo = Arc::new(Memory::default());
    let service = service(repo.clone());
    assert!(matches!(
        service.create(input(Some("missing-parent".into()))),
        Err(ApplicationError::NotFound(_))
    ));
    assert!(repo.runs.lock().expect("runs").is_empty());
}
