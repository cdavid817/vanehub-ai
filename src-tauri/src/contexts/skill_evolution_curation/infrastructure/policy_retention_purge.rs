use super::policy_retention_store::*;
use super::repository_support::{from_sql_u64, parse_state, sql_u64};
use super::{append_system_event, SystemAuditEvent};
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, Transaction};

pub(super) fn expire_open_candidate(
    transaction: &Transaction<'_>,
    candidate_id: &str,
    occurred_at_ms: i64,
    reason: &str,
) -> Result<(), CuratorPolicyRetentionError> {
    let (prior_state, revision) = candidate_state(transaction, candidate_id)?;
    let next_revision = revision
        .checked_add(1)
        .ok_or(CuratorPolicyRetentionError::InvalidInput)?;
    transaction.execute(
        "UPDATE evolution_curator_candidates SET state='superseded',current_draft_id=NULL,current_preview_id=NULL,
         staleness_json='[]',revision=?1,updated_at_ms=?2 WHERE candidate_id=?3 AND revision=?4",
        params![sql_u64(next_revision).map_err(map_repo)?, occurred_at_ms, candidate_id,
            sql_u64(revision).map_err(map_repo)?],
    ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    purge_detail_rows(transaction, candidate_id)?;
    redact_snapshot(transaction, candidate_id)?;
    append_event(
        transaction,
        &SystemAuditEvent {
            candidate_id,
            event_kind: CuratorEventKind::Superseded,
            occurred_at_ms,
            prior_state: Some(prior_state),
            next_state: CuratorCandidateState::Superseded,
            object_revision: next_revision,
            reason_code: reason,
        },
    )
}

pub(super) fn purge_candidate_detail(
    transaction: &Transaction<'_>,
    candidate_id: &str,
    occurred_at_ms: i64,
    reason: &str,
) -> Result<(), CuratorPolicyRetentionError> {
    let (state, revision) = candidate_state(transaction, candidate_id)?;
    let next_revision = revision
        .checked_add(1)
        .ok_or(CuratorPolicyRetentionError::InvalidInput)?;
    transaction.execute(
        "UPDATE evolution_curator_candidates SET current_draft_id=NULL,current_preview_id=NULL,revision=?1,
         updated_at_ms=?2 WHERE candidate_id=?3 AND revision=?4",
        params![sql_u64(next_revision).map_err(map_repo)?, occurred_at_ms, candidate_id,
            sql_u64(revision).map_err(map_repo)?],
    ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    purge_detail_rows(transaction, candidate_id)?;
    redact_snapshot(transaction, candidate_id)?;
    append_event(
        transaction,
        &SystemAuditEvent {
            candidate_id,
            event_kind: CuratorEventKind::ContentPurged,
            occurred_at_ms,
            prior_state: Some(state),
            next_state: state,
            object_revision: next_revision,
            reason_code: reason,
        },
    )
}

fn candidate_state(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<(CuratorCandidateState, u64), CuratorPolicyRetentionError> {
    let (state, revision) = transaction
        .query_row(
            "SELECT state,revision FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        )
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    Ok((
        parse_state(&state).map_err(map_repo)?,
        from_sql_u64(revision).map_err(map_repo)?,
    ))
}

fn purge_detail_rows(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<(), CuratorPolicyRetentionError> {
    for sql in [
        "DELETE FROM evolution_curator_previews WHERE candidate_id=?1",
        "DELETE FROM evolution_curator_draft_assessments WHERE candidate_id=?1",
        "DELETE FROM evolution_curator_drafts WHERE candidate_id=?1",
        "DELETE FROM evolution_curator_candidate_sources WHERE candidate_id=?1",
    ] {
        transaction
            .execute(sql, [candidate_id])
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    }
    Ok(())
}

fn redact_snapshot(
    transaction: &Transaction<'_>,
    candidate_id: &str,
) -> Result<(), CuratorPolicyRetentionError> {
    let json: String = transaction
        .query_row(
            "SELECT snapshot_json FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| row.get(0),
        )
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    let mut snapshot: CuratorCandidateSnapshot =
        serde_json::from_str(&json).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    snapshot.evidence_ids.clear();
    snapshot.evidence_sources.clear();
    let redacted =
        serde_json::to_string(&snapshot).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    transaction
        .execute(
            "UPDATE evolution_curator_candidates SET snapshot_json=?1 WHERE candidate_id=?2",
            params![redacted, candidate_id],
        )
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    Ok(())
}

pub(super) fn append_event(
    transaction: &Transaction<'_>,
    event: &SystemAuditEvent<'_>,
) -> Result<(), CuratorPolicyRetentionError> {
    append_system_event(transaction, event).map_err(map_repo)
}

fn map_repo(_: super::CuratorRepositoryError) -> CuratorPolicyRetentionError {
    CuratorPolicyRetentionError::Storage
}
