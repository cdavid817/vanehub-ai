use super::*;

pub(crate) const MIN_ACTIVITY_RETENTION_DAYS: u16 = 30;
pub(crate) const MAX_ACTIVITY_RETENTION_DAYS: u16 = 365;
pub(crate) const MAX_ACTIVITY_EXPORT_ITEMS: u32 = 10_000;
pub(crate) const MAX_ACTIVITY_EXPORT_BYTES: u64 = 100 * 1024 * 1024;

pub(crate) fn severity_at_least(actual: ActivitySeverity, minimum: ActivitySeverity) -> bool {
    severity_rank(actual) >= severity_rank(minimum)
}

pub(crate) fn is_mandatory_retention_event(envelope: &EvolutionActivityEnvelopeV1) -> bool {
    severity_at_least(envelope.severity, ActivitySeverity::Warning)
        && matches!(
            envelope.attention_kind,
            ActivityAttentionKind::Security
                | ActivityAttentionKind::Integrity
                | ActivityAttentionKind::Regression
                | ActivityAttentionKind::ApplicationFailure
                | ActivityAttentionKind::Breaker
        )
}

pub(crate) fn timeline_policy_allows(
    envelope: &EvolutionActivityEnvelopeV1,
    minimum: ActivitySeverity,
) -> bool {
    severity_at_least(envelope.severity, minimum) || is_mandatory_retention_event(envelope)
}

impl EvolutionActivityPreferences {
    pub(crate) fn validate(&self) -> Result<(), ActivityEnvelopeError> {
        sanitize_text(
            &self.canonical_scope_id,
            "preferences.canonical_scope_id",
            160,
        )?;
        if !(MIN_ACTIVITY_RETENTION_DAYS..=MAX_ACTIVITY_RETENTION_DAYS)
            .contains(&self.read_retention_days)
            || !(MIN_ACTIVITY_RETENTION_DAYS..=MAX_ACTIVITY_RETENTION_DAYS)
                .contains(&self.detail_retention_days)
            || self.export_item_limit == 0
            || self.export_item_limit > MAX_ACTIVITY_EXPORT_ITEMS
            || self.export_size_limit_bytes == 0
            || self.export_size_limit_bytes > MAX_ACTIVITY_EXPORT_BYTES
        {
            return Err(ActivityEnvelopeError::InvalidField("preferences.limits"));
        }
        Ok(())
    }
}

const fn severity_rank(severity: ActivitySeverity) -> u8 {
    match severity {
        ActivitySeverity::Info => 0,
        ActivitySeverity::Warning => 1,
        ActivitySeverity::Error => 2,
        ActivitySeverity::Critical => 3,
    }
}
