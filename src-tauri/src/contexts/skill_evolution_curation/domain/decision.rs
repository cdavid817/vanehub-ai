use super::*;

pub(crate) const CURATOR_MAX_DECISION_NOTE_CHARACTERS: usize = 1_000;
pub(crate) const CURATOR_MIN_DEFER_MS: i64 = 24 * 60 * 60 * 1_000;
pub(crate) const CURATOR_MAX_DEFER_MS: i64 = 180 * CURATOR_MIN_DEFER_MS;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorTrustedActor {
    actor_class: CuratorActorClass,
    occurred_at_ms: i64,
}

impl CuratorTrustedActor {
    pub(crate) fn local_interactive_user(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::LocalInteractiveUser,
            occurred_at_ms,
        }
    }

    pub(crate) fn system(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::System,
            occurred_at_ms,
        }
    }

    pub(crate) fn web_mock_interactive_user(occurred_at_ms: i64) -> Self {
        Self {
            actor_class: CuratorActorClass::WebMockInteractiveUser,
            occurred_at_ms,
        }
    }

    pub(crate) fn actor_class(self) -> CuratorActorClass {
        self.actor_class
    }

    pub(crate) fn occurred_at_ms(self) -> i64 {
        self.occurred_at_ms
    }

    pub(crate) fn is_interactive(self) -> bool {
        matches!(
            self.actor_class,
            CuratorActorClass::LocalInteractiveUser | CuratorActorClass::WebMockInteractiveUser
        )
    }

    pub(crate) fn permits_native_application(self) -> bool {
        self.actor_class == CuratorActorClass::LocalInteractiveUser
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorReadyDraftWitness {
    pub(crate) draft_revision: u64,
    pub(crate) assessment_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApprovalPreviewWitness {
    pub(crate) preview_id: String,
    pub(crate) witness_hash: String,
    pub(crate) effective_diff_hash: String,
    pub(crate) draft_revision: u64,
    pub(crate) assessment_id: String,
    pub(crate) issued_at_ms: i64,
    pub(crate) expires_at_ms: i64,
    pub(crate) diffs_complete: bool,
    pub(crate) validation_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDecisionBinding {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) candidate_hash: String,
    pub(crate) policy_hash: String,
    pub(crate) maximum_defer_days: u16,
    pub(crate) state: CuratorCandidateState,
    pub(crate) staleness: Vec<CuratorStalenessReason>,
    pub(crate) ready_draft: Option<CuratorReadyDraftWitness>,
    pub(crate) current_preview: Option<CuratorApprovalPreviewWitness>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorRejectRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) idempotency_key: &'a str,
    pub(crate) reason: CuratorRejectionReason,
    pub(crate) note: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorDeferRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) idempotency_key: &'a str,
    pub(crate) reason: CuratorDeferReason,
    pub(crate) note: Option<&'a str>,
    pub(crate) review_after_ms: Option<i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorResumeRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) expected_candidate_hash: &'a str,
    pub(crate) expected_policy_hash: &'a str,
    pub(crate) expected_draft_revision: Option<u64>,
    pub(crate) expected_assessment_id: Option<&'a str>,
    pub(crate) idempotency_key: &'a str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorApprovalRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) confirmed_preview_hash: &'a str,
    pub(crate) confirmed_effective_diff_hash: &'a str,
    pub(crate) idempotency_key: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDecisionOutcome {
    pub(crate) decision_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) state: CuratorCandidateState,
    pub(crate) duplicate: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorApprovalAuthorization {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) preview_id: String,
    pub(crate) preview_hash: String,
    pub(crate) effective_diff_hash: String,
    pub(crate) actor_class: CuratorActorClass,
    pub(crate) native_application_allowed: bool,
}
