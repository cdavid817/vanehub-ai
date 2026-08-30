use super::{
    AutomaticApplicationStoreError, PreflightRepositoryError, SqliteAutomaticApplicationRepository,
    SqlitePreflightRepository,
};
use crate::contexts::{
    skill_evolution_curation::api::{CuratorSystemPolicyApplyInput, SkillEvolutionCurationApi},
    skill_evolution_orchestration::{application::*, domain::*},
};

impl AutomaticPreflightConsumptionPort for SqlitePreflightRepository {
    fn consume_or_recover(
        &self,
        witness_id: &str,
        proof_hash: &str,
        overlay_preview_hash: &str,
        now_ms: i64,
    ) -> Result<AutomaticPreflightWitnessV1, AutomaticApplicationError> {
        match self.consume(witness_id, proof_hash, overlay_preview_hash, now_ms) {
            Ok(value) => Ok(value),
            Err(PreflightRepositoryError::AlreadyConsumed) => self
                .recover_consumed(witness_id, proof_hash, overlay_preview_hash)
                .map_err(map_preflight),
            Err(error) => Err(map_preflight(error)),
        }
    }
}

impl SystemPolicyCuratorPort for SkillEvolutionCurationApi {
    fn apply(
        &self,
        request: SystemPolicyCuratorRequestV1,
    ) -> Result<SystemPolicyCuratorReceiptV1, AutomaticApplicationError> {
        let receipt = self
            .apply_system_policy(CuratorSystemPolicyApplyInput {
                candidate_id: request.candidate_id,
                expected_candidate_revision: request.expected_candidate_revision,
                preview_hash: request.preview_hash,
                effective_diff_hash: request.effective_diff_hash,
                idempotency_key: request.idempotency_key,
                run_id: request.run_id,
                eligibility_id: request.eligibility_id,
                eligibility_proof_hash: request.eligibility_proof_hash,
                preflight_witness_hash: request.preflight_witness_hash,
                policy_witness_hash: request.policy_witness_hash,
                rate_reservation_id: request.rate_reservation_id,
                authorized_at_ms: request.authorized_at_ms,
            })
            .map_err(|_| AutomaticApplicationError::CuratorUnavailable)?;
        Ok(SystemPolicyCuratorReceiptV1 {
            application_id: receipt.application_id,
            applied: receipt.applied,
            overlay_revision: receipt.overlay_revision,
            overlay_history_id: receipt.overlay_history_id,
            failure_code: receipt.failure_code,
        })
    }
}

impl AutomaticApplicationFinalizationPort for SqliteAutomaticApplicationRepository {
    fn finalize(
        &self,
        application: &AutomaticEvolutionApplicationV1,
        probation: &AutoApplyProbationV1,
        run_item_id: &str,
        expected_rate_revision: u64,
    ) -> Result<bool, AutomaticApplicationError> {
        SqliteAutomaticApplicationRepository::finalize(
            self,
            application,
            probation,
            run_item_id,
            expected_rate_revision,
        )
        .map_err(map_finalization)
    }
}

fn map_preflight(error: PreflightRepositoryError) -> AutomaticApplicationError {
    match error {
        PreflightRepositoryError::InvalidInput => AutomaticApplicationError::InvalidInput,
        PreflightRepositoryError::Storage => AutomaticApplicationError::Storage,
        PreflightRepositoryError::Conflict
        | PreflightRepositoryError::Expired
        | PreflightRepositoryError::AlreadyConsumed
        | PreflightRepositoryError::NotFound => AutomaticApplicationError::PreflightUnavailable,
    }
}

fn map_finalization(error: AutomaticApplicationStoreError) -> AutomaticApplicationError {
    match error {
        AutomaticApplicationStoreError::InvalidInput => AutomaticApplicationError::InvalidInput,
        AutomaticApplicationStoreError::Storage => AutomaticApplicationError::Storage,
        AutomaticApplicationStoreError::Conflict | AutomaticApplicationStoreError::NotFound => {
            AutomaticApplicationError::FinalizationConflict
        }
    }
}
