use super::assessment_repository::target_universe_hash;
use crate::contexts::skill_evolution_assessment::domain::AssessmentWitness;
use crate::platform::database::NativeDatabase;
use rusqlite::params;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AttemptLeaseError {
    InvalidInput,
    LineageUnavailable,
    NotFound,
    LeaseUnavailable,
    Immutable,
    Storage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttemptLease {
    pub(crate) attempt_id: String,
    pub(crate) owner: String,
    pub(crate) expires_at_ms: i64,
}

pub(crate) struct PendingAssessmentAttempt<'a> {
    pub(crate) attempt_id: &'a str,
    pub(crate) witness: &'a AssessmentWitness,
    pub(crate) model_evaluation_allowed: bool,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PendingAttemptOutcome {
    Created { attempt_id: String },
    Coalesced { attempt_id: String },
}

#[derive(Clone)]
pub(crate) struct SqliteAttemptLeaseRepository {
    database: NativeDatabase,
}

impl SqliteAttemptLeaseRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn create_pending(
        &self,
        request: &PendingAssessmentAttempt<'_>,
    ) -> Result<PendingAttemptOutcome, AttemptLeaseError> {
        if request.attempt_id.trim().is_empty()
            || request.witness.input.seed_id.trim().is_empty()
            || request.created_at_ms < 0
        {
            return Err(AttemptLeaseError::InvalidInput);
        }
        let witness_hash = request.witness.canonical_hash();
        let connection = self
            .database
            .connection()
            .map_err(|_| AttemptLeaseError::Storage)?;
        let inserted = connection.execute(
            "INSERT INTO evolution_assessment_attempts (attempt_id, seed_id, seed_revision, \
             witness_hash, status, seed_fingerprint, lineage_hash, target_universe_hash, \
             sanitizer_version, selector_policy_version, lexical_policy_version, \
             gate_policy_version, routing_policy_version, confidence_policy_version, \
             evaluator_config_hash, consent_version, model_evaluation_allowed, is_current, \
             created_at_ms) VALUES (?1,?2,?3,?4,'pending',?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,1,?17)",
            params![
                request.attempt_id,
                request.witness.input.seed_id,
                request.witness.input.seed_revision,
                witness_hash,
                request.witness.input.seed_fingerprint,
                request.witness.input.lineage_hash,
                target_universe_hash(request.witness)
                    .map_err(|_| AttemptLeaseError::InvalidInput)?,
                request.witness.input.sanitizer_version,
                request.witness.selector_policy_version,
                request.witness.lexical_policy_version,
                request.witness.gate_policy_version,
                request.witness.routing_policy_version,
                request.witness.confidence_policy_version,
                request.witness.evaluator_configuration,
                request.witness.consent_version,
                i64::from(request.model_evaluation_allowed),
                request.created_at_ms,
            ],
        );
        match inserted {
            Ok(_) => Ok(PendingAttemptOutcome::Created {
                attempt_id: request.attempt_id.to_string(),
            }),
            Err(error)
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                let attempt_id = connection.query_row(
                    "SELECT attempt_id FROM evolution_assessment_attempts \
                         WHERE seed_id=?1 AND seed_revision=?2 AND witness_hash=?3",
                    params![
                        request.witness.input.seed_id,
                        request.witness.input.seed_revision,
                        witness_hash,
                    ],
                    |row| row.get(0),
                );
                match attempt_id {
                    Ok(attempt_id) => Ok(PendingAttemptOutcome::Coalesced { attempt_id }),
                    Err(rusqlite::Error::QueryReturnedNoRows) => {
                        let seed_exists: i64 = connection
                            .query_row(
                                "SELECT EXISTS(SELECT 1 FROM evolution_candidate_seeds WHERE seed_id=?1)",
                                [&request.witness.input.seed_id],
                                |row| row.get(0),
                            )
                            .map_err(|_| AttemptLeaseError::Storage)?;
                        if seed_exists == 0 {
                            Err(AttemptLeaseError::LineageUnavailable)
                        } else {
                            Err(AttemptLeaseError::Storage)
                        }
                    }
                    Err(_) => Err(AttemptLeaseError::Storage),
                }
            }
            Err(_) => Err(AttemptLeaseError::Storage),
        }
    }

    pub(crate) fn claim(
        &self,
        attempt_id: &str,
        owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
    ) -> Result<AttemptLease, AttemptLeaseError> {
        validate_lease_input(attempt_id, owner, now_ms, lease_duration_ms)?;
        let expires_at_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or(AttemptLeaseError::InvalidInput)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| AttemptLeaseError::Storage)?;
        let updated = connection
            .execute(
                "UPDATE evolution_assessment_attempts SET status='running', lease_owner=?1, \
                 lease_expires_at_ms=?2, heartbeat_at_ms=?3 WHERE attempt_id=?4 AND \
                 (status='pending' OR (status='running' AND lease_expires_at_ms <= ?3))",
                params![owner, expires_at_ms, now_ms, attempt_id],
            )
            .map_err(|_| AttemptLeaseError::Storage)?;
        if updated == 1 {
            return Ok(AttemptLease {
                attempt_id: attempt_id.to_string(),
                owner: owner.to_string(),
                expires_at_ms,
            });
        }
        Err(classify_unavailable(&connection, attempt_id)?)
    }

    pub(crate) fn heartbeat(
        &self,
        attempt_id: &str,
        owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
    ) -> Result<AttemptLease, AttemptLeaseError> {
        validate_lease_input(attempt_id, owner, now_ms, lease_duration_ms)?;
        let expires_at_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or(AttemptLeaseError::InvalidInput)?;
        let connection = self
            .database
            .connection()
            .map_err(|_| AttemptLeaseError::Storage)?;
        let updated = connection
            .execute(
                "UPDATE evolution_assessment_attempts SET lease_expires_at_ms=?1, \
                 heartbeat_at_ms=?2 WHERE attempt_id=?3 AND status='running' \
                 AND lease_owner=?4 AND lease_expires_at_ms > ?2",
                params![expires_at_ms, now_ms, attempt_id, owner],
            )
            .map_err(|_| AttemptLeaseError::Storage)?;
        if updated == 1 {
            Ok(AttemptLease {
                attempt_id: attempt_id.to_string(),
                owner: owner.to_string(),
                expires_at_ms,
            })
        } else {
            Err(classify_unavailable(&connection, attempt_id)?)
        }
    }

    pub(crate) fn recover_expired(&self, now_ms: i64) -> Result<usize, AttemptLeaseError> {
        if now_ms < 0 {
            return Err(AttemptLeaseError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| AttemptLeaseError::Storage)?;
        connection
            .execute(
                "UPDATE evolution_assessment_attempts SET status='pending', lease_owner=NULL, \
                 lease_expires_at_ms=NULL, heartbeat_at_ms=NULL WHERE status='running' \
                 AND lease_expires_at_ms <= ?1",
                [now_ms],
            )
            .map_err(|_| AttemptLeaseError::Storage)
    }
}

fn validate_lease_input(
    attempt_id: &str,
    owner: &str,
    now_ms: i64,
    lease_duration_ms: i64,
) -> Result<(), AttemptLeaseError> {
    if attempt_id.trim().is_empty()
        || owner.trim().is_empty()
        || owner.len() > 128
        || now_ms < 0
        || !(1..=300_000).contains(&lease_duration_ms)
    {
        return Err(AttemptLeaseError::InvalidInput);
    }
    Ok(())
}

fn classify_unavailable(
    connection: &rusqlite::Connection,
    attempt_id: &str,
) -> Result<AttemptLeaseError, AttemptLeaseError> {
    let status = connection.query_row(
        "SELECT status FROM evolution_assessment_attempts WHERE attempt_id=?1",
        [attempt_id],
        |row| row.get::<_, String>(0),
    );
    match status {
        Ok(status) if matches!(status.as_str(), "completed" | "superseded" | "failed") => {
            Ok(AttemptLeaseError::Immutable)
        }
        Ok(_) => Ok(AttemptLeaseError::LeaseUnavailable),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(AttemptLeaseError::NotFound),
        Err(_) => Err(AttemptLeaseError::Storage),
    }
}
