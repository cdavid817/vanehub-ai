use crate::contexts::skill_evolution_generation::domain::{
    FrozenGenerationInputV1, GenerationJobV1,
};
use rusqlite::{params, Connection, OptionalExtension};

use super::{
    canonical_hash, canonical_json, job_status_name, stage_name, GenerationPersistenceError,
    JobTransition, PersistGenerationOutcome,
};

pub(crate) struct GenerationJobRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationJobRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn persist_job(
        &self,
        job: &GenerationJobV1,
        input: &FrozenGenerationInputV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate_job(job, input)?;
        let input_json = canonical_json(input)?;
        let input_hash = canonical_hash(input)?;
        if input_hash != job.input_witness_hash {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let budget_json = canonical_json(&job.budget)?;
        let usage_json = canonical_json(&job.usage)?;
        let inserted = self.connection.execute(
            "INSERT INTO evolution_generation_jobs
             (job_id,schema_version,request_id,workspace_id,seed_id,seed_revision,
              assessment_attempt_id,assessment_revision,status,current_stage,input_witness_json,
              input_witness_hash,current_attempt,budget_json,usage_json,safe_failure_code,
              supersedes_job_id,revision,created_at_ms,updated_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,1,?18,?19)",
            params![
                job.job_id,
                job.schema_version,
                job.request_id,
                job.workspace_id,
                input.seed_id,
                input.seed_revision,
                input.assessment_attempt_id,
                input.assessment_revision,
                job_status_name(job.status),
                job.current_stage.map(stage_name),
                input_json,
                input_hash,
                job.current_attempt,
                budget_json,
                usage_json,
                job.safe_failure_code,
                job.supersedes_job_id,
                job.created_at_ms,
                job.updated_at_ms,
            ],
        );
        match inserted {
            Ok(_) => Ok(PersistGenerationOutcome::Inserted {
                id: job.job_id.clone(),
            }),
            Err(error) if is_constraint(&error) => self.coalesced_job(job, &input_hash),
            Err(_) => Err(GenerationPersistenceError::Storage),
        }
    }

    pub(crate) fn transition_job(
        &self,
        transition: &JobTransition<'_>,
    ) -> Result<u64, GenerationPersistenceError> {
        if transition.job_id.trim().is_empty()
            || transition.expected_revision == 0
            || transition.updated_at_ms < 0
            || transition.usage_json.len() > 4096
        {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        let expected_revision = i64::try_from(transition.expected_revision)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let changed = self.connection.execute(
            "UPDATE evolution_generation_jobs SET status=?1,current_stage=?2,usage_json=?3,
             safe_failure_code=?4,updated_at_ms=?5,revision=revision+1
             WHERE job_id=?6 AND revision=?7 AND status NOT IN ('completed','cancelled','failed','superseded')",
            params![
                job_status_name(transition.status),
                transition.current_stage.map(stage_name),
                transition.usage_json,
                transition.safe_failure_code,
                transition.updated_at_ms,
                transition.job_id,
                expected_revision,
            ],
        ).map_err(|_| GenerationPersistenceError::Storage)?;
        if changed == 1 {
            return Ok(transition.expected_revision + 1);
        }
        let current = self
            .connection
            .query_row(
                "SELECT revision,status FROM evolution_generation_jobs WHERE job_id=?1",
                [transition.job_id],
                |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        match current {
            Some((_, status)) if is_terminal(&status) => Err(GenerationPersistenceError::Immutable),
            Some(_) => Err(GenerationPersistenceError::Conflict),
            None => Err(GenerationPersistenceError::InvalidInput),
        }
    }

    fn coalesced_job(
        &self,
        job: &GenerationJobV1,
        input_hash: &str,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        let existing = self.connection.query_row(
            "SELECT job_id,input_witness_hash FROM evolution_generation_jobs WHERE request_id=?1",
            [&job.request_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        ).optional().map_err(|_| GenerationPersistenceError::Storage)?;
        match existing {
            Some((id, hash)) if hash == input_hash => {
                Ok(PersistGenerationOutcome::Coalesced { id })
            }
            Some(_) => Err(GenerationPersistenceError::Conflict),
            None => Err(GenerationPersistenceError::Conflict),
        }
    }
}

fn validate_job(
    job: &GenerationJobV1,
    input: &FrozenGenerationInputV1,
) -> Result<(), GenerationPersistenceError> {
    if job.job_id.trim().is_empty()
        || job.request_id.trim().is_empty()
        || job.request_id != input.request_id
        || input.seed_id.trim().is_empty()
        || input.assessment_attempt_id.trim().is_empty()
        || job.current_attempt == 0
        || job.created_at_ms < 0
        || job.updated_at_ms < job.created_at_ms
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn is_terminal(status: &str) -> bool {
    matches!(status, "completed" | "cancelled" | "failed" | "superseded")
}

fn is_constraint(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
}
