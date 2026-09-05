use super::{decision_policy::*, *};
use crate::contexts::skill_evolution_curation::domain::*;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub(crate) struct CuratorApplicationService<'a, S, O> {
    pub(super) store: &'a mut S,
    pub(super) overlays: &'a O,
    pub(super) actor: CuratorTrustedActor,
}

impl<'a, S, O> CuratorApplicationService<'a, S, O>
where
    S: CuratorApplicationStore,
    O: CuratorOverlayApplicationPort,
{
    pub(crate) fn new(store: &'a mut S, overlays: &'a O, actor: CuratorTrustedActor) -> Self {
        Self {
            store,
            overlays,
            actor,
        }
    }

    pub(crate) fn approve(
        &mut self,
        request: CuratorApprovalRequest<'_>,
    ) -> Result<CuratorApplicationOutcome, CuratorApplicationServiceError> {
        validate_actor(self.actor).map_err(policy_error)?;
        validate_action_key(request.idempotency_key).map_err(policy_error)?;
        let application_id = stable_id(
            "curator-application",
            request.candidate_id,
            request.idempotency_key,
        );
        if let Some(prepared) = self.store.existing_application(
            &application_id,
            request.candidate_id,
            request.expected_candidate_revision,
            request.confirmed_preview_hash,
            request.confirmed_effective_diff_hash,
            None,
        )? {
            let overlay_request = overlay_request(&prepared);
            return self.resolve_duplicate(prepared, &overlay_request);
        }
        let binding = self.store.application_binding(request.candidate_id)?;
        let preview = validate_approval(request, &binding.decision, self.actor.occurred_at_ms())
            .map_err(policy_error)?;
        let decision_id = stable_id(
            "curator-approval",
            request.candidate_id,
            request.idempotency_key,
        );
        let intent = CuratorApplicationIntent {
            outbox_id: format!("outbox-{application_id}"),
            application_id: application_id.clone(),
            decision: CuratorDecision {
                decision_id,
                candidate_id: request.candidate_id.to_string(),
                candidate_revision: request.expected_candidate_revision,
                kind: CuratorDecisionKind::Approve,
                actor_class: self.actor.actor_class(),
                reason_code: "explicit_preview_approval".to_string(),
                note_hash: None,
                preview_hash: Some(preview.witness_hash.clone()),
                decided_at_ms: self.actor.occurred_at_ms(),
            },
            idempotency_key: request.idempotency_key.to_string(),
            approved_witness_hash: preview.witness_hash.clone(),
            approved_diff_hash: preview.effective_diff_hash.clone(),
            expected_effective_hash: binding.overlay_witnesses.proposed_effective_hash.clone(),
            expected_state: CuratorCandidateState::ReadyForReview,
            system_policy_authorization: None,
        };
        let prepared = self.store.prepare_application_intent(&intent)?;
        let overlay_request = overlay_request(&prepared);
        if prepared.duplicate {
            return self.resolve_duplicate(prepared, &overlay_request);
        }
        match self.overlays.apply(&overlay_request) {
            Ok(receipt) if receipt.effective_diff_hash == intent.expected_effective_hash => {
                let application = self.store.finalize_application(
                    &application_id,
                    prepared.application.revision,
                    Ok(&receipt),
                    self.actor.occurred_at_ms(),
                )?;
                Ok(CuratorApplicationOutcome::Applied(application))
            }
            Ok(_) => self.finalize_failure(
                &application_id,
                prepared.application.revision,
                CuratorApplicationFailure::Stale,
            ),
            Err(failure) => {
                self.finalize_failure(&application_id, prepared.application.revision, failure)
            }
        }
    }

    pub(crate) fn recover_pending(
        &mut self,
    ) -> Result<Vec<CuratorApplicationOutcome>, CuratorApplicationServiceError> {
        let pending = self
            .store
            .pending_applications(CURATOR_RECOVERY_PAGE_LIMIT)?;
        let mut outcomes = Vec::with_capacity(pending.len());
        for prepared in pending {
            let request = overlay_request(&prepared);
            let outcome = match self.overlays.find_committed(&request) {
                Ok(Some(receipt))
                    if receipt.effective_diff_hash == request.witnesses.proposed_effective_hash =>
                {
                    let application = self.store.finalize_application(
                        &prepared.application.application_id,
                        prepared.application.revision,
                        Ok(&receipt),
                        self.actor.occurred_at_ms(),
                    )?;
                    CuratorApplicationOutcome::Applied(application)
                }
                Ok(Some(_)) | Ok(None) => {
                    let application = self.store.finalize_application(
                        &prepared.application.application_id,
                        prepared.application.revision,
                        Err(CuratorApplicationFailure::Unavailable),
                        self.actor.occurred_at_ms(),
                    )?;
                    CuratorApplicationOutcome::Failed(application)
                }
                Err(failure) => return Err(failure.into()),
            };
            outcomes.push(outcome);
        }
        Ok(outcomes)
    }

    pub(crate) fn prepare_retry(
        &mut self,
        candidate_id: &str,
        expected_candidate_revision: u64,
    ) -> Result<u64, CuratorApplicationServiceError> {
        validate_actor(self.actor).map_err(policy_error)?;
        Ok(self.store.prepare_failed_retry(
            candidate_id,
            expected_candidate_revision,
            self.actor.occurred_at_ms(),
        )?)
    }

    pub(super) fn resolve_duplicate(
        &mut self,
        prepared: CuratorPreparedApplication,
        request: &CuratorOverlayApplicationRequest,
    ) -> Result<CuratorApplicationOutcome, CuratorApplicationServiceError> {
        match prepared.application.status {
            CuratorApplicationStatus::Applied | CuratorApplicationStatus::Reconciled => {
                Ok(CuratorApplicationOutcome::Applied(prepared.application))
            }
            CuratorApplicationStatus::Failed => {
                Ok(CuratorApplicationOutcome::Failed(prepared.application))
            }
            CuratorApplicationStatus::IntentRecorded | CuratorApplicationStatus::Applying => {
                match self.overlays.find_committed(request)? {
                    Some(receipt)
                        if receipt.effective_diff_hash
                            == request.witnesses.proposed_effective_hash =>
                    {
                        let application = self.store.finalize_application(
                            &prepared.application.application_id,
                            prepared.application.revision,
                            Ok(&receipt),
                            self.actor.occurred_at_ms(),
                        )?;
                        Ok(CuratorApplicationOutcome::Applied(application))
                    }
                    Some(_) => Err(CuratorApplicationServiceError::RecoveryRequired),
                    None => Err(CuratorApplicationServiceError::RecoveryRequired),
                }
            }
        }
    }

    pub(super) fn finalize_failure(
        &mut self,
        application_id: &str,
        application_revision: u64,
        failure: CuratorApplicationFailure,
    ) -> Result<CuratorApplicationOutcome, CuratorApplicationServiceError> {
        let application = self.store.finalize_application(
            application_id,
            application_revision,
            Err(failure),
            self.actor.occurred_at_ms(),
        )?;
        Ok(CuratorApplicationOutcome::Failed(application))
    }
}

