use super::preview_binding_store::{load_binding_row, validate_current};
use super::repository_support::{from_sql_u64, parse_state, sql_u64, state_name};
use super::{append_system_event, SqliteCuratorRepository, SystemAuditEvent};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, Transaction, TransactionBehavior};

impl CuratorPreviewStore for SqliteCuratorRepository<'_> {
    fn preview_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorPreviewBinding, CuratorPreviewStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        let row = load_binding_row(&transaction, candidate_id)?;
        transaction
            .commit()
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        row.try_into_binding(candidate_id)
    }

    fn persist_preview(
        &mut self,
        preview: &CuratorPreview,
    ) -> Result<u64, CuratorPreviewStoreError> {
        validate_preview(preview)?;
        let witnesses = serde_json::to_string(&preview.witnesses)
            .map_err(|_| CuratorPreviewStoreError::InvalidInput)?;
        let diffs = serde_json::to_string(&preview.diffs)
            .map_err(|_| CuratorPreviewStoreError::InvalidInput)?;
        let validation = serde_json::to_string(&preview.validation)
            .map_err(|_| CuratorPreviewStoreError::InvalidInput)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        validate_current(&transaction, preview)?;
        transaction
            .execute(
                "UPDATE evolution_curator_previews SET invalidated_at_ms=?1
             WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
                params![preview.issued_at_ms, preview.candidate_id],
            )
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        transaction.execute(
            "INSERT INTO evolution_curator_previews
             (preview_id,candidate_id,candidate_revision,draft_id,draft_revision,draft_assessment_id,
              witness_hash,effective_diff_hash,witnesses_json,diff_projection_json,validation_json,
              issued_at_ms,expires_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)",
            params![preview.preview_id,preview.candidate_id,sql_u64(preview.candidate_revision).map_err(map_repository_error)?,
                preview.draft_id,sql_u64(preview.draft_revision).map_err(map_repository_error)?,preview.assessment_id,
                preview.witness_hash,preview.effective_diff_hash,witnesses,diffs,validation,
                preview.issued_at_ms,preview.expires_at_ms],
        ).map_err(|_| CuratorPreviewStoreError::Storage)?;
        let prior_revision = preview
            .candidate_revision
            .checked_sub(1)
            .ok_or(CuratorPreviewStoreError::InvalidInput)?;
        let updated = transaction.execute(
            "UPDATE evolution_curator_candidates SET current_preview_id=?1,staleness_json='[]',revision=?2,updated_at_ms=?3
             WHERE candidate_id=?4 AND revision=?5 AND state='ready_for_review'",
            params![preview.preview_id,sql_u64(preview.candidate_revision).map_err(map_repository_error)?,preview.issued_at_ms,
                preview.candidate_id,sql_u64(prior_revision).map_err(map_repository_error)?],
        ).map_err(|_| CuratorPreviewStoreError::Storage)?;
        if updated != 1 {
            return Err(CuratorPreviewStoreError::Conflict);
        }
        append_preview_event(&transaction, preview, prior_revision)?;
        transaction
            .commit()
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        Ok(preview.candidate_revision)
    }

    fn invalidate_preview(
        &mut self,
        invalidation: &CuratorPreviewInvalidation,
    ) -> Result<u64, CuratorPreviewStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        let (state, revision, staleness_json) = transaction
            .query_row(
                "SELECT state,revision,staleness_json FROM evolution_curator_candidates WHERE candidate_id=?1",
                [&invalidation.candidate_id],
                |row| Ok((row.get::<_,String>(0)?,row.get::<_,i64>(1)?,row.get::<_,String>(2)?)),
            )
            .optional().map_err(|_| CuratorPreviewStoreError::Storage)?
            .ok_or(CuratorPreviewStoreError::NotFound)?;
        let current_revision = from_sql_u64(revision).map_err(map_repository_error)?;
        if current_revision != invalidation.expected_candidate_revision {
            return Err(CuratorPreviewStoreError::Conflict);
        }
        let current_state = parse_state(&state).map_err(map_repository_error)?;
        let next_revision = current_revision
            .checked_add(1)
            .ok_or(CuratorPreviewStoreError::InvalidInput)?;
        let staleness = add_staleness(&staleness_json, invalidation.reason)?;
        transaction.execute(
            "UPDATE evolution_curator_previews SET invalidated_at_ms=?1 WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
            params![invalidation.occurred_at_ms,invalidation.candidate_id],
        ).map_err(|_| CuratorPreviewStoreError::Storage)?;
        let updated = transaction.execute(
            "UPDATE evolution_curator_candidates SET current_preview_id=NULL,staleness_json=?1,revision=?2,updated_at_ms=?3
             WHERE candidate_id=?4 AND revision=?5",
            params![staleness,sql_u64(next_revision).map_err(map_repository_error)?,invalidation.occurred_at_ms,
                invalidation.candidate_id,revision],
        ).map_err(|_| CuratorPreviewStoreError::Storage)?;
        if updated != 1 {
            return Err(CuratorPreviewStoreError::Conflict);
        }
        append_system_event(
            &transaction,
            &SystemAuditEvent {
                candidate_id: &invalidation.candidate_id,
                event_kind: CuratorEventKind::PreviewInvalidated,
                occurred_at_ms: invalidation.occurred_at_ms,
                prior_state: Some(current_state),
                next_state: current_state,
                object_revision: next_revision,
                reason_code: staleness_reason(invalidation.reason),
            },
        )
        .map_err(map_repository_error)?;
        transaction
            .commit()
            .map_err(|_| CuratorPreviewStoreError::Storage)?;
        Ok(next_revision)
    }
}

