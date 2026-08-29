use super::intake_source::load_checks;
use super::repository_support::{from_sql_u64, parse_state, sql_u64, state_name};
use super::{append_system_event, SqliteCuratorRepository, SystemAuditEvent};
use crate::contexts::skill_evolution_curation::{application::*, domain::*};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Deserialize;

impl CuratorDraftReviewStore for SqliteCuratorRepository<'_> {
    fn review_binding(
        &mut self,
        candidate_id: &str,
    ) -> Result<CuratorDraftReviewBinding, CuratorDraftReviewStoreError> {
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Deferred)
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        let row = transaction
            .query_row(
                "SELECT c.revision,c.witness_hash,c.state,c.assessment_attempt_id,c.assessment_revision,
                 c.target_skill_id,c.target_revision,d.draft_id,d.revision,d.body_hash,d.kind,d.rationale,
                 d.expected_effective_change,d.evidence_ids_json,a.normalized_explanation_json
                 FROM evolution_curator_candidates c
                 JOIN evolution_curator_drafts d ON d.draft_id=c.current_draft_id
                 JOIN evolution_assessment_attempts a ON a.attempt_id=c.assessment_attempt_id
                 WHERE c.candidate_id=?1 ORDER BY d.revision DESC LIMIT 1",
                [candidate_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, String>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                        row.get::<_, String>(13)?,
                        row.get::<_, Option<String>>(14)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?
            .ok_or(CuratorDraftReviewStoreError::NotFound)?;
        let checks = load_checks(&transaction, &row.3).map_err(map_repository_error)?;
        let evidence_ids =
            serde_json::from_str(&row.13).map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        let original_lesson_shape = parse_lesson_shape(row.14.as_deref())?;
        transaction
            .commit()
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        Ok(CuratorDraftReviewBinding {
            candidate_id: candidate_id.to_string(),
            candidate_revision: from_sql_u64(row.0).map_err(map_repository_error)?,
            candidate_witness_hash: row.1,
            state: parse_state(&row.2).map_err(map_repository_error)?,
            assessment_attempt_id: row.3,
            assessment_revision: row.4,
            target_skill_id: row.5,
            target_revision: row.6,
            draft_id: row.7,
            draft_revision: from_sql_u64(row.8).map_err(map_repository_error)?,
            draft_hash: row.9,
            draft_kind: row.10,
            rationale: row.11,
            expected_effective_change: row.12,
            evidence_ids,
            original_checks: checks,
            original_lesson_shape,
        })
    }

    fn persist_draft_assessment(
        &mut self,
        assessment: &CuratorDraftAssessment,
        occurred_at_ms: i64,
    ) -> Result<u64, CuratorDraftReviewStoreError> {
        validate_assessment(assessment, occurred_at_ms)?;
        let checks_json = serde_json::to_string(&assessment.checks)
            .map_err(|_| CuratorDraftReviewStoreError::InvalidInput)?;
        let transaction = self
            .connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        let current = transaction
            .query_row(
                "SELECT c.revision,c.state,c.witness_hash,c.target_skill_id,c.target_revision,
                 d.draft_id,d.revision,d.body_hash FROM evolution_curator_candidates c
                 JOIN evolution_curator_drafts d ON d.draft_id=c.current_draft_id
                 WHERE c.candidate_id=?1 ORDER BY d.revision DESC LIMIT 1",
                [&assessment.candidate_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, i64>(6)?,
                        row.get::<_, String>(7)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?
            .ok_or(CuratorDraftReviewStoreError::NotFound)?;
        validate_current(assessment, &current)?;
        transaction.execute(
            "INSERT INTO evolution_curator_draft_assessments
             (draft_assessment_id,candidate_id,candidate_revision,draft_id,draft_revision,draft_hash,
              candidate_witness_hash,target_skill_id,target_revision,checks_json,approvable,
              model_evaluation_allowed,model_consulted,model_fallback_reason,witness_hash,created_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
            params![assessment.assessment_id,assessment.candidate_id,sql_u64(assessment.candidate_revision).map_err(map_repository_error)?,
                assessment.draft_id,sql_u64(assessment.draft_revision).map_err(map_repository_error)?,assessment.draft_hash,
                assessment.candidate_witness_hash,assessment.target_skill_id,assessment.target_revision,checks_json,
                i64::from(assessment.approvable),i64::from(assessment.model_evaluation_allowed),i64::from(assessment.model_consulted),
                assessment.model_fallback_reason,assessment.witness_hash,occurred_at_ms],
        ).map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        let next_revision = assessment.candidate_revision + 1;
        let next_state = if assessment.approvable {
            CuratorCandidateState::ReadyForReview
        } else {
            CuratorCandidateState::AwaitingDraft
        };
        let staleness = if assessment.approvable {
            "[]"
        } else {
            "[\"draft_changed\"]"
        };
        let updated = transaction.execute(
            "UPDATE evolution_curator_candidates SET state=?1,staleness_json=?2,revision=?3,updated_at_ms=?4
             WHERE candidate_id=?5 AND revision=?6 AND state='awaiting_draft'",
            params![state_name(next_state),staleness,sql_u64(next_revision).map_err(map_repository_error)?,occurred_at_ms,
                assessment.candidate_id,sql_u64(assessment.candidate_revision).map_err(map_repository_error)?],
        ).map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        if updated != 1 {
            return Err(CuratorDraftReviewStoreError::Conflict);
        }
        append_system_event(
            &transaction,
            &SystemAuditEvent {
                candidate_id: &assessment.candidate_id,
                event_kind: CuratorEventKind::DraftAssessed,
                occurred_at_ms,
                prior_state: Some(CuratorCandidateState::AwaitingDraft),
                next_state,
                object_revision: next_revision,
                reason_code: if assessment.approvable {
                    "draft_quality_passed"
                } else {
                    "draft_quality_blocked"
                },
            },
        )
        .map_err(map_repository_error)?;
        transaction
            .commit()
            .map_err(|_| CuratorDraftReviewStoreError::Storage)?;
        Ok(next_revision)
    }
}

type CurrentDraftRow = (i64, String, String, String, String, String, i64, String);

fn validate_current(
    assessment: &CuratorDraftAssessment,
    current: &CurrentDraftRow,
) -> Result<(), CuratorDraftReviewStoreError> {
    if current.0 != sql_u64(assessment.candidate_revision).map_err(map_repository_error)?
        || current.1 != "awaiting_draft"
        || current.2 != assessment.candidate_witness_hash
        || current.3 != assessment.target_skill_id
        || current.4 != assessment.target_revision
        || current.5 != assessment.draft_id
        || current.6 != sql_u64(assessment.draft_revision).map_err(map_repository_error)?
        || current.7 != assessment.draft_hash
    {
        return Err(CuratorDraftReviewStoreError::Conflict);
    }
    Ok(())
}

fn validate_assessment(
    assessment: &CuratorDraftAssessment,
    occurred_at_ms: i64,
) -> Result<(), CuratorDraftReviewStoreError> {
    if occurred_at_ms < 0
        || assessment.checks.len() != CURATOR_DRAFT_CHECK_ORDER_V1.len()
        || assessment.assessment_id.trim().is_empty()
        || assessment.witness_hash.trim().is_empty()
        || assessment.model_consulted && !assessment.model_evaluation_allowed
    {
        return Err(CuratorDraftReviewStoreError::InvalidInput);
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Explanation {
    lesson_shape: Option<LessonShape>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct LessonShape {
    trigger: Option<String>,
    required_behavior: Option<String>,
    prohibited_behavior: Option<String>,
    verification: Option<String>,
    environment: Option<String>,
    content_kinds: Vec<String>,
}

fn parse_lesson_shape(
    value: Option<&str>,
) -> Result<CuratorDraftLessonShape, CuratorDraftReviewStoreError> {
    let shape = value
        .and_then(|json| serde_json::from_str::<Explanation>(json).ok())
        .and_then(|explanation| explanation.lesson_shape)
        .ok_or(CuratorDraftReviewStoreError::Storage)?;
    Ok(CuratorDraftLessonShape {
        trigger: shape.trigger.unwrap_or_default(),
        required_behavior: shape.required_behavior.unwrap_or_default(),
        prohibited_behavior: shape.prohibited_behavior.unwrap_or_default(),
        verification: shape.verification.unwrap_or_default(),
        environment: shape.environment.unwrap_or_default(),
        content_kinds: shape.content_kinds,
    })
}

fn map_repository_error(error: super::CuratorRepositoryError) -> CuratorDraftReviewStoreError {
    match error {
        super::CuratorRepositoryError::NotFound => CuratorDraftReviewStoreError::NotFound,
        super::CuratorRepositoryError::Conflict(_) => CuratorDraftReviewStoreError::Conflict,
        super::CuratorRepositoryError::Storage => CuratorDraftReviewStoreError::Storage,
        _ => CuratorDraftReviewStoreError::InvalidInput,
    }
}
