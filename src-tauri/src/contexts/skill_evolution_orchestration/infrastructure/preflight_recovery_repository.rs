use super::{PreflightRepositoryError, SqlitePreflightRepository};
use crate::contexts::skill_evolution_orchestration::domain::{
    is_safe_identifier, AutomaticPreflightWitnessV1,
};
use rusqlite::OptionalExtension;

impl SqlitePreflightRepository {
    pub(crate) fn recover_consumed(
        &self,
        witness_id: &str,
        proof_hash: &str,
        overlay_preview_hash: &str,
    ) -> Result<AutomaticPreflightWitnessV1, PreflightRepositoryError> {
        if !is_safe_identifier(witness_id, 256) {
            return Err(PreflightRepositoryError::InvalidInput);
        }
        let connection = self
            .database
            .connection()
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let row = connection
            .query_row(
                "SELECT witness_id,run_id,eligibility_id,eligibility_proof_hash,reservation_id,
                 overlay_preview_hash,proof_hash,issued_at_ms,expires_at_ms,revision
                 FROM evolution_auto_preflight_witnesses
                 WHERE witness_id=?1 AND status='consumed'",
                [witness_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, i64>(8)?,
                        row.get::<_, i64>(9)?,
                    ))
                },
            )
            .optional()
            .map_err(|_| PreflightRepositoryError::Storage)?
            .ok_or(PreflightRepositoryError::NotFound)?;
        if row.5 != overlay_preview_hash || row.6 != proof_hash {
            return Err(PreflightRepositoryError::Conflict);
        }
        Ok(AutomaticPreflightWitnessV1 {
            witness_id: row.0,
            run_id: row.1,
            eligibility_id: row.2,
            eligibility_proof_hash: row.3,
            reservation_id: row.4,
            overlay_preview_hash: row.5,
            proof_hash: row.6,
            issued_at_ms: row.7,
            expires_at_ms: row.8,
            revision: u64::try_from(row.9).map_err(|_| PreflightRepositoryError::Storage)?,
        })
    }
}
