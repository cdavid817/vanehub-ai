use crate::contexts::skill_evolution_generation::{
    application::PreparedQuarantinedSkillV1, domain::GenerationQuarantineStatus,
};
use rusqlite::{params, Connection, OptionalExtension};

use super::{sha256_bytes, GenerationPersistenceError, PersistGenerationOutcome};

pub(crate) struct GenerationQuarantineRepository<'connection> {
    connection: &'connection Connection,
}

impl<'connection> GenerationQuarantineRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn persist(
        &self,
        prepared: &PreparedQuarantinedSkillV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        validate(prepared)?;
        let revision = i64::try_from(prepared.proposal.revision)
            .map_err(|_| GenerationPersistenceError::InvalidInput)?;
        let result = self.connection.execute(
            "INSERT INTO evolution_generated_skill_quarantine
             (proposal_id,job_id,status,candidate_id,scope,workspace_id,rendered_skill_md,
              artifact_hash,catalog_witness_hash,revision,created_at_ms,updated_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?11)",
            params![
                prepared.proposal.proposal_id,
                prepared.proposal.job_id,
                status_name(prepared.proposal.status),
                prepared.proposal.candidate_id,
                prepared.proposal.scope,
                prepared.proposal.workspace_id,
                prepared.rendered_skill_md,
                prepared.proposal.artifact_hash,
                prepared.proposal.catalog_witness_hash,
                revision,
                prepared.created_at_ms,
            ],
        );
        match result {
            Ok(_) => Ok(PersistGenerationOutcome::Inserted {
                id: prepared.proposal.proposal_id.clone(),
            }),
            Err(error) if constraint(&error) => self.coalesce(prepared),
            Err(_) => Err(GenerationPersistenceError::Storage),
        }
    }

    pub(crate) fn rendered_skill_md(
        &self,
        proposal_id: &str,
    ) -> Result<Option<String>, GenerationPersistenceError> {
        if proposal_id.trim().is_empty() {
            return Err(GenerationPersistenceError::InvalidInput);
        }
        self.connection
            .query_row(
                "SELECT rendered_skill_md FROM evolution_generated_skill_quarantine WHERE proposal_id=?1",
                [proposal_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)
    }

    fn coalesce(
        &self,
        prepared: &PreparedQuarantinedSkillV1,
    ) -> Result<PersistGenerationOutcome, GenerationPersistenceError> {
        let stored = self
            .connection
            .query_row(
                "SELECT job_id,artifact_hash,catalog_witness_hash FROM evolution_generated_skill_quarantine WHERE proposal_id=?1",
                [&prepared.proposal.proposal_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?)),
            )
            .optional()
            .map_err(|_| GenerationPersistenceError::Storage)?;
        if stored
            == Some((
                prepared.proposal.job_id.clone(),
                prepared.proposal.artifact_hash.clone(),
                prepared.proposal.catalog_witness_hash.clone(),
            ))
        {
            Ok(PersistGenerationOutcome::Coalesced {
                id: prepared.proposal.proposal_id.clone(),
            })
        } else {
            Err(GenerationPersistenceError::Conflict)
        }
    }
}

fn validate(prepared: &PreparedQuarantinedSkillV1) -> Result<(), GenerationPersistenceError> {
    let scope_valid = match prepared.proposal.scope.as_str() {
        "user" => prepared.proposal.workspace_id.is_none(),
        "project" => prepared
            .proposal
            .workspace_id
            .as_ref()
            .is_some_and(|id| !id.trim().is_empty()),
        _ => false,
    };
    if prepared.proposal.status != GenerationQuarantineStatus::Quarantined
        || prepared.proposal.proposal_id.trim().is_empty()
        || prepared.proposal.job_id.trim().is_empty()
        || prepared.proposal.candidate_id.trim().is_empty()
        || prepared.proposal.revision == 0
        || prepared.proposal.catalog_witness_hash.trim().is_empty()
        || prepared.created_at_ms < 0
        || !scope_valid
        || sha256_bytes(prepared.rendered_skill_md.as_bytes()) != prepared.proposal.artifact_hash
    {
        return Err(GenerationPersistenceError::InvalidInput);
    }
    Ok(())
}

fn status_name(status: GenerationQuarantineStatus) -> &'static str {
    match status {
        GenerationQuarantineStatus::PendingValidation => "pending_validation",
        GenerationQuarantineStatus::Quarantined => "quarantined",
        GenerationQuarantineStatus::Reviewable => "reviewable",
        GenerationQuarantineStatus::Rejected => "rejected",
        GenerationQuarantineStatus::Applied => "applied",
        GenerationQuarantineStatus::Purged => "purged",
        GenerationQuarantineStatus::Superseded => "superseded",
    }
}

fn constraint(error: &rusqlite::Error) -> bool {
    error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation)
}
