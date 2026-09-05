use super::api::SkillEvolutionOrchestrationApi;
use rusqlite::{params, OptionalExtension};
use serde_json::{json, Value};

impl SkillEvolutionOrchestrationApi {
    pub(crate) fn scheduler_overview(&self, workspace_id: &str) -> Result<Value, String> {
        validate_id(workspace_id)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mode = connection
            .query_row(
                "SELECT mode FROM evolution_orchestration_policy WHERE workspace_id=?1",
                [workspace_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| storage())?
            .unwrap_or_else(|| "off".into());
        let pending_triggers = connection
            .query_row(
                "SELECT COUNT(*) FROM evolution_run_requests
             WHERE workspace_id=?1 AND status IN ('pending','claimed')",
                [workspace_id],
                |row| row.get::<_, i64>(0),
            )
            .map_err(|_| storage())?;
        let active_run_id = connection
            .query_row(
                "SELECT run_id FROM evolution_runs WHERE workspace_id=?1 AND status IN
             ('requested','waiting_idle','running','partial','cancel_requested','recovered')
             ORDER BY updated_at_ms DESC LIMIT 1",
                [workspace_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|_| storage())?;
        let breakers_open = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM evolution_auto_breakers WHERE workspace_id=?1
             AND status!='closed')",
                [workspace_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(|_| storage())?;
        let idle_gate = if active_run_id.is_some() {
            "waiting"
        } else {
            "unavailable"
        };
        let trigger_counters = trigger_counters(&connection, workspace_id)?;
        let idle = if active_run_id.is_some() {
            json!({ "state": "waiting", "safeReasons": ["active_run"] })
        } else {
            json!({ "state": "unavailable", "safeReasons": ["runtime_snapshot_not_persisted"] })
        };
        Ok(json!({ "workspaceId": workspace_id, "mode": mode,
            "pendingTriggers": pending_triggers, "activeRunId": active_run_id,
            "idleGate": idle_gate,
            "automaticMutationAvailable": mode == "enabled" && !breakers_open && idle_gate == "ready",
            "triggerCounters": trigger_counters, "idle": idle }))
    }

    pub(crate) fn policy_projection(
        &self,
        workspace_id: &str,
        now_ms: i64,
    ) -> Result<Value, String> {
        let policy = crate::contexts::skill_evolution_orchestration::infrastructure::OrchestrationRepository::new(
            self.database.clone(),
        ).policy(workspace_id, now_ms).map_err(map_persistence)?;
        Ok(
            json!({ "workspaceId": policy.workspace_id, "mode": policy.mode,
            "allowedSkillIds": policy.allowed_skill_ids,
            "consent": policy.consent.map(|consent| json!({
                "disclosureVersion": consent.disclosure_version,
                "disclosureHash": consent.witness_hash,
                "acceptedAtMs": consent.acknowledged_at_ms,
            })), "revision": policy.revision, "updatedAtMs": policy.updated_at_ms }),
        )
    }

    pub(crate) fn runs(
        &self,
        workspace_id: &str,
        cursor: Option<&str>,
        limit: usize,
    ) -> Result<Value, String> {
        validate_query(workspace_id, cursor, limit)?;
        let offset = parse_cursor(cursor)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection.prepare(
            "SELECT run_id,workspace_id,status,current_stage,policy_witness_hash,safe_failure_code,
             budget_json,usage_json,revision,created_at_ms,updated_at_ms FROM evolution_runs WHERE workspace_id=?1
             ORDER BY updated_at_ms DESC,run_id LIMIT ?2 OFFSET ?3",
        ).map_err(|_| storage())?;
        let items = statement.query_map(params![workspace_id, limit as i64, offset as i64], |row| Ok(json!({
            "runId": row.get::<_, String>(0)?, "workspaceId": row.get::<_, String>(1)?,
            "status": row.get::<_, String>(2)?, "currentStage": row.get::<_, Option<String>>(3)?,
            "policyWitnessHash": row.get::<_, String>(4)?, "safeFailureCode": row.get::<_, Option<String>>(5)?,
            "budget": json_column(row, 6)?, "usage": json_column(row, 7)?,
            "revision": row.get::<_, i64>(8)?, "createdAtMs": row.get::<_, i64>(9)?,
            "updatedAtMs": row.get::<_, i64>(10)? }))).map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>().map_err(|_| storage())?;
        page(items, offset, limit)
    }

    pub(crate) fn run_detail(&self, run_id: &str) -> Result<Value, String> {
        validate_id(run_id)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let run = connection.query_row(
            "SELECT run_id,workspace_id,status,current_stage,policy_witness_hash,safe_failure_code,
             budget_json,usage_json,revision,created_at_ms,updated_at_ms FROM evolution_runs WHERE run_id=?1", [run_id],
            |row| Ok(json!({ "runId": row.get::<_,String>(0)?, "workspaceId": row.get::<_,String>(1)?,
                "status": row.get::<_,String>(2)?, "currentStage": row.get::<_,Option<String>>(3)?,
                "policyWitnessHash": row.get::<_,String>(4)?, "safeFailureCode": row.get::<_,Option<String>>(5)?,
                "budget": json_column(row, 6)?, "usage": json_column(row, 7)?,
                "revision": row.get::<_,i64>(8)?, "createdAtMs": row.get::<_,i64>(9)?,
                "updatedAtMs": row.get::<_,i64>(10)? })),
        ).optional().map_err(|_| storage())?.ok_or_else(|| "not_found".to_string())?;
        let stages = rows_json(
            &connection,
            "SELECT stage_id,stage,attempt,status,safe_failure_code,started_at_ms,completed_at_ms
             FROM evolution_run_stages WHERE run_id=?1 ORDER BY rowid",
            run_id,
            |row| {
                Ok(json!({
                "stageId": row.get::<_,String>(0)?, "runId": run_id,
                "stage": row.get::<_,String>(1)?, "attempt": row.get::<_,i64>(2)?,
                "status": row.get::<_,String>(3)?, "safeFailureCode": row.get::<_,Option<String>>(4)?,
                "startedAtMs": row.get::<_,Option<i64>>(5)?,
                "completedAtMs": row.get::<_,Option<i64>>(6)? }))
            },
        )?;
        let checkpoints = rows_json(&connection,
            "SELECT checkpoint_id,stage,status,cursor_record_id,continuation_not_before_ms,committed_at_ms
             FROM evolution_run_checkpoints WHERE run_id=?1 ORDER BY committed_at_ms", run_id, |row| Ok(json!({
                "checkpointId": row.get::<_,String>(0)?, "runId": run_id,
                "stage": row.get::<_,String>(1)?, "status": row.get::<_,String>(2)?,
                "cursorRecordId": row.get::<_,Option<String>>(3)?,
                "continuationNotBeforeMs": row.get::<_,Option<i64>>(4)?,
                "committedAtMs": row.get::<_,i64>(5)? })))?;
        let mut object = run.as_object().cloned().ok_or_else(storage)?;
        object.insert("stages".into(), Value::Array(stages));
        object.insert("checkpoints".into(), Value::Array(checkpoints));
        Ok(Value::Object(object))
    }
}

