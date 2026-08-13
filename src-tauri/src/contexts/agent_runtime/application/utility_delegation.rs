use super::{AgentClockPort, AgentRuntimeApplicationError};
use crate::contexts::agent_runtime::domain::{
    UtilityDelegationAttempt, UtilityDelegationCounts, UtilityDelegationRequest,
    UtilityDelegationResult, UtilityDelegationSnapshot, UtilityDelegationTerminal,
};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use uuid::Uuid;

pub(crate) trait UtilityDelegationResolutionPort: Send + Sync {
    fn resolve(
        &self,
        skill_id: &str,
        canonical_workspace: Option<&str>,
    ) -> Result<UtilityDelegationSnapshot, AgentRuntimeApplicationError>;

    fn current_revision(
        &self,
        skill_id: &str,
        canonical_workspace: Option<&str>,
    ) -> Result<String, AgentRuntimeApplicationError>;
}

pub(crate) trait UtilityChildExecutionPort: Send + Sync {
    fn execute(
        &self,
        request: &UtilityDelegationRequest,
        snapshot: &UtilityDelegationSnapshot,
        cancellation: Arc<AtomicBool>,
    ) -> UtilityChildExecutionOutcome;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityChildExecutionOutcome {
    pub(crate) terminal: UtilityDelegationTerminal,
    pub(crate) summary: Option<String>,
    pub(crate) duration_ms: u64,
    pub(crate) counts: UtilityDelegationCounts,
    pub(crate) limit_reason: Option<String>,
}

pub(crate) trait UtilityDelegationLifecyclePort: Send + Sync {
    fn project(&self, fact: UtilityDelegationLifecycleFact);
}

pub(crate) trait UtilityDelegationEvidencePort: Send + Sync {
    fn project(&self, fact: UtilityDelegationEvidenceFact);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UtilityDelegationLifecycleFact {
    pub(crate) agent_id: String,
    pub(crate) delegation_id: String,
    pub(crate) attempt_id: String,
    pub(crate) parent_run_id: String,
    pub(crate) parent_span_id: String,
    pub(crate) session_id: String,
    pub(crate) message_id: String,
    pub(crate) canonical_workspace: Option<String>,
    pub(crate) skill_id: String,
    pub(crate) revision: String,
    pub(crate) occurred_at: String,
    pub(crate) terminal: Option<UtilityDelegationTerminal>,
    pub(crate) duration_ms: Option<u64>,
    pub(crate) counts: UtilityDelegationCounts,
    pub(crate) limit_reason: Option<String>,
}

pub(crate) type UtilityDelegationEvidenceFact = UtilityDelegationLifecycleFact;

#[derive(Clone)]
pub(crate) struct UtilityDelegationApplicationPorts {
    pub(crate) resolver: Arc<dyn UtilityDelegationResolutionPort>,
    pub(crate) executor: Arc<dyn UtilityChildExecutionPort>,
    pub(crate) lifecycle: Arc<dyn UtilityDelegationLifecyclePort>,
    pub(crate) evidence: Arc<dyn UtilityDelegationEvidencePort>,
    pub(crate) clock: Arc<dyn AgentClockPort>,
}

#[derive(Clone)]
pub(crate) struct UtilityDelegationApplicationService {
    ports: UtilityDelegationApplicationPorts,
}

impl UtilityDelegationApplicationService {
    pub(crate) fn new(ports: UtilityDelegationApplicationPorts) -> Self {
        Self { ports }
    }

