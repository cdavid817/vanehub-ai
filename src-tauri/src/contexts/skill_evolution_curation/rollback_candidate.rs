use super::{api::SkillEvolutionCurationApi, api_models::CuratorApiError};
use rusqlite::{params, OptionalExtension};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorRollbackCandidateInput {
    pub(crate) rollback_candidate_id: String,
    pub(crate) source_application_id: String,
    pub(crate) probation_id: String,
    pub(crate) workspace_id: String,
    pub(crate) skill_id: String,
    pub(crate) prior_effective_hash: String,
    pub(crate) current_effective_hash: String,
    pub(crate) observation_witness_hash: String,
    pub(crate) security_urgent: bool,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorRollbackCandidateReceipt {
    pub(crate) rollback_candidate_id: String,
    pub(crate) duplicate: bool,
}

impl SkillEvolutionCurationApi {
    pub(crate) fn enqueue_rollback_candidate(
        &self,
        input: &CuratorRollbackCandidateInput,
    ) -> Result<CuratorRollbackCandidateReceipt, CuratorApiError> {
        validate(input)?;
        let connection = self.connection()?;
        let changed = connection
            .execute(
                "INSERT OR IGNORE INTO evolution_curator_rollback_candidates
                 SELECT ?1,a.candidate_id,?2,?3,?4,?5,?6,?7,?8,?9,'pending',?10
                 FROM evolution_curator_applications a
                 WHERE a.application_id=?2 AND a.status IN ('applied','reconciled')",
                params![
                    input.rollback_candidate_id,
                    input.source_application_id,
                    input.probation_id,
                    input.workspace_id,
                    input.skill_id,
                    input.prior_effective_hash,
                    input.current_effective_hash,
                    input.observation_witness_hash,
                    if input.security_urgent {
                        "security"
                    } else {
                        "standard"
                    },
                    input.created_at_ms,
                ],
            )
            .map_err(|_| CuratorApiError::new("storage_unavailable"))?;
        if changed == 1 {
            return Ok(CuratorRollbackCandidateReceipt {
                rollback_candidate_id: input.rollback_candidate_id.clone(),
                duplicate: false,
            });
        }
        let existing = connection
            .query_row(
                "SELECT rollback_candidate_id,source_application_id,probation_id,workspace_id,
                 skill_id,prior_effective_hash,current_effective_hash,observation_witness_hash,
                 urgency,created_at_ms FROM evolution_curator_rollback_candidates
                 WHERE source_application_id=?1",
                [&input.source_application_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| CuratorApiError::new("storage_unavailable"))?
            .ok_or_else(|| CuratorApiError::new("not_found"))?;
        let expected_urgency = if input.security_urgent {
            "security"
        } else {
            "standard"
        };
        if existing
            != (
                input.rollback_candidate_id.clone(),
                input.source_application_id.clone(),
                input.probation_id.clone(),
                input.workspace_id.clone(),
                input.skill_id.clone(),
                input.prior_effective_hash.clone(),
                input.current_effective_hash.clone(),
                input.observation_witness_hash.clone(),
                expected_urgency.into(),
                input.created_at_ms,
            )
        {
            return Err(CuratorApiError::new("stale_conflict"));
        }
        Ok(CuratorRollbackCandidateReceipt {
            rollback_candidate_id: existing.0,
            duplicate: true,
        })
    }
}

fn validate(value: &CuratorRollbackCandidateInput) -> Result<(), CuratorApiError> {
    let fields = [
        value.rollback_candidate_id.as_str(),
        value.source_application_id.as_str(),
        value.probation_id.as_str(),
        value.workspace_id.as_str(),
        value.skill_id.as_str(),
        value.prior_effective_hash.as_str(),
        value.current_effective_hash.as_str(),
        value.observation_witness_hash.as_str(),
    ];
    if value.created_at_ms < 0
        || fields
            .iter()
            .any(|field| field.is_empty() || field.len() > 256)
    {
        return Err(CuratorApiError::new("invalid_input"));
    }
    Ok(())
}
