use rusqlite::{OptionalExtension, Transaction};
use sha2::{Digest, Sha256};

use super::*;

pub(super) struct RebuildRow {
    pub(super) scope_kind: String,
    pub(super) canonical_scope_id: String,
    pub(super) shadow_generation_id: String,
    pub(super) prior_generation_id: String,
    pub(super) status: String,
    pub(super) processed_items: u64,
    pub(super) item_budget: u64,
}

pub(super) fn load_rebuild(
    transaction: &Transaction<'_>,
    rebuild_id: &str,
) -> Result<RebuildRow, ActivityProjectionRepositoryError> {
    transaction
        .query_row(
            "SELECT scope_kind,canonical_scope_id,shadow_generation_id,prior_generation_id,
                    status,processed_items,item_budget
             FROM evolution_activity_rebuilds WHERE rebuild_id=?1",
            [rebuild_id],
            |r| {
                Ok(RebuildRow {
                    scope_kind: r.get(0)?,
                    canonical_scope_id: r.get(1)?,
                    shadow_generation_id: r.get(2)?,
                    prior_generation_id: r.get(3)?,
                    status: r.get(4)?,
                    processed_items: r.get::<_, i64>(5)?.max(0) as u64,
                    item_budget: r.get::<_, i64>(6)?.max(0) as u64,
                })
            },
        )
        .optional()?
        .ok_or(ActivityProjectionRepositoryError::InvalidInput)
}

pub(super) fn rebuild_session_id(
    row: &RebuildRow,
) -> Result<String, ActivityProjectionRepositoryError> {
    stable_system_activity_session_id(
        ActivityKind::SkillEvolution,
        parse_enum(&row.scope_kind)?,
        &row.canonical_scope_id,
    )
    .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

pub(super) fn source_snapshot(
    transaction: &Transaction<'_>,
    scope_kind: ActivityScopeKind,
    canonical_scope_id: &str,
) -> Result<std::collections::BTreeMap<String, (i64, u64)>, ActivityProjectionRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT source_domain,MAX(committed_at_ms),MAX(source_sequence)
         FROM evolution_activity_envelopes
         WHERE scope_kind=?1 AND canonical_scope_id=?2 GROUP BY source_domain",
    )?;
    let rows = statement.query_map(params![enum_text(scope_kind)?, canonical_scope_id], |r| {
        Ok((
            r.get::<_, String>(0)?,
            (r.get::<_, i64>(1)?, r.get::<_, i64>(2)?.max(0) as u64),
        ))
    })?;
    rows.collect::<Result<_, _>>()
        .map_err(ActivityProjectionRepositoryError::from)
}

pub(super) fn checkpoint_domain(
    transaction: &Transaction<'_>,
    rebuild_id: &str,
    source_domain: &str,
    event_id: &str,
    now_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    let prior: Option<String> = transaction
        .query_row(
            "SELECT receipt_hash FROM evolution_activity_rebuild_checkpoints
             WHERE rebuild_id=?1 AND source_domain=?2",
            params![rebuild_id, source_domain],
            |r| r.get(0),
        )
        .optional()?;
    let receipt_hash = hash_text(&format!("{}:{event_id}", prior.as_deref().unwrap_or("")));
    transaction.execute(
        "INSERT INTO evolution_activity_rebuild_checkpoints
         (rebuild_id,source_domain,opaque_cursor,high_watermark,processed_items,receipt_hash,
          updated_at_ms)
         VALUES (?1,?2,?3,?3,1,?4,?5)
         ON CONFLICT(rebuild_id,source_domain) DO UPDATE SET opaque_cursor=excluded.opaque_cursor,
           high_watermark=excluded.high_watermark,processed_items=processed_items+1,
           receipt_hash=excluded.receipt_hash,updated_at_ms=excluded.updated_at_ms",
        params![rebuild_id, source_domain, event_id, receipt_hash, now_ms],
    )?;
    Ok(())
}

