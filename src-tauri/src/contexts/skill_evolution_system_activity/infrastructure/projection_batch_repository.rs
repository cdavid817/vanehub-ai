use rusqlite::{params, OptionalExtension, Transaction};

use super::safe_identity_repository::persist_safe_identities;
use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::{
    application::{
        ActivityProjectionBatch, ActivityProjectionBatchResult, ActivityProjectionStore,
        ActivityProjectionStoreError,
    },
    domain::*,
};

impl SqliteActivityProjectionRepository<'_> {
    pub(crate) fn commit_projection_batch(
        &self,
        batch: &ActivityProjectionBatch,
    ) -> Result<ActivityProjectionBatchResult, ActivityProjectionRepositoryError> {
        validate_batch(batch)?;
        let transaction = self.connection.unchecked_transaction()?;
        let mut inserted = 0;
        for event in &batch.events {
            inserted += usize::from(persist_source_event(&transaction, event)?);
        }
        persist_checkpoint(&transaction, &batch.checkpoint)?;
        transaction.commit()?;
        Ok(ActivityProjectionBatchResult {
            scanned: batch.events.len(),
            inserted,
            replayed: batch.events.len().saturating_sub(inserted),
            has_more: batch.checkpoint.pending_count > 0,
        })
    }
}

impl ActivityProjectionStore for SqliteActivityProjectionRepository<'_> {
    fn cursor(
        &self,
        domain: EvolutionSourceDomain,
    ) -> Result<Option<ActivityDomainCursor>, ActivityProjectionStoreError> {
        SqliteActivityProjectionRepository::cursor(self, domain).map_err(map_store_error)
    }

    fn commit_batch(
        &self,
        batch: &ActivityProjectionBatch,
    ) -> Result<ActivityProjectionBatchResult, ActivityProjectionStoreError> {
        self.commit_projection_batch(batch).map_err(map_store_error)
    }

    fn record_failure(
        &self,
        domain: EvolutionSourceDomain,
        gap: Option<ActivityGapCode>,
        failure: ActivityProjectionFailureCode,
        expected_revision: u64,
    ) -> Result<(), ActivityProjectionStoreError> {
        SqliteActivityProjectionRepository::record_failure(
            self,
            domain,
            gap,
            failure,
            expected_revision,
        )
        .map(|_| ())
        .map_err(map_store_error)
    }
}

