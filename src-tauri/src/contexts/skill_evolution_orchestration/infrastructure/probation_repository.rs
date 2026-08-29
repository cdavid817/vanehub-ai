use super::probation_store_queries::*;
use super::probation_store_support::*;
use crate::{
    contexts::skill_evolution_orchestration::domain::{
        canonical_hash, evaluate_probation, is_safe_identifier, ProbationEvaluation,
        ProbationObservationV1, PROBATION_REGRESSION_POLICY_V1,
    },
    platform::database::NativeDatabase,
};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProbationRepositoryError {
    InvalidInput,
    Conflict,
    NotFound,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProbationRecordOutcome {
    pub(crate) evaluation: ProbationEvaluation,
    pub(crate) rollback_candidate_id: Option<String>,
    pub(crate) security_escalated: bool,
    pub(crate) duplicate: bool,
}

#[derive(Clone)]
pub(crate) struct SqliteProbationRepository {
    database: NativeDatabase,
}

impl SqliteProbationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn record_observation(
        &self,
        observation: &ProbationObservationV1,
        now_ms: i64,
    ) -> Result<ProbationRecordOutcome, ProbationRepositoryError> {
        validate_observation(observation, now_ms)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| ProbationRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProbationRepositoryError::Storage)?;
        let probation = load_probation(&transaction, &observation.probation_id)?
            .ok_or(ProbationRepositoryError::NotFound)?;
        if let Some(existing) = load_observation(&transaction, &observation.observation_id)? {
            if existing != *observation {
                return Err(ProbationRepositoryError::Conflict);
            }
            let outcome = current_outcome(&transaction, &probation, true)?;
            transaction
                .commit()
                .map_err(|_| ProbationRepositoryError::Storage)?;
            return Ok(outcome);
        }
        let mut observations = load_observations(&transaction, &probation.probation_id)?;
        observations.push(observation.clone());
        let evaluation = evaluate_probation(&probation, &observations, now_ms)
            .map_err(|_| ProbationRepositoryError::Conflict)?;
        insert_observation(&transaction, observation)?;
        let mut rollback_candidate_id = None;
        let security_escalated = evaluation == ProbationEvaluation::Regressed
            && observations.iter().any(is_security_regression);
        if evaluation == ProbationEvaluation::Regressed {
            let witness_hash = regression_witness(&probation.probation_id, &observations)?;
            let rollback_id = rollback_candidate_id_for(&probation.application_id)?;
            insert_rollback_candidate(
                &transaction,
                &probation,
                &rollback_id,
                &witness_hash,
                security_escalated,
                now_ms,
            )?;
            open_skill_suspension(&transaction, &probation, now_ms)?;
            if security_escalated {
                open_security_workspace_breaker(&transaction, &probation, now_ms)?;
            }
            rollback_candidate_id = Some(rollback_id);
        }
        transition_probation(&transaction, &probation, evaluation)?;
        transaction
            .commit()
            .map_err(|_| ProbationRepositoryError::Storage)?;
        Ok(ProbationRecordOutcome {
            evaluation,
            rollback_candidate_id,
            security_escalated,
            duplicate: false,
        })
    }

    pub(crate) fn evaluate_expired(
        &self,
        probation_id: &str,
        now_ms: i64,
    ) -> Result<ProbationRecordOutcome, ProbationRepositoryError> {
        if !is_safe_identifier(probation_id, 256) || now_ms < 0 {
            return Err(ProbationRepositoryError::InvalidInput);
        }
        let mut connection = self
            .database
            .connection()
            .map_err(|_| ProbationRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ProbationRepositoryError::Storage)?;
        let probation = load_probation(&transaction, probation_id)?
            .ok_or(ProbationRepositoryError::NotFound)?;
        let observations = load_observations(&transaction, probation_id)?;
        let evaluation = evaluate_probation(&probation, &observations, now_ms)
            .map_err(|_| ProbationRepositoryError::Conflict)?;
        if evaluation != ProbationEvaluation::Healthy {
            return Err(ProbationRepositoryError::Conflict);
        }
        transition_probation(&transaction, &probation, evaluation)?;
        transaction
            .commit()
            .map_err(|_| ProbationRepositoryError::Storage)?;
        Ok(ProbationRecordOutcome {
            evaluation,
            rollback_candidate_id: None,
            security_escalated: false,
            duplicate: false,
        })
    }
}

fn validate_observation(
    value: &ProbationObservationV1,
    now_ms: i64,
) -> Result<(), ProbationRepositoryError> {
    let identifiers = [
        value.observation_id.as_str(),
        value.probation_id.as_str(),
        value.source_kind.as_str(),
        value.source_id.as_str(),
        value.safe_category.as_str(),
    ];
    if now_ms < 0
        || value.observed_at_ms < 0
        || value.observed_at_ms > now_ms
        || identifiers
            .iter()
            .any(|value| !is_safe_identifier(value, 256))
        || value.witness_hash.is_empty()
        || value.witness_hash.len() > 256
    {
        return Err(ProbationRepositoryError::InvalidInput);
    }
    Ok(())
}

fn regression_witness(
    probation_id: &str,
    observations: &[ProbationObservationV1],
) -> Result<String, ProbationRepositoryError> {
    let mut witnesses = observations
        .iter()
        .filter(|value| {
            value.verified
                && value.negative
                && (value.baseline_exceeded || value.harmful_correction)
        })
        .map(|value| value.witness_hash.as_str())
        .collect::<Vec<_>>();
    witnesses.sort_unstable();
    witnesses.dedup();
    canonical_hash(&(PROBATION_REGRESSION_POLICY_V1, probation_id, witnesses))
        .map_err(|_| ProbationRepositoryError::InvalidInput)
}

fn rollback_candidate_id_for(application_id: &str) -> Result<String, ProbationRepositoryError> {
    let hash = canonical_hash(&("rollback-candidate-v1", application_id))
        .map_err(|_| ProbationRepositoryError::InvalidInput)?;
    Ok(format!("rollback-{}", hash.trim_start_matches("sha256:")))
}
