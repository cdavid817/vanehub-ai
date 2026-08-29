const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationRetentionPolicyV1 {
    pub(crate) failed_cancelled_days: u16,
    pub(crate) completed_package_days: u16,
}

impl Default for GenerationRetentionPolicyV1 {
    fn default() -> Self {
        Self {
            failed_cancelled_days: 180,
            completed_package_days: 365,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct GenerationRetentionCutoffsV1 {
    pub(crate) failed_cancelled_before_ms: i64,
    pub(crate) completed_package_before_ms: i64,
}

pub(crate) fn retention_cutoffs(
    policy: GenerationRetentionPolicyV1,
    now_ms: i64,
) -> Option<GenerationRetentionCutoffsV1> {
    if now_ms < 0
        || !(30..=180).contains(&policy.failed_cancelled_days)
        || !(180..=365).contains(&policy.completed_package_days)
    {
        return None;
    }
    Some(GenerationRetentionCutoffsV1 {
        failed_cancelled_before_ms: now_ms
            .checked_sub(i64::from(policy.failed_cancelled_days) * DAY_MS)?,
        completed_package_before_ms: now_ms
            .checked_sub(i64::from(policy.completed_package_days) * DAY_MS)?,
    })
}
