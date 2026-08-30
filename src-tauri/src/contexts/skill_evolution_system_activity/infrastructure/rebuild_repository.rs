use rusqlite::{params, OptionalExtension, Transaction};
use sha2::{Digest, Sha256};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

/// Where a rebuild attempt stands after one bounded call. `NeedsCatchUp` is not a failure: new
/// source events committed while the shadow was building, and the caller advances again before
/// asking for activation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ActivityRebuildStep {
    Running { processed_items: u64 },
    Validating,
    Ready,
    NeedsCatchUp,
    Active,
}

impl SqliteActivityProjectionRepository<'_> {
    /// Starts a scoped shadow rebuild. The current generation stays active and readable; the
    /// shadow generation receives reprojected items from retained canonical envelopes only, so no
    /// model call, assessment, generation, Curator decision, or Overlay action can occur.
    pub(crate) fn begin_rebuild(
        &self,
        scope_kind: ActivityScopeKind,
        canonical_scope_id: &str,
        item_budget: u64,
        now_ms: i64,
    ) -> Result<ActivityRebuild, ActivityProjectionRepositoryError> {
        if now_ms < 0
            || item_budget == 0
            || sanitize_text(canonical_scope_id, "rebuild.scope", 200).is_err()
        {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            scope_kind,
            canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let prior_generation_id: String = transaction
            .query_row(
                "SELECT active_generation_id FROM evolution_system_activity_sessions
                 WHERE session_id=?1",
                [&session_id],
                |row| row.get(0),
            )
            .optional()?
            .ok_or(ActivityProjectionRepositoryError::InvalidInput)?;
        let snapshot = source_snapshot(&transaction, scope_kind, canonical_scope_id)?;
        let snapshot_json = serde_json::to_string(&snapshot)
            .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
        let rebuild_id = format!("rebuild:{session_id}:{now_ms}");
        let shadow_generation_id = format!("{prior_generation_id}:rebuild:{now_ms}");
        transaction.execute(
            "INSERT INTO evolution_activity_rebuilds
             (rebuild_id,scope_kind,canonical_scope_id,source_snapshot_json,source_snapshot_hash,
              shadow_generation_id,prior_generation_id,status,item_budget,created_at_ms,
              updated_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,'running',?8,?9,?9)",
            params![
                rebuild_id,
                enum_text(scope_kind)?,
                canonical_scope_id,
                snapshot_json,
                hash_text(&snapshot_json),
                shadow_generation_id,
                prior_generation_id,
                to_i64(item_budget)?,
                now_ms,
            ],
        )?;
        transaction.commit()?;
        self.rebuild(&rebuild_id)?
            .ok_or(ActivityProjectionRepositoryError::Storage)
    }

    /// Reprojects the next bounded batch of retained envelopes into the shadow generation and
    /// checkpoints afterwards, so a crash resumes instead of restarting. Returns `Validating`
    /// once every retained envelope for the scope is present in the shadow.
    pub(crate) fn advance_rebuild(
        &self,
        rebuild_id: &str,
        batch_limit: u64,
        now_ms: i64,
    ) -> Result<ActivityRebuildStep, ActivityProjectionRepositoryError> {
        if now_ms < 0 || batch_limit == 0 {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let transaction = self.connection.unchecked_transaction()?;
        let row = load_rebuild(&transaction, rebuild_id)?;
        if row.status != "running" {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            parse_enum(&row.scope_kind)?,
            &row.canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let remaining_budget = row.item_budget.saturating_sub(row.processed_items);
        let limit = to_i64(batch_limit.min(remaining_budget.max(1)))?;
        let batch = {
            let mut statement = transaction.prepare(
                "SELECT e.event_id,e.source_domain,e.created FROM (
                     SELECT event_id,source_domain,committed_at_ms AS created,source_sequence
                     FROM evolution_activity_envelopes
                     WHERE scope_kind=?1 AND canonical_scope_id=?2
                 ) e
                 LEFT JOIN evolution_activity_items i
                   ON i.event_id=e.event_id AND i.session_id=?3 AND i.generation_id=?4
                 WHERE i.item_id IS NULL
                 ORDER BY e.created,e.source_sequence,e.event_id
                 LIMIT ?5",
            )?;
            let rows = statement.query_map(
                params![
                    row.scope_kind,
                    row.canonical_scope_id,
                    session_id,
                    row.shadow_generation_id,
                    limit
                ],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
            )?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        if batch.is_empty() {
            transaction.execute(
                "UPDATE evolution_activity_rebuilds SET status='validating',updated_at_ms=?1
                 WHERE rebuild_id=?2",
                params![now_ms, rebuild_id],
            )?;
            transaction.commit()?;
            return Ok(ActivityRebuildStep::Validating);
        }
        let mut next_sequence: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM evolution_activity_items
             WHERE session_id=?1 AND generation_id=?2",
            params![session_id, row.shadow_generation_id],
            |r| r.get(0),
        )?;
        let mut processed = row.processed_items;
        for (event_id, source_domain) in &batch {
            next_sequence += 1;
            let item_id = stable_activity_item_id(&session_id, &row.shadow_generation_id, event_id);
            // The supersession relation is part of the canonical envelope and must survive
            // reprojection, so the shadow item re-reads it from the envelope body instead of
            // defaulting to NULL and silently flattening every supersession chain.
            transaction.execute(
                "INSERT INTO evolution_activity_items
                 (item_id,session_id,generation_id,sequence,event_id,supersedes_event_id,
                  created_at_ms)
                 SELECT ?1,?2,?3,?4,event_id,
                        json_extract(envelope_json,'$.supersedesEventId'),?5
                 FROM evolution_activity_envelopes
                 WHERE event_id=?6",
                params![
                    item_id,
                    session_id,
                    row.shadow_generation_id,
                    next_sequence,
                    now_ms,
                    event_id,
                ],
            )?;
            processed = processed.saturating_add(1);
            checkpoint_domain(&transaction, rebuild_id, source_domain, event_id, now_ms)?;
        }
        transaction.execute(
            "UPDATE evolution_activity_rebuilds SET processed_items=?1,updated_at_ms=?2
             WHERE rebuild_id=?3",
            params![to_i64(processed)?, now_ms, rebuild_id],
        )?;
        transaction.commit()?;
        Ok(ActivityRebuildStep::Running {
            processed_items: processed,
        })
    }

    /// Validates the shadow generation against retained sources: item count, dense ordering,
    /// envelope integrity, and no open source gap. A mismatch marks the rebuild failed and leaves
    /// the prior generation active.
    pub(crate) fn validate_rebuild(
        &self,
        rebuild_id: &str,
        now_ms: i64,
    ) -> Result<ActivityRebuildStep, ActivityProjectionRepositoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let row = load_rebuild(&transaction, rebuild_id)?;
        if row.status != "validating" {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            parse_enum(&row.scope_kind)?,
            &row.canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let failure = validation_failure(&transaction, &row, &session_id)?;
        if let Some(code) = failure {
            transaction.execute(
                "UPDATE evolution_activity_rebuilds
                 SET status='failed',failure_code=?1,updated_at_ms=?2 WHERE rebuild_id=?3",
                params![code, now_ms, rebuild_id],
            )?;
            transaction.commit()?;
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        let validation_hash = shadow_hash(&transaction, &session_id, &row.shadow_generation_id)?;
        transaction.execute(
            "UPDATE evolution_activity_rebuilds
             SET status='ready',validation_hash=?1,updated_at_ms=?2 WHERE rebuild_id=?3",
            params![validation_hash, now_ms, rebuild_id],
        )?;
        transaction.commit()?;
        Ok(ActivityRebuildStep::Ready)
    }

    /// Atomically activates a validated, gap-free shadow generation. Read cursors are remapped by
    /// source order so nothing becomes newly unread merely because of rebuild; notification
    /// receipts are keyed by event identity and are untouched, so nothing is re-sent. The prior
    /// generation's items are retained through the recovery window rather than deleted.
    pub(crate) fn activate_rebuild(
        &self,
        rebuild_id: &str,
        now_ms: i64,
    ) -> Result<ActivityRebuildStep, ActivityProjectionRepositoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let row = load_rebuild(&transaction, rebuild_id)?;
        if row.status != "ready" {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        let session_id = stable_system_activity_session_id(
            ActivityKind::SkillEvolution,
            parse_enum(&row.scope_kind)?,
            &row.canonical_scope_id,
        )
        .map_err(|_| ActivityProjectionRepositoryError::InvalidInput)?;
        let missing: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM evolution_activity_envelopes e
             LEFT JOIN evolution_activity_items i
               ON i.event_id=e.event_id AND i.session_id=?1 AND i.generation_id=?2
             WHERE e.scope_kind=?3 AND e.canonical_scope_id=?4 AND i.item_id IS NULL",
            params![
                session_id,
                row.shadow_generation_id,
                row.scope_kind,
                row.canonical_scope_id
            ],
            |r| r.get(0),
        )?;
        if missing > 0 {
            transaction.execute(
                "UPDATE evolution_activity_rebuilds SET status='running',validation_hash=NULL,
                 updated_at_ms=?1 WHERE rebuild_id=?2",
                params![now_ms, rebuild_id],
            )?;
            transaction.commit()?;
            return Ok(ActivityRebuildStep::NeedsCatchUp);
        }
        remap_read_state(&transaction, &row, &session_id)?;
        copy_dashboard_state(&transaction, &row, now_ms)?;
        let last_sequence: i64 = transaction.query_row(
            "SELECT COALESCE(MAX(sequence),0) FROM evolution_activity_items
             WHERE session_id=?1 AND generation_id=?2",
            params![session_id, row.shadow_generation_id],
            |r| r.get(0),
        )?;
        transaction.execute(
            "UPDATE evolution_system_activity_sessions
             SET active_generation_id=?1,last_sequence=?2,last_projected_at_ms=?3
             WHERE session_id=?4",
            params![row.shadow_generation_id, last_sequence, now_ms, session_id],
        )?;
        transaction.execute(
            "UPDATE evolution_activity_rebuilds SET status='active',updated_at_ms=?1
             WHERE rebuild_id=?2",
            params![now_ms, rebuild_id],
        )?;
        transaction.commit()?;
        self.project_unread(&session_id, super::LOCAL_ACTIVITY_USER_ID, now_ms)?;
        Ok(ActivityRebuildStep::Active)
    }

    /// Cancels a rebuild that has not activated: the shadow generation's items are removed and
    /// the active generation is untouched.
    pub(crate) fn cancel_rebuild(
        &self,
        rebuild_id: &str,
        now_ms: i64,
    ) -> Result<(), ActivityProjectionRepositoryError> {
        let transaction = self.connection.unchecked_transaction()?;
        let row = load_rebuild(&transaction, rebuild_id)?;
        if row.status == "active" {
            return Err(ActivityProjectionRepositoryError::Conflict);
        }
        transaction.execute(
            "DELETE FROM evolution_activity_items WHERE generation_id=?1",
            [&row.shadow_generation_id],
        )?;
        transaction.execute(
            "UPDATE evolution_activity_rebuilds SET status='cancelled',updated_at_ms=?1
             WHERE rebuild_id=?2",
            params![now_ms, rebuild_id],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub(crate) fn rebuild(
        &self,
        rebuild_id: &str,
    ) -> Result<Option<ActivityRebuild>, ActivityProjectionRepositoryError> {
        let row = self
            .connection
            .query_row(
                "SELECT rebuild_id,scope_kind,canonical_scope_id,shadow_generation_id,
                        source_snapshot_hash,status,processed_items,item_budget,revision
                 FROM evolution_activity_rebuilds WHERE rebuild_id=?1",
                [rebuild_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                        r.get::<_, String>(5)?,
                        r.get::<_, i64>(6)?,
                        r.get::<_, i64>(7)?,
                        r.get::<_, i64>(8)?,
                    ))
                },
            )
            .optional()?;
        let Some((id, scope_kind, scope_id, shadow, snapshot_hash, status, processed, budget, rev)) =
            row
        else {
            return Ok(None);
        };
        Ok(Some(ActivityRebuild {
            rebuild_id: id,
            scope_kind: parse_enum(&scope_kind)?,
            canonical_scope_id: scope_id,
            shadow_generation_id: shadow,
            source_snapshot_hash: snapshot_hash,
            status: parse_enum(&status)?,
            processed_items: from_i64(processed)?,
            item_budget: from_i64(budget)?,
            revision: from_i64(rev)?,
        }))
    }
}

struct RebuildRow {
    scope_kind: String,
    canonical_scope_id: String,
    shadow_generation_id: String,
    prior_generation_id: String,
    status: String,
    processed_items: u64,
    item_budget: u64,
}

fn load_rebuild(
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

fn source_snapshot(
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

fn checkpoint_domain(
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

fn validation_failure(
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

fn shadow_hash(
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
    Ok(format!("sha256:{}", hex_bytes(&hasher.finalize())))
}

fn remap_read_state(
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

fn copy_dashboard_state(
    transaction: &Transaction<'_>,
    row: &RebuildRow,
    now_ms: i64,
) -> Result<(), ActivityProjectionRepositoryError> {
    transaction.execute(
        "INSERT OR IGNORE INTO evolution_activity_dashboard_state
         (scope_kind,canonical_scope_id,generation_id,materialization_kind,state_json,
          last_event_id,updated_at_ms,revision)
         SELECT scope_kind,canonical_scope_id,?1,materialization_kind,state_json,last_event_id,
                ?2,1
         FROM evolution_activity_dashboard_state
         WHERE scope_kind=?3 AND canonical_scope_id=?4 AND generation_id=?5",
        params![
            row.shadow_generation_id,
            now_ms,
            row.scope_kind,
            row.canonical_scope_id,
            row.prior_generation_id,
        ],
    )?;
    Ok(())
}

fn hash_text(text: &str) -> String {
    format!("sha256:{}", hex_bytes(&Sha256::digest(text.as_bytes())))
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn enum_text(value: impl serde::Serialize) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn parse_enum<T: serde::de::DeserializeOwned>(
    value: &str,
) -> Result<T, ActivityProjectionRepositoryError> {
    serde_json::from_value(serde_json::Value::String(value.into()))
        .map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn to_i64(value: u64) -> Result<i64, ActivityProjectionRepositoryError> {
    i64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::InvalidInput)
}

fn from_i64(value: i64) -> Result<u64, ActivityProjectionRepositoryError> {
    u64::try_from(value).map_err(|_| ActivityProjectionRepositoryError::Storage)
}