fn validate_preview(preview: &CuratorPreview) -> Result<(), CuratorPreviewStoreError> {
    if preview.candidate_revision < 2
        || preview.issued_at_ms < 0
        || preview.expires_at_ms.checked_sub(preview.issued_at_ms) != Some(CURATOR_PREVIEW_TTL_MS)
        || preview.invalidated_at_ms.is_some()
        || preview.preview_id.trim().is_empty()
        || preview.witness_hash.trim().is_empty()
        || preview.effective_diff_hash.trim().is_empty()
    {
        return Err(CuratorPreviewStoreError::InvalidInput);
    }
    Ok(())
}

fn append_preview_event(
    transaction: &Transaction<'_>,
    preview: &CuratorPreview,
    prior_revision: u64,
) -> Result<(), CuratorPreviewStoreError> {
    append_system_event(
        transaction,
        &SystemAuditEvent {
            candidate_id: &preview.candidate_id,
            event_kind: CuratorEventKind::Previewed,
            occurred_at_ms: preview.issued_at_ms,
            prior_state: Some(CuratorCandidateState::ReadyForReview),
            next_state: CuratorCandidateState::ReadyForReview,
            object_revision: preview.candidate_revision,
            reason_code: "overlay_preview_issued",
        },
    )
    .map_err(map_repository_error)?;
    debug_assert_eq!(preview.candidate_revision, prior_revision + 1);
    Ok(())
}

fn add_staleness(
    json: &str,
    reason: CuratorStalenessReason,
) -> Result<String, CuratorPreviewStoreError> {
    let mut values: Vec<CuratorStalenessReason> =
        serde_json::from_str(json).map_err(|_| CuratorPreviewStoreError::Storage)?;
    if !values.contains(&reason) {
        values.push(reason);
    }
    serde_json::to_string(&values).map_err(|_| CuratorPreviewStoreError::Storage)
}

fn staleness_reason(reason: CuratorStalenessReason) -> &'static str {
    match reason {
        CuratorStalenessReason::BaseChanged => "base_changed",
        CuratorStalenessReason::OverlayChanged => "overlay_changed",
        CuratorStalenessReason::PinChanged => "pin_changed",
        CuratorStalenessReason::TrustChanged => "trust_changed",
        CuratorStalenessReason::ConflictChanged => "conflict_changed",
        CuratorStalenessReason::PolicyChanged => "policy_changed",
        CuratorStalenessReason::PreviewExpired => "preview_expired",
        CuratorStalenessReason::AssessmentChanged => "assessment_changed",
        CuratorStalenessReason::TargetChanged => "target_changed",
        CuratorStalenessReason::EvidencePurged => "evidence_purged",
        CuratorStalenessReason::DraftChanged => "draft_changed",
    }
}

fn map_repository_error(error: super::CuratorRepositoryError) -> CuratorPreviewStoreError {
    match error {
        super::CuratorRepositoryError::NotFound => CuratorPreviewStoreError::NotFound,
        super::CuratorRepositoryError::Conflict(_) => CuratorPreviewStoreError::Conflict,
        super::CuratorRepositoryError::Storage => CuratorPreviewStoreError::Storage,
        _ => CuratorPreviewStoreError::InvalidInput,
    }
}
