use super::{SafeDocumentError, TrustedAuditContext, ValidatedDraftDocument};
use crate::contexts::skill_evolution_curation::domain::*;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistCandidateOutcome {
    Inserted,
    Existing { candidate_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PersistDecisionOutcome {
    Inserted,
    Existing { decision_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateConflict {
    pub(crate) current_revision: u64,
    pub(crate) current_state: CuratorCandidateState,
}

pub(crate) struct CandidateTransitionRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_revision: u64,
    pub(crate) transition: CuratorTransition,
    pub(crate) event_kind: CuratorEventKind,
    pub(crate) reason_code: Option<&'a str>,
    pub(crate) audit: TrustedAuditContext,
}

pub(crate) struct DecisionPersistence<'a> {
    pub(crate) decision: &'a CuratorDecision,
    pub(crate) idempotency_key: &'a str,
    pub(crate) review_after_ms: Option<i64>,
}

pub(crate) struct DraftPersistence<'a> {
    pub(crate) draft: &'a CuratorDraftRevision,
    pub(crate) document: &'a ValidatedDraftDocument,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) occurred_at_ms: i64,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorRepositoryError {
    #[error("curator repository input is invalid")]
    InvalidInput,
    #[error("curator candidate was not found")]
    NotFound,
    #[error("curator candidate changed concurrently")]
    Conflict(CandidateConflict),
    #[error("curator persistence failed")]
    Storage,
    #[error(transparent)]
    Transition(#[from] CuratorTransitionError),
    #[error(transparent)]
    UnsafeDocument(#[from] SafeDocumentError),
}
