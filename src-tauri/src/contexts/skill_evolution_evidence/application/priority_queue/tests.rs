use super::*;
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, FeedbackState, SourceFidelity, EVIDENCE_ENVELOPE_SCHEMA_V1,
};

fn feedback(event: usize, state: FeedbackState) -> EvidenceSourceEnvelope {
    EvidenceSourceEnvelope::ExplicitFeedback {
        schema_version: EVIDENCE_ENVELOPE_SCHEMA_V1,
        common: EnvelopeCommon {
            source_event_id: format!("event-{event}"),
            occurred_at: "2026-08-13T10:00:00Z".to_string(),
            stable_agent_id: None,
            session_id: None,
            message_id: None,
            run_id: None,
            attempt_id: None,
            workspace: None,
            fidelity: SourceFidelity::Native,
            observed_skill_revisions: Vec::new(),
        },
        feedback: state,
        feedback_revision: event as u64,
        correction_note: (state == FeedbackState::Corrected).then(|| "safe correction".to_string()),
    }
}

#[test]
fn weak_lane_cannot_consume_reserved_capacity() {
    let queue = EvidencePriorityQueue::new();
    for event in 0..320 {
        assert!(matches!(
            queue.enqueue(feedback(event, FeedbackState::Unhelpful)),
            EnqueueOutcome::Accepted { .. }
        ));
    }
    assert_eq!(
        queue.enqueue(feedback(321, FeedbackState::Unhelpful)),
        EnqueueOutcome::Dropped {
            priority: EvidencePriority::Weak
        }
    );
    assert_eq!(queue.depth(), 320);
}

#[test]
fn critical_work_evicts_oldest_lower_priority_at_capacity() {
    let queue = EvidencePriorityQueue::new();
    for event in 0..320 {
        let _ = queue.enqueue(feedback(event, FeedbackState::Unhelpful));
    }
    for event in 320..512 {
        let _ = queue.enqueue(feedback(event, FeedbackState::Corrected));
    }
    assert_eq!(queue.depth(), EVIDENCE_QUEUE_CAPACITY);
    assert_eq!(
        queue.enqueue(feedback(513, FeedbackState::Corrected)),
        EnqueueOutcome::Accepted {
            priority: EvidencePriority::Critical,
            evicted: Some(EvidencePriority::Weak)
        }
    );
    assert_eq!(queue.depth(), EVIDENCE_QUEUE_CAPACITY);
}

#[test]
fn dequeue_is_priority_ordered_and_fifo_within_a_lane() {
    let queue = EvidencePriorityQueue::new();
    let _ = queue.enqueue(feedback(1, FeedbackState::Unhelpful));
    let _ = queue.enqueue(feedback(2, FeedbackState::Corrected));
    let _ = queue.enqueue(feedback(3, FeedbackState::Corrected));
    queue.close(true);

    let first = queue.dequeue().expect("first");
    let second = queue.dequeue().expect("second");
    assert_eq!(first.priority, EvidencePriority::Critical);
    assert_eq!(first.envelope.common().source_event_id, "event-2");
    assert_eq!(second.envelope.common().source_event_id, "event-3");
}

#[test]
fn non_draining_close_discards_pending_items() {
    let queue = EvidencePriorityQueue::new();
    let _ = queue.enqueue(feedback(1, FeedbackState::Corrected));
    assert_eq!(queue.close(false).iter().sum::<usize>(), 1);
    assert!(queue.dequeue().is_none());
    assert!(matches!(
        queue.enqueue(feedback(2, FeedbackState::Corrected)),
        EnqueueOutcome::Closed { .. }
    ));
}
