use super::api_process_adapter::{summarize_turns, wire_format_for, REQUEST_TIMEOUT};
use crate::contexts::agent_runtime::application::{
    AgentLog, AgentLogLevel, AgentLoggingPort, ApiAgentGateway, ApiCredentialPort,
    UtilityChildExecutionOutcome, UtilityChildExecutionPort, UtilityDelegationEvidenceFact,
    UtilityDelegationEvidencePort, UtilityDelegationLifecycleFact, UtilityDelegationLifecyclePort,
};
use crate::contexts::agent_runtime::domain::{
    UtilityDelegationCounts, UtilityDelegationRequest, UtilityDelegationSnapshot,
    UtilityDelegationTerminal,
};
use crate::contexts::execution_observability::api::{
    ExecutionEvent, ExecutionRunId, ExecutionTelemetryPort, SafeAttributeValue, SafeAttributes,
    SpanId,
};
use crate::contexts::skill_evolution_evidence::application::ProjectionDisposition;
use crate::contexts::skill_evolution_evidence::application::{
    DelegatedUtilityFact, RuntimeEvidenceProjector,
};
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, ObservedSkillRevision, SkillAssociationKind, SourceFidelity, UtilityOutcome,
};
use crate::platform::network::blocking_http_client;
use serde_json::json;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

#[derive(Clone)]
pub(crate) struct NativeUtilityChildExecutor {
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
}

impl NativeUtilityChildExecutor {
    pub(crate) fn new(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
    ) -> Self {
        Self {
            credentials,
            config,
        }
    }
}

impl UtilityChildExecutionPort for NativeUtilityChildExecutor {
    fn execute(
        &self,
        request: &UtilityDelegationRequest,
        snapshot: &UtilityDelegationSnapshot,
        cancellation: Arc<AtomicBool>,
    ) -> UtilityChildExecutionOutcome {
        let started = Instant::now();
        if cancellation.load(Ordering::SeqCst) {
            return outcome(UtilityDelegationTerminal::Cancelled, started, None);
        }
        let api_key = match self.credentials.fetch(&request.agent_id) {
            Ok(Some(value)) => value,
            _ => return outcome(UtilityDelegationTerminal::Failed, started, None),
        };
        let config = match self.config.provider_config(&request.agent_id) {
            Ok(Some(value)) => value,
            _ => return outcome(UtilityDelegationTerminal::Failed, started, None),
        };
        let wire_format = match wire_format_for(&config) {
            Ok(value) => value,
            Err(_) => return outcome(UtilityDelegationTerminal::Failed, started, None),
        };
        let request_timeout = utility_request_timeout(request.limits.duration_ms);
        let client = match blocking_http_client(request_timeout) {
            Ok(value) => value,
            Err(_) => return outcome(UtilityDelegationTerminal::Failed, started, None),
        };
        let turns = [json!({ "role": "user", "content": request.task })];
        let result = summarize_turns(
            &wire_format,
            &client,
            &api_key,
            &config.model_id,
            Some(&snapshot.instructions),
            &turns,
            "Return only the bounded specialist result.",
            &cancellation,
        );
        if started.elapsed().as_millis() as u64 >= request.limits.duration_ms {
            return outcome(
                UtilityDelegationTerminal::TimedOut,
                started,
                Some("duration"),
            );
        }
        match result {
            Ok(Some(summary)) if summary.chars().count() > request.limits.result_chars => {
                UtilityChildExecutionOutcome {
                    terminal: UtilityDelegationTerminal::Limited,
                    summary: Some(summary.chars().take(request.limits.result_chars).collect()),
                    duration_ms: elapsed_ms(started),
                    counts: UtilityDelegationCounts::default(),
                    limit_reason: Some("result-characters".to_string()),
                }
            }
            Ok(summary) => UtilityChildExecutionOutcome {
                terminal: UtilityDelegationTerminal::Succeeded,
                summary,
                duration_ms: elapsed_ms(started),
                counts: UtilityDelegationCounts::default(),
                limit_reason: None,
            },
            Err(error) if error == "cancelled" => {
                outcome(UtilityDelegationTerminal::Cancelled, started, None)
            }
            Err(_) => outcome(UtilityDelegationTerminal::Failed, started, None),
        }
    }
}

