use rusqlite::OptionalExtension;

use super::*;

impl SqliteActivityProjectionRepository<'_> {
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
        let session_id = rebuild_session_id(&row)?;
        if let Some(code) = validation_failure(&transaction, &row, &session_id)? {
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

pub(super) fn copy_dashboard_state(
    transaction: &rusqlite::Transaction<'_>,
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
