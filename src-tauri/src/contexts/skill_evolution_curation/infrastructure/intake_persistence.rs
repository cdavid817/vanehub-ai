use super::audit_chain::{append_system_event, SystemAuditEvent};
use super::intake_source::{assessment_route_name, hash_envelope};
use super::repository_support::{confidence_name, parse_state, risk_name, route_name, state_name};
use super::{safe_snapshot_json, CandidateConflict, CuratorRepositoryError};
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::{params, OptionalExtension, Transaction};

pub(super) fn insert_candidate(
    tx: &Transaction<'_>,
    snapshot: &CuratorCandidateSnapshot,
) -> Result<(), CuratorRepositoryError> {
    let snapshot_json = safe_snapshot_json(snapshot)?;
    tx.execute("INSERT INTO evolution_curator_candidates (candidate_id,schema_version,workspace_id,seed_id,seed_revision,
        assessment_attempt_id,assessment_revision,target_skill_id,target_revision,overlay_scope,route,risk,confidence,
        policy_witness_hash,witness_hash,snapshot_json,state,staleness_json,revision,created_at_ms,updated_at_ms)
        VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,'[]',1,?18,?18)", params![
        snapshot.candidate_id, snapshot.schema_version, snapshot.workspace_id, snapshot.seed_id, snapshot.seed_revision,
        snapshot.assessment_attempt_id, snapshot.assessment_revision, snapshot.target_skill_id, snapshot.target_revision,
        snapshot.overlay_scope, route_name(snapshot.route), risk_name(snapshot.risk), confidence_name(snapshot.confidence),
        snapshot.policy_witness_hash, snapshot.witness_hash, snapshot_json, state_name(snapshot.state), snapshot.created_at_ms,
    ]).map_err(|error| if error.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        CuratorRepositoryError::Conflict(CandidateConflict { current_revision: 1, current_state: CuratorCandidateState::AwaitingDraft })
    } else { CuratorRepositoryError::Storage })?;
    for source in &snapshot.evidence_sources {
        tx.execute("INSERT INTO evolution_curator_candidate_sources (candidate_id,evidence_id,evidence_revision,lineage_hash)
            VALUES (?1,?2,?3,?4)", params![snapshot.candidate_id, source.evidence_id, source.evidence_revision, source.lineage_hash])
            .map_err(|_| CuratorRepositoryError::Storage)?;
    }
    Ok(())
}
pub(super) fn supersede_open(
    tx: &Transaction<'_>,
    seed_id: &str,
    successor: Option<&str>,
    now_ms: i64,
) -> Result<(), CuratorRepositoryError> {
    let mut statement = tx.prepare("SELECT candidate_id,state,revision FROM evolution_curator_candidates WHERE seed_id=?1
        AND state IN ('pending','awaiting_draft','ready_for_review','deferred','apply_failed') AND (?2 IS NULL OR candidate_id<>?2) ORDER BY candidate_id")
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let rows = statement
        .query_map(params![seed_id, successor], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(|_| CuratorRepositoryError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CuratorRepositoryError::Storage)?;
    drop(statement);
    for (candidate_id, state, revision) in rows {
        let prior = parse_state(&state)?;
        let next_revision = u64::try_from(revision)
            .map_err(|_| CuratorRepositoryError::Storage)?
            .checked_add(1)
            .ok_or(CuratorRepositoryError::Storage)?;
        tx.execute(
            "UPDATE evolution_curator_previews SET invalidated_at_ms=?1
             WHERE candidate_id=?2 AND invalidated_at_ms IS NULL",
            params![now_ms, candidate_id],
        )
        .map_err(|_| CuratorRepositoryError::Storage)?;
        tx.execute("UPDATE evolution_curator_candidates SET state='superseded',staleness_json='[\"assessment_changed\"]',
            current_preview_id=NULL,superseded_by_candidate_id=?1,revision=?2,updated_at_ms=?3 WHERE candidate_id=?4", params![successor, next_revision as i64, now_ms, candidate_id])
            .map_err(|_| CuratorRepositoryError::Storage)?;
        append_system_event(
            tx,
            &SystemAuditEvent {
                candidate_id: &candidate_id,
                event_kind: CuratorEventKind::Superseded,
                occurred_at_ms: now_ms,
                prior_state: Some(prior),
                next_state: CuratorCandidateState::Superseded,
                object_revision: next_revision,
                reason_code: "current_assessment_changed",
            },
        )?;
    }
    Ok(())
}
pub(super) fn record_receipt(
    tx: &Transaction<'_>,
    envelope: &AssessmentCompletionEnvelopeV1,
    envelope_hash: &str,
    outcome: &str,
    candidate_id: Option<&str>,
    now_ms: i64,
) -> Result<(), CuratorRepositoryError> {
    tx.execute("INSERT INTO evolution_curator_intake_receipts (envelope_hash,assessment_attempt_id,assessment_revision,
        route,witness_hash,outcome,candidate_id,received_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![envelope_hash,
        envelope.assessment_attempt_id, envelope.assessment_revision, assessment_route_name(envelope.route), envelope.witness_hash,
        outcome, candidate_id, now_ms]).map_err(|_| CuratorRepositoryError::Storage)?;
    Ok(())
}
pub(super) fn existing_receipt(
    tx: &Transaction<'_>,
    hash: &str,
) -> Result<Option<CuratorIntakeOutcome>, CuratorRepositoryError> {
    let row = tx.query_row("SELECT outcome,candidate_id FROM evolution_curator_intake_receipts WHERE envelope_hash=?1", [hash],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))).optional().map_err(|_| CuratorRepositoryError::Storage)?;
    row.map(|(outcome, candidate)| match (outcome.as_str(), candidate) {
        ("candidate_created", Some(candidate_id)) => {
            Ok(CuratorIntakeOutcome::ExistingCandidate { candidate_id })
        }
        ("non_approvable", _) => Ok(CuratorIntakeOutcome::NonApprovableRecorded),
        ("non_current", _) => Ok(CuratorIntakeOutcome::NonCurrentRejected),
        ("purged_evidence", _) => Ok(CuratorIntakeOutcome::PurgedEvidenceRejected),
        _ => Err(CuratorRepositoryError::Storage),
    })
    .transpose()
}
pub(super) fn load_policy_hash(
    tx: &Transaction<'_>,
    workspace: &str,
) -> Result<String, CuratorRepositoryError> {
    tx.query_row(
        "SELECT policy_hash FROM evolution_curator_policy WHERE workspace_id=?1",
        [workspace],
        |row| row.get(0),
    )
    .optional()
    .map_err(|_| CuratorRepositoryError::Storage)
    .and_then(|value| {
        value.map_or_else(
            || {
                policy_hash(&CuratorPolicyV1::manual_default(workspace.to_string()))
                    .map_err(|_| CuratorRepositoryError::InvalidInput)
            },
            Ok,
        )
    })
}
