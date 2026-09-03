use crate::contexts::skill_evolution_generation::domain::{
    GenerationModelCallRecordV1, GenerationModelOutcome,
};
use rusqlite::{params, Connection, OptionalExtension};

use super::{GenerationPersistenceError, PersistGenerationOutcome};

pub(crate) struct GenerationModelCallRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationModelCallRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn persist(
        &self,
        record: &GenerationModelCallRecordV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate(record)?;
        let input_tokens = i64::try_from(record.input_tokens)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let output_tokens = i64::try_from(record.output_tokens)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let latency_ms = i64::try_from(record.latency_ms)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let inserted = self.connection.execute(
            "INSERT INTO evolution_generation_model_calls
             (model_call_id,stage_attempt_id,purpose,provider_protocol,provider_profile_id,model_id,
              prompt_template_version,response_schema_version,outcome,input_tokens,output_tokens,
              latency_ms,structured_response_hash,safe_failure_code,created_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                record.model_call_id,
                record.stage_attempt_id,
                record.purpose,
                record.provider_protocol,
                record.provider_profile_id,
                record.model_id,
                record.prompt_template_version,
                record.response_schema_version,
                outcome_name(record.outcome),
                input_tokens,
                output_tokens,
                latency_ms,
                record.structured_response_hash,
                record.safe_failure_code,
                record.created_at_ms
            ],
        );
        match inserted {
            Ok(_) => Ok(PersistGenerationOutcome::Inserted {
                id: record.model_call_id.clone(),
            }),
            Err(error) if constraint(&error) => self.coalesce(record),
            Err(_) => Err(GenerationPersistenceError::Storage),
        }
    }

    fn coalesce(
        &self,
        record: &GenerationModelCallRecordV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        let stored = self.connection.query_row(
            "SELECT stage_attempt_id,outcome,structured_response_hash FROM evolution_generation_model_calls WHERE model_call_id=?1",
            [&record.model_call_id], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,Option<String>>(2)?)),
        ).optional().map_err(|_| GenerationPersistenceError::Storage)?;
        if stored
            == Some((
                record.stage_attempt_id.clone(),
                outcome_name(record.outcome).into(),
                record.structured_response_hash.clone(),
            ))
        {
            Ok(PersistGenerationOutcome::Coalesced {
                id: record.model_call_id.clone(),
            })
        } else {
            Err(GenerationPersistenceError::Conflict)
        }
    }
}

fn validate(record: &GenerationModelCallRecordV1) -> Result<(), GenerationPersistenceError> {
    if record.model_call_id.trim().is_empty()
        || record.stage_attempt_id.trim().is_empty()
        || record.purpose != "skill_evolution_generation"
        || record.prompt_template_version.trim().is_empty()
        || record.response_schema_version.trim().is_empty()
        || record.created_at_ms < 0
        || (record.outcome == GenerationModelOutcome::Valid
            && record.structured_response_hash.is_none())
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn outcome_name(outcome: GenerationModelOutcome) -> &'static str {
    match outcome {
        GenerationModelOutcome::Valid => "valid",
        GenerationModelOutcome::ProviderUnavailable => "provider_unavailable",
        GenerationModelOutcome::Timeout => "timeout",
        GenerationModelOutcome::RateLimited => "rate_limited",
        GenerationModelOutcome::MalformedJson => "malformed_json",
        GenerationModelOutcome::InvalidSchema => "invalid_schema",
        GenerationModelOutcome::OversizedOutput => "oversized_output",
        GenerationModelOutcome::ConsentLost => "consent_lost",
        GenerationModelOutcome::ProviderFailure => "provider_failure",
    }
}

fn constraint(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
}
