use super::{ApplicationError, OperationIdGenerator};
use crate::contexts::operations::domain::{
    AgentRun, RunCreation, RunEvent, RunLink, RunOwner, RunRecoveryPolicy, RunRunner, RunState,
    RunTransition, RunTrigger,
};
use std::sync::Arc;

const MAX_PAGE_SIZE: usize = 100;

pub(crate) trait RunClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait AgentRunRepository: Send + Sync {
    fn insert(&self, run: &AgentRun, event: &RunEvent) -> Result<(), ApplicationError>;
    fn get(&self, id: &str) -> Result<AgentRun, ApplicationError>;
    fn save(
        &self,
        expected_version: u64,
        run: &AgentRun,
        event: Option<&RunEvent>,
    ) -> Result<(), ApplicationError>;
    fn list(
        &self,
        filter: &RunListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<AgentRun>, ApplicationError>;
    fn children(&self, parent_id: &str) -> Result<Vec<AgentRun>, ApplicationError>;
    fn events(
        &self,
        run_id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ApplicationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RunRecoveryDecision {
    Blocked,
    Interrupted,
}

pub(crate) trait RunOwnerRecoveryPort: Send + Sync {
    fn reconcile(&self, run: &AgentRun) -> Result<RunRecoveryDecision, ApplicationError>;
}

pub(crate) trait RunCancellationPort: Send + Sync {
    fn request_cancel(&self, run: &AgentRun) -> Result<(), ApplicationError>;
}

pub(crate) trait RunLifecycleEventPort: Send + Sync {
    fn record(&self, run_id: &str, event: &RunEvent) -> Result<(), ApplicationError>;
}

struct DefaultRunRuntimePorts;

impl RunOwnerRecoveryPort for DefaultRunRuntimePorts {
    fn reconcile(&self, run: &AgentRun) -> Result<RunRecoveryDecision, ApplicationError> {
        Ok(match run.recovery_policy {
            RunRecoveryPolicy::NotRecoverable => RunRecoveryDecision::Interrupted,
            RunRecoveryPolicy::OwnerReconciles => RunRecoveryDecision::Blocked,
        })
    }
}

impl RunCancellationPort for DefaultRunRuntimePorts {
    fn request_cancel(&self, _run: &AgentRun) -> Result<(), ApplicationError> {
        Ok(())
    }
}

impl RunLifecycleEventPort for DefaultRunRuntimePorts {
    fn record(&self, _run_id: &str, _event: &RunEvent) -> Result<(), ApplicationError> {
        Ok(())
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateAgentRun {
    #[serde(default)]
    pub(crate) id: Option<String>,
    pub(crate) owner: RunOwner,
    #[serde(default)]
    pub(crate) links: Vec<RunLink>,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) recovery_policy: RunRecoveryPolicy,
    #[serde(default)]
    pub(crate) runner: Option<RunRunner>,
    pub(crate) max_retries: u32,
    pub(crate) witness: String,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunPage {
    pub(crate) items: Vec<AgentRun>,
    pub(crate) offset: usize,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RunListFilter {
    pub(crate) owner_type: Option<String>,
    pub(crate) owner_id: Option<String>,
    pub(crate) parent_run_id: Option<String>,
    pub(crate) state: Option<RunState>,
}

#[derive(Clone)]
pub(crate) struct AgentRunService {
    repository: Arc<dyn AgentRunRepository>,
    clock: Arc<dyn RunClockPort>,
    ids: Arc<dyn OperationIdGenerator>,
    recovery: Arc<dyn RunOwnerRecoveryPort>,
    cancellation: Arc<dyn RunCancellationPort>,
    lifecycle: Arc<dyn RunLifecycleEventPort>,
    evidence: Arc<dyn super::OperationsEvidencePort>,
}

impl AgentRunService {
    pub(crate) fn new(
        repository: Arc<dyn AgentRunRepository>,
        clock: Arc<dyn RunClockPort>,
        ids: Arc<dyn OperationIdGenerator>,
    ) -> Self {
        let defaults = Arc::new(DefaultRunRuntimePorts);
        Self {
            repository,
            clock,
            ids,
            recovery: defaults.clone(),
            cancellation: defaults.clone(),
            lifecycle: defaults.clone(),
            evidence: Arc::new(super::NoOperationsEvidence),
        }
        .with_runtime_ports(defaults.clone(), defaults.clone(), defaults)
    }

    /// Bootstrap swaps in the real publisher; the default keeps a build with no bridge running.
    pub(crate) fn with_evidence(
        mut self,
        evidence: Arc<dyn super::OperationsEvidencePort>,
    ) -> Self {
        self.evidence = evidence;
        self
    }

    pub(crate) fn with_runtime_ports(
        mut self,
        recovery: Arc<dyn RunOwnerRecoveryPort>,
        cancellation: Arc<dyn RunCancellationPort>,
        lifecycle: Arc<dyn RunLifecycleEventPort>,
    ) -> Self {
        self.recovery = recovery;
        self.cancellation = cancellation;
        self.lifecycle = lifecycle;
        self
    }

    pub(crate) fn with_recovery_port(mut self, recovery: Arc<dyn RunOwnerRecoveryPort>) -> Self {
        self.recovery = recovery;
        self
    }

    pub(crate) fn create(&self, input: CreateAgentRun) -> Result<AgentRun, ApplicationError> {
        if let Some(parent_id) = &input.parent_run_id {
            let parent = self.repository.get(parent_id)?;
            if parent.state.is_terminal() {
                return Err(ApplicationError::Invalid(
                    "terminal run cannot accept children".into(),
                ));
            }
        }
        let now = self.clock.now();
        let (run, event) = AgentRun::create(RunCreation {
            id: input.id.unwrap_or_else(|| self.ids.next_id(&now)),
            owner: input.owner,
            links: input.links,
            parent_run_id: input.parent_run_id,
            recovery_policy: input.recovery_policy,
            runner: input.runner,
            max_retries: input.max_retries,
            timestamp: now,
            witness: input.witness,
        })?;
        self.repository.insert(&run, &event)?;
        let _ = self.lifecycle.record(&run.id, &event);
        Ok(run)
    }

    pub(crate) fn transition(
        &self,
        id: &str,
        expected_version: u64,
        trigger: RunTrigger,
        reason_code: Option<String>,
        witness: String,
    ) -> Result<AgentRun, ApplicationError> {
        let mut run = self.repository.get(id)?;
        if run.state.is_terminal() && run.last_witness == witness {
            run.transition(RunTransition {
                trigger,
                timestamp: self.clock.now(),
                reason_code,
                witness,
            })?;
            return Ok(run);
        }
        if run.version != expected_version {
            return Err(ApplicationError::Conflict);
        }
        let previous = run.version;
        let event = run.transition(RunTransition {
            trigger,
            timestamp: self.clock.now(),
            reason_code,
            witness,
        })?;
        self.repository.save(previous, &run, event.as_ref())?;
        if let Some(event) = &event {
            let _ = self.lifecycle.record(&run.id, event);
        }
        self.publish_failure_evidence(&run);
        Ok(run)
    }

    /// Reports a run that ended in failure, and only that.
    ///
    /// A session id is required rather than inferred: `owner_id` means a session only when
    /// `owner_type` says so, and filing a failure against a guessed session would attribute one
    /// user's failed run to another's timeline. When the owner is something else, nothing is
    /// published — an absent record is honest, a misattributed one is not.
    fn publish_failure_evidence(&self, run: &AgentRun) {
        if run.state != RunState::Failed || run.owner.owner_type != "session" {
            return;
        }
        self.evidence
            .try_publish(super::OperationsEvidenceSignal::OperationFailed {
                session_id: run.owner.owner_id.clone(),
                operation_id: run.id.clone(),
                run_id: None,
                // The run's own stable code, or a generic one. The error text stays in the log
                // store, which already holds it and already redacts it.
                reason_code: run
                    .reason_code
                    .clone()
                    .unwrap_or_else(|| "run_failed".to_string()),
                occurred_at: run.updated_at.clone(),
            });
    }

    pub(crate) fn cancel_tree(
        &self,
        id: &str,
        expected_version: u64,
        trigger: RunTrigger,
        witness: String,
    ) -> Result<AgentRun, ApplicationError> {
        if !matches!(
            trigger,
            RunTrigger::CancelUser
                | RunTrigger::CancelParent
                | RunTrigger::CancelTimeout
                | RunTrigger::CancelShutdown
        ) {
            return Err(ApplicationError::Invalid(
                "invalid cancellation trigger".into(),
            ));
        }
        let current = self.repository.get(id)?;
        if current.state == RunState::Cancelled && current.last_witness == witness {
            return Ok(current);
        }
        if current.version != expected_version {
            return Err(ApplicationError::Conflict);
        }
        let run = self.transition(id, expected_version, trigger, None, witness)?;
        // Durable terminal intent wins the race; signalling is cooperative and
        // cannot roll back the already-authoritative cancellation decision.
        let _ = self.cancellation.request_cancel(&run);
        for child in self.repository.children(id)? {
            if !child.state.is_terminal() {
                self.cancel_tree(
                    &child.id,
                    child.version,
                    RunTrigger::CancelParent,
                    format!("parent:{}:{}", id, run.version),
                )?;
            }
        }
        Ok(run)
    }

    pub(crate) fn resume(
        &self,
        id: &str,
        version: u64,
        witness: String,
    ) -> Result<AgentRun, ApplicationError> {
        let run = self.repository.get(id)?;
        if !matches!(
            run.state,
            RunState::Paused | RunState::Blocked | RunState::Stuck
        ) {
            return Err(ApplicationError::Invalid("run cannot be resumed".into()));
        }
        self.transition(id, version, RunTrigger::Resume, None, witness)
    }

    pub(crate) fn get(&self, id: &str) -> Result<AgentRun, ApplicationError> {
        self.repository.get(id)
    }

    pub(crate) fn list(
        &self,
        filter: RunListFilter,
        offset: usize,
        limit: usize,
    ) -> Result<RunPage, ApplicationError> {
        let limit = limit.clamp(1, MAX_PAGE_SIZE);
        Ok(RunPage {
            items: self.repository.list(&filter, offset, limit)?,
            offset,
            limit,
        })
    }

    pub(crate) fn events(
        &self,
        id: &str,
        offset: usize,
        limit: usize,
    ) -> Result<Vec<RunEvent>, ApplicationError> {
        self.repository
            .events(id, offset, limit.clamp(1, MAX_PAGE_SIZE))
    }

    pub(crate) fn reconcile_after_restart(&self) -> Result<usize, ApplicationError> {
        let mut offset = 0;
        let mut reconciled = 0;
        loop {
            let page = self
                .repository
                .list(&RunListFilter::default(), offset, MAX_PAGE_SIZE)?;
            if page.is_empty() {
                break;
            }
            offset += page.len();
            for run in page {
                if run.state.is_terminal() {
                    continue;
                }
                if run.state == RunState::Blocked
                    && run.reason_code.as_deref() == Some("recovery_required")
                {
                    continue;
                }
                let decision =
                    if matches!(run.state, RunState::WaitingApproval | RunState::WaitingUser) {
                        RunRecoveryDecision::Interrupted
                    } else {
                        self.recovery.reconcile(&run)?
                    };
                let (trigger, reason) = match decision {
                    RunRecoveryDecision::Blocked => (RunTrigger::Block, "recovery_required"),
                    RunRecoveryDecision::Interrupted => {
                        (RunTrigger::InterruptRestart, "interrupted_restart")
                    }
                };
                self.transition(
                    &run.id,
                    run.version,
                    trigger,
                    Some(reason.into()),
                    format!("restart:{}:{}", run.id, run.version),
                )?;
                reconciled += 1;
            }
            if offset % MAX_PAGE_SIZE != 0 {
                break;
            }
        }
        Ok(reconciled)
    }
}

#[cfg(test)]
#[path = "run_service_tests.rs"]
mod tests;
