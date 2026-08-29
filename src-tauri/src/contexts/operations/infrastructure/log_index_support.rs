//! The small adapters the log index needs: what time it is, what to call a repair, where a notice
//! goes, and where the index reports its own trouble.

use crate::contexts::operations::application::{
    BackfillOperationPublisher, LogIndexClock, LogIndexDiagnostics, LogIndexIdGenerator,
    PostCommitLogNoticePublisher, SessionLogBackfillStatus, SessionLogNotice,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};

/// The Tauri event a log subscriber listens on.
pub(crate) const SESSION_LOG_EVENT: &str = "session-log:appended";

/// The Tauri event a repair-progress subscriber listens on.
pub(crate) const SESSION_LOG_REPAIR_EVENT: &str = "session-log:repair";

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SystemLogIndexClock;

impl LogIndexClock for SystemLogIndexClock {
    fn now(&self) -> String {
        chrono::Utc::now().to_rfc3339()
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct UuidLogIndexIds;

impl LogIndexIdGenerator for UuidLogIndexIds {
    fn next_operation_id(&self) -> String {
        format!("log-repair-{}", uuid::Uuid::new_v4())
    }
}

/// One bounded notice per indexed record.
///
/// Identifiers, ordering, correlation and coverage. Never the line: a view that wants the row
/// fetches it by id, which keeps the event bus from carrying the corpus and keeps one authoritative
/// shape for a row instead of two that can disagree.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TauriLogNotice {
    /// What this announces, as a discriminant the client switches on. Without it a gap would be
    /// told apart from a row by whether `recordId` happened to be empty, and a subscriber that
    /// missed the distinction would go looking for a record that never existed.
    notice_kind: &'static str,
    record_id: String,
    sequence: i64,
    occurred_at: String,
    level: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    trace_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    span_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    operation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seat_id: Option<String>,
    coverage_state: &'static str,
    /// Gap only, and always a count with a code — never the records themselves. What was lost is
    /// exactly what nobody was able to redact, so it is the one thing that must not be described.
    #[serde(skip_serializing_if = "is_zero")]
    dropped_count: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<String>,
}

fn is_zero(value: &u32) -> bool {
    *value == 0
}

impl From<SessionLogNotice> for TauriLogNotice {
    fn from(notice: SessionLogNotice) -> Self {
        Self {
            notice_kind: notice.kind.token(),
            dropped_count: notice.dropped_count,
            reason_code: notice.reason_code,
            record_id: notice.record_id,
            sequence: notice.sequence,
            occurred_at: notice.occurred_at,
            level: notice.level.token(),
            session_id: notice.correlation.session_id,
            run_id: notice.correlation.run_id,
            trace_id: notice.correlation.trace_id,
            span_id: notice.correlation.span_id,
            operation_id: notice.correlation.operation_id,
            agent_id: notice.correlation.agent_id,
            seat_id: notice.correlation.seat_id,
            coverage_state: notice.coverage_state.token(),
        }
    }
}

#[derive(Clone)]
pub(crate) struct TauriLogNoticePublisher {
    app: AppHandle,
}

impl TauriLogNoticePublisher {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl PostCommitLogNoticePublisher for TauriLogNoticePublisher {
    fn publish(&self, notice: SessionLogNotice) {
        let _ = self
            .app
            .emit(SESSION_LOG_EVENT, TauriLogNotice::from(notice));
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TauriRepairStatus {
    operation_id: String,
    state: &'static str,
    files_completed: u32,
    files_total: u32,
    records_indexed: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    updated_at: Option<String>,
}

#[derive(Clone)]
pub(crate) struct TauriBackfillPublisher {
    app: AppHandle,
}

impl TauriBackfillPublisher {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl BackfillOperationPublisher for TauriBackfillPublisher {
    fn publish(&self, status: SessionLogBackfillStatus) {
        let _ = self.app.emit(
            SESSION_LOG_REPAIR_EVENT,
            TauriRepairStatus {
                operation_id: status.operation_id,
                state: status.state.token(),
                files_completed: status.files_completed,
                files_total: status.files_total,
                records_indexed: status.records_indexed,
                reason_code: status.reason_code,
                updated_at: status.updated_at,
            },
        );
    }
}

/// Where the index reports its own trouble.
///
/// Held in memory rather than written through the unified log, and that is the whole design: the
/// index is what a log write feeds, so diagnosing a failed index write by logging it would produce
/// another record, another failed write, and another diagnosis. A bounded ring of codes is read by
/// whoever asks instead.
#[derive(Default)]
pub(crate) struct BoundedLogIndexDiagnostics {
    entries: Mutex<Vec<(String, BTreeMap<String, String>)>>,
}

/// How many diagnostics are kept. Small on purpose: they are codes, and the newest are the ones
/// that explain the state a reader is looking at.
const MAX_DIAGNOSTICS: usize = 64;

impl LogIndexDiagnostics for BoundedLogIndexDiagnostics {
    fn report(&self, reason_code: &str, context: BTreeMap<String, String>) {
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if entries.len() >= MAX_DIAGNOSTICS {
            entries.remove(0);
        }
        entries.push((reason_code.to_string(), context));
    }
}

impl BoundedLogIndexDiagnostics {
    #[cfg(test)]
    pub(crate) fn codes(&self) -> Vec<String> {
        self.entries
            .lock()
            .map(|entries| entries.iter().map(|(code, _)| code.clone()).collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::operations::application::{
        IndexedLogLevel, LogCorrelation, SessionLogCoverageState, SessionLogNoticeKind,
    };

    /// The notice carries identifiers and never the line. A payload that included the message would
    /// put the corpus on the event bus and give a row two shapes that can disagree.
    #[test]
    fn a_log_notice_carries_identifiers_and_no_content() {
        let notice = TauriLogNotice::from(SessionLogNotice {
            kind: SessionLogNoticeKind::Appended,
            record_id: "record-1".to_string(),
            sequence: 12,
            occurred_at: "2026-08-24T10:00:00Z".to_string(),
            level: IndexedLogLevel::Error,
            correlation: LogCorrelation {
                session_id: Some("session-1".to_string()),
                ..LogCorrelation::default()
            },
            coverage_state: SessionLogCoverageState::Partial,
            dropped_count: 0,
            reason_code: None,
        });

        let payload = serde_json::to_value(notice).expect("notice");
        assert_eq!(payload["noticeKind"], "appended");
        assert_eq!(payload["recordId"], "record-1");
        assert_eq!(payload["sequence"], 12);
        assert_eq!(payload["level"], "error");
        assert_eq!(payload["sessionId"], "session-1");
        assert_eq!(payload["coverageState"], "partial");
        assert!(payload.get("message").is_none());
        assert!(payload.get("category").is_none());
        assert!(payload.get("context").is_none());
        // A correlation the record does not have is absent rather than null, so a reader tests one
        // thing instead of two.
        assert!(payload.get("runId").is_none());
        // Gap metadata is absent on a row notice, so a client reading `droppedCount` on the wrong
        // kind gets nothing rather than a zero that looks like a measured value.
        assert!(payload.get("droppedCount").is_none());
        assert!(payload.get("reasonCode").is_none());
    }

    /// A gap says how many and why, and nothing else.
    ///
    /// The records behind a gap are the ones nobody managed to redact — they never reached the
    /// index, so nothing downstream ever saw them redacted either. A count and a code is therefore
    /// the most that can be published about them, and the absent `recordId` is what stops a
    /// subscriber from trying to fetch one.
    #[test]
    fn a_gap_notice_carries_a_count_and_a_code_and_names_no_record() {
        let notice = TauriLogNotice::from(SessionLogNotice {
            kind: SessionLogNoticeKind::Gap,
            record_id: String::new(),
            sequence: 40,
            occurred_at: String::new(),
            level: IndexedLogLevel::Warn,
            correlation: LogCorrelation::default(),
            coverage_state: SessionLogCoverageState::Partial,
            dropped_count: 3,
            reason_code: Some("log_receipt_dropped".to_string()),
        });

        let payload = serde_json::to_value(notice).expect("notice");
        assert_eq!(payload["noticeKind"], "gap");
        assert_eq!(payload["droppedCount"], 3);
        assert_eq!(payload["reasonCode"], "log_receipt_dropped");
        assert_eq!(payload["coverageState"], "partial");
        // In sequence with the rows around it: a gap a subscriber learned about out of order would
        // be applied to the wrong part of its view.
        assert_eq!(payload["sequence"], 40);
        assert_eq!(payload["recordId"], "");
        // Attributing a dropped receipt to a session would be a guess presented as a fact — the
        // receipt that carried the correlation is the thing that was lost.
        assert!(payload.get("sessionId").is_none());
        assert!(payload.get("message").is_none());
    }

    #[test]
    fn diagnostics_stay_bounded_and_keep_the_newest() {
        let diagnostics = BoundedLogIndexDiagnostics::default();
        for index in 0..(MAX_DIAGNOSTICS + 5) {
            diagnostics.report(&format!("code-{index}"), BTreeMap::new());
        }

        let codes = diagnostics.codes();
        assert_eq!(codes.len(), MAX_DIAGNOSTICS);
        assert_eq!(codes.last().map(String::as_str), Some("code-68"));
    }
}
