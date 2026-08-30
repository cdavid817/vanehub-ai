use crate::contexts::skill_evolution_assessment::application::{
    AssessmentQueueError, AssessmentQueueLane, AssessmentQueuePersistence, AssessmentQueueRequest,
    QueueEnqueueOutcome,
};
use crate::platform::database::NativeDatabase;
use rusqlite::{params, OptionalExtension, TransactionBehavior};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AssessmentQueueLease {
    pub(crate) queue_id: String,
    pub(crate) seed_id: String,
    pub(crate) witness_hash: String,
    pub(crate) lane: AssessmentQueueLane,
    pub(crate) attempt_count: u32,
    pub(crate) owner: String,
    pub(crate) expires_at_ms: i64,
}

#[derive(Clone)]
pub(crate) struct SqliteAssessmentQueueRepository {
    database: NativeDatabase,
    capacity: usize,
}

impl AssessmentQueuePersistence for SqliteAssessmentQueueRepository {
    fn enqueue(
        &self,
        request: &AssessmentQueueRequest,
    ) -> Result<QueueEnqueueOutcome, AssessmentQueueError> {
        Self::enqueue(self, request)
    }
}

impl SqliteAssessmentQueueRepository {
    pub(crate) fn new(
        database: NativeDatabase,
        capacity: usize,
    ) -> Result<Self, AssessmentQueueError> {
        if capacity == 0 || capacity > 10_000 {
            return Err(AssessmentQueueError::InvalidInput);
        }
        Ok(Self { database, capacity })
    }

    pub(crate) fn enqueue(
        &self,
        request: &AssessmentQueueRequest,
    ) -> Result<QueueEnqueueOutcome, AssessmentQueueError> {
        validate_request(request)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| AssessmentQueueError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;
        if let Some(queue_id) = transaction
            .query_row(
                "SELECT queue_id FROM evolution_assessment_queue_state \
                 WHERE seed_id=?1 AND witness_hash=?2 AND lane=?3",
                params![request.seed_id, request.witness_hash, request.lane.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| AssessmentQueueError::Storage)?
        {
            return Ok(QueueEnqueueOutcome::Coalesced { queue_id });
        }
        let active: i64 = transaction
            .query_row(
                "SELECT COUNT(*) FROM evolution_assessment_queue_state \
                 WHERE status IN ('queued','leased')",
                [],
                |row| row.get(0),
            )
            .map_err(|_| AssessmentQueueError::Storage)?;
        if active >= self.capacity as i64 {
            if request.lane == AssessmentQueueLane::OptionalModel {
                return Ok(QueueEnqueueOutcome::OptionalFallback);
            }
            let displaced = transaction
                .execute(
                    "UPDATE evolution_assessment_queue_state SET status='fallback', updated_at_ms=?1 \
                     WHERE queue_id=(SELECT queue_id FROM evolution_assessment_queue_state \
                     WHERE status='queued' AND lane='optional_model' \
                     ORDER BY priority ASC, created_at_ms DESC, queue_id DESC LIMIT 1)",
                    [request.created_at_ms],
                )
                .map_err(|_| AssessmentQueueError::Storage)?;
            if displaced == 0 {
                return Ok(QueueEnqueueOutcome::Saturated);
            }
        }
        let inserted = transaction.execute(
            "INSERT INTO evolution_assessment_queue_state \
             (queue_id,seed_id,witness_hash,lane,status,priority,attempt_count,available_at_ms,created_at_ms,updated_at_ms) \
             VALUES (?1,?2,?3,?4,'queued',?5,0,?6,?7,?7)",
            params![request.queue_id, request.seed_id, request.witness_hash, request.lane.as_str(), request.priority, request.available_at_ms, request.created_at_ms],
        );
        match inserted {
            Ok(_) => {
                transaction
                    .commit()
                    .map_err(|_| AssessmentQueueError::Storage)?;
                Ok(QueueEnqueueOutcome::Scheduled {
                    queue_id: request.queue_id.clone(),
                })
            }
            Err(error)
                if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) =>
            {
                let seed_exists: i64 = transaction
                    .query_row(
                        "SELECT EXISTS(SELECT 1 FROM evolution_candidate_seeds WHERE seed_id=?1)",
                        [&request.seed_id],
                        |row| row.get(0),
                    )
                    .map_err(|_| AssessmentQueueError::Storage)?;
                if seed_exists == 0 {
                    Err(AssessmentQueueError::LineageUnavailable)
                } else {
                    Err(AssessmentQueueError::Storage)
                }
            }
            Err(_) => Err(AssessmentQueueError::Storage),
        }
    }

