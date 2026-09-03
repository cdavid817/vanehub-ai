use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    contexts::skill_evolution_orchestration::domain::{
        is_safe_identifier, AutomaticPreflightWitnessV1, AUTOMATIC_PREFLIGHT_TTL_MS,
    },
    platform::database::NativeDatabase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PreflightRepositoryError {
    InvalidInput,
    Conflict,
    Expired,
    AlreadyConsumed,
    NotFound,
    Storage,
}

#[derive(Clone)]
pub(crate) struct SqlitePreflightRepository {
    pub(super) database: NativeDatabase,
}

impl SqlitePreflightRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn issue(
        &self,
        witness: &AutomaticPreflightWitnessV1,
    ) -> Result<bool, PreflightRepositoryError> {
        validate(witness)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let source_is_current = transaction
            .query_row(
                "SELECT EXISTS(
                   SELECT 1 FROM evolution_auto_eligibility eligibility
                   JOIN evolution_auto_rate_reservations reservation
                     ON reservation.reservation_id=?3 AND reservation.run_id=?1
                    AND reservation.status='reserved'
                   WHERE eligibility.eligibility_id=?2 AND eligibility.run_id=?1
                     AND eligibility.result='eligible' AND eligibility.proof_hash=?4
                 )",
                params![
                    witness.run_id,
                    witness.eligibility_id,
                    witness.reservation_id,
                    witness.eligibility_proof_hash,
                ],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        if !source_is_current {
            return Err(PreflightRepositoryError::Conflict);
        }
        if let Some((proof_hash, status)) = transaction
            .query_row(
                "SELECT proof_hash,status FROM evolution_auto_preflight_witnesses
                 WHERE witness_id=?1",
                [&witness.witness_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
            )
            .optional()
            .map_err(|_| PreflightRepositoryError::Storage)?
        {
            return if proof_hash == witness.proof_hash && status == "active" {
                Ok(false)
            } else {
                Err(PreflightRepositoryError::Conflict)
            };
        }
        transaction
            .execute(
                "UPDATE evolution_auto_preflight_witnesses
                 SET status='expired',revision=revision+1
                 WHERE eligibility_id=?1 AND status='active' AND expires_at_ms<=?2",
                params![witness.eligibility_id, witness.issued_at_ms],
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let active: bool = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM evolution_auto_preflight_witnesses
                 WHERE eligibility_id=?1 AND status='active')",
                [&witness.eligibility_id],
                |row| row.get(0),
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        if active {
            return Err(PreflightRepositoryError::Conflict);
        }
        transaction
            .execute(
                "INSERT INTO evolution_auto_preflight_witnesses VALUES
                 (?1,?2,?3,?4,?5,?6,?7,?8,?9,NULL,'active',0)",
                params![
                    witness.witness_id,
                    witness.run_id,
                    witness.eligibility_id,
                    witness.eligibility_proof_hash,
                    witness.reservation_id,
                    witness.overlay_preview_hash,
                    witness.proof_hash,
                    witness.issued_at_ms,
                    witness.expires_at_ms,
                ],
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        transaction
            .commit()
            .map_err(|_| PreflightRepositoryError::Storage)?;
        Ok(true)
    }

    pub(crate) fn consume(
        &self,
        witness_id: &str,
        proof_hash: &str,
        current_overlay_preview_hash: &str,
        now_ms: i64,
    ) -> Result<AutomaticPreflightWitnessV1, PreflightRepositoryError> {
        if !is_safe_identifier(witness_id, 256) || now_ms < 0 {
            return Err(PreflightRepositoryError::InvalidInput);
        }
        let mut connection = self
            .database
            .connection()
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| PreflightRepositoryError::Storage)?;
        let witness = load(&transaction, witness_id)?.ok_or(PreflightRepositoryError::NotFound)?;
        let status = transaction
            .query_row(
                "SELECT status FROM evolution_auto_preflight_witnesses WHERE witness_id=?1",
                [witness_id],
                |row| row.get::<_, String>(0),
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        match status.as_str() {
            "consumed" => return Err(PreflightRepositoryError::AlreadyConsumed),
            "expired" => return Err(PreflightRepositoryError::Expired),
            "active" => {}
            _ => return Err(PreflightRepositoryError::Storage),
        }
        if now_ms >= witness.expires_at_ms {
            transaction
                .execute(
                    "UPDATE evolution_auto_preflight_witnesses
                     SET status='expired',revision=revision+1
                     WHERE witness_id=?1 AND status='active'",
                    [witness_id],
                )
                .map_err(|_| PreflightRepositoryError::Storage)?;
            transaction
                .commit()
                .map_err(|_| PreflightRepositoryError::Storage)?;
            return Err(PreflightRepositoryError::Expired);
        }
        if witness.proof_hash != proof_hash
            || witness.overlay_preview_hash != current_overlay_preview_hash
        {
            return Err(PreflightRepositoryError::Conflict);
        }
        let changed = transaction
            .execute(
                "UPDATE evolution_auto_preflight_witnesses
                 SET status='consumed',consumed_at_ms=?1,revision=revision+1
                 WHERE witness_id=?2 AND status='active' AND revision=?3",
                params![now_ms, witness_id, sql_revision(witness.revision)?],
            )
            .map_err(|_| PreflightRepositoryError::Storage)?;
        if changed != 1 {
            return Err(PreflightRepositoryError::Conflict);
        }
        transaction
            .commit()
            .map_err(|_| PreflightRepositoryError::Storage)?;
        Ok(AutomaticPreflightWitnessV1 {
            revision: witness.revision.saturating_add(1),
            ..witness
        })
    }
}

fn load(
    transaction: &rusqlite::Transaction<'_>,
    witness_id: &str,
) -> Result<Option<AutomaticPreflightWitnessV1>, PreflightRepositoryError> {
    transaction
        .query_row(
            "SELECT witness_id,run_id,eligibility_id,eligibility_proof_hash,reservation_id,
             overlay_preview_hash,proof_hash,issued_at_ms,expires_at_ms,revision
             FROM evolution_auto_preflight_witnesses WHERE witness_id=?1",
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
        .map(|row| {
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
        })
        .transpose()
}

fn validate(value: &AutomaticPreflightWitnessV1) -> Result<(), PreflightRepositoryError> {
    if !is_safe_identifier(&value.witness_id, 256)
        || !is_safe_identifier(&value.run_id, 256)
        || !is_safe_identifier(&value.eligibility_id, 256)
        || !is_safe_identifier(&value.reservation_id, 256)
        || !valid_hash(&value.eligibility_proof_hash)
        || !valid_hash(&value.overlay_preview_hash)
        || !valid_hash(&value.proof_hash)
        || value.issued_at_ms < 0
        || value.expires_at_ms
            != value
                .issued_at_ms
                .saturating_add(AUTOMATIC_PREFLIGHT_TTL_MS)
        || value.revision != 0
    {
        return Err(PreflightRepositoryError::InvalidInput);
    }
    Ok(())
}

fn valid_hash(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn sql_revision(value: u64) -> Result<i64, PreflightRepositoryError> {
    i64::try_from(value).map_err(|_| PreflightRepositoryError::InvalidInput)
}
