use super::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) const MIN_OPEN_RETENTION_DAYS: u16 = 1;
pub(crate) const MIN_TERMINAL_RETENTION_DAYS: u16 = 1;
pub(crate) const MIN_DRAFT_DISPLAY_BYTES: u32 = 1_024;
pub(crate) const MAX_DRAFT_DISPLAY_BYTES: u32 = 16 * 1_024;
pub(crate) const MIN_DIFF_DISPLAY_BYTES: u32 = 4 * 1_024;
pub(crate) const MAX_DIFF_DISPLAY_BYTES: u32 = 64 * 1_024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPolicyUpdateV1 {
    pub(crate) schema_version: u16,
    pub(crate) enqueue_routes: Vec<CuratorRoute>,
    pub(crate) require_rejection_reason: bool,
    pub(crate) require_defer_reason: bool,
    pub(crate) maximum_defer_days: u16,
    pub(crate) open_retention_days: u16,
    pub(crate) terminal_retention_days: u16,
    pub(crate) notifications_enabled: bool,
    pub(crate) digest_enabled: bool,
    pub(crate) draft_display_limit_bytes: u32,
    pub(crate) diff_display_limit_bytes: u32,
}

impl CuratorPolicyUpdateV1 {
    pub(crate) fn validate(&self) -> Result<(), CuratorPolicyValidationError> {
        let routes_are_fixed = self.enqueue_routes.len() == 2
            && self.enqueue_routes.contains(&CuratorRoute::Advance)
            && self
                .enqueue_routes
                .contains(&CuratorRoute::NeedsHumanReview);
        if self.schema_version != CURATOR_SCHEMA_VERSION_V1 {
            return Err(CuratorPolicyValidationError::UnsupportedVersion);
        }
        if !routes_are_fixed {
            return Err(CuratorPolicyValidationError::QueueRoutesLocked);
        }
        if !self.require_rejection_reason || !self.require_defer_reason {
            return Err(CuratorPolicyValidationError::DecisionReasonRequired);
        }
        if !(1..=180).contains(&self.maximum_defer_days) {
            return Err(CuratorPolicyValidationError::DeferBounds);
        }
        if !(MIN_OPEN_RETENTION_DAYS..=DEFAULT_OPEN_RETENTION_DAYS)
            .contains(&self.open_retention_days)
            || !(MIN_TERMINAL_RETENTION_DAYS..=DEFAULT_TERMINAL_RETENTION_DAYS)
                .contains(&self.terminal_retention_days)
        {
            return Err(CuratorPolicyValidationError::RetentionBounds);
        }
        if !(MIN_DRAFT_DISPLAY_BYTES..=MAX_DRAFT_DISPLAY_BYTES)
            .contains(&self.draft_display_limit_bytes)
            || !(MIN_DIFF_DISPLAY_BYTES..=MAX_DIFF_DISPLAY_BYTES)
                .contains(&self.diff_display_limit_bytes)
        {
            return Err(CuratorPolicyValidationError::DisplayBounds);
        }
        Ok(())
    }

    pub(crate) fn materialize(self, workspace_id: String, revision: u64) -> CuratorPolicyV1 {
        CuratorPolicyV1 {
            schema_version: self.schema_version,
            workspace_id,
            enqueue_routes: self.enqueue_routes,
            require_rejection_reason: self.require_rejection_reason,
            require_defer_reason: self.require_defer_reason,
            maximum_defer_days: self.maximum_defer_days,
            open_retention_days: self.open_retention_days,
            terminal_retention_days: self.terminal_retention_days,
            notifications_enabled: self.notifications_enabled,
            digest_enabled: self.digest_enabled,
            draft_display_limit_bytes: self.draft_display_limit_bytes,
            diff_display_limit_bytes: self.diff_display_limit_bytes,
            revision,
        }
    }
}

impl From<CuratorPolicyV1> for CuratorPolicyUpdateV1 {
    fn from(value: CuratorPolicyV1) -> Self {
        Self {
            schema_version: value.schema_version,
            enqueue_routes: value.enqueue_routes,
            require_rejection_reason: value.require_rejection_reason,
            require_defer_reason: value.require_defer_reason,
            maximum_defer_days: value.maximum_defer_days,
            open_retention_days: value.open_retention_days,
            terminal_retention_days: value.terminal_retention_days,
            notifications_enabled: value.notifications_enabled,
            digest_enabled: value.digest_enabled,
            draft_display_limit_bytes: value.draft_display_limit_bytes,
            diff_display_limit_bytes: value.diff_display_limit_bytes,
        }
    }
}

pub(crate) fn policy_hash(
    policy: &CuratorPolicyV1,
) -> Result<String, CuratorPolicyValidationError> {
    let bytes =
        serde_json::to_vec(policy).map_err(|_| CuratorPolicyValidationError::Serialization)?;
    let digest = Sha256::digest(bytes);
    let hex: String = digest.iter().map(|byte| format!("{byte:02x}")).collect();
    Ok(format!("sha256:{hex}"))
}

#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratorPolicyValidationError {
    #[error("unsupported curator policy version")]
    UnsupportedVersion,
    #[error("curator queue routes are fixed to manual governance routes")]
    QueueRoutesLocked,
    #[error("curator rejection and defer reasons remain required")]
    DecisionReasonRequired,
    #[error("curator defer bounds are invalid")]
    DeferBounds,
    #[error("curator retention may only be reduced within safe bounds")]
    RetentionBounds,
    #[error("curator display limit is outside the safe bounds")]
    DisplayBounds,
    #[error("curator policy serialization failed")]
    Serialization,
}