fn rows_json(
    connection: &rusqlite::Connection,
    sql: &str,
    id: &str,
    project: impl Fn(&rusqlite::Row<'_>) -> rusqlite::Result<Value>,
) -> Result<Vec<Value>, String> {
    let mut statement = connection.prepare(sql).map_err(|_| storage())?;
    let rows = statement
        .query_map([id], project)
        .map_err(|_| storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| storage())?;
    Ok(rows)
}

fn json_column(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<Value> {
    let encoded = row.get::<_, String>(index)?;
    serde_json::from_str(&encoded).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })
}

fn trigger_counters(
    connection: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Value, String> {
    let mut counters = serde_json::Map::from_iter([
        ("startupRecovery".into(), json!(0)),
        ("periodicMaintenance".into(), json!(0)),
        ("applicationIdleTransition".into(), json!(0)),
        ("agentRunCompletion".into(), json!(0)),
        ("conversationCompletion".into(), json!(0)),
        ("explicitFeedbackCommit".into(), json!(0)),
        ("verificationCompletion".into(), json!(0)),
        ("delegatedUtilityCompletion".into(), json!(0)),
        ("relevantPolicyOrSkillChange".into(), json!(0)),
        ("manualRunRequest".into(), json!(0)),
    ]);
    let mut statement = connection.prepare(
        "SELECT family,COUNT(*) FROM evolution_trigger_receipts WHERE workspace_id=?1 GROUP BY family",
    ).map_err(|_| storage())?;
    let rows = statement
        .query_map([workspace_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })
        .map_err(|_| storage())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| storage())?;
    for (family, count) in rows {
        let key = match family.as_str() {
            "startup_recovery" => "startupRecovery",
            "periodic_maintenance" => "periodicMaintenance",
            "application_idle_transition" => "applicationIdleTransition",
            "agent_run_completion" => "agentRunCompletion",
            "conversation_completion" => "conversationCompletion",
            "explicit_feedback_commit" => "explicitFeedbackCommit",
            "verification_completion" => "verificationCompletion",
            "delegated_utility_completion" => "delegatedUtilityCompletion",
            "relevant_policy_or_skill_change" => "relevantPolicyOrSkillChange",
            "manual_run_request" => "manualRunRequest",
            _ => return Err(storage()),
        };
        counters.insert(key.into(), json!(count));
    }
    Ok(Value::Object(counters))
}

pub(super) fn validate_query(
    workspace_id: &str,
    cursor: Option<&str>,
    limit: usize,
) -> Result<(), String> {
    validate_id(workspace_id)?;
    if !(1..=100).contains(&limit) || cursor.is_some_and(|value| value.parse::<usize>().is_err()) {
        return Err("invalid_input".into());
    }
    Ok(())
}
pub(super) fn validate_id(value: &str) -> Result<(), String> {
    if value.is_empty() || value.len() > 256 {
        Err("invalid_input".into())
    } else {
        Ok(())
    }
}
pub(super) fn parse_cursor(value: Option<&str>) -> Result<usize, String> {
    value
        .unwrap_or("0")
        .parse()
        .map_err(|_| "invalid_input".into())
}
pub(super) fn page(items: Vec<Value>, offset: usize, limit: usize) -> Result<Value, String> {
    let count = items.len();
    Ok(
        json!({ "items": items, "nextCursor": (count == limit).then(|| (offset + count).to_string()) }),
    )
}
pub(super) fn map_persistence(
    error: crate::contexts::skill_evolution_orchestration::infrastructure::OrchestrationPersistenceError,
) -> String {
    match error { crate::contexts::skill_evolution_orchestration::infrastructure::OrchestrationPersistenceError::InvalidInput => "invalid_input",
        crate::contexts::skill_evolution_orchestration::infrastructure::OrchestrationPersistenceError::Conflict => "stale_conflict",
        crate::contexts::skill_evolution_orchestration::infrastructure::OrchestrationPersistenceError::NotFound => "not_found",
        _ => "storage_unavailable" }.into()
}
fn storage() -> String {
    "storage_unavailable".into()
}
