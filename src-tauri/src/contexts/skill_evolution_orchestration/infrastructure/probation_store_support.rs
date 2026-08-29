use super::{ProbationRecordOutcome, ProbationRepositoryError};
use crate::contexts::skill_evolution_orchestration::domain::{
    AutoApplyProbationV1, ProbationEvaluation, ProbationObservationV1, ProbationStatus,
    BREAKER_HEALTH_CHECK_VERSION_V1,
};
use rusqlite::{params, OptionalExtension, Transaction};

pub(super) fn insert_observation(
    transaction: &Transaction<'_>,
    value: &ProbationObservationV1,
) -> Result<(), ProbationRepositoryError> {
    let source_revision =
        i64::try_from(value.source_revision).map_err(|_| ProbationRepositoryError::InvalidInput)?;
    transaction
        .execute(
            "INSERT INTO evolution_probation_observations VALUES
             (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                value.observation_id,
                value.probation_id,
                value.source_kind,
                value.source_id,
                source_revision,
                value.verified,
                value.negative,
                value.baseline_exceeded,
                value.harmful_correction,
                value.safe_category,
                value.witness_hash,
                value.observed_at_ms
            ],
        )
        .map_err(|_| ProbationRepositoryError::Storage)?;
    Ok(())
}

pub(super) fn transition_probation(
    transaction: &Transaction<'_>,
    probation: &AutoApplyProbationV1,
    evaluation: ProbationEvaluation,
) -> Result<(), ProbationRepositoryError> {
    let status = match evaluation {
        ProbationEvaluation::Active => return Ok(()),
        ProbationEvaluation::Healthy => "healthy",
        ProbationEvaluation::Regressed => "regressed",
    };
    let expected =
        i64::try_from(probation.revision).map_err(|_| ProbationRepositoryError::InvalidInput)?;
    let changed = transaction
        .execute(
            "UPDATE evolution_auto_probations SET status=?1,revision=revision+1
             WHERE probation_id=?2 AND status='active' AND revision=?3",
            params![status, probation.probation_id, expected],
        )
        .map_err(|_| ProbationRepositoryError::Storage)?;
    if changed != 1 {
        return Err(ProbationRepositoryError::Conflict);
    }
    transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_curator_notification_receipts
             SELECT ca.candidate_id,c.revision,'probation_regression','pending',NULL
             FROM evolution_auto_applications aa
             JOIN evolution_curator_applications ca ON ca.application_id=aa.curator_application_id
             JOIN evolution_curator_candidates c ON c.candidate_id=ca.candidate_id
             WHERE aa.application_id=?1",
            [&probation.application_id],
        )
        .map_err(|_| ProbationRepositoryError::Storage)?;
    Ok(())
}

pub(super) fn insert_rollback_candidate(
    transaction: &Transaction<'_>,
    probation: &AutoApplyProbationV1,
    rollback_id: &str,
    witness_hash: &str,
    security: bool,
    now_ms: i64,
) -> Result<(), ProbationRepositoryError> {
    let changed = transaction
        .execute(
            "INSERT INTO evolution_curator_rollback_candidates
             SELECT ?1,ca.candidate_id,aa.application_id,?2,?3,?4,?5,?6,?7,?8,'pending',?9
             FROM evolution_auto_applications aa JOIN evolution_curator_applications ca
               ON ca.application_id=aa.curator_application_id
             WHERE aa.application_id=?10 AND ca.status IN ('applied','reconciled')",
            params![
                rollback_id,
                probation.probation_id,
                probation.workspace_id,
                probation.skill_id,
                probation.prior_effective_hash,
                probation.current_effective_hash,
                witness_hash,
                if security { "security" } else { "standard" },
                now_ms,
                probation.application_id
            ],
        )
        .map_err(|_| ProbationRepositoryError::Storage)?;
    if changed != 1 {
        return Err(ProbationRepositoryError::Conflict);
    }
    Ok(())
}

pub(super) fn open_skill_suspension(
    transaction: &Transaction<'_>,
    probation: &AutoApplyProbationV1,
    now_ms: i64,
) -> Result<(), ProbationRepositoryError> {
    open_breaker(
        transaction,
        &format!(
            "skill-breaker-{}-{}",
            probation.workspace_id, probation.skill_id
        ),
        &probation.workspace_id,
        Some(&probation.skill_id),
        "probation_regression",
        &probation.application_id,
        now_ms,
    )
}

pub(super) fn open_security_workspace_breaker(
    transaction: &Transaction<'_>,
    probation: &AutoApplyProbationV1,
    now_ms: i64,
) -> Result<(), ProbationRepositoryError> {
    open_breaker(
        transaction,
        &format!("workspace-breaker-{}", probation.workspace_id),
        &probation.workspace_id,
        None,
        "security_regression",
        &probation.application_id,
        now_ms,
    )
}

fn open_breaker(
    transaction: &Transaction<'_>,
    breaker_id: &str,
    workspace_id: &str,
    skill_id: Option<&str>,
    cause: &str,
    application_id: &str,
    now_ms: i64,
) -> Result<(), ProbationRepositoryError> {
    transaction.execute(
        "INSERT INTO evolution_auto_breakers VALUES
         (?1,?2,?3,'open',?4,NULL,?5,?6,0,NULL,?7,?7,1)
         ON CONFLICT(breaker_id) DO UPDATE SET status='open',safe_cause_code=excluded.safe_cause_code,
         source_application_id=excluded.source_application_id,health_probe_passed=0,
         acknowledged_actor=NULL,updated_at_ms=excluded.updated_at_ms,revision=revision+1",
        params![breaker_id, workspace_id, skill_id, cause, application_id,
            BREAKER_HEALTH_CHECK_VERSION_V1, now_ms],
    ).map_err(|_| ProbationRepositoryError::Storage)?;
    Ok(())
}

pub(super) fn is_security_regression(value: &ProbationObservationV1) -> bool {
    value.verified
        && value.negative
        && (value.safe_category == "security" || value.safe_category.starts_with("security_"))
}

pub(super) fn current_outcome(
    transaction: &Transaction<'_>,
    probation: &AutoApplyProbationV1,
    duplicate: bool,
) -> Result<ProbationRecordOutcome, ProbationRepositoryError> {
    let rollback_candidate_id = transaction
        .query_row(
            "SELECT rollback_candidate_id FROM evolution_curator_rollback_candidates
         WHERE source_application_id=?1",
            [&probation.application_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(|_| ProbationRepositoryError::Storage)?;
    let evaluation = match probation.status {
        ProbationStatus::Active => ProbationEvaluation::Active,
        ProbationStatus::Healthy | ProbationStatus::Expired => ProbationEvaluation::Healthy,
        ProbationStatus::Regressed | ProbationStatus::Suspended => ProbationEvaluation::Regressed,
    };
    let security_escalated = rollback_candidate_id.as_ref().is_some_and(|_| {
        transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM evolution_auto_breakers WHERE workspace_id=?1
             AND skill_id IS NULL AND status!='closed' AND safe_cause_code='security_regression')",
                [&probation.workspace_id],
                |row| row.get(0),
            )
            .unwrap_or(false)
    });
    Ok(ProbationRecordOutcome {
        evaluation,
        rollback_candidate_id,
        security_escalated,
        duplicate,
    })
}
