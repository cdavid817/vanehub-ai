use super::{
    ClaimRecoveryCandidateRequest, PublishRecoveryRequest, RecoveryBatchResult,
    SessionApplicationLog, SessionApplicationLogLevel, SessionClockPort, SessionLoggingPort,
    SessionRecoveryEvent, SessionRecoveryEventKind, SessionRecoveryEventPort, SessionRepository,
    SessionTerminalEvidencePort, SessionTransactionPort, SessionsApplicationError,
};
use crate::contexts::sessions::domain::evidence::{
    LiveHandleEvidence, OperationTerminalStatus, SessionTerminalEvidence, ToolActivityEvidence,
};
use crate::contexts::sessions::domain::recovery::{
    RecoveryDecision, RecoveryEvidenceReference, RecoveryTrigger, SessionRecoveryReport,
};
use crate::contexts::sessions::domain::recovery_decision::decide_recovery;
use crate::contexts::sessions::domain::{MessageRole, SessionId};
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct SessionRecoveryCoordinator {
    sessions: Arc<dyn SessionRepository>,
    transactions: Arc<dyn SessionTransactionPort>,
    evidence: Arc<dyn SessionTerminalEvidencePort>,
    clock: Arc<dyn SessionClockPort>,
    logging: Arc<dyn SessionLoggingPort>,
    events: Arc<dyn SessionRecoveryEventPort>,
}

struct NoopRecoveryEvents;

impl SessionRecoveryEventPort for NoopRecoveryEvents {
    fn publish_recovery_event(
        &self,
        _event: SessionRecoveryEvent,
    ) -> Result<(), SessionsApplicationError> {
        Ok(())
    }
}

impl SessionRecoveryCoordinator {
    pub(crate) fn new(
        sessions: Arc<dyn SessionRepository>,
        transactions: Arc<dyn SessionTransactionPort>,
        evidence: Arc<dyn SessionTerminalEvidencePort>,
        clock: Arc<dyn SessionClockPort>,
        logging: Arc<dyn SessionLoggingPort>,
    ) -> Self {
        Self {
            sessions,
            transactions,
            evidence,
            clock,
            logging,
            events: Arc::new(NoopRecoveryEvents),
        }
    }

    pub(crate) fn with_events(mut self, events: Arc<dyn SessionRecoveryEventPort>) -> Self {
        self.events = events;
        self
    }

    #[cfg(test)]
    pub(crate) fn run_batch(
        &self,
        limit: usize,
        trigger: RecoveryTrigger,
    ) -> Result<RecoveryBatchResult, SessionsApplicationError> {
        let candidates = match self.sessions.recovery_candidates(limit.clamp(1, 100)) {
            Ok(candidates) => candidates,
            Err(SessionsApplicationError::RetryableStorage(_)) => {
                self.log_service(
                    SessionApplicationLogLevel::Warn,
                    "Recovery candidate scan was deferred because storage is temporarily unavailable.",
                );
                return Ok(RecoveryBatchResult::default());
            }
            Err(error) => return Err(error),
        };
        self.run_candidates(candidates, trigger)
    }

