use super::policy_retention_support::load_policy_from_connection;
use super::repository_support::{from_sql_u64, parse_state};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{Connection, OptionalExtension};

pub(super) fn load_decision_binding(
    connection: &Connection,
    candidate_id: &str,
) -> Result<CuratorDecisionBinding, CuratorDecisionStoreError> {
    let (revision, candidate_hash, policy_hash, state, staleness_json, preview_id, workspace_id) = connection
        .query_row(
            "SELECT revision,witness_hash,policy_witness_hash,state,staleness_json,current_preview_id,workspace_id
             FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, String>(6)?,
                ))
            },
        )
        .optional()
        .map_err(|_| CuratorDecisionStoreError::Storage)?
        .ok_or(CuratorDecisionStoreError::NotFound)?;
    let candidate_revision = from_sql_u64(revision).map_err(map_error)?;
    let maximum_defer_days = load_policy_from_connection(connection, &workspace_id)
        .map_err(|_| CuratorDecisionStoreError::Storage)?
        .maximum_defer_days;
    Ok(CuratorDecisionBinding {
        candidate_id: candidate_id.into(),
        candidate_revision,
        candidate_hash,
        policy_hash,
        maximum_defer_days,
        state: parse_state(&state).map_err(map_error)?,
        staleness: serde_json::from_str(&staleness_json)
            .map_err(|_| CuratorDecisionStoreError::Storage)?,
        ready_draft: load_ready_draft(connection, candidate_id)?,
        current_preview: preview_id
            .map(|preview_id| {
                load_preview(connection, candidate_id, candidate_revision, &preview_id)
            })
            .transpose()?,
    })
}

fn load_ready_draft(
    connection: &Connection,
    candidate_id: &str,
) -> Result<Option<CuratorReadyDraftWitness>, CuratorDecisionStoreError> {
    let row = connection
        .query_row(
            "SELECT d.revision,a.draft_assessment_id FROM evolution_curator_candidates c
             JOIN evolution_curator_drafts d ON d.draft_id=c.current_draft_id
             JOIN evolution_curator_draft_assessments a ON a.candidate_id=c.candidate_id
              AND a.draft_id=d.draft_id AND a.draft_revision=d.revision AND a.draft_hash=d.body_hash
             WHERE c.candidate_id=?1 AND a.approvable=1 AND a.invalidated_at_ms IS NULL
             ORDER BY a.created_at_ms DESC LIMIT 1",
            [candidate_id],
            |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(|_| CuratorDecisionStoreError::Storage)?;
    row.map(|(revision, assessment_id)| {
        Ok(CuratorReadyDraftWitness {
            draft_revision: from_sql_u64(revision).map_err(map_error)?,
            assessment_id,
        })
    })
    .transpose()
}

fn load_preview(
    connection: &Connection,
    candidate_id: &str,
    candidate_revision: u64,
    preview_id: &str,
) -> Result<CuratorApprovalPreviewWitness, CuratorDecisionStoreError> {
    let row = connection
        .query_row(
            "SELECT candidate_revision,witness_hash,effective_diff_hash,draft_revision,
                    draft_assessment_id,diff_projection_json,validation_json,issued_at_ms,expires_at_ms
             FROM evolution_curator_previews
             WHERE preview_id=?1 AND candidate_id=?2 AND invalidated_at_ms IS NULL",
            [preview_id, candidate_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|_| CuratorDecisionStoreError::Storage)?
        .ok_or(CuratorDecisionStoreError::Storage)?;
    if from_sql_u64(row.0).map_err(map_error)? != candidate_revision {
        return Err(CuratorDecisionStoreError::Storage);
    }
    let diffs: CuratorPreviewDiffs =
        serde_json::from_str(&row.5).map_err(|_| CuratorDecisionStoreError::Storage)?;
    let validation: CuratorPreviewValidation =
        serde_json::from_str(&row.6).map_err(|_| CuratorDecisionStoreError::Storage)?;
    Ok(CuratorApprovalPreviewWitness {
        preview_id: preview_id.into(),
        witness_hash: row.1,
        effective_diff_hash: row.2,
        draft_revision: from_sql_u64(row.3).map_err(map_error)?,
        assessment_id: row.4,
        issued_at_ms: row.7,
        expires_at_ms: row.8,
        diffs_complete: diffs.base_to_current.complete
            && diffs.current_to_proposed.complete
            && diffs.base_to_proposed.complete,
        validation_complete: validation.scan_passed
            && validation.can_commit
            && !validation.pinned
            && validation.trusted
            && validation.conflict_count == 0
            && validation.conflicts_complete
            && validation.rules_complete,
    })
}

fn map_error(error: super::CuratorRepositoryError) -> CuratorDecisionStoreError {
    match error {
        super::CuratorRepositoryError::NotFound => CuratorDecisionStoreError::NotFound,
        super::CuratorRepositoryError::Conflict(_) => CuratorDecisionStoreError::Conflict,
        super::CuratorRepositoryError::Storage => CuratorDecisionStoreError::Storage,
        _ => CuratorDecisionStoreError::InvalidInput,
    }
}
