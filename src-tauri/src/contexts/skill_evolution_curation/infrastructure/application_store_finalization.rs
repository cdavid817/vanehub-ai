use super::application_binding_store::load_application;
use super::repository_support::{from_sql_u64, sql_u64, state_name};
use super::{append_system_event, SystemAuditEvent};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, TransactionBehavior};

pub(super) fn finalize(
    connection: &mut rusqlite::Connection,
    application_id: &str,
    expected_application_revision: u64,
    result: Result<&CuratorOverlayApplicationReceipt, CuratorApplicationFailure>,
    occurred_at_ms: i64,
) -> Result<CuratorApplication, CuratorApplicationStoreError> {
    if application_id.trim().is_empty() || occurred_at_ms < 0 {
        return Err(CuratorApplicationStoreError::InvalidInput);
    }
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    let current = load_application(&transaction, application_id)?;
    if matches!(
        current.status,
        CuratorApplicationStatus::Applied
            | CuratorApplicationStatus::Failed
            | CuratorApplicationStatus::Reconciled
    ) {
        return Ok(current);
    }
    if current.revision != expected_application_revision {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    let (candidate_state, candidate_revision) = transaction
        .query_row(
            "SELECT state,revision FROM evolution_curator_candidates WHERE candidate_id=?1",
            [&current.candidate_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .optional()
        .map_err(|_| CuratorApplicationStoreError::Storage)?
        .ok_or(CuratorApplicationStoreError::NotFound)?;
    if candidate_state != "applying" {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    let candidate_revision = from_sql_u64(candidate_revision).map_err(map_repository_error)?;
    let next_candidate_revision = candidate_revision
        .checked_add(1)
        .ok_or(CuratorApplicationStoreError::InvalidInput)?;
    let next_application_revision = current
        .revision
        .checked_add(1)
        .ok_or(CuratorApplicationStoreError::InvalidInput)?;
    let finalization = Finalization::from_result(result);
    update_application(
        &transaction,
        application_id,
        current.revision,
        next_application_revision,
        &finalization,
        occurred_at_ms,
    )?;
    update_candidate(
        &transaction,
        &current.candidate_id,
        candidate_revision,
        next_candidate_revision,
        finalization.candidate_state,
        occurred_at_ms,
    )?;
    update_outbox(&transaction, application_id, &finalization, occurred_at_ms)?;
    append_system_event(
        &transaction,
        &SystemAuditEvent {
            candidate_id: &current.candidate_id,
            event_kind: finalization.event_kind,
            occurred_at_ms,
            prior_state: Some(CuratorCandidateState::Applying),
            next_state: finalization.candidate_state,
            object_revision: next_candidate_revision,
            reason_code: &finalization.reason_code,
        },
    )
    .map_err(map_repository_error)?;
    transaction
        .commit()
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    load_application(connection, application_id)
}

pub(super) fn prepare_retry(
    connection: &mut rusqlite::Connection,
    candidate_id: &str,
    expected_candidate_revision: u64,
    occurred_at_ms: i64,
) -> Result<u64, CuratorApplicationStoreError> {
    if candidate_id.trim().is_empty() || occurred_at_ms < 0 {
        return Err(CuratorApplicationStoreError::InvalidInput);
    }
    let next_revision = expected_candidate_revision
        .checked_add(1)
        .ok_or(CuratorApplicationStoreError::InvalidInput)?;
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    transaction
        .execute(
            "UPDATE evolution_curator_previews SET invalidated_at_ms=COALESCE(invalidated_at_ms,?1)
             WHERE candidate_id=?2",
            params![occurred_at_ms, candidate_id],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    let staleness = serde_json::to_string(&vec![CuratorStalenessReason::PreviewExpired])
        .map_err(|_| CuratorApplicationStoreError::InvalidInput)?;
    let updated = transaction
        .execute(
            "UPDATE evolution_curator_candidates SET state='ready_for_review',current_preview_id=NULL,
             staleness_json=?1,revision=?2,updated_at_ms=?3
             WHERE candidate_id=?4 AND state='apply_failed' AND revision=?5",
            params![
                staleness,
                sql_u64(next_revision).map_err(map_repository_error)?,
                occurred_at_ms,
                candidate_id,
                sql_u64(expected_candidate_revision).map_err(map_repository_error)?
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    if updated != 1 {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    append_system_event(
        &transaction,
        &SystemAuditEvent {
            candidate_id,
            event_kind: CuratorEventKind::PreviewInvalidated,
            occurred_at_ms,
            prior_state: Some(CuratorCandidateState::ApplyFailed),
            next_state: CuratorCandidateState::ReadyForReview,
            object_revision: next_revision,
            reason_code: "retry_requires_fresh_preview_and_approval",
        },
    )
    .map_err(map_repository_error)?;
    transaction
        .commit()
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    Ok(next_revision)
}

struct Finalization {
    status: CuratorApplicationStatus,
    candidate_state: CuratorCandidateState,
    event_kind: CuratorEventKind,
    reason_code: String,
    overlay_revision: Option<String>,
    overlay_history_id: Option<String>,
    failure_code: Option<String>,
}

impl Finalization {
    fn from_result(
        result: Result<&CuratorOverlayApplicationReceipt, CuratorApplicationFailure>,
    ) -> Self {
        match result {
            Ok(receipt) => Self {
                status: if receipt.duplicate {
                    CuratorApplicationStatus::Reconciled
                } else {
                    CuratorApplicationStatus::Applied
                },
                candidate_state: CuratorCandidateState::Applied,
                event_kind: CuratorEventKind::Applied,
                reason_code: if receipt.duplicate {
                    "overlay_commit_reconciled"
                } else {
                    "overlay_commit_applied"
                }
                .to_string(),
                overlay_revision: Some(receipt.overlay_revision.clone()),
                overlay_history_id: Some(receipt.overlay_history_id.clone()),
                failure_code: None,
            },
            Err(failure) => Self {
                status: CuratorApplicationStatus::Failed,
                candidate_state: CuratorCandidateState::ApplyFailed,
                event_kind: CuratorEventKind::ApplicationFailed,
                reason_code: failure.code().to_string(),
                overlay_revision: None,
                overlay_history_id: None,
                failure_code: Some(failure.code().to_string()),
            },
        }
    }
}

fn update_application(
    transaction: &rusqlite::Transaction<'_>,
    application_id: &str,
    current_revision: u64,
    next_revision: u64,
    finalization: &Finalization,
    occurred_at_ms: i64,
) -> Result<(), CuratorApplicationStoreError> {
    let updated = transaction
        .execute(
            "UPDATE evolution_curator_applications SET status=?1,overlay_revision=?2,
         overlay_history_id=?3,failure_code=?4,revision=?5,updated_at_ms=?6
         WHERE application_id=?7 AND revision=?8 AND status IN ('intent_recorded','applying')",
            params![
                status_name(finalization.status),
                finalization.overlay_revision,
                finalization.overlay_history_id,
                finalization.failure_code,
                sql_u64(next_revision).map_err(map_repository_error)?,
                occurred_at_ms,
                application_id,
                sql_u64(current_revision).map_err(map_repository_error)?
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    if updated != 1 {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    Ok(())
}

fn update_candidate(
    transaction: &rusqlite::Transaction<'_>,
    candidate_id: &str,
    current_revision: u64,
    next_revision: u64,
    next_state: CuratorCandidateState,
    occurred_at_ms: i64,
) -> Result<(), CuratorApplicationStoreError> {
    let updated = transaction
        .execute(
            "UPDATE evolution_curator_candidates SET state=?1,revision=?2,updated_at_ms=?3
         WHERE candidate_id=?4 AND state='applying' AND revision=?5",
            params![
                state_name(next_state),
                sql_u64(next_revision).map_err(map_repository_error)?,
                occurred_at_ms,
                candidate_id,
                sql_u64(current_revision).map_err(map_repository_error)?
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    if updated != 1 {
        return Err(CuratorApplicationStoreError::Conflict);
    }
    Ok(())
}

fn update_outbox(
    transaction: &rusqlite::Transaction<'_>,
    application_id: &str,
    finalization: &Finalization,
    occurred_at_ms: i64,
) -> Result<(), CuratorApplicationStoreError> {
    transaction
        .execute(
            "UPDATE evolution_curator_outbox SET status=?1,attempt_count=attempt_count+1,
         completed_at_ms=?2,lease_owner=NULL,lease_expires_at_ms=NULL WHERE application_id=?3",
            params![
                if finalization.failure_code.is_some() {
                    "failed"
                } else {
                    "completed"
                },
                occurred_at_ms,
                application_id
            ],
        )
        .map_err(|_| CuratorApplicationStoreError::Storage)?;
    Ok(())
}

fn status_name(value: CuratorApplicationStatus) -> &'static str {
    match value {
        CuratorApplicationStatus::IntentRecorded => "intent_recorded",
        CuratorApplicationStatus::Applying => "applying",
        CuratorApplicationStatus::Applied => "applied",
        CuratorApplicationStatus::Failed => "failed",
        CuratorApplicationStatus::Reconciled => "reconciled",
    }
}

fn map_repository_error(_: super::CuratorRepositoryError) -> CuratorApplicationStoreError {
    CuratorApplicationStoreError::Storage
}
