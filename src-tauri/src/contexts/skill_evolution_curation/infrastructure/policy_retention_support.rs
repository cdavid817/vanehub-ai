use super::policy_retention_purge::append_event;
use super::policy_retention_store::*;
use super::repository_support::{from_sql_u64, parse_state, sql_u64};
use super::SystemAuditEvent;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, OptionalExtension, Transaction};

const DAY_MS: i64 = 24 * 60 * 60 * 1_000;

pub(super) fn load_policy_from_connection(
    connection: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<CuratorPolicyV1, CuratorPolicyRetentionError> {
    let json = connection
        .query_row(
            "SELECT policy_json FROM evolution_curator_policy WHERE workspace_id=?1",
            [workspace_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    parse_policy(json, workspace_id)
}

pub(super) fn load_policy_from_transaction(
    transaction: &Transaction<'_>,
    workspace_id: &str,
) -> Result<CuratorPolicyV1, CuratorPolicyRetentionError> {
    let json = transaction
        .query_row(
            "SELECT policy_json FROM evolution_curator_policy WHERE workspace_id=?1",
            [workspace_id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    parse_policy(json, workspace_id)
}

fn parse_policy(
    json: Option<String>,
    workspace_id: &str,
) -> Result<CuratorPolicyV1, CuratorPolicyRetentionError> {
    json.map_or_else(
        || Ok(CuratorPolicyV1::manual_default(workspace_id.to_string())),
        |value| serde_json::from_str(&value).map_err(|_| CuratorPolicyRetentionError::Storage),
    )
}

pub(super) fn persist_policy(
    transaction: &Transaction<'_>,
    policy: &CuratorPolicyV1,
    hash: &str,
    occurred_at_ms: i64,
) -> Result<(), CuratorPolicyRetentionError> {
    let json = serde_json::to_string(policy).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    transaction.execute(
        "INSERT INTO evolution_curator_policy (workspace_id,schema_version,policy_json,policy_hash,revision,updated_at_ms)
         VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(workspace_id) DO UPDATE SET policy_json=excluded.policy_json,
         policy_hash=excluded.policy_hash,revision=excluded.revision,updated_at_ms=excluded.updated_at_ms",
        params![policy.workspace_id, policy.schema_version, json, hash,
            sql_u64(policy.revision).map_err(map_repo)?, occurred_at_ms],
    ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    Ok(())
}

pub(super) fn rebind_open_candidates(
    transaction: &Transaction<'_>,
    workspace_id: &str,
    policy_hash: &str,
    occurred_at_ms: i64,
) -> Result<u64, CuratorPolicyRetentionError> {
    let rows = candidate_rows(transaction, workspace_id)?;
    for (candidate_id, state, revision) in &rows {
        let next_revision = revision
            .checked_add(1)
            .ok_or(CuratorPolicyRetentionError::InvalidInput)?;
        let stale = serde_json::to_string(&vec![CuratorStalenessReason::PolicyChanged])
            .map_err(|_| CuratorPolicyRetentionError::Storage)?;
        transaction.execute(
            "UPDATE evolution_curator_previews SET invalidated_at_ms=COALESCE(invalidated_at_ms,?1)
             WHERE candidate_id=?2",
            params![occurred_at_ms, candidate_id],
        ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
        transaction.execute(
            "UPDATE evolution_curator_candidates SET policy_witness_hash=?1,current_preview_id=NULL,
             staleness_json=?2,revision=?3,updated_at_ms=?4 WHERE candidate_id=?5 AND revision=?6",
            params![policy_hash, stale, sql_u64(next_revision).map_err(map_repo)?, occurred_at_ms,
                candidate_id, sql_u64(*revision).map_err(map_repo)?],
        ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
        append_event(
            transaction,
            &SystemAuditEvent {
                candidate_id,
                event_kind: CuratorEventKind::PolicyChanged,
                occurred_at_ms,
                prior_state: Some(*state),
                next_state: *state,
                object_revision: next_revision,
                reason_code: "workspace_policy_revision_changed",
            },
        )?;
    }
    Ok(rows.len() as u64)
}

fn candidate_rows(
    transaction: &Transaction<'_>,
    workspace_id: &str,
) -> Result<Vec<(String, CuratorCandidateState, u64)>, CuratorPolicyRetentionError> {
    let mut statement = transaction.prepare(
        "SELECT candidate_id,state,revision FROM evolution_curator_candidates WHERE workspace_id=?1
         AND state IN ('pending','awaiting_draft','ready_for_review','deferred','apply_failed')",
    ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    let rows = statement
        .query_map([workspace_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    rows.map(|row| {
        let (id, state, revision) = row.map_err(|_| CuratorPolicyRetentionError::Storage)?;
        Ok((
            id,
            parse_state(&state).map_err(map_repo)?,
            from_sql_u64(revision).map_err(map_repo)?,
        ))
    })
    .collect()
}

pub(super) fn candidate_ids(
    transaction: &Transaction<'_>,
    workspace_id: &str,
    predicate: &str,
    cutoff: i64,
) -> Result<Vec<String>, CuratorPolicyRetentionError> {
    let sql = format!("SELECT candidate_id FROM evolution_curator_candidates WHERE workspace_id=?1 AND {predicate}");
    let mut statement = transaction
        .prepare(&sql)
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    let rows = statement
        .query_map(params![workspace_id, cutoff], |row| row.get::<_, String>(0))
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| CuratorPolicyRetentionError::Storage)
}

pub(super) fn evidence_candidates(
    transaction: &Transaction<'_>,
    evidence_id: &str,
) -> Result<Vec<(String, CuratorCandidateState)>, CuratorPolicyRetentionError> {
    let mut statement = transaction.prepare(
        "SELECT DISTINCT c.candidate_id,c.state FROM evolution_curator_candidates c
         JOIN evolution_curator_candidate_sources s ON s.candidate_id=c.candidate_id WHERE s.evidence_id=?1",
    ).map_err(|_| CuratorPolicyRetentionError::Storage)?;
    let rows = statement
        .query_map([evidence_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|_| CuratorPolicyRetentionError::Storage)?;
    rows.map(|row| {
        let (id, state) = row.map_err(|_| CuratorPolicyRetentionError::Storage)?;
        Ok((id, parse_state(&state).map_err(map_repo)?))
    })
    .collect()
}

pub(super) fn retention_cutoff(now_ms: i64, days: u16) -> Result<i64, CuratorPolicyRetentionError> {
    now_ms
        .checked_sub(i64::from(days) * DAY_MS)
        .ok_or(CuratorPolicyRetentionError::InvalidInput)
}

fn map_repo(_: super::CuratorRepositoryError) -> CuratorPolicyRetentionError {
    CuratorPolicyRetentionError::Storage
}
