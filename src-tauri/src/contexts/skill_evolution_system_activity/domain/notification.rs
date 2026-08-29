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

/// One closed digest window, claimed exactly once for notification delivery. Navigation is the
/// scope plus the window range: the UI opens the system session filtered to that range rather than
/// a single detail target, which is why this carries no allowlisted detail descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityDigestNotification {
    pub(crate) scope_kind: ActivityScopeKind,
    pub(crate) canonical_scope_id: String,
    pub(crate) cadence: ActivityDigestCadence,
    pub(crate) window_started_at_ms: i64,
    pub(crate) window_ends_at_ms: i64,
    pub(crate) counts_by_event_code: std::collections::BTreeMap<String, u32>,
    pub(crate) highest_severity: ActivitySeverity,
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