pub(super) fn validation_failure(
    transaction: &Transaction<'_>,
    row: &RebuildRow,
    session_id: &str,
) -> Result<Option<&'static str>, ActivityProjectionRepositoryError> {
    let expected: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM evolution_activity_envelopes
         WHERE scope_kind=?1 AND canonical_scope_id=?2",
        params![row.scope_kind, row.canonical_scope_id],
        |r| r.get(0),
    )?;
    let (count, max_sequence, distinct): (i64, i64, i64) = transaction.query_row(
        "SELECT COUNT(*),COALESCE(MAX(sequence),0),COUNT(DISTINCT sequence)
         FROM evolution_activity_items WHERE session_id=?1 AND generation_id=?2",
        params![session_id, row.shadow_generation_id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    if count != expected {
        return Ok(Some("item_count_mismatch"));
    }
    if max_sequence != count || distinct != count {
        return Ok(Some("sequence_not_dense"));
    }
    let gaps: i64 = transaction.query_row(
        "SELECT COUNT(*) FROM evolution_activity_domain_cursors WHERE gap_code IS NOT NULL",
        [],
        |r| r.get(0),
    )?;
    if gaps > 0 {
        return Ok(Some("source_gap_open"));
    }
    let mut statement = transaction.prepare(
        "SELECT e.envelope_json FROM evolution_activity_items i
         JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
         WHERE i.session_id=?1 AND i.generation_id=?2",
    )?;
    let envelopes = statement.query_map(params![session_id, row.shadow_generation_id], |r| {
        r.get::<_, String>(0)
    })?;
    for envelope_json in envelopes {
        let envelope: EvolutionActivityEnvelopeV1 = serde_json::from_str(&envelope_json?)
            .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
        if envelope.validate().is_err() {
            return Ok(Some("envelope_invalid"));
        }
    }
    Ok(None)
}

pub(super) fn shadow_hash(
    transaction: &Transaction<'_>,
    session_id: &str,
    shadow_generation_id: &str,
) -> Result<String, ActivityProjectionRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT event_id FROM evolution_activity_items
         WHERE session_id=?1 AND generation_id=?2 ORDER BY sequence",
    )?;
    let ids = statement.query_map(params![session_id, shadow_generation_id], |r| {
        r.get::<_, String>(0)
    })?;
    let mut hasher = Sha256::new();
    for event_id in ids {
        hasher.update(event_id?.as_bytes());
        hasher.update(b"\n");
    }
    Ok(format!(
        "sha256:{}",
        crate::platform::hashing::hex(&hasher.finalize())
    ))
}

pub(super) fn remap_read_state(
    transaction: &Transaction<'_>,
    row: &RebuildRow,
    session_id: &str,
) -> Result<(), ActivityProjectionRepositoryError> {
    let readers = {
        let mut statement = transaction.prepare(
            "SELECT user_id,highest_read_sequence FROM evolution_activity_read_state
             WHERE session_id=?1",
        )?;
        let rows = statement.query_map([session_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?))
        })?;
        rows.collect::<Result<Vec<_>, _>>()?
    };
    if readers.is_empty() {
        return Ok(());
    }
    let rebuilt = load_positions(transaction, session_id, &row.shadow_generation_id)?;
    for (user_id, highest_read) in readers {
        let prior_key = read_order_key(
            transaction,
            session_id,
            &row.prior_generation_id,
            highest_read,
        )?;
        let mapped = map_rebuilt_read_sequence(prior_key.as_ref(), &rebuilt);
        transaction.execute(
            "UPDATE evolution_activity_read_state SET highest_read_sequence=?1,
             revision=revision+1 WHERE session_id=?2 AND user_id=?3",
            params![to_i64(mapped)?, session_id, user_id],
        )?;
    }
    Ok(())
}

fn load_positions(
    transaction: &Transaction<'_>,
    session_id: &str,
    generation_id: &str,
) -> Result<Vec<RebuiltActivityPosition>, ActivityProjectionRepositoryError> {
    let mut statement = transaction.prepare(
        "SELECT i.sequence,e.committed_at_ms,e.source_sequence,e.event_id
         FROM evolution_activity_items i
         JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
         WHERE i.session_id=?1 AND i.generation_id=?2 ORDER BY i.sequence",
    )?;
    let rows = statement.query_map(params![session_id, generation_id], |r| {
        Ok(RebuiltActivityPosition {
            sequence: r.get::<_, i64>(0)?.max(0) as u64,
            source_order: ActivityReadOrderKey {
                committed_at_ms: r.get(1)?,
                source_sequence: r.get::<_, i64>(2)?.max(0) as u64,
                event_id: r.get(3)?,
            },
        })
    })?;
    rows.collect::<Result<_, _>>()
        .map_err(ActivityProjectionRepositoryError::from)
}

fn read_order_key(
    transaction: &Transaction<'_>,
    session_id: &str,
    generation_id: &str,
    sequence: i64,
) -> Result<Option<ActivityReadOrderKey>, ActivityProjectionRepositoryError> {
    if sequence <= 0 {
        return Ok(None);
    }
    transaction
        .query_row(
            "SELECT e.committed_at_ms,e.source_sequence,e.event_id
             FROM evolution_activity_items i
             JOIN evolution_activity_envelopes e ON e.event_id=i.event_id
             WHERE i.session_id=?1 AND i.generation_id=?2 AND i.sequence<=?3
             ORDER BY i.sequence DESC LIMIT 1",
            params![session_id, generation_id, sequence],
            |r| {
                Ok(ActivityReadOrderKey {
                    committed_at_ms: r.get(0)?,
                    source_sequence: r.get::<_, i64>(1)?.max(0) as u64,
                    event_id: r.get(2)?,
                })
            },
        )
        .optional()
        .map_err(ActivityProjectionRepositoryError::from)
}

pub(super) fn hash_text(text: &str) -> String {
    crate::platform::hashing::sha256_tagged(text.as_bytes())
}

pub(super) fn enum_text(
    value: impl serde::Serialize,
) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

pub(super) fn parse_enum<T: serde::de::DeserializeOwned>(
    value: &str,
) -> Result<T, ActivityProjectionRepositoryError> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| ActivityProjectionRepositoryError::Storage)
}

pub(super) fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

pub(super) fn from_i64(value: i64) -> Result<u64, ActivityProjectionRepositoryError> {
    u64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::Storage)
}
