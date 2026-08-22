use crate::contexts::execution_observability::application::evidence::models::EvidenceNotice;
use crate::contexts::execution_observability::application::{
    EvidenceApplicationError, EvidenceClockPort, EvidenceGapDiagnosticsPort,
    EvidenceIdGeneratorPort, EvidenceRedactionValidatorPort, PostCommitEvidenceNoticePublisherPort,
};
use crate::contexts::execution_observability::domain::{
    EvidenceSessionId, EvidenceSourceContext, ExecutionEvidenceEvent, SourceEventId,
};
use crate::contexts::operations::api::{DiagnosticLog, DiagnosticLogPort, LogSeverity};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter};

/// The native event channel the frontend subscribes to. It must stay in lockstep with the
/// listener name in the Tauri evidence transport; a mismatch is a silently dead subscription.
pub(crate) const EVIDENCE_EVENT_CHANNEL: &str = "execution-evidence:event";

pub(crate) struct SystemEvidenceClock;

impl EvidenceClockPort for SystemEvidenceClock {
    fn now_rfc3339(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

/// Event ids are generated here rather than accepted from a producer, so two producers cannot
/// collide on one id. Idempotency rides on `(source_context, source_event_id)` instead.
pub(crate) struct UuidEvidenceIdGenerator;

impl EvidenceIdGeneratorPort for UuidEvidenceIdGenerator {
    fn next_event_id(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }
}

/// The deployment-policy gate that runs after the domain has already validated an event.
///
/// It can only reject. The payload enum is what makes unsafe content unrepresentable; this exists
/// for rules a type cannot express, and letting it rewrite would allow it to turn an invalid event
/// into a plausible one.
pub(crate) struct DomainEvidenceRedactionValidator;

impl EvidenceRedactionValidatorPort for DomainEvidenceRedactionValidator {
    fn validate(&self, event: &ExecutionEvidenceEvent) -> Result<(), EvidenceApplicationError> {
        if event.correlation().session().is_none() {
            return Err(EvidenceApplicationError::Storage(
                "evidence reached the redaction gate without a session".to_string(),
            ));
        }
        Ok(())
    }
}

/// The wire shape of a live notice.
///
/// Identifiers, a sequence, a kind, and bounded counts. Nothing here is derived from the payload,
/// which is what makes it impossible for command text, a log line, a file path, a diff, or a
/// secret to reach the event channel — redaction cannot be re-applied once a value has crossed it.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceNoticeEvent {
    kind: &'static str,
    sequence: i64,
    session_id: String,
    occurred_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    record_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    command_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seat_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dropped_count: Option<u32>,
}

impl From<&EvidenceNotice> for EvidenceNoticeEvent {
    fn from(notice: &EvidenceNotice) -> Self {
        Self {
            kind: notice.kind.as_str(),
            sequence: notice.sequence,
            session_id: notice.session_id.as_str().to_string(),
            occurred_at: notice.occurred_at.clone(),
            record_id: notice.record_id.clone(),
            run_id: notice.run_id.clone(),
            trace_id: notice.trace_id.clone(),
            span_id: notice.span_id.clone(),
            operation_id: notice.operation_id.clone(),
            command_id: notice.command_id.clone(),
            seat_id: notice
                .seat_id
                .as_ref()
                .map(|seat| seat.as_str().to_string()),
            dropped_count: notice.dropped_count,
        }
    }
}

pub(crate) struct TauriEvidenceNoticePublisher {
    app: AppHandle,
}

impl TauriEvidenceNoticePublisher {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl PostCommitEvidenceNoticePublisherPort for TauriEvidenceNoticePublisher {
    /// A failed emit is dropped rather than retried or propagated. The event is already durable
    /// and the subscriber's next page query will find it; failing the write to keep a notification
    /// honest would lose the record entirely.
    fn publish(&self, notice: &EvidenceNotice) {
        let _ = self
            .app
            .emit(EVIDENCE_EVENT_CHANNEL, EvidenceNoticeEvent::from(notice));
    }
}

/// One diagnostic per source identity per window.
///
/// A producer bug usually repeats, and an unthrottled report would fill the log with one line. The
/// identity and a reason code are all that is written — never the two payloads and never a diff
/// between them, because that would put the content the journal declined into a second place.
const DIAGNOSTIC_WINDOW: Duration = Duration::from_secs(60);

pub(crate) struct RateLimitedEvidenceDiagnostics {
    logging: Arc<dyn DiagnosticLogPort>,
    seen: Mutex<BTreeMap<String, Instant>>,
}

impl RateLimitedEvidenceDiagnostics {
    pub(crate) fn new(logging: Arc<dyn DiagnosticLogPort>) -> Self {
        Self {
            logging,
            seen: Mutex::new(BTreeMap::new()),
        }
    }

