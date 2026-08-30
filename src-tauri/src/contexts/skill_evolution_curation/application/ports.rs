use crate::contexts::skill_evolution_curation::domain::*;
use serde_json::Value;
use std::{future::Future, pin::Pin};
use thiserror::Error;

pub(crate) struct PreparedCuratorDraft {
    pub(crate) draft: CuratorDraftRevision,
    pub(crate) body: Value,
    pub(crate) scanner_version: String,
    pub(crate) expected_candidate_revision: u64,
}

pub(crate) trait CuratorDraftStore {
    fn candidate_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDraftCandidateBinding, CuratorDraftStoreError>;

    fn persist_prepared_draft(
        &mut self,
        prepared: &PreparedCuratorDraft,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorDraftStoreError>;

    fn record_draft_rejection(
        &mut self,
        candidate_id: &str,
        expected_revision: u64,
        reason_code: &str,
        scanner_version: &str,
        occurred_at_ms: i64,
    ) -> Result<(), CuratorDraftStoreError>;
}

pub(crate) trait CuratorOverlayDraftValidationPort {
    fn dry_validate(
        &self,
        binding: &CuratorDraftCandidateBinding,
        mutation: &CuratorDraftMutationInput,
    ) -> Result<CuratorOverlayValidationReceipt, CuratorOverlayValidationError>;
}

pub(crate) trait CuratorDraftReviewStore {
    fn review_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDraftReviewBinding, CuratorDraftReviewStoreError>;

    fn persist_draft_assessment(
        &mut self,
        assessment: &CuratorDraftAssessment,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorDraftReviewStoreError>;
}

pub(crate) trait CuratorDraftQualityPort: Send + Sync {
    fn review<'a>(
        &'a self,
        input: &'a CuratorDraftQualityInput,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<CuratorDraftQualityReceipt, CuratorDraftQualityError>>
                + Send
                + 'a,
        >,
    >;
}

pub(crate) trait CuratorPreviewStore {
    fn preview_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorPreviewBinding, CuratorPreviewStoreError>;

    fn persist_preview(
        &mut self,
        preview: &CuratorPreview,
    ) -> Result<u64, CuratorPreviewStoreError>;

    fn invalidate_preview(
        &mut self,
        invalidation: &CuratorPreviewInvalidation,
    ) -> Result<u64, CuratorPreviewStoreError>;
}

pub(crate) trait CuratorOverlayPreviewPort {
    fn preview(
        &self,
        binding: &CuratorPreviewBinding,
    ) -> Result<CuratorOverlayPreviewReceipt, CuratorOverlayPreviewError>;
}

pub(crate) trait CuratorDecisionStore {
    fn existing_decision(
        &mut self,
        candidate_id: &str,
        kind: CuratorDecisionKind,
        idempotency_key: &str,
    ) -> Result<Option<CuratorDecisionOutcome>, CuratorDecisionStoreError>;

    fn decision_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDecisionBinding, CuratorDecisionStoreError>;

    fn persist_decision_mutation(
        &mut self,
        mutation: &CuratorDecisionMutation<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionStoreError>;
}

pub(crate) trait CuratorApplicationStore {
    fn existing_application(
        &mut self,
        application_id: &str,
        candidate_id: &str,
        expected_candidate_revision: u64,
        approved_witness_hash: &str,
        approved_diff_hash: &str,
        system_policy_authorization: Option<&CuratorSystemPolicyAuthorizationV1>,
    ) -> Result<Option<CuratorPreparedApplication>, CuratorApplicationStoreError>;

    fn application_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorApplicationBinding, CuratorApplicationStoreError>;

    fn prepare_application_intent(
        &mut self,
        intent: &CuratorApplicationIntent,
    ) -> Result<CuratorPreparedApplication, CuratorApplicationStoreError>;

    fn finalize_application(
        &mut self,
        application_id: &str,
        expected_application_revision: u64,
        result: Result<&CuratorOverlayApplicationReceipt, CuratorApplicationFailure>,
        occurred_at_ms: i64,
    ) -> Result<CuratorApplication, CuratorApplicationStoreError>;

    fn pending_applications(
        &mut self,
        limit: usize,
    ) -> Result<Vec<CuratorPreparedApplication>, CuratorApplicationStoreError>;

    fn prepare_failed_retry(
        &mut self,
        candidate_id: &str,
        expected_candidate_revision: u64,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorApplicationStoreError>;
}

pub(crate) trait CuratorOverlayApplicationPort {
    fn apply(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<CuratorOverlayApplicationReceipt, CuratorApplicationFailure>;

    fn find_committed(
        &self,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<Option<CuratorOverlayApplicationReceipt>, CuratorApplicationFailure>;
}

pub(crate) struct CuratorDecisionMutation<'a> {
    pub(crate) decision: &'a CuratorDecision,
    pub(crate) idempotency_key: &'a str,
    pub(crate) expected_state: CuratorCandidateState,
    pub(crate) transition: CuratorTransition,
    pub(crate) event_kind: CuratorEventKind,
    pub(crate) review_after_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CuratorDraftStoreError {
    #[error("curator draft candidate was not found")]
    NotFound,
    #[error("curator draft candidate changed concurrently")]
    Conflict,
    #[error("curator draft persistence failed")]
    Storage,
    #[error("curator draft persistence input is invalid")]
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("overlay draft validation failed: {reason_code}")]
pub(crate) struct CuratorOverlayValidationError {
    pub(crate) reason_code: String,
    pub(crate) scanner_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CuratorDraftReviewStoreError {
    #[error("curator draft review source was not found")]
    NotFound,
    #[error("curator draft review changed concurrently")]
    Conflict,
    #[error("curator draft review persistence failed")]
    Storage,
    #[error("curator draft review persistence input is invalid")]
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("curator draft quality review failed: {reason_code}")]
pub(crate) struct CuratorDraftQualityError {
    pub(crate) reason_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CuratorPreviewStoreError {
    #[error("curator preview source was not found")]
    NotFound,
    #[error("curator preview changed concurrently")]
    Conflict,
    #[error("curator preview persistence failed")]
    Storage,
    #[error("curator preview persistence input is invalid")]
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("overlay preview failed: {reason_code}")]
pub(crate) struct CuratorOverlayPreviewError {
    pub(crate) reason_code: String,
    pub(crate) staleness: Option<CuratorStalenessReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CuratorDecisionStoreError {
    #[error("curator decision source was not found")]
    NotFound,
    #[error("curator decision changed concurrently")]
    Conflict,
    #[error("curator decision persistence failed")]
    Storage,
    #[error("curator decision persistence input is invalid")]
    InvalidInput,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum CuratorApplicationStoreError {
    #[error("curator application source was not found")]
    NotFound,
    #[error("curator application changed concurrently")]
    Conflict,
    #[error("curator application persistence failed")]
    Storage,
    #[error("curator application input is invalid")]
    InvalidInput,
}
