use super::ProbationRepositoryError;
use crate::contexts::skill_evolution_orchestration::domain::{
    AutoApplyProbationV1, ProbationObservationV1, ProbationStatus,
};
use rusqlite::{OptionalExtension, Transaction};

pub(super) fn load_probation(
    transaction: &Transaction<'_>,
    probation_id: &str,
) -> Result<Option<AutoApplyProbationV1>, ProbationRepositoryError> {
    transaction
        .query_row(
            "SELECT probation_id,application_id,workspace_id,skill_id,status,
             prior_effective_hash,current_effective_hash,evidence_fingerprint,
             evidence_categories_json,baseline_witness_hash,starts_at_ms,ends_at_ms,revision
             FROM evolution_auto_probations WHERE probation_id=?1",
            [probation_id],
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
                    row.get::<_, String>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                ))
            },
        )
        .optional()
        .map_err(|_| ProbationRepositoryError::Storage)?
        .map(|row| {
            Ok(AutoApplyProbationV1 {
                probation_id: row.0,
                application_id: row.1,
                workspace_id: row.2,
                skill_id: row.3,
                status: parse_status(&row.4)?,
                prior_effective_hash: row.5,
                current_effective_hash: row.6,
                evidence_fingerprint: row.7,
                evidence_categories: serde_json::from_str(&row.8)
                    .map_err(|_| ProbationRepositoryError::Storage)?,
                baseline_witness_hash: row.9,
                starts_at_ms: row.10,
                ends_at_ms: row.11,
                revision: u64::try_from(row.12).map_err(|_| ProbationRepositoryError::Storage)?,
            })
        })
        .transpose()
}

pub(super) fn load_observation(
    transaction: &Transaction<'_>,
    observation_id: &str,
) -> Result<Option<ProbationObservationV1>, ProbationRepositoryError> {
    transaction
        .query_row(
            "SELECT observation_id,probation_id,source_kind,source_id,source_revision,
             verified,negative,baseline_exceeded,harmful_correction,safe_category,witness_hash,observed_at_ms
             FROM evolution_probation_observations WHERE observation_id=?1",
            [observation_id],
            observation_from_row,
        )
        .optional()
        .map_err(|_| ProbationRepositoryError::Storage)?
        .map(try_observation)
        .transpose()
}

pub(super) fn load_observations(
    transaction: &Transaction<'_>,
    probation_id: &str,
) -> Result<Vec<ProbationObservationV1>, ProbationRepositoryError> {
    let mut statement = transaction
        .prepare(
            "SELECT observation_id,probation_id,source_kind,source_id,source_revision,
         verified,negative,baseline_exceeded,harmful_correction,safe_category,witness_hash,observed_at_ms
         FROM evolution_probation_observations WHERE probation_id=?1
         ORDER BY observed_at_ms,observation_id",
        )
        .map_err(|_| ProbationRepositoryError::Storage)?;
    let rows = statement
        .query_map([probation_id], observation_from_row)
        .map_err(|_| ProbationRepositoryError::Storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| ProbationRepositoryError::Storage)?;
    rows.into_iter().map(try_observation).collect()
}

type ObservationRow = (
    String,
    String,
    String,
    String,
    i64,
    bool,
    bool,
    bool,
    bool,
    String,
    String,
    i64,
);

fn observation_from_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
        row.get(6)?,
        row.get(7)?,
        row.get(8)?,
        row.get(9)?,
        row.get(10)?,
        row.get(11)?,
    ))
}

fn try_observation(
    row: ObservationRow,
) -> Result<ProbationObservationV1, ProbationRepositoryError> {
    Ok(ProbationObservationV1 {
        observation_id: row.0,
        probation_id: row.1,
        source_kind: row.2,
        source_id: row.3,
        source_revision: u64::try_from(row.4).map_err(|_| ProbationRepositoryError::Storage)?,
        verified: row.5,
        negative: row.6,
        baseline_exceeded: row.7,
        harmful_correction: row.8,
        safe_category: row.9,
        witness_hash: row.10,
        observed_at_ms: row.11,
    })
}

fn parse_status(value: &str) -> Result<ProbationStatus, ProbationRepositoryError> {
    match value {
        "active" => Ok(ProbationStatus::Active),
        "healthy" => Ok(ProbationStatus::Healthy),
        "regressed" => Ok(ProbationStatus::Regressed),
        "expired" => Ok(ProbationStatus::Expired),
        "suspended" => Ok(ProbationStatus::Suspended),
        _ => Err(ProbationRepositoryError::Storage),
    }
}