    fn run_candidates(
        &self,
        candidates: Vec<super::RecoveryCandidateClaim>,
        trigger: RecoveryTrigger,
    ) -> Result<RecoveryBatchResult, SessionsApplicationError> {
        let mut result = RecoveryBatchResult {
            scanned: candidates.len(),
            ..Default::default()
        };
        for candidate in candidates {
            let candidate_session_id = candidate.session_id.clone();
            let candidate_run_id = candidate.observed_execution_run_id.clone();
            let claim =
                match self
                    .transactions
                    .claim_recovery_candidate(&ClaimRecoveryCandidateRequest {
                        candidate,
                        claimed_at: self.clock.now(),
                    }) {
                    Ok(claim) => claim,
                    Err(SessionsApplicationError::RetryableStorage(_)) => {
                        result.deferred += 1;
                        self.log(
                        SessionApplicationLogLevel::Warn,
                        "Recovery claim was deferred because storage is temporarily unavailable.",
                        candidate_session_id,
                        candidate_run_id,
                        None,
                    );
                        continue;
                    }
                    Err(error) => return Err(error),
                };
            let Some(claim) = claim else {
                result.stale += 1;
                self.log(
                    SessionApplicationLogLevel::Debug,
                    "Recovery candidate changed before it could be claimed.",
                    candidate_session_id,
                    candidate_run_id,
                    None,
                );
                continue;
            };
            let session_id = SessionId::parse(&claim.session_id)?;
            let _ = self.events.publish_recovery_event(SessionRecoveryEvent {
                kind: SessionRecoveryEventKind::Started,
                session_id: claim.session_id.clone(),
                recovery_revision: claim.recovery_revision,
            });
            if claim.structurally_invalid {
                self.publish_structural_failure(&claim, trigger, &mut result)?;
                continue;
            }
            let evidence = match self
                .evidence
                .read_terminal_evidence(&session_id, claim.observed_execution_run_id.as_deref())
            {
                Ok(evidence) => evidence,
                Err(SessionsApplicationError::StructuralRecoveryEvidence(_)) => {
                    self.publish_structural_failure(&claim, trigger, &mut result)?;
                    continue;
                }
                Err(_error) => {
                    self.defer_claim(
                        &claim,
                        &mut result,
                        SessionApplicationLogLevel::Warn,
                        "Recovery evidence was temporarily unavailable; candidate deferred.",
                    )?;
                    continue;
                }
            };
            let decision = decide_recovery(&evidence);
            if decision.decision == RecoveryDecision::RetryLater {
                self.defer_claim(
                    &claim,
                    &mut result,
                    SessionApplicationLogLevel::Info,
                    "Recovery evidence is not yet conclusive; candidate deferred.",
                )?;
                continue;
            }
            let created_at = self.clock.now();
            let recovery_revision = claim.recovery_revision + 1;
            let report = SessionRecoveryReport::new(
                format!("recovery:{}:{recovery_revision}", claim.session_id),
                claim.session_id.clone(),
                recovery_revision,
                trigger,
                evidence.session.lifecycle.as_str().to_string(),
                claim.observed_execution_run_id.clone(),
                decision.decision,
                decision.reason_codes,
                evidence_references(&evidence),
                created_at.clone(),
            );
            let report_id = report.report_id().to_string();
            let published_event_kind = match report.decision() {
                RecoveryDecision::ActionRequired => SessionRecoveryEventKind::ActionRequired,
                RecoveryDecision::Quarantined => SessionRecoveryEventKind::Quarantined,
                _ => SessionRecoveryEventKind::Completed,
            };
            let session_id = claim.session_id.clone();
            let execution_run_id = claim.observed_execution_run_id.clone();
            let assistant_message_id = evidence
                .messages()
                .iter()
                .rev()
                .find(|message| {
                    message.role == MessageRole::Assistant
                        && message.execution_run_id.as_deref()
                            == claim.observed_execution_run_id.as_deref()
                })
                .map(|message| message.message_id.clone());
            let published = match self.transactions.publish_recovery(&PublishRecoveryRequest {
                claim,
                assistant_message_id,
                report,
                published_at: created_at,
            }) {
                Ok(published) => published,
                Err(SessionsApplicationError::RetryableStorage(_)) => {
                    result.deferred += 1;
                    self.log(
                        SessionApplicationLogLevel::Warn,
                        "Recovery publication was deferred because storage is temporarily unavailable.",
                        session_id,
                        execution_run_id,
                        Some(report_id),
                    );
                    continue;
                }
                Err(error) => return Err(error),
            };
            if published {
                result.published += 1;
                let _ = self.events.publish_recovery_event(SessionRecoveryEvent {
                    kind: published_event_kind,
                    session_id: session_id.clone(),
                    recovery_revision,
                });
                self.log(
                    SessionApplicationLogLevel::Info,
                    "Recovery decision was published.",
                    session_id,
                    execution_run_id,
                    Some(report_id),
                );
            } else {
                result.stale += 1;
                self.log(
                    SessionApplicationLogLevel::Debug,
                    "Recovery publication was skipped because the candidate changed.",
                    session_id,
                    execution_run_id,
                    Some(report_id),
                );
            }
        }
        Ok(result)
    }

