use rusqlite::{params, OptionalExtension, TransactionBehavior};

use crate::{
    contexts::skill_evolution_orchestration::domain::{
        is_safe_identifier, reconciled_rate_status, AutoRateReservationV1,
        RateReservationHistoryObservationV1, RateReservationStatus, AUTOMATIC_RUN_LIMIT_V1,
        AUTOMATIC_SKILL_LIMIT_7D_V1, AUTOMATIC_WORKSPACE_LIMIT_24H_V1, ROLLING_DAY_MS,
        ROLLING_WEEK_MS,
    },
    platform::database::NativeDatabase,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AutomaticRateLimit {
    Run,
    Workspace24Hours,
    Skill7Days,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RateReservationError {
    InvalidInput,
    Limited(AutomaticRateLimit),
    Conflict,
    NotFound,
    Storage,
}

#[derive(Clone)]
pub(crate) struct SqliteRateReservationRepository {
    database: NativeDatabase,
}

impl SqliteRateReservationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn reserve(
        &self,
        reservation: &AutoRateReservationV1,
    ) -> Result<bool, RateReservationError> {
        validate_reservation(reservation)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| RateReservationError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| RateReservationError::Storage)?;
        if let Some(existing) = load(&transaction, &reservation.reservation_id)? {
            return if existing == *reservation {
                Ok(false)
            } else {
                Err(RateReservationError::Conflict)
            };
        }
        enforce_limit(
            count(&transaction, "run_id", &reservation.run_id, 0)?,
            AUTOMATIC_RUN_LIMIT_V1,
            AutomaticRateLimit::Run,
        )?;
        enforce_limit(
            count(
                &transaction,
                "workspace_id",
                &reservation.workspace_id,
                reservation.reserved_at_ms.saturating_sub(ROLLING_DAY_MS),
            )?,
            AUTOMATIC_WORKSPACE_LIMIT_24H_V1,
            AutomaticRateLimit::Workspace24Hours,
        )?;
        enforce_limit(
            count(
                &transaction,
                "skill_id",
                &reservation.skill_id,
                reservation.reserved_at_ms.saturating_sub(ROLLING_WEEK_MS),
            )?,
            AUTOMATIC_SKILL_LIMIT_7D_V1,
            AutomaticRateLimit::Skill7Days,
        )?;
        transaction
            .execute(
                "INSERT INTO evolution_auto_rate_reservations VALUES
                 (?1,?2,?3,?4,'reserved',NULL,?5,?6,0)",
                params![
                    reservation.reservation_id,
                    reservation.run_id,
                    reservation.workspace_id,
                    reservation.skill_id,
                    reservation.reserved_at_ms,
                    reservation.updated_at_ms,
                ],
            )
            .map_err(|_| RateReservationError::Storage)?;
        transaction
            .commit()
            .map_err(|_| RateReservationError::Storage)?;
        Ok(true)
    }

    pub(crate) fn reconcile(
        &self,
        reservation_id: &str,
        expected_revision: u64,
        observation: &RateReservationHistoryObservationV1,
        now_ms: i64,
    ) -> Result<AutoRateReservationV1, RateReservationError> {
        if !is_safe_identifier(reservation_id, 256) || now_ms < 0 {
            return Err(RateReservationError::InvalidInput);
        }
        validate_observation(observation)?;
        let mut connection = self
            .database
            .connection()
            .map_err(|_| RateReservationError::Storage)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| RateReservationError::Storage)?;
        let current = load(&transaction, reservation_id)?.ok_or(RateReservationError::NotFound)?;
        if current.revision != expected_revision {
            return Err(RateReservationError::Conflict);
        }
        let status = reconciled_rate_status(observation);
        let application_id = observation.automatic_application_id.clone();
        let next_revision = current
            .revision
            .checked_add(1)
            .ok_or(RateReservationError::InvalidInput)?;
        let changed = transaction
            .execute(
                "UPDATE evolution_auto_rate_reservations
                 SET status=?1,application_id=?2,updated_at_ms=?3,revision=?4
                 WHERE reservation_id=?5 AND revision=?6",
                params![
                    status_name(status),
                    application_id,
                    now_ms,
                    sql_revision(next_revision)?,
                    reservation_id,
                    sql_revision(expected_revision)?,
                ],
            )
            .map_err(|_| RateReservationError::Storage)?;
        if changed != 1 {
            return Err(RateReservationError::Conflict);
        }
        transaction
            .commit()
            .map_err(|_| RateReservationError::Storage)?;
        Ok(AutoRateReservationV1 {
            status,
            application_id: observation.automatic_application_id.clone(),
            updated_at_ms: now_ms,
            revision: next_revision,
            ..current
        })
    }
}

