use super::api_models::*;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

const MAX_PAGE: usize = 100;

pub(super) fn queue(connection: &Connection, query: CuratorQueueQuery) -> CuratorApiResult {
    let limit = bounded_limit(query.limit)?;
    let offset = cursor(query.cursor.as_deref())?;
    validate_queue(&query)?;
    let mut statement = connection.prepare(
        "SELECT c.candidate_id,c.target_skill_id,c.state,c.route,c.risk,EXISTS(SELECT 1 FROM
         evolution_curator_draft_assessments a WHERE a.candidate_id=c.candidate_id
         AND a.approvable=1 AND a.invalidated_at_ms IS NULL),
         staleness_json,revision,updated_at_ms,EXISTS(SELECT 1 FROM evolution_curator_notification_receipts n
         WHERE n.candidate_id=c.candidate_id AND n.candidate_revision=c.revision AND n.delivery_status='pending')
         FROM evolution_curator_candidates c WHERE c.workspace_id=?1
         ORDER BY CASE state WHEN 'ready_for_review' THEN 0 WHEN 'apply_failed' THEN 1
         WHEN 'awaiting_draft' THEN 2 WHEN 'pending' THEN 3 WHEN 'deferred' THEN 4 ELSE 5 END,
         CASE risk WHEN 'high' THEN 0 WHEN 'medium' THEN 1 ELSE 2 END,updated_at_ms,candidate_id",
    ).map_err(|_| storage())?;
    let rows = statement.query_map([&query.workspace_id], |row| {
        Ok(json!({"candidateId":row.get::<_,String>(0)?,"targetSkillId":row.get::<_,String>(1)?,
            "state":row.get::<_,String>(2)?,"route":row.get::<_,String>(3)?,"risk":row.get::<_,String>(4)?,
            "draftReady":row.get::<_,bool>(5)?,"staleness":serde_json::from_str::<Value>(&row.get::<_,String>(6)?)
                .unwrap_or_else(|_| json!([])),"revision":row.get::<_,i64>(7)?,"updatedAtMs":row.get::<_,i64>(8)?,
            "notificationPending":row.get::<_,bool>(9)?}))
    }).map_err(|_| storage())?.collect::<Result<Vec<_>,_>>().map_err(|_| storage())?;
    let filtered = rows
        .into_iter()
        .filter(|item| matches_query(item, &query))
        .collect::<Vec<_>>();
    let total = filtered.len();
    let items = filtered
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(strip_internal)
        .collect::<Vec<_>>();
    let next = (offset + items.len() < total).then(|| (offset + items.len()).to_string());
    Ok(json!({"items":items,"nextCursor":next,"totalCount":total,"complete":next.is_none()}))
}

pub(super) fn detail(connection: &Connection, candidate_id: &str) -> CuratorApiResult {
    if candidate_id.trim().is_empty() {
        return Err(invalid());
    }
    let row = connection.query_row(
        "SELECT snapshot_json,state,staleness_json,revision,updated_at_ms,current_preview_id,created_at_ms
         FROM evolution_curator_candidates WHERE candidate_id=?1", [candidate_id], |row| Ok((
            row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,i64>(3)?,
            row.get::<_,i64>(4)?,row.get::<_,Option<String>>(5)?,row.get::<_,i64>(6)?)))
        .optional().map_err(|_| storage())?.ok_or_else(not_found)?;
    let mut value = serde_json::from_str::<Value>(&row.0).map_err(|_| storage())?;
    let object = value.as_object_mut().ok_or_else(storage)?;
    object.insert("state".into(), json!(row.1));
    object.insert(
        "staleness".into(),
        serde_json::from_str(&row.2).map_err(|_| storage())?,
    );
    object.insert("revision".into(), json!(row.3));
    object.insert("updatedAtMs".into(), json!(row.4));
    object.insert(
        "draftReady".into(),
        json!(draft_ready(connection, candidate_id)?),
    );
    object.insert(
        "drafts".into(),
        Value::Array(drafts(connection, candidate_id)?),
    );
    object.insert("createdAtMs".into(), json!(row.6));
    if let Some(preview_id) = row.5 {
        object.insert("currentPreview".into(), preview(connection, &preview_id)?);
    }
    if let Some(application) = application(connection, candidate_id)? {
        object.insert("application".into(), application);
    }
    Ok(value)
}

