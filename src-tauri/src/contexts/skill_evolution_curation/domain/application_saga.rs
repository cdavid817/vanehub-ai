use super::*;

pub(crate) const CURATOR_RECOVERY_PAGE_LIMIT: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApplicationBinding {
    pub(crate) decision: CuratorDecisionBinding,
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) overlay_scope: String,
    pub(crate) mutation: CuratorDraftMutationInput,
    pub(crate) overlay_witnesses: CuratorApplicationOverlayWitnesses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApplicationOverlayWitnesses {
    pub(crate) expected_overlay_revision: Option<u64>,
    pub(crate) base_instruction_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) proposed_effective_hash: String,
    pub(crate) expected_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApplicationIntent {
    pub(crate) application_id: String,
    pub(crate) outbox_id: String,
    pub(crate) decision: CuratorDecision,
    pub(crate) idempotency_key: String,
    pub(crate) approved_witness_hash: String,
    pub(crate) approved_diff_hash: String,
    pub(crate) expected_effective_hash: String,
    pub(crate) expected_state: CuratorCandidateState,
    pub(crate) system_policy_authorization: Option<CuratorSystemPolicyAuthorizationV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorSystemPolicyAuthorizationV1 {
    pub(crate) run_id: String,
    pub(crate) eligibility_id: String,
    pub(crate) eligibility_proof_hash: String,
    pub(crate) preflight_witness_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) rate_reservation_id: String,
    pub(crate) authorized_at_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorSystemPolicyApplicationRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) preview_hash: &'a str,
    pub(crate) effective_diff_hash: &'a str,
    pub(crate) idempotency_key: &'a str,
    pub(crate) authorization: &'a CuratorSystemPolicyAuthorizationV1,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorPreparedApplication {
    pub(crate) application: CuratorApplication,
    pub(crate) binding: CuratorApplicationBinding,
    pub(crate) duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorOverlayApplicationRequest {
    pub(crate) application_id: String,
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) overlay_scope: String,
    pub(crate) mutation: CuratorDraftMutationInput,
    pub(crate) witnesses: CuratorApplicationOverlayWitnesses,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorOverlayApplicationReceipt {
    pub(crate) overlay_revision: String,
    pub(crate) overlay_history_id: String,
    pub(crate) effective_diff_hash: String,
    pub(crate) duplicate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CuratorApplicationFailure {
    Pinned,
    Stale,
    Conflict,
    Validation,
    Integrity,
    Filesystem,
    Unavailable,
}

impl CuratorApplicationFailure {
    pub(crate) fn code(self) -> &'static str {
        match self {
            Self::Pinned => "overlay_pinned",
            Self::Stale => "overlay_stale",
            Self::Conflict => "overlay_conflict",
            Self::Validation => "overlay_validation_failed",
            Self::Integrity => "overlay_integrity_failed",
            Self::Filesystem => "overlay_filesystem_failed",
            Self::Unavailable => "overlay_unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CuratorApplicationOutcome {
    Applied(CuratorApplication),
    Failed(CuratorApplication),
}
