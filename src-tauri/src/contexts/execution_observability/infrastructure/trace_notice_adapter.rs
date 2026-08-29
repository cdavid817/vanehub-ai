//! Where a committed trace transition reaches the window.
//!
//! One event, carrying identifiers and a status. Not the span, not its attributes, not its name —
//! a view that received those would hold a second shape for a span it can already fetch, and the
//! two disagree the moment anything writes to it again.

use crate::contexts::execution_observability::application::{
    TraceTransitionNotice, TraceTransitionPublisherPort,
};
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// The channel a Traces subscriber listens on.
///
/// Has to match the frontend verbatim. A mismatch produces a subscription that never fires and
/// never errors — the one failure a live view cannot detect from the inside, because "nothing
/// changed" and "I am not being told about changes" look identical.
pub(crate) const TRACE_TRANSITION_EVENT: &str = "execution-trace:transition";

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct TraceTransitionEvent {
    kind: &'static str,
    run_id: String,
    trace_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    span_id: Option<String>,
    status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    occurred_at: Option<String>,
    /// Whether this changes the run list rather than one open timeline.
    ///
    /// Computed here rather than left to the client, so a busy run's spans cannot be made to
    /// invalidate the list by a view that guessed differently from the one next to it.
    affects_run_list: bool,
}

impl From<&TraceTransitionNotice> for TraceTransitionEvent {
    fn from(notice: &TraceTransitionNotice) -> Self {
        Self {
            kind: notice.kind.token(),
            run_id: notice.run_id.clone(),
            trace_id: notice.trace_id.clone(),
            span_id: notice.span_id.clone(),
            status: notice.status.as_str(),
            occurred_at: notice.occurred_at.clone(),
            affects_run_list: notice.kind.affects_run_list(),
        }
    }
}

pub(crate) struct TauriTraceTransitionPublisher {
    app: AppHandle,
}

impl TauriTraceTransitionPublisher {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl TraceTransitionPublisherPort for TauriTraceTransitionPublisher {
    /// A failed emit is dropped rather than retried or propagated.
    ///
    /// The transition is already committed, and the subscriber's next query will find it. Failing
    /// the write to keep a notification honest would lose the record the notification was about —
    /// which is the exact inversion this whole surface exists to avoid.
    fn publish(&self, notice: &TraceTransitionNotice) {
        let _ = self
            .app
            .emit(TRACE_TRANSITION_EVENT, TraceTransitionEvent::from(notice));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::execution_observability::application::TraceTransitionKind;
    use crate::contexts::execution_observability::domain::ExecutionStatus;

    fn notice(kind: TraceTransitionKind, span_id: Option<&str>) -> TraceTransitionNotice {
        TraceTransitionNotice {
            kind,
            run_id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0".to_string(),
            trace_id: "4bf92f3577b34da6a3ce929d0e0e4736".to_string(),
            span_id: span_id.map(str::to_string),
            status: ExecutionStatus::Running,
            occurred_at: None,
        }
    }

    /// The event carries identifiers and nothing a redaction pass would have to think about.
    #[test]
    fn a_transition_event_carries_identifiers_and_no_content() {
        let event = TraceTransitionEvent::from(&notice(
            TraceTransitionKind::SpanStarted,
            Some("00f067aa0ba902b7"),
        ));

        let payload = serde_json::to_value(event).expect("event");
        assert_eq!(payload["kind"], "span-started");
        assert_eq!(payload["spanId"], "00f067aa0ba902b7");
        assert_eq!(payload["status"], "running");
        // A span's name, its attributes and its events all stay behind the timeline query. Putting
        // them here would give a span two shapes that can disagree.
        assert!(payload.get("name").is_none());
        assert!(payload.get("attributes").is_none());
        assert!(payload.get("events").is_none());
    }

    /// A run transition invalidates the list; a span transition does not.
    ///
    /// Decided natively so two views cannot answer it differently, and so a busy run's spans
    /// cannot be made to re-fetch the run list once per span.
    #[test]
    fn only_a_run_transition_says_it_affects_the_run_list() {
        let run = TraceTransitionEvent::from(&notice(TraceTransitionKind::RunFinished, None));
        let span = TraceTransitionEvent::from(&notice(
            TraceTransitionKind::SpanFinished,
            Some("00f067aa0ba902b7"),
        ));

        assert!(run.affects_run_list);
        assert!(!span.affects_run_list);
    }

    /// A run transition names no span, and the field is absent rather than null.
    #[test]
    fn a_run_transition_names_no_span() {
        let event = TraceTransitionEvent::from(&notice(TraceTransitionKind::RunStarted, None));

        let payload = serde_json::to_value(event).expect("event");
        // Absent rather than null, so a reader tests one thing instead of two.
        assert!(payload.get("spanId").is_none());
        assert!(payload.get("occurredAt").is_none());
    }
}
