use rusqlite::{params, OptionalExtension};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

mod lifecycle;
mod support;
use lifecycle::copy_dashboard_state;
use support::*;

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
}
