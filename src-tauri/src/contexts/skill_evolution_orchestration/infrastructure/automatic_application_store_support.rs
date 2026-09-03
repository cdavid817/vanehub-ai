use super::AutomaticApplicationStoreError;
use crate::contexts::skill_evolution_orchestration::domain::{
    canonical_json, AutoApplyProbationV1,
};
use rusqlite::{OptionalExtension, Transaction};

pub(super) fn probation_matches(
    transaction: &Transaction<'_>,
    value: &AutoApplyProbationV1,
) -> Result<bool, AutomaticApplicationStoreError> {
    let stored = transaction
        .query_row(
            "SELECT probation_id,workspace_id,skill_id,status,prior_effective_hash,
             current_effective_hash,evidence_fingerprint,evidence_categories_json,
             baseline_witness_hash,starts_at_ms,ends_at_ms,revision
             FROM evolution_auto_probations WHERE application_id=?1",
            [&value.application_id],
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
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                ))
            },
        )
        .optional()
        .map_err(|_| AutomaticApplicationStoreError::Storage)?;
    let categories = canonical_json(&value.evidence_categories)
        .map_err(|_| AutomaticApplicationStoreError::InvalidInput)?;
    Ok(stored.is_some_and(|row| {
        row == (
            value.probation_id.clone(),
            value.workspace_id.clone(),
            value.skill_id.clone(),
            "active".to_string(),
            value.prior_effective_hash.clone(),
            value.current_effective_hash.clone(),
            value.evidence_fingerprint.clone(),
            categories,
            value.baseline_witness_hash.clone(),
            value.starts_at_ms,
            value.ends_at_ms,
            0,
        )
    }))
}