fn outcome(
    terminal: UtilityDelegationTerminal,
    started: Instant,
    limit_reason: Option<&str>,
) -> UtilityChildExecutionOutcome {
    UtilityChildExecutionOutcome {
        terminal,
        summary: None,
        duration_ms: elapsed_ms(started),
        counts: UtilityDelegationCounts::default(),
        limit_reason: limit_reason.map(str::to_string),
    }
}

fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis().try_into().unwrap_or(u64::MAX)
}

fn utility_request_timeout(duration_ms: u64) -> std::time::Duration {
    REQUEST_TIMEOUT.min(std::time::Duration::from_millis(duration_ms))
}

#[derive(Clone)]
pub(crate) struct RuntimeUtilityLifecycleProjector {
    evidence: RuntimeEvidenceProjector,
    logging: Arc<dyn AgentLoggingPort>,
    telemetry: Arc<dyn ExecutionTelemetryPort>,
    sequence: Arc<AtomicU64>,
    evidence_drop_count: Arc<AtomicU64>,
}

impl RuntimeUtilityLifecycleProjector {
    pub(crate) fn new(
        evidence: RuntimeEvidenceProjector,
        logging: Arc<dyn AgentLoggingPort>,
        telemetry: Arc<dyn ExecutionTelemetryPort>,
    ) -> Self {
        Self {
            evidence,
            logging,
            telemetry,
            sequence: Arc::new(AtomicU64::new(0)),
            evidence_drop_count: Arc::new(AtomicU64::new(0)),
        }
    }
}

impl UtilityDelegationLifecyclePort for RuntimeUtilityLifecycleProjector {
    fn project(&self, fact: UtilityDelegationLifecycleFact) {
        let state = fact
            .terminal
            .map(UtilityDelegationTerminal::as_str)
            .unwrap_or("started");
        self.record_observability(&fact, state);
        let _ = self.logging.record(AgentLog {
            level: if matches!(
                fact.terminal,
                Some(UtilityDelegationTerminal::Failed | UtilityDelegationTerminal::TimedOut)
            ) {
                AgentLogLevel::Warn
            } else {
                AgentLogLevel::Info
            },
            category: "session.runtime.utility-delegation".to_string(),
            message: format!(
                "Utility delegation {state}; skill={}; revision={}; tools={}; approvals={}",
                fact.skill_id, fact.revision, fact.counts.tool_calls, fact.counts.approvals
            ),
            agent_id: Some(fact.agent_id),
            session_id: Some(fact.session_id),
            operation_id: Some(fact.attempt_id),
            run_id: Some(fact.parent_run_id),
            trace_id: None,
            span_id: None,
            occurred_at: fact.occurred_at,
        });
    }
}

impl RuntimeUtilityLifecycleProjector {
    fn record_observability(&self, fact: &UtilityDelegationLifecycleFact, state: &str) {
        let (Ok(run_id), Ok(span_id), Ok(attributes)) = (
            ExecutionRunId::parse(fact.parent_run_id.clone()),
            SpanId::parse(fact.parent_span_id.clone()),
            SafeAttributes::try_from_entries([
                (
                    "utility.skill_id".to_string(),
                    SafeAttributeValue::String(fact.skill_id.clone()),
                ),
                (
                    "utility.revision".to_string(),
                    SafeAttributeValue::String(fact.revision.clone()),
                ),
                (
                    "utility.state".to_string(),
                    SafeAttributeValue::String(state.to_string()),
                ),
                (
                    "utility.duration_ms".to_string(),
                    SafeAttributeValue::Integer(
                        fact.duration_ms
                            .unwrap_or_default()
                            .try_into()
                            .unwrap_or(i64::MAX),
                    ),
                ),
                (
                    "utility.tool_count".to_string(),
                    SafeAttributeValue::Integer(i64::from(fact.counts.tool_calls)),
                ),
                (
                    "utility.approval_count".to_string(),
                    SafeAttributeValue::Integer(i64::from(fact.counts.approvals)),
                ),
            ]),
        ) else {
            return;
        };
        let _ = self.telemetry.record_event(&ExecutionEvent {
            run_id,
            span_id,
            sequence: self.sequence.fetch_add(1, Ordering::SeqCst),
            name: format!("utility.delegation.{state}"),
            timestamp: fact.occurred_at.clone(),
            attributes,
        });
    }

