use super::CuratorRepositoryError;
use crate::contexts::skill_evolution_curation::domain::*;
use rusqlite::Transaction;
use sha2::{Digest, Sha256};

pub(super) struct AssessmentRecord {
    pub(super) seed_id: String,
    pub(super) seed_revision: String,
    pub(super) workspace_id: String,
    route: String,
    confidence: String,
    risk: String,
    witness_hash: String,
    pub(super) lineage_hash: String,
    pub(super) is_current: bool,
}
pub(super) struct TargetRecord {
    skill_id: String,
    revision: String,
    scope: String,
}

pub(super) fn validate_envelope(
    envelope: &AssessmentCompletionEnvelopeV1,
    now_ms: i64,
) -> Result<(), CuratorRepositoryError> {
    if envelope.schema_version != 1
        || envelope.assessment_attempt_id.trim().is_empty()
        || envelope.assessment_revision.trim().is_empty()
        || envelope.witness_hash.trim().is_empty()
        || now_ms < 0
    {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    Ok(())
}
pub(super) fn load_assessment(
    tx: &Transaction<'_>,
    id: &str,
) -> Result<AssessmentRecord, CuratorRepositoryError> {
    tx.query_row("SELECT a.seed_id,a.seed_revision,COALESCE(s.workspace,'global'),a.route,a.confidence,a.risk,
        a.witness_hash,a.lineage_hash,a.is_current FROM evolution_assessment_attempts a JOIN evolution_candidate_seeds s
        ON s.seed_id=a.seed_id WHERE a.attempt_id=?1 AND a.status='completed'", [id], |row| Ok(AssessmentRecord {
        seed_id: row.get(0)?, seed_revision: row.get(1)?, workspace_id: row.get(2)?, route: row.get(3)?, confidence: row.get(4)?,
        risk: row.get(5)?, witness_hash: row.get(6)?, lineage_hash: row.get(7)?, is_current: row.get::<_, i64>(8)? != 0,
    })).map_err(|error| match error { rusqlite::Error::QueryReturnedNoRows => CuratorRepositoryError::NotFound, _ => CuratorRepositoryError::Storage })
}
pub(super) fn validate_authoritative(
    envelope: &AssessmentCompletionEnvelopeV1,
    record: &AssessmentRecord,
) -> Result<(), CuratorRepositoryError> {
    if envelope.assessment_revision != record.witness_hash
        || envelope.witness_hash != record.witness_hash
        || assessment_route_name(envelope.route) != record.route
    {
        return Err(CuratorRepositoryError::InvalidInput);
    }
    Ok(())
}
pub(super) fn load_target(
    tx: &Transaction<'_>,
    id: &str,
) -> Result<TargetRecord, CuratorRepositoryError> {
    tx.query_row("SELECT skill_id,revision_hash,scope FROM evolution_assessment_targets WHERE attempt_id=?1 ORDER BY ordinal LIMIT 1",
        [id], |row| Ok(TargetRecord { skill_id: row.get(0)?, revision: row.get(1)?, scope: row.get(2)? }))
        .map_err(|error| match error { rusqlite::Error::QueryReturnedNoRows => CuratorRepositoryError::InvalidInput, _ => CuratorRepositoryError::Storage })
}
pub(super) fn load_evidence(
    tx: &Transaction<'_>,
    envelope: &AssessmentCompletionEnvelopeV1,
    lineage: &str,
) -> Result<Option<Vec<CuratorEvidenceSource>>, CuratorRepositoryError> {
    let mut statement = tx.prepare("SELECT l.evidence_id,s.sanitizer_version FROM evolution_assessment_evidence_links l
        LEFT JOIN evolution_signals s ON s.signal_id=l.evidence_id WHERE l.attempt_id=?1 ORDER BY l.evidence_id")
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let rows = statement
        .query_map([&envelope.assessment_attempt_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, Option<i64>>(1)?))
        })
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let mut sources = Vec::new();
    for row in rows {
        let (id, revision) = row.map_err(|_| CuratorRepositoryError::Storage)?;
        let Some(revision) = revision else {
            return Ok(None);
        };
        sources.push(CuratorEvidenceSource {
            evidence_id: id,
            evidence_revision: format!("sanitizer-v{revision}"),
            lineage_hash: lineage.to_owned(),
        });
    }
    if sources.is_empty() {
        Ok(None)
    } else {
        Ok(Some(sources))
    }
}
pub(super) fn load_checks(
    tx: &Transaction<'_>,
    id: &str,
) -> Result<Vec<CuratorQualityCheck>, CuratorRepositoryError> {
    let mut statement = tx.prepare("SELECT kind,result,reason_code FROM evolution_assessment_checks WHERE attempt_id=?1 ORDER BY ordinal")
        .map_err(|_| CuratorRepositoryError::Storage)?;
    let rows = statement
        .query_map([id], |row| {
            Ok(CuratorQualityCheck {
                code: row.get(0)?,
                result: parse_check(&row.get::<_, String>(1)?)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                reason_code: row.get(2)?,
            })
        })
        .map_err(|_| CuratorRepositoryError::Storage)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|_| CuratorRepositoryError::Storage)
}
pub(super) fn build_snapshot(
    envelope: &AssessmentCompletionEnvelopeV1,
    assessment: &AssessmentRecord,
    target: &TargetRecord,
    sources: Vec<CuratorEvidenceSource>,
    quality_checks: Vec<CuratorQualityCheck>,
    policy_hash: String,
    now_ms: i64,
) -> Result<CuratorCandidateSnapshot, CuratorRepositoryError> {
    let candidate_id = format!(
        "curator-{}",
        hash_text(&format!(
            "{}|{}|{}",
            envelope.assessment_revision, target.skill_id, target.revision
        ))
    );
    let evidence_ids = sources
        .iter()
        .map(|source| source.evidence_id.clone())
        .collect();
    let witness_hash = hash_text(&format!(
        "{}|{}|{}|{}|{}",
        envelope.witness_hash,
        target.skill_id,
        target.revision,
        assessment.seed_revision,
        policy_hash
    ));
    Ok(CuratorCandidateSnapshot {
        schema_version: 1,
        candidate_id,
        workspace_id: assessment.workspace_id.clone(),
        seed_id: assessment.seed_id.clone(),
        seed_revision: assessment.seed_revision.clone(),
        assessment_attempt_id: envelope.assessment_attempt_id.clone(),
        assessment_revision: envelope.assessment_revision.clone(),
        target_skill_id: target.skill_id.clone(),
        target_revision: target.revision.clone(),
        overlay_scope: if target.scope == "project" {
            "project".into()
        } else {
            "user".into()
        },
        route: approvable_route(envelope.route).ok_or(CuratorRepositoryError::InvalidInput)?,
        risk: parse_risk(&assessment.risk)?,
        confidence: parse_confidence(&assessment.confidence)?,
        evidence_ids,
        evidence_sources: sources,
        quality_checks,
        assessment_witness_hash: envelope.witness_hash.clone(),
        policy_witness_hash: policy_hash,
        witness_hash,
        state: CuratorCandidateState::AwaitingDraft,
        staleness: vec![],
        revision: 1,
        created_at_ms: now_ms,
        updated_at_ms: now_ms,
    })
}
pub(super) fn approvable_route(route: CuratorAssessmentRoute) -> Option<CuratorRoute> {
    match route {
        CuratorAssessmentRoute::Advance => Some(CuratorRoute::Advance),
        CuratorAssessmentRoute::NeedsHumanReview => Some(CuratorRoute::NeedsHumanReview),
        _ => None,
    }
}
pub(super) fn assessment_route_name(route: CuratorAssessmentRoute) -> &'static str {
    match route {
        CuratorAssessmentRoute::Advance => "advance",
        CuratorAssessmentRoute::Drop => "drop",
        CuratorAssessmentRoute::RecordMemoryOnly => "record_memory_only",
        CuratorAssessmentRoute::MergeDuplicate => "merge_duplicate",
        CuratorAssessmentRoute::NeedsHumanReview => "needs_human_review",
    }
}
fn parse_risk(value: &str) -> Result<CuratorRisk, CuratorRepositoryError> {
    match value {
        "low" => Ok(CuratorRisk::Low),
        "medium" => Ok(CuratorRisk::Medium),
        "high" => Ok(CuratorRisk::High),
        _ => Err(CuratorRepositoryError::Storage),
    }
}
fn parse_confidence(value: &str) -> Result<CuratorConfidence, CuratorRepositoryError> {
    match value {
        "low" => Ok(CuratorConfidence::Low),
        "medium" => Ok(CuratorConfidence::Medium),
        "high" => Ok(CuratorConfidence::High),
        _ => Err(CuratorRepositoryError::Storage),
    }
}
fn parse_check(value: &str) -> Result<CuratorCheckResult, ()> {
    match value {
        "pass" => Ok(CuratorCheckResult::Pass),
        "fail" => Ok(CuratorCheckResult::Fail),
        "review" => Ok(CuratorCheckResult::Review),
        "not_applicable" => Ok(CuratorCheckResult::NotApplicable),
        _ => Err(()),
    }
}
pub(super) fn hash_envelope<T: serde::Serialize>(
    value: &T,
) -> Result<String, CuratorRepositoryError> {
    serde_json::to_vec(value)
        .map(|bytes| hash_bytes(&bytes))
        .map_err(|_| CuratorRepositoryError::InvalidInput)
}
fn hash_text(value: &str) -> String {
    hash_bytes(value.as_bytes())
}
fn hash_bytes(value: &[u8]) -> String {
    crate::platform::hashing::sha256_hex(value)
}