    fn should_report(&self, key: &str, now: Instant) -> bool {
        // A poisoned lock must not silence a diagnostic: reporting twice is better than losing the
        // only signal that evidence is going missing.
        let mut seen = match self.seen.lock() {
            Ok(seen) => seen,
            Err(poisoned) => poisoned.into_inner(),
        };
        match seen.get(key) {
            Some(last) if now.duration_since(*last) < DIAGNOSTIC_WINDOW => false,
            _ => {
                seen.insert(key.to_string(), now);
                true
            }
        }
    }
}

impl EvidenceGapDiagnosticsPort for RateLimitedEvidenceDiagnostics {
    fn record_conflict(
        &self,
        source_context: EvidenceSourceContext,
        source_event_id: &SourceEventId,
    ) {
        let key = format!(
            "conflict:{}:{}",
            source_context.as_str(),
            source_event_id.as_str()
        );
        if !self.should_report(&key, Instant::now()) {
            return;
        }
        let mut context = BTreeMap::new();
        context.insert(
            "sourceContext".to_string(),
            source_context.as_str().to_string(),
        );
        context.insert(
            "sourceEventId".to_string(),
            source_event_id.as_str().to_string(),
        );
        context.insert(
            "reasonCode".to_string(),
            "evidence_conflicting_source_event".to_string(),
        );
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "execution.evidence".to_string(),
            message: "A different evidence event is already recorded for this source id."
                .to_string(),
            context,
        });
    }