pub(super) fn overlay_request(
    prepared: &CuratorPreparedApplication,
) -> CuratorOverlayApplicationRequest {
    CuratorOverlayApplicationRequest {
        application_id: prepared.application.application_id.clone(),
        workspace_id: prepared.binding.workspace_id.clone(),
        target_skill_id: prepared.binding.target_skill_id.clone(),
        overlay_scope: prepared.binding.overlay_scope.clone(),
        mutation: prepared.binding.mutation.clone(),
        witnesses: prepared.binding.overlay_witnesses.clone(),
    }
}

pub(super) fn stable_id(prefix: &str, candidate_id: &str, idempotency_key: &str) -> String {
    let digest = Sha256::digest(format!("{prefix}:{candidate_id}:{idempotency_key}").as_bytes());
    format!(
        "{prefix}-{}",
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    )
}

pub(super) fn policy_error(reason: &'static str) -> CuratorApplicationServiceError {
    match reason {
        "interactive_actor_required" => CuratorApplicationServiceError::Unauthorized,
        "approval_preview_missing" => CuratorApplicationServiceError::PreviewMissing,
        "approval_preview_expired" => CuratorApplicationServiceError::PreviewExpired,
        "approval_preview_mismatch" => CuratorApplicationServiceError::PreviewMismatch,
        "approval_preview_incomplete" => CuratorApplicationServiceError::PreviewIncomplete,
        "approval_candidate_stale" => CuratorApplicationServiceError::Conflict,
        _ => CuratorApplicationServiceError::InvalidInput(reason),
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum CuratorApplicationServiceError {
    #[error("interactive curator actor is required")]
    Unauthorized,
    #[error("curator application changed concurrently")]
    Conflict,
    #[error("curator application input is invalid: {0}")]
    InvalidInput(&'static str),
    #[error("curator approval preview is missing")]
    PreviewMissing,
    #[error("curator approval preview expired")]
    PreviewExpired,
    #[error("curator approval preview does not match")]
    PreviewMismatch,
    #[error("curator approval preview is incomplete")]
    PreviewIncomplete,
    #[error("curator application requires recovery")]
    RecoveryRequired,
    #[error(transparent)]
    Store(#[from] CuratorApplicationStoreError),
    #[error("overlay application failed: {0:?}")]
    Overlay(CuratorApplicationFailure),
}

impl From<CuratorApplicationFailure> for CuratorApplicationServiceError {
    fn from(value: CuratorApplicationFailure) -> Self {
        Self::Overlay(value)
    }
}
