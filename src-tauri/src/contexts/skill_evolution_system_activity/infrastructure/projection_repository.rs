use std::str::FromStr;

use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;

use crate::contexts::skill_evolution_system_activity::domain::*;

pub(crate) struct SqliteActivityProjectionRepository<'connection> {
    pub(super) connection: &'connection Connection,
}

impl<'connection> SqliteActivityProjectionRepository<'connection> {
    pub(crate) fn new(connection: &'connection Connection) -> Self {
        Self { connection }
    }

    pub(crate) fn acquire_lease(
        &self,
        owner_id: &str,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<ActivityProjectionLease, ActivityProjectionRepositoryError> {
        validate_lease(owner_id, now_ms, expires_at_ms)?;
        self.connection.execute(
            "INSERT INTO evolution_activity_projection_leases
             (lease_key,owner_id,expires_at_ms,heartbeat_at_ms,revision)
             VALUES ('global',?1,?2,?3,1)
             ON CONFLICT(lease_key) DO UPDATE SET
               owner_id=excluded.owner_id,expires_at_ms=excluded.expires_at_ms,
               heartbeat_at_ms=excluded.heartbeat_at_ms,revision=revision+1
             WHERE evolution_activity_projection_leases.owner_id=excluded.owner_id
                OR evolution_activity_projection_leases.expires_at_ms<=excluded.heartbeat_at_ms",
            params![owner_id, expires_at_ms, now_ms],
        )?;
        let lease = self
            .lease()?
            .ok_or(ActivityProjectionRepositoryError::Storage)?;
        if lease.owner_id != owner_id {
            return Err(ActivityProjectionRepositoryError::LeaseHeld);
        }
        Ok(lease)
    }

    pub(crate) fn heartbeat_lease(
        &self,
        owner_id: &str,
        expected_revision: u64,
        now_ms: i64,
        expires_at_ms: i64,
    ) -> Result<ActivityProjectionLease, ActivityProjectionRepositoryError> {
        validate_lease(owner_id, now_ms, expires_at_ms)?;
        let changed = self.connection.execute(
            "UPDATE evolution_activity_projection_leases
             SET expires_at_ms=?1,heartbeat_at_ms=?2,revision=revision+1
             WHERE lease_key='global' AND owner_id=?3 AND revision=?4 AND expires_at_ms>?2",
            params![expires_at_ms, now_ms, owner_id, to_i64(expected_revision)?],
        )?;
        if changed != 1 {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        self.lease()?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn lease(
        &self,
    ) -> Result<Option<ActivityProjectionLease>, ActivityProjectionRepositoryError> {
        self.connection
            .query_row(
                "SELECT owner_id,expires_at_ms,heartbeat_at_ms,revision
                 FROM evolution_activity_projection_leases WHERE lease_key='global'",
                [],
                |row| {
                    Ok(ActivityProjectionLease {
                        owner_id: row.get(0)?,
                        expires_at_ms: row.get(1)?,
                        heartbeat_at_ms: row.get(2)?,
                        revision: from_i64(row.get(3)?)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub(crate) fn checkpoint(
        &self,
        checkpoint: &ActivityDomainCheckpoint,
    ) -> Result<ActivityDomainCursor, ActivityProjectionRepositoryError> {
        validate_checkpoint(checkpoint)?;
        let changed =
            if checkpoint.expected_revision == 0 {
                self.connection.execute(
                    "INSERT OR IGNORE INTO evolution_activity_domain_cursors
                 (source_domain,opaque_cursor,last_sequence,last_source_hash,retention_floor,
                  pending_count,oldest_pending_at_ms,last_success_at_ms,revision)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,1)",
                    params![
                        checkpoint.source_domain.as_str(),
                        checkpoint.opaque_cursor.expose(),
                        to_i64(checkpoint.last_sequence)?,
                        checkpoint.last_source_hash,
                        checkpoint
                            .retention_floor
                            .as_ref()
                            .map(OpaqueDomainCursor::expose),
                        to_i64(checkpoint.pending_count)?,
                        checkpoint.oldest_pending_at_ms,
                        checkpoint.last_success_at_ms,
                    ],
                )?
            } else {
                self.connection.execute(
                "UPDATE evolution_activity_domain_cursors SET opaque_cursor=?2,last_sequence=?3,
                   last_source_hash=?4,retention_floor=?5,pending_count=?6,
                   oldest_pending_at_ms=?7,gap_code=NULL,failure_code=NULL,last_success_at_ms=?8,
                   revision=revision+1
                 WHERE source_domain=?1 AND revision=?9 AND last_sequence<=?3",
                params![
                    checkpoint.source_domain.as_str(),
                    checkpoint.opaque_cursor.expose(),
                    to_i64(checkpoint.last_sequence)?,
                    checkpoint.last_source_hash,
                    checkpoint.retention_floor.as_ref().map(OpaqueDomainCursor::expose),
                    to_i64(checkpoint.pending_count)?,
                    checkpoint.oldest_pending_at_ms,
                    checkpoint.last_success_at_ms,
                    to_i64(checkpoint.expected_revision)?,
                ],
            )?
            };
        if changed != 1 {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        self.cursor(checkpoint.source_domain)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn record_failure(
        &self,
        domain: EvolutionSourceDomain,
        gap: Option<ActivityGapCode>,
        failure: ActivityProjectionFailureCode,
        expected_revision: u64,
    ) -> Result<ActivityDomainCursor, ActivityProjectionRepositoryError> {
        let gap_code = enum_json(gap)?;
        let failure_code = enum_json(Some(failure))?;
        let changed = if expected_revision == 0 {
            self.connection.execute(
                "INSERT OR IGNORE INTO evolution_activity_domain_cursors
                 (source_domain,last_sequence,pending_count,gap_code,failure_code,revision)
                 VALUES (?1,0,0,?2,?3,1)",
                params![domain.as_str(), gap_code, failure_code],
            )?
        } else {
            self.connection.execute(
                "UPDATE evolution_activity_domain_cursors SET gap_code=?1,failure_code=?2,
                 revision=revision+1 WHERE source_domain=?3 AND revision=?4",
                params![
                    gap_code,
                    failure_code,
                    domain.as_str(),
                    to_i64(expected_revision)?,
                ],
            )?
        };
        if changed != 1 {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        self.cursor(domain)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    pub(crate) fn cursor(
        &self,
        domain: EvolutionSourceDomain,
    ) -> Result<Option<ActivityDomainCursor>, ActivityProjectionRepositoryError> {
        self.connection
            .query_row(
                "SELECT source_domain,opaque_cursor,last_sequence,last_source_hash,retention_floor,
                        pending_count,oldest_pending_at_ms,gap_code,failure_code,revision
                 FROM evolution_activity_domain_cursors WHERE source_domain=?1",
                [domain.as_str()],
                parse_cursor,
            )
            .optional()
            .map_err(Into::into)
    }
}

fn parse_cursor(row: &rusqlite::Row<'_>) -> rusqlite::Result<ActivityDomainCursor> {
    let domain: String = row.get(0)?;
    let cursor: Option<String> = row.get(1)?;
    let floor: Option<String> = row.get(4)?;
    Ok(ActivityDomainCursor {
        source_domain: EvolutionSourceDomain::from_str(&domain).map_err(sql_conversion)?,
        opaque_cursor: cursor
            .map(OpaqueDomainCursor::parse)
            .transpose()
            .map_err(sql_conversion)?,
        last_sequence: from_i64(row.get(2)?)?,
        last_source_hash: row.get(3)?,
        retention_floor: floor
            .map(OpaqueDomainCursor::parse)
            .transpose()
            .map_err(sql_conversion)?,
        pending_count: from_i64(row.get(5)?)?,
        oldest_pending_at_ms: row.get(6)?,
        gap: parse_enum(row.get(7)?)?,
        failure_code: parse_enum(row.get(8)?)?,
        revision: from_i64(row.get(9)?)?,
    })
}

fn validate_lease(
    owner_id: &str,
    now_ms: i64,
    expires_at_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    if sanitize_text(owner_id, "lease.owner_id", 160).is_err() || expires_at_ms <= now_ms {
        return Err(ActivityProjectionRepositoryError::InvalidInput);
    }
    Ok(())
}

fn validate_checkpoint(
    checkpoint: &ActivityDomainCheckpoint,
) -> Result<(), ActivityProjectionRepositoryError> {
    if checkpoint.last_sequence == 0
        || checkpoint.last_source_hash.is_empty()
        || checkpoint.last_success_at_ms < 0
    {
        return Err(ActivityProjectionRepositoryError::InvalidInput);
    }
    Ok(())
}

fn enum_json<T: serde::Serialize>(
    value: Option<T>,
) -> Result<Option<String>, ActivityProjectionRepositoryError> {
    let Some(item) = value else {
        return Ok(None);
    };
    serde_json::to_value(item)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .map(Some)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn parse_enum<T: serde::de::DeserializeOwned>(
    value: Option<String>,
) -> rusqlite::Result<Option<T>> {
    value
        .map(|item| serde_json::from_value(serde_json::Value::String(item)).map_err(sql_conversion))
        .transpose()
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn from_i64(value: i64) -> rusqlite::Result<u64> {
    u64::try_from(value).map_err(sql_conversion)
}

fn sql_conversion(error: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(error))
}

#[derive(Debug, Error, PartialEq, Eq)]
pub(crate) enum ActivityProjectionRepositoryError {
    #[error("projection coordinator lease is held by another owner")]
    LeaseHeld,
    #[error("projection repository optimistic state changed")]
    Conflict,
    #[error("projection repository input is invalid")]
    InvalidInput,
    #[error("projection receipt conflicts with committed identity")]
    ReceiptCollision,
    #[error("projection repository storage failed")]
    Storage,
}

impl From<rusqlite::Error> for ActivityProjectionRepositoryError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}