    fn record_dropped(&self, session_id: &EvidenceSessionId, dropped_count: u32) {
        let key = format!("dropped:{}", session_id.as_str());
        if !self.should_report(&key, Instant::now()) {
            return;
        }
        let mut context = BTreeMap::new();
        context.insert("sessionId".to_string(), session_id.as_str().to_string());
        context.insert("droppedCount".to_string(), dropped_count.to_string());
        context.insert(
            "reasonCode".to_string(),
            "evidence_dropped_events".to_string(),
        );
        let _ = self.logging.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Warn,
            category: "execution.evidence".to_string(),
            message: "Bounded evidence queue dropped events before they reached the journal."
                .to_string(),
            context,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::application::evidence::models::EvidenceNoticeKind;
    use crate::contexts::operations::application::ApplicationError;

    #[derive(Default)]
    struct CapturingDiagnostics {
        logs: Mutex<Vec<DiagnosticLog>>,
    }

    impl DiagnosticLogPort for CapturingDiagnostics {
        fn write_diagnostic(&self, log: DiagnosticLog) -> Result<(), ApplicationError> {
            self.logs.lock().expect("logs").push(log);
            Ok(())
        }
    }

    fn session() -> EvidenceSessionId {
        EvidenceSessionId::parse("session-1").expect("session")
    }

    #[test]
    fn a_repeated_conflict_reports_once_per_window() {
        let sink = Arc::new(CapturingDiagnostics::default());
        let diagnostics = RateLimitedEvidenceDiagnostics::new(sink.clone());
        let source = SourceEventId::parse("source-1").expect("source");

        for _ in 0..5 {
            diagnostics.record_conflict(EvidenceSourceContext::AgentRuntime, &source);
        }

        let logs = sink.logs.lock().expect("logs");
        assert_eq!(logs.len(), 1, "a repeating producer bug reports once");
        assert_eq!(logs[0].category, "execution.evidence");
        assert_eq!(
            logs[0].context.get("reasonCode").map(String::as_str),
            Some("evidence_conflicting_source_event")
        );
        // The diagnostic names the identity and nothing else. Stated as an allowlist over the
        // context keys rather than a scan for forbidden words: a conflict report is one place a
        // payload could plausibly be attached "just for debugging", and only an allowlist makes
        // adding one fail here.
        let keys: Vec<&str> = logs[0].context.keys().map(String::as_str).collect();
        assert_eq!(keys, ["reasonCode", "sourceContext", "sourceEventId"]);
    }

    #[test]
    fn a_different_source_id_is_reported_separately() {
        let sink = Arc::new(CapturingDiagnostics::default());
        let diagnostics = RateLimitedEvidenceDiagnostics::new(sink.clone());

        diagnostics.record_conflict(
            EvidenceSourceContext::AgentRuntime,
            &SourceEventId::parse("source-1").expect("source"),
        );
        diagnostics.record_conflict(
            EvidenceSourceContext::Workspaces,
            &SourceEventId::parse("source-1").expect("source"),
        );

        assert_eq!(sink.logs.lock().expect("logs").len(), 2);
    }

    #[test]
    fn a_dropped_report_carries_only_the_session_and_a_count() {
        let sink = Arc::new(CapturingDiagnostics::default());
        let diagnostics = RateLimitedEvidenceDiagnostics::new(sink.clone());

        diagnostics.record_dropped(&session(), 4);

        let logs = sink.logs.lock().expect("logs");
        assert_eq!(
            logs[0].context.get("droppedCount").map(String::as_str),
            Some("4")
        );
        assert_eq!(logs[0].context.len(), 3);
    }

    /// A mismatched channel name is a subscription that never fires and never errors, which is
    /// indistinguishable from a session where nothing happened. Nothing but agreement between two
    /// string literals in two languages prevents it, so the agreement is asserted.
    #[test]
    fn the_event_channel_matches_the_typescript_listener() {
        let transport =
            include_str!("../../../../../../src/services/tauri-native-evidence-transport.ts");
        assert!(
            transport.contains(&format!("\"{EVIDENCE_EVENT_CHANNEL}\"")),
            "the frontend listens on a different channel than {EVIDENCE_EVENT_CHANNEL}"
        );
    }

    /// The event channel is the one place redaction cannot be re-applied, so the serialized notice
    /// is asserted field by field rather than trusted.
    #[test]
    fn a_serialized_notice_carries_identifiers_only() {
        let notice = EvidenceNotice {
            kind: EvidenceNoticeKind::RecordAppended,
            sequence: 12,
            session_id: session(),
            occurred_at: "2026-01-01T00:00:00Z".to_string(),
            record_id: Some("command:command-1".to_string()),
            run_id: Some("run-1".to_string()),
            trace_id: None,
            span_id: None,
            operation_id: None,
            command_id: Some("command-1".to_string()),
            seat_id: None,
            dropped_count: None,
        };

        let json = serde_json::to_value(EvidenceNoticeEvent::from(&notice)).expect("notice json");
        let object = json.as_object().expect("object");

        assert_eq!(
            object.get("kind").and_then(|v| v.as_str()),
            Some("record-appended")
        );
        assert_eq!(object.get("sequence").and_then(|v| v.as_i64()), Some(12));
        assert_eq!(
            object.get("sessionId").and_then(|v| v.as_str()),
            Some("session-1")
        );
        assert_eq!(
            object.get("commandId").and_then(|v| v.as_str()),
            Some("command-1")
        );
        // Absent correlation is omitted rather than serialized as null.
        assert!(!object.contains_key("traceId"));
        assert!(!object.contains_key("droppedCount"));
        // Nothing outside the identifier allowlist may appear.
        let allowed = [
            "kind",
            "sequence",
            "sessionId",
            "occurredAt",
            "recordId",
            "runId",
            "traceId",
            "spanId",
            "operationId",
            "commandId",
            "seatId",
            "droppedCount",
        ];
        for key in object.keys() {
            assert!(
                allowed.contains(&key.as_str()),
                "unexpected notice field: {key}"
            );
        }
    }
}
