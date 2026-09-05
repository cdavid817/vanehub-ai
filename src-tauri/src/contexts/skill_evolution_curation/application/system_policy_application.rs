use super::{application_service::*, decision_policy::*, *};
use crate::contexts::skill_evolution_curation::domain::*;

impl<S, O> CuratorApplicationService<'_, S, O>
where
    S: CuratorApplicationStore,
    O: CuratorOverlayApplicationPort,
{
    pub(crate) fn apply_system_policy(
        &mut self,
        request: CuratorSystemPolicyApplicationRequest<'_>,
    ) -> Result<CuratorApplicationOutcome, CuratorApplicationServiceError> {
        validate_system_policy_request(self.actor, request)?;
        let application_id = stable_id(
            "curator-system-application",
            request.candidate_id,
            request.idempotency_key,
        );
        if let Some(prepared) = self.store.existing_application(
            &application_id,
            request.candidate_id,
            request.expected_candidate_revision,
            request.preview_hash,
            request.effective_diff_hash,
            Some(request.authorization),
        )? {
            let overlay_request = overlay_request(&prepared);
            return self.resolve_duplicate(prepared, &overlay_request);
        }
        let binding = self.store.application_binding(request.candidate_id)?;
        validate_system_binding(request, &binding, self.actor.occurred_at_ms())?;
        let intent = system_policy_intent(request, &binding, &application_id);
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
}

fn validate_system_policy_request(
    actor: CuratorTrustedActor,
    request: CuratorSystemPolicyApplicationRequest<'_>,
) -> Result<(), CuratorApplicationServiceError> {
    if actor.actor_class() != CuratorActorClass::System
        || actor.occurred_at_ms() < 0
        || actor.occurred_at_ms() != request.authorization.authorized_at_ms
    {
        return Err(CuratorApplicationServiceError::Unauthorized);
    }
    validate_action_key(request.idempotency_key).map_err(policy_error)?;
    Ok(())
}

fn validate_system_binding(
    request: CuratorSystemPolicyApplicationRequest<'_>,
    binding: &CuratorApplicationBinding,
    now_ms: i64,
) -> Result<(), CuratorApplicationServiceError> {
    if !matches!(
        binding.mutation,
        CuratorDraftMutationInput::LearnedGuidance { .. }
    ) {
        return Err(CuratorApplicationServiceError::InvalidInput(
            "system_policy_mutation_forbidden",
        ));
    }
    validate_approval(
        CuratorApprovalRequest {
            candidate_id: request.candidate_id,
            expected_candidate_revision: request.expected_candidate_revision,
            confirmed_preview_hash: request.preview_hash,
            confirmed_effective_diff_hash: request.effective_diff_hash,
            idempotency_key: request.idempotency_key,
        },
        &binding.decision,
        now_ms,
    )
    .map_err(policy_error)?;
    Ok(())
}

fn system_policy_intent(
    request: CuratorSystemPolicyApplicationRequest<'_>,
    binding: &CuratorApplicationBinding,
    application_id: &str,
) -> CuratorApplicationIntent {
    CuratorApplicationIntent {
        application_id: application_id.to_string(),
        outbox_id: format!("outbox-{application_id}"),
        decision: CuratorDecision {
            decision_id: stable_id(
                "curator-system-authorization",
                request.candidate_id,
                request.idempotency_key,
            ),
            candidate_id: request.candidate_id.to_string(),
            candidate_revision: request.expected_candidate_revision,
            kind: CuratorDecisionKind::Approve,
            actor_class: CuratorActorClass::System,
            reason_code: "system_policy_authorized".to_string(),
            note_hash: None,
            preview_hash: Some(request.preview_hash.to_string()),
            decided_at_ms: request.authorization.authorized_at_ms,
        },
        idempotency_key: request.idempotency_key.to_string(),
        approved_witness_hash: request.preview_hash.to_string(),
        approved_diff_hash: request.effective_diff_hash.to_string(),
        expected_effective_hash: binding.overlay_witnesses.proposed_effective_hash.clone(),
        expected_state: CuratorCandidateState::ReadyForReview,
        system_policy_authorization: Some(request.authorization.clone()),
    }
}