fn persist_source_event(
    transaction: &Transaction<'_>,
    event: &VerifiedProjectionEvent,
) -> Result<bool, ActivityProjectionRepositoryError> {
    let envelope = &event.envelope;
    let envelope_json = serde_json::to_string(envelope).map_err(storage)?;
    let payload_json = envelope
        .payload
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(storage)?;
    transaction.execute(
        "INSERT OR IGNORE INTO evolution_activity_envelopes
         (event_id,schema_version,event_code,source_domain,source_id,source_revision,
          source_sequence,scope_kind,canonical_scope_id,occurred_at_ms,committed_at_ms,
          severity,status,attention_kind,envelope_json,payload_json,projection_version,content_hash)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18)",
        params![
            envelope.event_id,
            i64::from(envelope.schema_version),
            enum_text(envelope.event_code)?,
            envelope.source_domain,
            envelope.source_id,
            envelope.source_revision,
            to_i64(envelope.source_sequence)?,
            enum_text(envelope.scope_kind)?,
            envelope.canonical_scope_id,
            envelope.occurred_at_ms,
            envelope.committed_at_ms,
            enum_text(envelope.severity)?,
            enum_text(envelope.status)?,
            enum_text(envelope.attention_kind)?,
            envelope_json,
            payload_json,
            i64::from(envelope.projection_policy_version),
            envelope.content_hash,
        ],
    )?;
    let stored_hash: Option<String> = transaction
        .query_row(
            "SELECT content_hash FROM evolution_activity_envelopes WHERE event_id=?1",
            [&envelope.event_id],
            |row| row.get(0),
        )
        .optional()?;
    if stored_hash.as_deref() != Some(envelope.content_hash.as_str()) {
        return Err(ActivityProjectionRepositoryError::ReceiptCollision);
    }
    persist_safe_identities(transaction, envelope)?;

    let changed = transaction.execute(
        "INSERT OR IGNORE INTO evolution_activity_source_receipts
         (source_domain,source_id,source_revision,event_code,projection_version,event_id,
          source_hash,committed_at_ms) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
        params![
            envelope.source_domain,
            envelope.source_id,
            envelope.source_revision,
            enum_text(envelope.event_code)?,
            i64::from(envelope.projection_policy_version),
            envelope.event_id,
            event.source_integrity_hash,
            envelope.committed_at_ms,
        ],
    )?;
    if changed == 1 {
        return Ok(true);
    }
    let existing: Option<(String, String)> = transaction
        .query_row(
            "SELECT event_id,source_hash FROM evolution_activity_source_receipts
             WHERE source_domain=?1 AND source_id=?2 AND source_revision=?3
               AND event_code=?4 AND projection_version=?5",
            params![
                envelope.source_domain,
                envelope.source_id,
                envelope.source_revision,
                enum_text(envelope.event_code)?,
                i64::from(envelope.projection_policy_version),
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    if existing
        == Some((
            envelope.event_id.clone(),
            event.source_integrity_hash.clone(),
        ))
    {
        Ok(false)
    } else {
        Err(ActivityProjectionRepositoryError::ReceiptCollision)
    }
}

fn persist_checkpoint(
    transaction: &Transaction<'_>,
    checkpoint: &ActivityDomainCheckpoint,
) -> Result<(), ActivityProjectionRepositoryError> {
    let changed = if checkpoint.expected_revision == 0 {
        transaction.execute(
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
        transaction.execute(
            "UPDATE evolution_activity_domain_cursors SET opaque_cursor=?2,last_sequence=?3,
             last_source_hash=?4,retention_floor=?5,pending_count=?6,oldest_pending_at_ms=?7,
             gap_code=NULL,failure_code=NULL,last_success_at_ms=?8,revision=revision+1
             WHERE source_domain=?1 AND revision=?9 AND last_sequence<=?3",
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
                to_i64(checkpoint.expected_revision)?,
            ],
        )?
    };
    if changed == 1 {
        Ok(())
    } else {
        Err(ActivityProjectionRepositoryError::Conflict)
    }
}

fn validate_batch(
    batch: &ActivityProjectionBatch,
) -> Result<(), ActivityProjectionRepositoryError> {
    let last = batch
        .events
        .last()
        .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
    if batch.events.len() > usize::from(MAX_SOURCE_SCAN_ITEMS)
        || last.source_cursor != batch.checkpoint.opaque_cursor
        || last.source_sequence != batch.checkpoint.last_sequence
        || last.source_integrity_hash != batch.checkpoint.last_source_hash
        || batch.events.iter().any(|event| {
            event.envelope.source_domain != batch.checkpoint.source_domain.as_str()
                || event.envelope.validate().is_err()
        })
    {
        return Err(ActivityProjectionRepositoryError::InvalidInput);
    }
    Ok(())
}

fn enum_text<T: serde::Serialize>(value: T) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn storage(_: impl std::fmt::Debug) -> ActivityProjectionRepositoryError {
    ActivityProjectionRepositoryError::Storage
}

fn map_store_error(error: ActivityProjectionRepositoryError) -> ActivityProjectionStoreError {
    match error {
        ActivityProjectionRepositoryError::Conflict
        | ActivityProjectionRepositoryError::LeaseHeld => ActivityProjectionStoreError::Conflict,
        ActivityProjectionRepositoryError::ReceiptCollision => {
            ActivityProjectionStoreError::ReceiptCollision
        }
        ActivityProjectionRepositoryError::InvalidInput => {
            ActivityProjectionStoreError::InvalidInput
        }
        ActivityProjectionRepositoryError::Storage
        | ActivityProjectionRepositoryError::Cancelled => ActivityProjectionStoreError::Storage,
    }
}