    pub(crate) fn execute(
        &self,
        request: UtilityDelegationRequest,
        cancellation: Arc<AtomicBool>,
    ) -> Result<UtilityDelegationResult, AgentRuntimeApplicationError> {
        request.validate()?;
        let snapshot = self
            .ports
            .resolver
            .resolve(&request.skill_id, request.canonical_workspace.as_deref())?;
        let revision = self
            .ports
            .resolver
            .current_revision(&request.skill_id, request.canonical_workspace.as_deref())?;
        let mut attempt = UtilityDelegationAttempt::admit(
            Uuid::now_v7().to_string(),
            Uuid::now_v7().to_string(),
            &request,
            snapshot,
            &revision,
        )?;
        attempt.start()?;
        self.ports.lifecycle.project(lifecycle_fact(
            &attempt,
            &request,
            self.ports.clock.now(),
            None,
        ));

        let outcome = self
            .ports
            .executor
            .execute(&request, &attempt.snapshot, cancellation);
        if attempt.finish(outcome.terminal)? {
            let terminal_fact =
                lifecycle_fact(&attempt, &request, self.ports.clock.now(), Some(&outcome));
            self.ports.lifecycle.project(terminal_fact.clone());
            self.ports.evidence.project(terminal_fact);
        }
        Ok(UtilityDelegationResult {
            delegation_id: attempt.delegation_id,
            attempt_id: attempt.attempt_id,
            skill_id: attempt.snapshot.skill_id,
            revision: attempt.snapshot.revision,
            terminal: outcome.terminal,
            summary: bounded_summary(outcome.summary, request.limits.result_chars),
            duration_ms: outcome.duration_ms,
            counts: outcome.counts,
            limit_reason: outcome.limit_reason,
        })
    }
}

fn lifecycle_fact(
    attempt: &UtilityDelegationAttempt,
    request: &UtilityDelegationRequest,
    occurred_at: String,
    outcome: Option<&UtilityChildExecutionOutcome>,
) -> UtilityDelegationLifecycleFact {
    UtilityDelegationLifecycleFact {
        agent_id: request.agent_id.clone(),
        delegation_id: attempt.delegation_id.clone(),
        attempt_id: attempt.attempt_id.clone(),
        parent_run_id: request.parent_run_id.clone(),
        parent_span_id: request.parent_span_id.clone(),
        session_id: request.session_id.clone(),
        message_id: request.message_id.clone(),
        canonical_workspace: request.canonical_workspace.clone(),
        skill_id: attempt.snapshot.skill_id.clone(),
        revision: attempt.snapshot.revision.clone(),
        occurred_at,
        terminal: outcome.map(|value| value.terminal),
        duration_ms: outcome.map(|value| value.duration_ms),
        counts: outcome
            .map(|value| value.counts.clone())
            .unwrap_or_default(),
        limit_reason: outcome.and_then(|value| value.limit_reason.clone()),
    }
}

fn bounded_summary(summary: Option<String>, limit: usize) -> Option<String> {
    summary.map(|value| value.chars().take(limit).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::UtilityDelegationLimits;
    use std::sync::Mutex;

    struct Resolver;

    impl UtilityDelegationResolutionPort for Resolver {
        fn resolve(
            &self,
            skill_id: &str,
            canonical_workspace: Option<&str>,
        ) -> Result<UtilityDelegationSnapshot, AgentRuntimeApplicationError> {
            Ok(UtilityDelegationSnapshot {
                skill_id: skill_id.to_string(),
                revision: "revision-1".to_string(),
                instructions: "Review the change.".to_string(),
                canonical_workspace: canonical_workspace.map(str::to_string),
            })
        }

        fn current_revision(
            &self,
            _skill_id: &str,
            _canonical_workspace: Option<&str>,
        ) -> Result<String, AgentRuntimeApplicationError> {
            Ok("revision-1".to_string())
        }
    }

    struct Executor;

    impl UtilityChildExecutionPort for Executor {
        fn execute(
            &self,
            _request: &UtilityDelegationRequest,
            _snapshot: &UtilityDelegationSnapshot,
            _cancellation: Arc<AtomicBool>,
        ) -> UtilityChildExecutionOutcome {
            UtilityChildExecutionOutcome {
                terminal: UtilityDelegationTerminal::Succeeded,
                summary: Some("safe result".to_string()),
                duration_ms: 12,
                counts: UtilityDelegationCounts {
                    tool_calls: 1,
                    approvals: 0,
                },
                limit_reason: None,
            }
        }
    }

    #[derive(Default)]
    struct Facts(Mutex<Vec<UtilityDelegationLifecycleFact>>);

    impl UtilityDelegationLifecyclePort for Facts {
        fn project(&self, fact: UtilityDelegationLifecycleFact) {
            self.0.lock().expect("facts").push(fact);
        }
    }

    impl UtilityDelegationEvidencePort for Facts {
        fn project(&self, fact: UtilityDelegationEvidenceFact) {
            self.0.lock().expect("facts").push(fact);
        }
    }

    struct Clock;

    impl AgentClockPort for Clock {
        fn now(&self) -> String {
            "2026-08-13T00:00:00Z".to_string()
        }
    }

    #[test]
    fn service_projects_started_and_terminal_facts_without_content() {
        let lifecycle = Arc::new(Facts::default());
        let evidence = Arc::new(Facts::default());
        let service = UtilityDelegationApplicationService::new(UtilityDelegationApplicationPorts {
            resolver: Arc::new(Resolver),
            executor: Arc::new(Executor),
            lifecycle: lifecycle.clone(),
            evidence: evidence.clone(),
            clock: Arc::new(Clock),
        });
        let result = service
            .execute(
                UtilityDelegationRequest {
                    agent_id: "onepiece".to_string(),
                    skill_id: "review-code".to_string(),
                    task: "private task".to_string(),
                    parent_run_id: "run-1".to_string(),
                    parent_span_id: "1111111111111111".to_string(),
                    session_id: "session-1".to_string(),
                    message_id: "message-1".to_string(),
                    canonical_workspace: Some("workspace-1".to_string()),
                    depth: 0,
                    limits: UtilityDelegationLimits::default(),
                },
                Arc::new(AtomicBool::new(false)),
            )
            .expect("execute");
        assert_eq!(result.terminal, UtilityDelegationTerminal::Succeeded);
        assert_eq!(lifecycle.0.lock().expect("facts").len(), 2);
        let evidence = evidence.0.lock().expect("facts");
        assert_eq!(evidence.len(), 1);
        assert_eq!(evidence[0].revision, "revision-1");
    }

    #[test]
    fn terminal_classifications_remain_distinct() {
        for terminal in [
            UtilityDelegationTerminal::Failed,
            UtilityDelegationTerminal::Cancelled,
            UtilityDelegationTerminal::TimedOut,
            UtilityDelegationTerminal::Limited,
            UtilityDelegationTerminal::Refused,
        ] {
            assert!(!terminal.as_str().is_empty());
        }
    }
}
