use super::{decision_policy::*, *};
use crate::contexts::skill_evolution_curation::domain::*;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct CuratorDecisionService<'a, S> {
    store: &'a mut S,
    actor: CuratorTrustedActor,
}

impl<'a, S> CuratorDecisionService<'a, S>
where
    S: CuratorDecisionStore + CuratorPreviewStore,
{
    pub(crate) fn new(store: &'a mut S, actor: CuratorTrustedActor) -> Self {
        Self { store, actor }
    }

    pub(crate) fn reject(
        &mut self,
        request: CuratorRejectRequest<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionServiceError> {
        self.prepare(request.idempotency_key)?;
        if let Some(existing) = self.store.existing_decision(
            request.candidate_id,
            CuratorDecisionKind::Reject,
            request.idempotency_key,
        )? {
            return Ok(existing);
        }
        let binding = self.current(request.candidate_id, request.expected_candidate_revision)?;
        let transition = allowed_transition(binding.state, CuratorTransition::Reject)?;
        let decision = self.decision(
            &binding,
            CuratorDecisionKind::Reject,
            rejection_reason(request.reason),
            note_hash(request.note)?,
            None,
            request.idempotency_key,
        );
        self.persist(
            &binding,
            &decision,
            request.idempotency_key,
            transition,
            CuratorEventKind::Rejected,
            None,
        )
    }

    pub(crate) fn defer(
        &mut self,
        request: CuratorDeferRequest<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionServiceError> {
        self.prepare(request.idempotency_key)?;
        if let Some(existing) = self.store.existing_decision(
            request.candidate_id,
            CuratorDecisionKind::Defer,
            request.idempotency_key,
        )? {
            return Ok(existing);
        }
        let binding = self.current(request.candidate_id, request.expected_candidate_revision)?;
        validate_defer_time(
            self.actor.occurred_at_ms(),
            request.review_after_ms,
            binding.maximum_defer_days,
        )?;
        let transition = allowed_transition(binding.state, CuratorTransition::Defer)?;
        let decision = self.decision(
            &binding,
            CuratorDecisionKind::Defer,
            defer_reason(request.reason),
            note_hash(request.note)?,
            None,
            request.idempotency_key,
        );
        self.persist(
            &binding,
            &decision,
            request.idempotency_key,
            transition,
            CuratorEventKind::Deferred,
            request.review_after_ms,
        )
    }

    pub(crate) fn resume(
        &mut self,
        request: CuratorResumeRequest<'_>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionServiceError> {
        self.prepare(request.idempotency_key)?;
        if let Some(existing) = self.store.existing_decision(
            request.candidate_id,
            CuratorDecisionKind::Resume,
            request.idempotency_key,
        )? {
            return Ok(existing);
        }
        let binding = self.current(request.candidate_id, request.expected_candidate_revision)?;
        let transition = validate_resume(request, &binding)?;
        let decision = self.decision(
            &binding,
            CuratorDecisionKind::Resume,
            "manual_resume",
            None,
            None,
            request.idempotency_key,
        );
        self.persist(
            &binding,
            &decision,
            request.idempotency_key,
            transition,
            CuratorEventKind::Resumed,
            None,
        )
    }

    pub(crate) fn authorize_approval(
        &mut self,
        request: CuratorApprovalRequest<'_>,
    ) -> Result<CuratorApprovalAuthorization, CuratorDecisionServiceError> {
        self.prepare(request.idempotency_key)?;
        let binding = self.current(request.candidate_id, request.expected_candidate_revision)?;
        let preview = match validate_approval(request, &binding, self.actor.occurred_at_ms()) {
            Ok(preview) => preview,
            Err("approval_preview_expired") => {
                self.store.invalidate_preview(&CuratorPreviewInvalidation {
                    candidate_id: binding.candidate_id,
                    expected_candidate_revision: binding.candidate_revision,
                    reason: CuratorStalenessReason::PreviewExpired,
                    occurred_at_ms: self.actor.occurred_at_ms(),
                })?;
                return Err(CuratorDecisionServiceError::PreviewExpired);
            }
            Err(reason) => return Err(policy_error(reason)),
        };
        Ok(CuratorApprovalAuthorization {
            candidate_id: binding.candidate_id.clone(),
            candidate_revision: binding.candidate_revision,
            preview_id: preview.preview_id.clone(),
            preview_hash: preview.witness_hash.clone(),
            effective_diff_hash: preview.effective_diff_hash.clone(),
            actor_class: self.actor.actor_class(),
            native_application_allowed: self.actor.permits_native_application(),
        })
    }

    fn prepare(&self, idempotency_key: &str) -> Result<(), CuratorDecisionServiceError> {
        validate_actor(self.actor)?;
        validate_action_key(idempotency_key)?;
        Ok(())
    }

    fn current(
        &mut self,
        candidate_id: &str,
        expected_revision: u64,
    ) -> Result<CuratorDecisionBinding, CuratorDecisionServiceError> {
        let binding = self.store.decision_binding(candidate_id)?;
        if binding.candidate_revision != expected_revision {
            return Err(CuratorDecisionServiceError::Conflict);
        }
        Ok(binding)
    }

    fn decision(
        &self,
        binding: &CuratorDecisionBinding,
        kind: CuratorDecisionKind,
        reason_code: &str,
        note_hash: Option<String>,
        preview_hash: Option<String>,
        idempotency_key: &str,
    ) -> CuratorDecision {
        CuratorDecision {
            decision_id: decision_id(&binding.candidate_id, kind, idempotency_key),
            candidate_id: binding.candidate_id.clone(),
            candidate_revision: binding.candidate_revision,
            kind,
            actor_class: self.actor.actor_class(),
            reason_code: reason_code.into(),
            note_hash,
            preview_hash,
            decided_at_ms: self.actor.occurred_at_ms(),
        }
    }

    fn persist(
        &mut self,
        binding: &CuratorDecisionBinding,
        decision: &CuratorDecision,
        idempotency_key: &str,
        transition: CuratorTransition,
        event_kind: CuratorEventKind,
        review_after_ms: Option<i64>,
    ) -> Result<CuratorDecisionOutcome, CuratorDecisionServiceError> {
        Ok(self
            .store
            .persist_decision_mutation(&CuratorDecisionMutation {
                decision,
                idempotency_key,
                expected_state: binding.state,
                transition,
                event_kind,
                review_after_ms,
            })?)
    }
}

fn allowed_transition(
    state: CuratorCandidateState,
    transition: CuratorTransition,
) -> Result<CuratorTransition, CuratorDecisionServiceError> {
    transition_candidate(state, transition)
        .map(|_| transition)
        .map_err(|_| CuratorDecisionServiceError::InvalidState)
}

fn decision_id(candidate_id: &str, kind: CuratorDecisionKind, key: &str) -> String {
    let material = format!("curator-decision-v1:{candidate_id}:{kind:?}:{key}");
    let digest = Sha256::digest(material.as_bytes());
    format!(
        "curator-decision-{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

fn policy_error(reason: &'static str) -> CuratorDecisionServiceError {
    match reason {
        "interactive_actor_required" => CuratorDecisionServiceError::Unauthorized,
        "approval_preview_missing" => CuratorDecisionServiceError::PreviewMissing,
        "approval_preview_expired" => CuratorDecisionServiceError::PreviewExpired,
        "approval_preview_mismatch" => CuratorDecisionServiceError::PreviewMismatch,
        "approval_preview_incomplete" => CuratorDecisionServiceError::PreviewIncomplete,
        "approval_candidate_stale"
        | "resume_witness_mismatch"
        | "resume_draft_witness_mismatch" => CuratorDecisionServiceError::Conflict,
        _ => CuratorDecisionServiceError::InvalidInput(reason),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorDecisionServiceError {
    #[error("interactive curator actor is required")]
    Unauthorized,
    #[error("curator candidate changed concurrently")]
    Conflict,
    #[error("curator decision is not allowed in the current state")]
    InvalidState,
    #[error("curator decision input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("curator approval preview is missing")]
    PreviewMissing,
    #[error("curator approval preview expired")]
    PreviewExpired,
    #[error("curator approval preview does not match")]
    PreviewMismatch,
    #[error("curator approval preview is incomplete")]
    PreviewIncomplete,
    #[error(transparent)]
    Store(#[from] CuratorDecisionStoreError),
    #[error(transparent)]
    PreviewStore(#[from] CuratorPreviewStoreError),
}

impl From<&'static str> for CuratorDecisionServiceError {
    fn from(reason: &'static str) -> Self {
        policy_error(reason)
    }
}