    fn report_evidence_drop(&self, fact: &UtilityDelegationEvidenceFact) {
        let count = self.evidence_drop_count.fetch_add(1, Ordering::SeqCst) + 1;
        if count.is_power_of_two() {
            let _ = self.logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.utility-delegation".to_string(),
                message: format!("Utility evidence projection dropped; count={count}"),
                agent_id: Some(fact.agent_id.clone()),
                session_id: Some(fact.session_id.clone()),
                operation_id: Some(fact.attempt_id.clone()),
                run_id: Some(fact.parent_run_id.clone()),
                trace_id: None,
                span_id: Some(fact.parent_span_id.clone()),
                occurred_at: fact.occurred_at.clone(),
            });
        }
    }
}

impl UtilityDelegationEvidencePort for RuntimeUtilityLifecycleProjector {
    fn project(&self, fact: UtilityDelegationEvidenceFact) {
        let Some(terminal) = fact.terminal else {
            return;
        };
        let outcome = match terminal {
            UtilityDelegationTerminal::Succeeded => UtilityOutcome::Succeeded,
            UtilityDelegationTerminal::Cancelled => UtilityOutcome::Cancelled,
            UtilityDelegationTerminal::Failed => UtilityOutcome::Failed,
            UtilityDelegationTerminal::TimedOut => UtilityOutcome::TimedOut,
            UtilityDelegationTerminal::Limited => UtilityOutcome::Limited,
            UtilityDelegationTerminal::Refused => UtilityOutcome::Refused,
        };
        let observed = ObservedSkillRevision {
            skill_id: fact.skill_id.clone(),
            revision: fact.revision.clone(),
            association_kind: SkillAssociationKind::Delegated,
            observed_at: fact.occurred_at.clone(),
        };
        let diagnostic_fact = fact.clone();
        let disposition = self.evidence.delegation(DelegatedUtilityFact {
            common: EnvelopeCommon {
                source_event_id: format!("utility:{}:{}", fact.delegation_id, fact.attempt_id),
                occurred_at: fact.occurred_at,
                stable_agent_id: Some(fact.agent_id),
                session_id: Some(fact.session_id),
                message_id: Some(fact.message_id),
                run_id: Some(fact.parent_run_id),
                attempt_id: Some(fact.attempt_id),
                workspace: self
                    .evidence
                    .workspace_scope(fact.canonical_workspace.as_deref()),
                fidelity: SourceFidelity::Native,
                observed_skill_revisions: vec![observed],
            },
            utility_skill_id: fact.skill_id,
            revision: fact.revision,
            outcome,
            duration_ms: fact.duration_ms.unwrap_or_default(),
            tool_count: fact.counts.tool_calls,
            approval_count: fact.counts.approvals,
        });
        if disposition == ProjectionDisposition::Dropped {
            self.report_evidence_drop(&diagnostic_fact);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::application::AgentRuntimeApplicationError;
    use crate::contexts::execution_observability::api::{
        CapturedTelemetryRecord, CapturingExecutionTelemetry,
    };
    use crate::contexts::skill_evolution_evidence::application::{
        EvidenceProjectionSink, ProjectionDisposition,
    };
    use crate::contexts::skill_evolution_evidence::domain::EvidenceSourceEnvelope;
    use std::sync::Mutex;

    #[derive(Default)]
    struct CaptureLogging(Mutex<Vec<AgentLog>>);

    impl AgentLoggingPort for CaptureLogging {
        fn record(&self, log: AgentLog) -> Result<(), AgentRuntimeApplicationError> {
            self.0.lock().expect("logs").push(log);
            Ok(())
        }
    }

    #[derive(Default)]
    struct CaptureEvidence(Mutex<Vec<EvidenceSourceEnvelope>>);

    impl EvidenceProjectionSink for CaptureEvidence {
        fn submit(&self, envelope: EvidenceSourceEnvelope) -> ProjectionDisposition {
            self.0.lock().expect("evidence").push(envelope);
            ProjectionDisposition::Accepted
        }
    }

    struct DroppingEvidence;

    impl EvidenceProjectionSink for DroppingEvidence {
        fn submit(&self, _envelope: EvidenceSourceEnvelope) -> ProjectionDisposition {
            ProjectionDisposition::Dropped
        }
    }

    fn terminal_fact() -> UtilityDelegationLifecycleFact {
        UtilityDelegationLifecycleFact {
            agent_id: "agent-safe".to_string(),
            delegation_id: "delegation-safe".to_string(),
            attempt_id: "attempt-safe".to_string(),
            parent_run_id: "run-safe".to_string(),
            parent_span_id: "1111111111111111".to_string(),
            session_id: "session-safe".to_string(),
            message_id: "message-safe".to_string(),
            canonical_workspace: Some("D:/private/customer-project".to_string()),
            skill_id: "utility-safe".to_string(),
            revision: "revision-safe".to_string(),
            occurred_at: "2026-08-14T00:00:00Z".to_string(),
            terminal: Some(UtilityDelegationTerminal::Succeeded),
            duration_ms: Some(12),
            counts: UtilityDelegationCounts {
                tool_calls: 0,
                approvals: 0,
            },
            limit_reason: None,
        }
    }

    #[test]
    fn projections_contain_only_safe_metadata_and_hash_workspace_paths() {
        let logs = Arc::new(CaptureLogging::default());
        let evidence = Arc::new(CaptureEvidence::default());
        let projector = RuntimeUtilityLifecycleProjector::new(
            RuntimeEvidenceProjector::enabled(evidence.clone(), &[9_u8; 32]),
            logs.clone(),
            Arc::new(CapturingExecutionTelemetry::default()),
        );
        UtilityDelegationLifecyclePort::project(&projector, terminal_fact());
        UtilityDelegationEvidencePort::project(&projector, terminal_fact());

        let log_json = format!("{:?}", logs.0.lock().expect("logs").first());
        let evidence_json = serde_json::to_string(&*evidence.0.lock().expect("evidence"))
            .expect("serialize evidence");
        for forbidden in [
            "private task content",
            "private Skill instructions",
            "private model output",
            "provider-secret-token",
            "D:/private/customer-project",
        ] {
            assert!(!log_json.contains(forbidden));
            assert!(!evidence_json.contains(forbidden));
        }
        assert!(log_json.contains("utility-safe"));
        assert!(evidence_json.contains("revision-safe"));
        assert!(!evidence_json.contains("customer-project"));
    }

    #[test]
    fn projection_sink_failures_do_not_change_the_utility_terminal_result() {
        let projector = RuntimeUtilityLifecycleProjector::new(
            RuntimeEvidenceProjector::disabled(),
            Arc::new(CaptureLogging::default()),
            Arc::new(CapturingExecutionTelemetry::default()),
        );
        UtilityDelegationLifecyclePort::project(&projector, terminal_fact());
        UtilityDelegationEvidencePort::project(&projector, terminal_fact());
    }

    #[test]
    fn configured_duration_caps_the_provider_request_timeout() {
        assert_eq!(
            utility_request_timeout(25),
            std::time::Duration::from_millis(25)
        );
        assert_eq!(utility_request_timeout(300_000), REQUEST_TIMEOUT);
    }

    #[test]
    fn lifecycle_projects_safe_parent_correlated_observability() {
        let telemetry = CapturingExecutionTelemetry::default();
        let projector = RuntimeUtilityLifecycleProjector::new(
            RuntimeEvidenceProjector::disabled(),
            Arc::new(CaptureLogging::default()),
            Arc::new(telemetry.clone()),
        );
        let mut fact = terminal_fact();
        fact.parent_run_id = "11111111-1111-1111-1111-111111111111".to_string();
        UtilityDelegationLifecyclePort::project(&projector, fact);
        let records = telemetry.records().expect("telemetry");
        assert!(matches!(
            records.first(),
            Some(CapturedTelemetryRecord::Event(event))
                if event.name == "utility.delegation.succeeded"
        ));
    }

    #[test]
    fn evidence_drop_diagnostics_are_rate_limited_and_content_free() {
        let logs = Arc::new(CaptureLogging::default());
        let projector = RuntimeUtilityLifecycleProjector::new(
            RuntimeEvidenceProjector::enabled(Arc::new(DroppingEvidence), &[7_u8; 32]),
            logs.clone(),
            Arc::new(CapturingExecutionTelemetry::default()),
        );
        for _ in 0..5 {
            UtilityDelegationEvidencePort::project(&projector, terminal_fact());
        }
        let logs = logs.0.lock().expect("logs");
        assert_eq!(logs.len(), 3);
        assert!(logs
            .iter()
            .all(|log| log.message.contains("projection dropped")));
    }
}