    pub(crate) fn run_until_drained(
        &self,
        batch_limit: usize,
        trigger: RecoveryTrigger,
    ) -> Result<RecoveryBatchResult, SessionsApplicationError> {
        let batch_limit = batch_limit.clamp(1, 100);
        let mut total = RecoveryBatchResult::default();
        let mut after_session_id = None;
        loop {
            let candidates = match self
                .sessions
                .recovery_candidates_after(after_session_id.as_deref(), batch_limit)
            {
                Ok(candidates) => candidates,
                Err(SessionsApplicationError::RetryableStorage(_)) => {
                    total.deferred += 1;
                    self.log_service(
                        SessionApplicationLogLevel::Warn,
                        "Recovery candidate scan was deferred because storage is temporarily unavailable.",
                    );
                    break;
                }
                Err(error) => return Err(error),
            };
            let Some(last_candidate) = candidates.last() else {
                break;
            };
            let next_cursor = last_candidate.session_id.clone();
            let batch = self.run_candidates(candidates, trigger)?;
            total.scanned += batch.scanned;
            total.published += batch.published;
            total.deferred += batch.deferred;
            total.stale += batch.stale;
            after_session_id = Some(next_cursor);
        }
        Ok(total)
    }

    pub(crate) fn run_startup_with_retry(
        &self,
        batch_limit: usize,
    ) -> Result<RecoveryBatchResult, SessionsApplicationError> {
        let initial = self.run_until_drained(batch_limit, RecoveryTrigger::Startup)?;
        if initial.deferred == 0 {
            return Ok(initial);
        }
        self.run_until_drained(batch_limit, RecoveryTrigger::ExplicitRetry)
    }

    fn defer_claim(
        &self,
        claim: &super::RecoveryCandidateClaim,
        result: &mut RecoveryBatchResult,
        level: SessionApplicationLogLevel,
        message: &str,
    ) -> Result<(), SessionsApplicationError> {
        match self.transactions.defer_recovery(claim, &self.clock.now()) {
            Ok(true) => result.deferred += 1,
            Ok(false) => result.stale += 1,
            Err(SessionsApplicationError::RetryableStorage(_)) => result.deferred += 1,
            Err(error) => return Err(error),
        }
        self.log(
            level,
            message,
            claim.session_id.clone(),
            claim.observed_execution_run_id.clone(),
            None,
        );
        Ok(())
    }

    fn publish_structural_failure(
        &self,
        claim: &super::RecoveryCandidateClaim,
        trigger: RecoveryTrigger,
        result: &mut RecoveryBatchResult,
    ) -> Result<(), SessionsApplicationError> {
        let created_at = self.clock.now();
        let recovery_revision = claim.recovery_revision + 1;
        let report = SessionRecoveryReport::new(
            format!("recovery:{}:{recovery_revision}", claim.session_id),
            claim.session_id.clone(),
            recovery_revision,
            trigger,
            claim.observed_lifecycle.clone(),
            claim.observed_execution_run_id.clone(),
            RecoveryDecision::Quarantined,
            vec![crate::contexts::sessions::domain::recovery::RecoveryReasonCode::InvalidExecutionCorrelation],
            vec![RecoveryEvidenceReference::Session {
                session_id: claim.session_id.clone(),
                state_revision: claim.state_revision,
                history_revision: claim.history_revision,
            }],
            created_at.clone(),
        );
        let report_id = report.report_id().to_string();
        match self.transactions.publish_recovery(&PublishRecoveryRequest {
            claim: claim.clone(),
            assistant_message_id: None,
            report,
            published_at: created_at,
        }) {
            Ok(true) => {
                result.published += 1;
                let _ = self.events.publish_recovery_event(SessionRecoveryEvent {
                    kind: SessionRecoveryEventKind::Quarantined,
                    session_id: claim.session_id.clone(),
                    recovery_revision,
                });
                self.log(
                    SessionApplicationLogLevel::Warn,
                    "Structurally invalid recovery evidence was quarantined.",
                    claim.session_id.clone(),
                    claim.observed_execution_run_id.clone(),
                    Some(report_id),
                );
            }
            Ok(false) => result.stale += 1,
            Err(SessionsApplicationError::RetryableStorage(_)) => result.deferred += 1,
            Err(error) => return Err(error),
        }
        Ok(())
    }

