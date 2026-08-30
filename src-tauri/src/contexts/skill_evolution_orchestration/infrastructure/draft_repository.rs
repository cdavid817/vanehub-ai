use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    contexts::skill_evolution_orchestration::{
        application::{
            AutomaticCorrectionDraftRequestV1, AutomaticDraftPipelineError, AutomaticDraftStore,
        },
        domain::DeterministicCorrectionDraftV1,
    },
    platform::database::NativeDatabase,
};

#[derive(Clone)]
pub(crate) struct SqliteAutomaticDraftRepository {
    database: NativeDatabase,
}

impl SqliteAutomaticDraftRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }
}

impl AutomaticDraftStore for SqliteAutomaticDraftRepository {
    fn persist(
        &self,
        request: &AutomaticCorrectionDraftRequestV1,
        record: &DeterministicCorrectionDraftV1,
    ) -> Result<(), AutomaticDraftPipelineError> {
        let mut connection = self
            .database
            .connection()
            .map_err(|_| AutomaticDraftPipelineError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| AutomaticDraftPipelineError::Storage)?;
        let source_is_current = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM evolution_correction_authorizations authorization
                   JOIN evolution_assessment_attempts assessment
                     ON assessment.attempt_id=?2 AND assessment.status='completed'
                    AND assessment.is_current=1 AND assessment.route='advance'
                   JOIN evolution_assessment_targets target
                     ON target.attempt_id=assessment.attempt_id AND target.ordinal=0
                    AND target.skill_id=?3 AND target.revision_hash=?4
                   WHERE authorization.authorization_id=?1 AND authorization.authorized=1
                     AND authorization.revoked_at_ms IS NULL
                 )",
                params![
                    request.authorization_id,
                    request.assessment_id,
                    request.target_skill_id,
                    request.target_revision
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| AutomaticDraftPipelineError::Storage)?;
        if !source_is_current {
            return Err(AutomaticDraftPipelineError::SourceUnavailable);
        }
        let existing_hash = transaction
            .query_row(
                "SELECT content_hash FROM evolution_deterministic_drafts
                 WHERE authorization_id=?1 AND assessment_id=?2 AND producer_version=?3",
                params![
                    record.authorization_id,
                    record.assessment_id,
                    record.producer_version
                ],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| AutomaticDraftPipelineError::Storage)?;
        if let Some(existing_hash) = existing_hash {
            return if existing_hash == record.content_hash {
                Ok(())
            } else {
                Err(AutomaticDraftPipelineError::Storage)
            };
        }
        transaction
            .execute(
                "INSERT INTO evolution_deterministic_drafts
                 (draft_id,workspace_id,target_skill_id,authorization_id,assessment_id,
                  producer_version,content_hash,content_size_bytes,provenance,
                  source_witness_hash,created_at_ms)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
                params![
                    record.draft_id,
                    record.workspace_id,
                    record.target_skill_id,
                    record.authorization_id,
                    record.assessment_id,
                    record.producer_version,
                    record.content_hash,
                    record.content_size_bytes,
                    record.provenance,
                    record.source_witness_hash,
                    record.created_at_ms,
                ],
            )
            .map_err(|_| AutomaticDraftPipelineError::Storage)?;
        transaction
            .commit()
            .map_err(|_| AutomaticDraftPipelineError::Storage)
    }
}
