use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::{platform::database::DatabaseError, platform::database::NativeDatabase};

pub(crate) fn apply_notification_schema(connection: &Connection) -> Result<(), DatabaseError> {
    connection.execute_batch(
        "CREATE TABLE IF NOT EXISTS evolution_orchestration_notification_receipts (
           event_id TEXT PRIMARY KEY NOT NULL,
           schema_version INTEGER NOT NULL CHECK(schema_version=1),
           event_kind TEXT NOT NULL CHECK(event_kind IN
             ('run_attention','automatic_application','probation_regression',
              'breaker_opened','breaker_recovered')),
           workspace_id TEXT NOT NULL,
           run_id TEXT,
           application_id TEXT,
           probation_id TEXT,
           breaker_id TEXT,
           skill_id TEXT,
           safe_reason_code TEXT,
           probation_ends_at_ms INTEGER,
           entity_revision INTEGER NOT NULL CHECK(entity_revision>=0),
           delivery_status TEXT NOT NULL CHECK(delivery_status IN ('pending','failed','delivered')),
           created_at_ms INTEGER NOT NULL,
           delivered_at_ms INTEGER
         );
         CREATE INDEX IF NOT EXISTS idx_evolution_notification_delivery
           ON evolution_orchestration_notification_receipts(delivery_status,created_at_ms,event_id);",
    )?;
    Ok(())
}

#[derive(Clone)]
pub(crate) struct EvolutionNotificationRepository {
    database: NativeDatabase,
}

impl EvolutionNotificationRepository {
    pub(crate) fn new(database: NativeDatabase) -> Self {
        Self { database }
    }

    pub(crate) fn pending(&self, now_ms: i64) -> Result<Vec<Value>, String> {
        let mut connection = self.database.connection().map_err(|_| storage())?;
        let transaction = connection.transaction().map_err(|_| storage())?;
        refresh(&transaction, now_ms)?;
        transaction.commit().map_err(|_| storage())?;
        let mut statement = connection.prepare(
            "SELECT event_id,event_kind,workspace_id,run_id,application_id,probation_id,
             breaker_id,skill_id,safe_reason_code,probation_ends_at_ms,entity_revision
             FROM evolution_orchestration_notification_receipts
             WHERE delivery_status IN ('pending','failed') ORDER BY created_at_ms,event_id LIMIT 100",
        ).map_err(|_| storage())?;
        let events = statement.query_map([], |row| Ok(json!({
            "schemaVersion": 1, "eventId": row.get::<_,String>(0)?,
            "eventKind": row.get::<_,String>(1)?, "workspaceId": row.get::<_,String>(2)?,
            "runId": row.get::<_,Option<String>>(3)?,
            "applicationId": row.get::<_,Option<String>>(4)?,
            "probationId": row.get::<_,Option<String>>(5)?,
            "breakerId": row.get::<_,Option<String>>(6)?, "skillId": row.get::<_,Option<String>>(7)?,
            "safeReasonCode": row.get::<_,Option<String>>(8)?,
            "probationEndsAtMs": row.get::<_,Option<i64>>(9)?, "entityRevision": row.get::<_,i64>(10)?,
        }))).map_err(|_| storage())?.collect::<Result<Vec<_>,_>>().map_err(|_| storage())?;
        Ok(events)
    }

    pub(crate) fn finish(
        &self,
        event_id: &str,
        delivered: bool,
        now_ms: i64,
    ) -> Result<(), String> {
        if event_id.is_empty() || event_id.len() > 512 {
            return Err("invalid_input".into());
        }
        let connection = self.database.connection().map_err(|_| storage())?;
        let status = if delivered { "delivered" } else { "failed" };
        let changed = connection
            .execute(
                "UPDATE evolution_orchestration_notification_receipts
             SET delivery_status=?1,delivered_at_ms=CASE WHEN ?1='delivered' THEN ?2 ELSE NULL END
             WHERE event_id=?3 AND delivery_status IN ('pending','failed')",
                params![status, now_ms, event_id],
            )
            .map_err(|_| storage())?;
        if changed == 1 {
            Ok(())
        } else {
            Err("stale_conflict".into())
        }
    }
}

fn refresh(transaction: &rusqlite::Transaction<'_>, now_ms: i64) -> Result<(), String> {
    transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_orchestration_notification_receipts
         SELECT 'run_attention:'||run_id||':'||revision,1,'run_attention',workspace_id,
         run_id,NULL,NULL,NULL,NULL,safe_failure_code,NULL,revision,'pending',?1,NULL
         FROM evolution_runs WHERE status IN ('partial','failed')",
            [now_ms],
        )
        .map_err(|_| storage())?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_orchestration_notification_receipts
         SELECT 'automatic_application:'||a.application_id,1,'automatic_application',r.workspace_id,
         a.run_id,a.application_id,p.probation_id,NULL,a.target_skill_id,NULL,p.ends_at_ms,0,
         'pending',?1,NULL FROM evolution_auto_applications a
         JOIN evolution_runs r ON r.run_id=a.run_id
         LEFT JOIN evolution_auto_probations p ON p.application_id=a.application_id",
            [now_ms],
        )
        .map_err(|_| storage())?;
    transaction.execute(
        "INSERT OR IGNORE INTO evolution_orchestration_notification_receipts
         SELECT 'probation_regression:'||probation_id||':'||revision,1,'probation_regression',
         workspace_id,NULL,application_id,probation_id,NULL,skill_id,'verified_regression',ends_at_ms,
         revision,'pending',?1,NULL FROM evolution_auto_probations WHERE status='regressed'",
        [now_ms],
    ).map_err(|_| storage())?;
    transaction
        .execute(
            "INSERT OR IGNORE INTO evolution_orchestration_notification_receipts
         SELECT CASE WHEN status='awaiting_acknowledgement' AND health_probe_passed=1
           THEN 'breaker_recovered:' ELSE 'breaker_opened:' END||breaker_id||':'||revision,
         1,CASE WHEN status='awaiting_acknowledgement' AND health_probe_passed=1
           THEN 'breaker_recovered' ELSE 'breaker_opened' END,workspace_id,NULL,NULL,NULL,
         breaker_id,skill_id,safe_cause_code,NULL,revision,'pending',?1,NULL
         FROM evolution_auto_breakers WHERE status!='closed'",
            [now_ms],
        )
        .map_err(|_| storage())?;
    Ok(())
}

fn storage() -> String {
    "storage_unavailable".into()
}