    pub(crate) fn claim_next(
        &self,
        owner: &str,
        now_ms: i64,
        lease_duration_ms: i64,
    ) -> Result<Option<AssessmentQueueLease>, AssessmentQueueError> {
        validate_lease(owner, now_ms, lease_duration_ms)?;
        let expires_at_ms = now_ms
            .checked_add(lease_duration_ms)
            .ok_or(AssessmentQueueError::InvalidInput)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| AssessmentQueueError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(map_sqlite_error)?;
        transaction.execute(
            "UPDATE evolution_assessment_queue_state SET status='queued', lease_owner=NULL, \
             lease_expires_at_ms=NULL, updated_at_ms=?1 WHERE status='leased' AND lease_expires_at_ms <= ?1",
            [now_ms],
        ).map_err(|_| AssessmentQueueError::Storage)?;
        let candidate = transaction.query_row(
            "SELECT queue_id,seed_id,witness_hash,lane,attempt_count FROM evolution_assessment_queue_state \
             WHERE status='queued' AND available_at_ms <= ?1 \
             ORDER BY CASE lane WHEN 'deterministic' THEN 0 ELSE 1 END, priority DESC, created_at_ms, queue_id LIMIT 1",
            [now_ms],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, i64>(4)?)),
        ).optional().map_err(|_| AssessmentQueueError::Storage)?;
        let Some((queue_id, seed_id, witness_hash, lane, attempt_count)) = candidate else {
            transaction
                .commit()
                .map_err(|_| AssessmentQueueError::Storage)?;
            return Ok(None);
        };
        let updated = transaction.execute(
            "UPDATE evolution_assessment_queue_state SET status='leased',attempt_count=attempt_count+1, \
             lease_owner=?1,lease_expires_at_ms=?2,updated_at_ms=?3 WHERE queue_id=?4 AND status='queued'",
            params![owner, expires_at_ms, now_ms, queue_id],
        ).map_err(|_| AssessmentQueueError::Storage)?;
        if updated != 1 {
            return Err(AssessmentQueueError::LeaseUnavailable);
        }
        transaction
            .commit()
            .map_err(|_| AssessmentQueueError::Storage)?;
        Ok(Some(AssessmentQueueLease {
            queue_id,
            seed_id,
            witness_hash,
            lane: if lane == "deterministic" {
                AssessmentQueueLane::Deterministic
            } else {
                AssessmentQueueLane::OptionalModel
            },
            attempt_count: u32::try_from(attempt_count + 1)
                .map_err(|_| AssessmentQueueError::Storage)?,
            owner: owner.to_string(),
            expires_at_ms,
        }))
    }
}

fn validate_request(request: &AssessmentQueueRequest) -> Result<(), AssessmentQueueError> {
    if request.queue_id.trim().is_empty()
        || request.seed_id.trim().is_empty()
        || request.witness_hash.trim().is_empty()
        || request.available_at_ms < 0
        || request.created_at_ms < 0
    {
        return Err(AssessmentQueueError::InvalidInput);
    }
    Ok(())
}

fn validate_lease(owner: &str, now_ms: i64, duration_ms: i64) -> Result<(), AssessmentQueueError> {
    if owner.trim().is_empty()
        || owner.len() > 128
        || now_ms < 0
        || !(1..=300_000).contains(&duration_ms)
    {
        return Err(AssessmentQueueError::InvalidInput);
    }
    Ok(())
}

fn map_sqlite_error(error: rusqlite::Error) -> AssessmentQueueError {
    match error.sqlite_error_code() {
        Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked) => {
            AssessmentQueueError::DatabaseLock
        }
        _ => AssessmentQueueError::Storage,
    }
}
