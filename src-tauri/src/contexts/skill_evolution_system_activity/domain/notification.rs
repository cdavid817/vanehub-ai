use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActivityNotificationPlan {
    Suppressed,
    Immediate,
    Digest(ActivityDigestCadence),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityNotificationOpenOutcome {
    PendingTimeline,
    Opened {
        session_id: String,
        sequence: u64,
        read_state: SystemActivityReadState,
    },
}

pub(crate) fn immediate_attention_eligible(envelope: &EvolutionActivityEnvelopeV1) -> bool {
    matches!(
        envelope.attention_kind,
        ActivityAttentionKind::Security
            | ActivityAttentionKind::Integrity
            | ActivityAttentionKind::ApplicationFailure
            | ActivityAttentionKind::Regression
            | ActivityAttentionKind::Breaker
    ) || (envelope.attention_kind == ActivityAttentionKind::Review
        && envelope.status == ActivityStatus::Blocked)
}

pub(crate) fn notification_plan(
    envelope: &EvolutionActivityEnvelopeV1,
    preferences: &EvolutionActivityPreferences,
) -> ActivityNotificationPlan {
    if immediate_attention_eligible(envelope)
        && severity_at_least(envelope.severity, preferences.notification_threshold)
    {
        ActivityNotificationPlan::Immediate
    } else {
        match preferences.digest_cadence {
            ActivityDigestCadence::Off => ActivityNotificationPlan::Suppressed,
            cadence => ActivityNotificationPlan::Digest(cadence),
        }
    }
}
