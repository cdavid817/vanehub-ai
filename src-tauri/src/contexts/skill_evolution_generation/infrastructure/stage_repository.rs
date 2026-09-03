use crate::contexts::skill_evolution_generation::domain::{
    GenerationStageAttemptV1, GenerationStageStatus,
};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    canonical_json, stage_name, GenerationPersistenceError, PersistGenerationOutcome,
    StageAttemptCompletion,
};

pub(crate) struct GenerationStageRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationStageRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn persist_attempt(
        &self,
        attempt: &GenerationStageAttemptV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate_attempt(attempt)?;
        let usage = canonical_json(&attempt.usage)?;
        let status = stage_status_name(attempt.status);
        let inserted = self.connection.execute(
            "INSERT INTO evolution_generation_stage_attempts
             (attempt_id,job_id,stage,attempt,status,input_hash,output_hash,usage_json,
              safe_failure_code,started_at_ms,completed_at_ms,superseded_by_attempt_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                attempt.attempt_id,
                attempt.job_id,
                stage_name(attempt.stage),
                attempt.attempt,
                status,
                attempt.input_hash,
                attempt.output_hash,
                usage,
                attempt.safe_failure_code,
                attempt.started_at_ms,
                attempt.completed_at_ms,
                attempt.superseded_by_attempt_id
            ],
        );
        match inserted {
            Ok(_) => Ok(PersistGenerationOutcome::Inserted {
                id: attempt.attempt_id.clone(),
            }),
            Err(error) if is_constraint(&error) => self.coalesced_attempt(attempt, status),
            Err(_) => Err(GenerationPersistenceError::Storage),
        }
    }

    pub(crate) fn complete_attempt(
        &self,
        completion: &StageAttemptCompletion<'_>,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate_completion(completion)?;
        let usage = canonical_json(completion.usage)?;
        let status = stage_status_name(completion.status);
        let changed = self.connection.execute(
            "UPDATE evolution_generation_stage_attempts SET status=?1,output_hash=?2,usage_json=?3,
             safe_failure_code=?4,completed_at_ms=?5,superseded_by_attempt_id=?6
             WHERE attempt_id=?7 AND status IN ('pending','running') AND input_hash=?8
             AND started_at_ms<=?5",
            params![status,completion.output_hash,usage,completion.safe_failure_code,
                completion.completed_at_ms,completion.superseded_by_attempt_id,
                completion.attempt_id,completion.expected_input_hash],
        ).map_err(|_| GenerationPersistenceError::Storage)?;
        if changed == 1 {
            return Ok(PersistGenerationOutcome::Inserted {
                id: completion.attempt_id.into(),
            });
        }
        let stored = self
            .connection
            .query_row(
                "SELECT status,input_hash,output_hash,usage_json,safe_failure_code,completed_at_ms,
             superseded_by_attempt_id FROM evolution_generation_stage_attempts WHERE attempt_id=?1",
                [completion.attempt_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, Option<String>>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, Option<String>>(4)?,
                        row.get::<_, Option<i64>>(5)?,
                        row.get::<_, Option<String>>(6)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        let expected = (
            status.into(),
            completion.expected_input_hash.into(),
            completion.output_hash.map(str::to_owned),
            usage,
            completion.safe_failure_code.map(str::to_owned),
            Some(completion.completed_at_ms),
            completion.superseded_by_attempt_id.map(str::to_owned),
        );
        match stored {
            Some(value) if value == expected => Ok(PersistGenerationOutcome::Coalesced {
                id: completion.attempt_id.into(),
            }),
            Some(value) if terminal_status(&value.0) => Err(GenerationPersistenceError::Immutable),
            Some(_) => Err(GenerationPersistenceError::Conflict),
            None => Err(GenerationPersistenceError::InvalidInput),
        }
    }

    fn coalesced_attempt(
        &self,
        attempt: &GenerationStageAttemptV1,
        status: &str,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        let stored = self.connection.query_row(
            "SELECT job_id,stage,attempt,status,input_hash,output_hash FROM evolution_generation_stage_attempts WHERE attempt_id=?1",
            [&attempt.attempt_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,
                row.get::<_,u16>(2)?,row.get::<_,String>(3)?,row.get::<_,String>(4)?,row.get::<_,Option<String>>(5)?)),
        ).optional().map_err(|_| GenerationPersistenceError::Storage)?;
        let expected = (
            attempt.job_id.clone(),
            stage_name(attempt.stage).into(),
            attempt.attempt,
            status.into(),
            attempt.input_hash.clone(),
            attempt.output_hash.clone(),
        );
        if stored == Some(expected) {
            Ok(PersistGenerationOutcome::Coalesced {
                id: attempt.attempt_id.clone(),
            })
        } else {
            Err(GenerationPersistenceError::Conflict)
        }
    }
}

fn validate_attempt(attempt: &GenerationStageAttemptV1) -> Result<(), GenerationPersistenceError> {
    if attempt.attempt_id.trim().is_empty()
        || attempt.job_id.trim().is_empty()
        || attempt.attempt == 0
        || attempt.input_hash.trim().is_empty()
        || attempt.started_at_ms < 0
        || attempt
            .completed_at_ms
            .is_some_and(|completed| completed < attempt.started_at_ms)
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn validate_completion(
    completion: &StageAttemptCompletion<'_>,
) -> Result<(), GenerationPersistenceError> {
    if completion.attempt_id.trim().is_empty()
        || completion.expected_input_hash.trim().is_empty()
        || completion.completed_at_ms < 0
        || !matches!(
            completion.status,
            GenerationStageStatus::Succeeded
                | GenerationStageStatus::Failed
                | GenerationStageStatus::Cancelled
                | GenerationStageStatus::Superseded
        )
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn stage_status_name(status: GenerationStageStatus) -> &'static str {
    match status {
        GenerationStageStatus::Pending => "pending",
        GenerationStageStatus::Running => "running",
        GenerationStageStatus::Succeeded => "succeeded",
        GenerationStageStatus::Failed => "failed",
        GenerationStageStatus::Cancelled => "cancelled",
        GenerationStageStatus::Superseded => "superseded",
    }
}

fn terminal_status(status: &str) -> bool {
    matches!(status, "succeeded" | "failed" | "cancelled" | "superseded")
}

fn is_constraint(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
}
