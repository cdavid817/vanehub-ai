use super::assessment_repository::target_universe_hash;
use crate::contexts::skill_evolution_assessment::domain::AssessmentWitness;
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SupersessionError {
    InvalidInput,
    NotFound,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WitnessRecheckOutcome {
    Current,
    Superseded {
        replacement_attempt_id: String,
        reason_code: String,
    },
}

pub(crate) struct WitnessRecheck<'a> {
    pub(crate) prior_attempt_id: &'a str,
    pub(crate) replacement_attempt_id: &'a str,
    pub(crate) original_witness: &'a AssessmentWitness,
    pub(crate) current_witness: &'a AssessmentWitness,
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) checked_at_ms: i64,
}

#[derive(Clone)]
pub(crate) struct SqliteSupersessionRepository {
    database: NativeDatabase,
}

impl SqliteSupersessionRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn recheck(
        &self,
        request: &WitnessRecheck<'_>,
    ) -> Result<WitnessRecheckOutcome, SupersessionError> {
        validate_request(request)?;
        let original_hash = request.original_witness.canonical_hash();
        let current_hash = request.current_witness.canonical_hash();
        if original_hash == current_hash {
            return Ok(WitnessRecheckOutcome::Current);
        }
        let reason_code = stale_reason(request.original_witness, request.current_witness);
        let connection = self
            .database
            .connection()
            .map_err(|_| SupersessionError::Storage)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| SupersessionError::Storage)?;
        if let Some(existing) = transaction
            .query_row(
                "SELECT replacement_attempt_id, reason_code FROM evolution_assessment_supersessions \
                 WHERE prior_attempt_id=?1",
                [request.prior_attempt_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| SupersessionError::Storage)?
        {
            return Ok(WitnessRecheckOutcome::Superseded {
                replacement_attempt_id: existing.0,
                reason_code: existing.1,
            });
        }
        let prior = transaction
            .query_row(
                "SELECT seed_id, witness_hash FROM evolution_assessment_attempts WHERE attempt_id=?1",
                [request.prior_attempt_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| SupersessionError::Storage)?
            .ok_or(SupersessionError::NotFound)?;
        if prior.0 != request.original_witness.input.seed_id || prior.1 != original_hash {
            return Err(SupersessionError::InvalidInput);
        }
        transaction
            .execute(
                "UPDATE evolution_assessment_attempts SET \
                 status=CASE WHEN status='completed' THEN status ELSE 'superseded' END, \
                 is_current=0, lease_owner=NULL, lease_expires_at_ms=NULL, heartbeat_at_ms=NULL \
                 WHERE attempt_id=?1",
                [request.prior_attempt_id],
            )
            .map_err(|_| SupersessionError::Storage)?;
        let replacement_attempt_id =
            insert_or_find_replacement(&transaction, request, &current_hash)?;
        transaction
            .execute(
                "INSERT INTO evolution_assessment_supersessions \
                 (prior_attempt_id,replacement_attempt_id,reason_code,changed_witness_hash,created_at_ms) \
                 VALUES (?1,?2,?3,?4,?5)",
                params![
                    request.prior_attempt_id,
                    replacement_attempt_id,
                    reason_code,
                    current_hash,
                    request.checked_at_ms,
                ],
            )
            .map_err(|_| SupersessionError::Storage)?;
        transaction
            .commit()
            .map_err(|_| SupersessionError::Storage)?;
        Ok(WitnessRecheckOutcome::Superseded {
            replacement_attempt_id,
            reason_code: reason_code.to_string(),
        })
    }
}

fn insert_or_find_replacement(
    transaction: &rusqlite::Transaction<'_>,
    request: &WitnessRecheck<'_>,
    current_hash: &str,
) -> Result<String, SupersessionError> {
    let witness = request.current_witness;
    let inserted = transaction.execute(
        "INSERT INTO evolution_assessment_attempts (attempt_id, seed_id, seed_revision, \
         witness_hash, status, seed_fingerprint, lineage_hash, target_universe_hash, \
         sanitizer_version, selector_policy_version, lexical_policy_version, gate_policy_version, \
         routing_policy_version, confidence_policy_version, evaluator_config_hash, consent_version, \
         model_evaluation_allowed, is_current, created_at_ms) \
         VALUES (?1,?2,?3,?4,'pending',?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,1,?17)",
        params![
            request.replacement_attempt_id,
            witness.input.seed_id,
            witness.input.seed_revision,
            current_hash,
            witness.input.seed_fingerprint,
            witness.input.lineage_hash,
            target_universe_hash(witness).map_err(|_| SupersessionError::InvalidInput)?,
            witness.input.sanitizer_version,
            witness.selector_policy_version,
            witness.lexical_policy_version,
            witness.gate_policy_version,
            witness.routing_policy_version,
            witness.confidence_policy_version,
            witness.evaluator_configuration,
            witness.consent_version,
            i64::from(request.model_evaluation_allowed),
            request.checked_at_ms,
        ],
    );
    match inserted {
        Ok(_) => Ok(request.replacement_attempt_id.to_string()),
        Err(error)
            if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
        {
            transaction
                .query_row(
                    "SELECT attempt_id FROM evolution_assessment_attempts \
                     WHERE seed_id=?1 AND seed_revision=?2 AND witness_hash=?3",
                    params![
                        witness.input.seed_id,
                        witness.input.seed_revision,
                        current_hash
                    ],
                    |row| row.get(0),
                )
                .map_err(|_| SupersessionError::Storage)
        }
        Err(_) => Err(SupersessionError::Storage),
    }
}

fn validate_request(request: &WitnessRecheck<'_>) -> Result<(), SupersessionError> {
    if request.prior_attempt_id.trim().is_empty()
        || request.replacement_attempt_id.trim().is_empty()
        || request.checked_at_ms < 0
        || request.original_witness.input.seed_id != request.current_witness.input.seed_id
    {
        return Err(SupersessionError::InvalidInput);
    }
    Ok(())
}

pub(super) fn stale_reason(
    original: &AssessmentWitness,
    current: &AssessmentWitness,
) -> &'static str {
    if original.input.seed_revision != current.input.seed_revision
        || original.input.seed_fingerprint != current.input.seed_fingerprint
        || original.input.lineage_hash != current.input.lineage_hash
    {
        "seed_witness_changed"
    } else if original.targets.iter().any(|target| {
        current.targets.iter().any(|candidate| {
            candidate.skill_id == target.skill_id
                && candidate.revision_hash == target.revision_hash
                && (candidate.lifecycle != target.lifecycle || candidate.trust != target.trust)
        })
    }) {
        "target_lifecycle_changed"
    } else if original.targets != current.targets {
        "target_revision_changed"
    } else if original.consent_version != current.consent_version {
        "consent_changed"
    } else if original.selector_policy_version != current.selector_policy_version
        || original.lexical_policy_version != current.lexical_policy_version
        || original.gate_policy_version != current.gate_policy_version
        || original.routing_policy_version != current.routing_policy_version
        || original.confidence_policy_version != current.confidence_policy_version
    {
        "policy_changed"
    } else if original.evaluator_configuration != current.evaluator_configuration {
        "evaluator_configuration_changed"
    } else {
        "assessment_witness_changed"
    }
}
