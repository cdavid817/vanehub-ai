use super::{api::SkillEvolutionCurationApi, api_models::CuratorApiError};
use crate::contexts::skill_evolution_curation::{
    application::CuratorApplicationService,
    domain::{
        CuratorApplicationOutcome, CuratorSystemPolicyApplicationRequest,
        CuratorSystemPolicyAuthorizationV1, CuratorTrustedActor,
    },
    infrastructure::{SkillApiCuratorApplication, SqliteCuratorRepository},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorSystemPolicyApplyInput {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) preview_hash: String,
    pub(crate) effective_diff_hash: String,
    pub(crate) idempotency_key: String,
    pub(crate) run_id: String,
    pub(crate) eligibility_id: String,
    pub(crate) eligibility_proof_hash: String,
    pub(crate) preflight_witness_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) rate_reservation_id: String,
    pub(crate) authorized_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorSystemPolicyApplyReceipt {
    pub(crate) application_id: String,
    pub(crate) applied: bool,
    pub(crate) overlay_revision: Option<String>,
    pub(crate) overlay_history_id: Option<String>,
    pub(crate) failure_code: Option<String>,
}

impl SkillEvolutionCurationApi {
    pub(crate) fn apply_system_policy(
        &self,
        input: CuratorSystemPolicyApplyInput,
    ) -> Result<CuratorSystemPolicyApplyReceipt, CuratorApiError> {
        let workspace = self.workspace(&input.candidate_id)?;
        let mut connection = self.connection()?;
        let mut repository = SqliteCuratorRepository::new(&mut connection);
        let overlay = SkillApiCuratorApplication::new(&self.skills, Some(&workspace));
        let authorization = authorization(&input);
        let outcome = CuratorApplicationService::new(
            &mut repository,
            &overlay,
            CuratorTrustedActor::system(input.authorized_at_ms),
        )
        .apply_system_policy(CuratorSystemPolicyApplicationRequest {
            candidate_id: &input.candidate_id,
            expected_candidate_revision: input.expected_candidate_revision,
            preview_hash: &input.preview_hash,
            effective_diff_hash: &input.effective_diff_hash,
            idempotency_key: &input.idempotency_key,
            authorization: &authorization,
        })
        .map_err(|error| self.action_error(&input.candidate_id, error.to_string()))?;
        let (application, applied) = match outcome {
            CuratorApplicationOutcome::Applied(value) => (value, true),
            CuratorApplicationOutcome::Failed(value) => (value, false),
        };
        Ok(CuratorSystemPolicyApplyReceipt {
            application_id: application.application_id,
            applied,
            overlay_revision: application.overlay_revision,
            overlay_history_id: application.overlay_history_id,
            failure_code: application.failure_code,
        })
    }
}

fn authorization(input: &CuratorSystemPolicyApplyInput) -> CuratorSystemPolicyAuthorizationV1 {
    CuratorSystemPolicyAuthorizationV1 {
        run_id: input.run_id.clone(),
        eligibility_id: input.eligibility_id.clone(),
        eligibility_proof_hash: input.eligibility_proof_hash.clone(),
        preflight_witness_hash: input.preflight_witness_hash.clone(),
        policy_witness_hash: input.policy_witness_hash.clone(),
        rate_reservation_id: input.rate_reservation_id.clone(),
        authorized_at_ms: input.authorized_at_ms,
    }
}