pub(super) fn audit(connection: &Connection, query: CuratorAuditQuery) -> CuratorApiResult {
    let limit = bounded_limit(query.limit)?;
    let offset = cursor(query.cursor.as_deref())?;
    if query.candidate_id.trim().is_empty() {
        return Err(invalid());
    }
    let mut statement = connection.prepare("SELECT sequence,event_kind,actor_class,occurred_at_ms,prior_state,
        next_state,object_revision,reason_code,event_hash FROM evolution_curator_events WHERE candidate_id=?1
        ORDER BY sequence LIMIT ?2 OFFSET ?3").map_err(|_| storage())?;
    let rows = statement.query_map(params![query.candidate_id,(limit+1) as i64,offset as i64], |row| Ok(json!({
        "sequence":row.get::<_,i64>(0)?,"eventKind":row.get::<_,String>(1)?,"actorClass":row.get::<_,String>(2)?,
        "occurredAtMs":row.get::<_,i64>(3)?,"priorState":row.get::<_,Option<String>>(4)?,
        "nextState":row.get::<_,String>(5)?,"objectRevision":row.get::<_,i64>(6)?,
        "reasonCode":row.get::<_,Option<String>>(7)?,"eventHash":row.get::<_,String>(8)?
    }))).map_err(|_| storage())?.collect::<Result<Vec<_>,_>>().map_err(|_| storage())?;
    let more = rows.len() > limit;
    Ok(
        json!({"items":rows.into_iter().take(limit).collect::<Vec<_>>(),
        "nextCursor":more.then(||(offset+limit).to_string()),"complete":!more}),
    )
}

pub(super) fn safe_state(connection: &Connection, candidate_id: &str) -> Option<CuratorSafeState> {
    connection
        .query_row(
            "SELECT candidate_id,revision,state,witness_hash,policy_witness_hash,current_preview_id
        FROM evolution_curator_candidates WHERE candidate_id=?1",
            [candidate_id],
            |row| {
                Ok(CuratorSafeState {
                    candidate_id: row.get(0)?,
                    revision: positive_u64(row, 1)?,
                    state: row.get(2)?,
                    witness_hash: row.get(3)?,
                    policy_witness_hash: row.get(4)?,
                    current_preview_id: row.get(5)?,
                })
            },
        )
        .optional()
        .ok()
        .flatten()
}

fn matches_query(item: &Value, query: &CuratorQueueQuery) -> bool {
    let text = |key| item.get(key).and_then(Value::as_str).unwrap_or("");
    let flag = |key| item.get(key).and_then(Value::as_bool).unwrap_or(false);
    query
        .skill_id
        .as_deref()
        .is_none_or(|value| text("targetSkillId") == value)
        && (query.states.is_empty() || query.states.iter().any(|value| value == text("state")))
        && (query.routes.is_empty() || query.routes.iter().any(|value| value == text("route")))
        && (query.risks.is_empty() || query.risks.iter().any(|value| value == text("risk")))
        && query
            .draft_ready
            .is_none_or(|value| flag("draftReady") == value)
        && query.stale.is_none_or(|value| {
            item["staleness"].as_array().is_some_and(|v| !v.is_empty()) == value
        })
        && query
            .notification_pending
            .is_none_or(|value| flag("notificationPending") == value)
        && query.updated_before_ms.is_none_or(|value| {
            item["updatedAtMs"]
                .as_i64()
                .is_some_and(|time| time < value)
        })
}

fn validate_queue(query: &CuratorQueueQuery) -> Result<(), CuratorApiError> {
    let allowed_states = [
        "pending",
        "awaiting_draft",
        "ready_for_review",
        "deferred",
        "rejected",
        "applying",
        "applied",
        "apply_failed",
        "superseded",
    ];
    let allowed_routes = ["advance", "needs_human_review"];
    let allowed_risks = ["low", "medium", "high"];
    if query.workspace_id.trim().is_empty()
        || !query
            .states
            .iter()
            .all(|v| allowed_states.contains(&v.as_str()))
        || !query
            .routes
            .iter()
            .all(|v| allowed_routes.contains(&v.as_str()))
        || !query
            .risks
            .iter()
            .all(|v| allowed_risks.contains(&v.as_str()))
    {
        return Err(invalid());
    }
    Ok(())
}

fn drafts(connection: &Connection, candidate_id: &str) -> Result<Vec<Value>, CuratorApiError> {
    let mut statement = connection.prepare("SELECT draft_id,revision,kind,validated_body_json,rationale,
        expected_effective_change,body_hash,created_at_ms FROM evolution_curator_drafts WHERE candidate_id=?1 ORDER BY revision DESC")
        .map_err(|_| storage())?;
    let rows = statement.query_map([candidate_id], |row| { let body:String=row.get(3)?; Ok(json!({"draftId":row.get::<_,String>(0)?,
        "revision":row.get::<_,i64>(1)?,"kind":row.get::<_,String>(2)?,"mutation":mutation(&row.get::<_,String>(2)?,&body),
        "rationale":row.get::<_,String>(4)?,"expectedEffectiveChange":row.get::<_,String>(5)?,
        "bodyHash":row.get::<_,String>(6)?,"createdAtMs":row.get::<_,i64>(7)?})) }).map_err(|_| storage())?
        .collect::<Result<Vec<_>,_>>().map_err(|_| storage())?;
    Ok(rows)
}

fn preview(connection: &Connection, id: &str) -> Result<Value, CuratorApiError> {
    connection.query_row("SELECT preview_id,candidate_id,candidate_revision,draft_revision,draft_assessment_id,
        witness_hash,effective_diff_hash,diff_projection_json,validation_json,issued_at_ms,expires_at_ms,invalidated_at_ms
        FROM evolution_curator_previews WHERE preview_id=?1", [id], |row| Ok(json!({"previewId":row.get::<_,String>(0)?,
        "candidateId":row.get::<_,String>(1)?,"candidateRevision":row.get::<_,i64>(2)?,"draftRevision":row.get::<_,i64>(3)?,
        "assessmentId":row.get::<_,String>(4)?,"witnessHash":row.get::<_,String>(5)?,"effectiveDiffHash":row.get::<_,String>(6)?,
        "diffs":serde_json::from_str::<Value>(&row.get::<_,String>(7)?).unwrap_or(json!({})),
        "validation":serde_json::from_str::<Value>(&row.get::<_,String>(8)?).unwrap_or(json!({})),
        "issuedAtMs":row.get::<_,i64>(9)?,"expiresAtMs":row.get::<_,i64>(10)?,"invalidatedAtMs":row.get::<_,Option<i64>>(11)?})))
        .map_err(|_| storage())
}

