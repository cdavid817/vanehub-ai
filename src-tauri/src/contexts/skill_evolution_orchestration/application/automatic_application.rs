use crate::contexts::skill_evolution_orchestration::domain::{
    AutoApplyProbationV1, AutomaticEvolutionApplicationV1, AutomaticPreflightWitnessV1,
    EvolutionActorProvenance, ProbationStatus, ROLLING_WEEK_MS,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutomaticApplicationCommandV1 {
    pub(crate) preflight_witness_id: String,
    pub(crate) preflight_proof_hash: String,
    pub(crate) current_overlay_preview_hash: String,
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) effective_diff_hash: String,
    pub(crate) policy_witness_hash: String,
    pub(crate) run_item_id: String,
    pub(crate) expected_rate_revision: u64,
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) prior_effective_hash: String,
    pub(crate) resulting_effective_hash: String,
    pub(crate) evidence_fingerprint: String,
    pub(crate) evidence_categories: Vec<String>,
    pub(crate) baseline_witness_hash: String,
    pub(crate) now_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SystemPolicyCuratorRequestV1 {
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
pub(crate) struct SystemPolicyCuratorReceiptV1 {
    pub(crate) application_id: String,
    pub(crate) applied: bool,
    pub(crate) overlay_revision: Option<String>,
    pub(crate) overlay_history_id: Option<String>,
    pub(crate) failure_code: Option<String>,
}

pub(crate) trait AutomaticPreflightConsumptionPort {
    fn consume_or_recover(
        &self,
        witness_id: &str,
        proof_hash: &str,
        overlay_preview_hash: &str,
        now_ms: i64,
    ) -> Result<AutomaticPreflightWitnessV1, AutomaticApplicationError>;
}

pub(crate) trait SystemPolicyCuratorPort {
    fn apply(
        &self,
        request: SystemPolicyCuratorRequestV1,
    ) -> Result<SystemPolicyCuratorReceiptV1, AutomaticApplicationError>;
}

pub(crate) trait AutomaticApplicationFinalizationPort {
    fn finalize(
        &self,
        application: &AutomaticEvolutionApplicationV1,
        probation: &AutoApplyProbationV1,
        run_item_id: &str,
        expected_rate_revision: u64,
    ) -> Result<bool, AutomaticApplicationError>;
}

pub(crate) struct AutomaticApplicationCoordinator<'a, P, C, F> {
    preflights: &'a P,
    curator: &'a C,
    finalizer: &'a F,
}

impl<'a, P, C, F> AutomaticApplicationCoordinator<'a, P, C, F>
where
    P: AutomaticPreflightConsumptionPort,
    C: SystemPolicyCuratorPort,
    F: AutomaticApplicationFinalizationPort,
{
    pub(crate) fn new(preflights: &'a P, curator: &'a C, finalizer: &'a F) -> Self {
        Self {
            preflights,
            curator,
            finalizer,
        }
    }

    pub(crate) fn apply(
        &self,
        command: AutomaticApplicationCommandV1,
    ) -> Result<AutomaticEvolutionApplicationV1, AutomaticApplicationError> {
        let preflight = self.preflights.consume_or_recover(
            &command.preflight_witness_id,
            &command.preflight_proof_hash,
            &command.current_overlay_preview_hash,
            command.now_ms,
        )?;
        let curator = self.curator.apply(curator_request(&command, &preflight))?;
        if !curator.applied
            || curator.overlay_revision.is_none()
            || curator.overlay_history_id.is_none()
            || curator.failure_code.is_some()
        {
            return Err(AutomaticApplicationError::CuratorFailed);
        }
        let application = automatic_application(&command, &preflight, &curator);
        let probation = probation(&command, &application);
        self.finalizer.finalize(
            &application,
            &probation,
            &command.run_item_id,
            command.expected_rate_revision,
        )?;
        Ok(application)
    }
}

fn curator_request(
    command: &AutomaticApplicationCommandV1,
    preflight: &AutomaticPreflightWitnessV1,
) -> SystemPolicyCuratorRequestV1 {
    SystemPolicyCuratorRequestV1 {
        candidate_id: command.candidate_id.clone(),
        expected_candidate_revision: command.expected_candidate_revision,
        preview_hash: preflight.overlay_preview_hash.clone(),
        effective_diff_hash: command.effective_diff_hash.clone(),
        idempotency_key: preflight.reservation_id.clone(),
        run_id: preflight.run_id.clone(),
        eligibility_id: preflight.eligibility_id.clone(),
        eligibility_proof_hash: preflight.eligibility_proof_hash.clone(),
        preflight_witness_hash: preflight.proof_hash.clone(),
        policy_witness_hash: command.policy_witness_hash.clone(),
        rate_reservation_id: preflight.reservation_id.clone(),
        authorized_at_ms: command.now_ms,
    }
}

fn automatic_application(
    command: &AutomaticApplicationCommandV1,
    preflight: &AutomaticPreflightWitnessV1,
    receipt: &SystemPolicyCuratorReceiptV1,
) -> AutomaticEvolutionApplicationV1 {
    AutomaticEvolutionApplicationV1 {
        application_id: receipt.application_id.clone(),
        run_id: preflight.run_id.clone(),
        eligibility_id: preflight.eligibility_id.clone(),
        preflight_witness_hash: preflight.proof_hash.clone(),
        policy_witness_hash: command.policy_witness_hash.clone(),
        rate_reservation_id: preflight.reservation_id.clone(),
        curator_application_id: receipt.application_id.clone(),
        overlay_application_id: receipt.application_id.clone(),
        target_skill_id: command.target_skill_id.clone(),
        prior_effective_hash: command.prior_effective_hash.clone(),
        resulting_effective_hash: command.resulting_effective_hash.clone(),
        actor: EvolutionActorProvenance::SystemPolicy,
        committed_at_ms: command.now_ms,
    }
}

fn probation(
    command: &AutomaticApplicationCommandV1,
    application: &AutomaticEvolutionApplicationV1,
) -> AutoApplyProbationV1 {
    AutoApplyProbationV1 {
        probation_id: format!("probation-{}", application.application_id),
        application_id: application.application_id.clone(),
        workspace_id: command.workspace_id.clone(),
        skill_id: command.target_skill_id.clone(),
        status: ProbationStatus::Active,
        prior_effective_hash: command.prior_effective_hash.clone(),
        current_effective_hash: command.resulting_effective_hash.clone(),
        evidence_fingerprint: command.evidence_fingerprint.clone(),
        evidence_categories: command.evidence_categories.clone(),
        baseline_witness_hash: command.baseline_witness_hash.clone(),
        starts_at_ms: command.now_ms,
        ends_at_ms: command.now_ms.saturating_add(ROLLING_WEEK_MS),
        revision: 0,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutomaticApplicationError {
    InvalidInput,
    PreflightUnavailable,
    CuratorUnavailable,
    CuratorFailed,
    FinalizationConflict,
    Storage,
}