    fn log_service(&self, level: SessionApplicationLogLevel, message: &str) {
        let _ = self.logging.write(SessionApplicationLog {
            level,
            category: "session.recovery".to_string(),
            message: message.to_string(),
            session_id: None,
            operation_id: None,
            execution_run_id: None,
            recovery_report_id: None,
        });
    }

    fn log(
        &self,
        level: SessionApplicationLogLevel,
        message: &str,
        session_id: String,
        execution_run_id: Option<String>,
        recovery_report_id: Option<String>,
    ) {
        let _ = self.logging.write(SessionApplicationLog {
            level,
            category: "session.recovery".to_string(),
            message: message.to_string(),
            session_id: Some(session_id),
            operation_id: None,
            execution_run_id,
            recovery_report_id,
        });
    }
}

fn evidence_references(evidence: &SessionTerminalEvidence) -> Vec<RecoveryEvidenceReference> {
    let mut references = vec![RecoveryEvidenceReference::Session {
        session_id: evidence.session.session_id.clone(),
        state_revision: evidence.session.state_revision,
        history_revision: evidence.session.history_revision,
    }];
    references.extend(evidence.messages().iter().map(|message| {
        RecoveryEvidenceReference::Message {
            message_id: message.message_id.clone(),
            execution_run_id: message.execution_run_id.clone(),
            status: message.status.as_str().to_string(),
        }
    }));
    if let Some(message) = evidence.conflicting_message() {
        references.push(RecoveryEvidenceReference::Message {
            message_id: message.message_id.clone(),
            execution_run_id: message.execution_run_id.clone(),
            status: message.status.as_str().to_string(),
        });
    }
    references.extend(evidence.operations().iter().map(|operation| {
        RecoveryEvidenceReference::Operation {
            operation_id: operation.operation_id.clone(),
            execution_run_id: operation.execution_run_id.clone(),
            status: operation_status(operation.status).to_string(),
        }
    }));
    if evidence
        .messages()
        .iter()
        .any(|message| !matches!(message.tool_activity, ToolActivityEvidence::None))
    {
        references.push(RecoveryEvidenceReference::ToolActivity {
            tool_use_id: "bounded-aggregate".to_string(),
            execution_run_id: evidence.observed_execution_run_id.clone(),
            status: "present".to_string(),
        });
    }
    references.push(RecoveryEvidenceReference::ProviderResumeMetadata {
        present: evidence.provider_resume.metadata_present,
    });
    references.push(RecoveryEvidenceReference::LiveRuntimeHandle {
        execution_run_id: evidence.observed_execution_run_id.clone(),
        present: evidence.live_handle == LiveHandleEvidence::Present,
    });
    references
}

fn operation_status(status: OperationTerminalStatus) -> &'static str {
    match status {
        OperationTerminalStatus::Running => "running",
        OperationTerminalStatus::Succeeded => "succeeded",
        OperationTerminalStatus::Failed => "failed",
        OperationTerminalStatus::Cancelled => "cancelled",
    }
}