fn application(connection: &Connection, id: &str) -> Result<Option<Value>, CuratorApiError> {
    let mut application = connection.query_row("SELECT application_id,status,overlay_revision,overlay_history_id,failure_code
        FROM evolution_curator_applications WHERE candidate_id=?1 ORDER BY updated_at_ms DESC LIMIT 1", [id], |row| Ok(json!({
        "applicationId":row.get::<_,String>(0)?,"status":row.get::<_,String>(1)?,"overlayRevision":row.get::<_,Option<String>>(2)?,
        "overlayHistoryId":row.get::<_,Option<String>>(3)?,"failureCode":row.get::<_,Option<String>>(4)?})))
        .optional().map_err(|_| storage())?;
    if let Some(value) = &mut application {
        let state = safe_state(connection, id).ok_or_else(not_found)?;
        let state = serde_json::to_value(state).map_err(|_| storage())?;
        value
            .as_object_mut()
            .ok_or_else(storage)?
            .extend(state.as_object().ok_or_else(storage)?.clone());
    }
    Ok(application)
}

fn draft_ready(connection: &Connection, id: &str) -> Result<bool, CuratorApiError> {
    connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM evolution_curator_draft_assessments
        WHERE candidate_id=?1 AND approvable=1 AND invalidated_at_ms IS NULL)",
            [id],
            |row| row.get(0),
        )
        .map_err(|_| storage())
}

fn mutation(kind: &str, body: &str) -> Value {
    let value = serde_json::from_str::<Value>(body).unwrap_or(json!({}));
    if kind == "learn_block" {
        json!({"kind":"learned_guidance","guidance":value["guidance"]})
    } else {
        json!({"kind":"exact_patch","oldString":value["oldString"],"newString":value["newString"],"replaceAll":value["replaceAll"]})
    }
}
fn strip_internal(mut value: Value) -> Value {
    value
        .as_object_mut()
        .map(|v| v.remove("notificationPending"));
    value
}
fn bounded_limit(limit: Option<usize>) -> Result<usize, CuratorApiError> {
    match limit.unwrap_or(20) {
        1..=MAX_PAGE => Ok(limit.unwrap_or(20)),
        _ => Err(invalid()),
    }
}
fn cursor(value: Option<&str>) -> Result<usize, CuratorApiError> {
    value
        .map(str::parse)
        .transpose()
        .map_err(|_| invalid())
        .map(Option::unwrap_or_default)
}
fn invalid() -> CuratorApiError {
    CuratorApiError::new("invalid_input")
}
fn not_found() -> CuratorApiError {
    CuratorApiError::new("not_found")
}
fn storage() -> CuratorApiError {
    CuratorApiError::new("storage_unavailable")
}

fn positive_u64(row: &rusqlite::Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(error),
        )
    })
}