fn count(
    transaction: &rusqlite::Transaction<'_>,
    column: &str,
    value: &str,
    since_ms: i64,
) -> Result<u8, RateReservationError> {
    let sql = format!(
        "SELECT COUNT(*) FROM evolution_auto_rate_reservations WHERE {column}=?1
         AND reserved_at_ms>=?2 AND status IN ('reserved','committed','recovery_required')"
    );
    transaction
        .query_row(&sql, params![value, since_ms], |row| row.get::<_, u8>(0))
        .map_err(|_| RateReservationError::Storage)
}

fn enforce_limit(
    current: u8,
    maximum: u8,
    limit: AutomaticRateLimit,
) -> Result<(), RateReservationError> {
    if current >= maximum {
        Err(RateReservationError::Limited(limit))
    } else {
        Ok(())
    }
}

fn load(
    transaction: &rusqlite::Transaction<'_>,
    reservation_id: &str,
) -> Result<Option<AutoRateReservationV1>, RateReservationError> {
    transaction
        .query_row(
            "SELECT reservation_id,run_id,workspace_id,skill_id,status,application_id,
             reserved_at_ms,updated_at_ms,revision FROM evolution_auto_rate_reservations
             WHERE reservation_id=?1",
            [reservation_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, Option<String>>(5)?,
                    row.get::<_, i64>(6)?,
                    row.get::<_, i64>(7)?,
                    row.get::<_, i64>(8)?,
                ))
            },
        )
        .optional()
        .map_err(|_| RateReservationError::Storage)?
        .map(from_row)
        .transpose()
}

fn from_row(
    row: (
        String,
        String,
        String,
        String,
        String,
        Option<String>,
        i64,
        i64,
        i64,
    ),
) -> Result<AutoRateReservationV1, RateReservationError> {
    Ok(AutoRateReservationV1 {
        reservation_id: row.0,
        run_id: row.1,
        workspace_id: row.2,
        skill_id: row.3,
        status: parse_status(&row.4)?,
        application_id: row.5,
        reserved_at_ms: row.6,
        updated_at_ms: row.7,
        revision: u64::try_from(row.8).map_err(|_| RateReservationError::Storage)?,
    })
}

fn validate_reservation(value: &AutoRateReservationV1) -> Result<(), RateReservationError> {
    if !is_safe_identifier(&value.reservation_id, 256)
        || !is_safe_identifier(&value.run_id, 256)
        || !is_safe_identifier(&value.workspace_id, 256)
        || !is_safe_identifier(&value.skill_id, 256)
        || value.status != RateReservationStatus::Reserved
        || value.application_id.is_some()
        || value.reserved_at_ms < 0
        || value.updated_at_ms != value.reserved_at_ms
        || value.revision != 0
    {
        return Err(RateReservationError::InvalidInput);
    }
    Ok(())
}

fn validate_observation(
    value: &RateReservationHistoryObservationV1,
) -> Result<(), RateReservationError> {
    if [
        value.automatic_application_id.as_deref(),
        value.curator_application_id.as_deref(),
        value.overlay_application_id.as_deref(),
    ]
    .into_iter()
    .flatten()
    .any(|id| !is_safe_identifier(id, 256))
    {
        return Err(RateReservationError::InvalidInput);
    }
    Ok(())
}

fn parse_status(value: &str) -> Result<RateReservationStatus, RateReservationError> {
    match value {
        "reserved" => Ok(RateReservationStatus::Reserved),
        "committed" => Ok(RateReservationStatus::Committed),
        "released" => Ok(RateReservationStatus::Released),
        "recovery_required" => Ok(RateReservationStatus::RecoveryRequired),
        _ => Err(RateReservationError::Storage),
    }
}

fn status_name(value: RateReservationStatus) -> &'static str {
    match value {
        RateReservationStatus::Reserved => "reserved",
        RateReservationStatus::Committed => "committed",
        RateReservationStatus::Released => "released",
        RateReservationStatus::RecoveryRequired => "recovery_required",
    }
}

fn sql_revision(value: u64) -> Result<i64, RateReservationError> {
    i64::try_from(value).map_err(|_| RateReservationError::InvalidInput)
}
