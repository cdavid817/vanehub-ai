use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use super::api::SkillEvolutionSystemActivityApi;
use super::api::{enum_value, invalid, map_error, parse_scope, storage, to_value};
use crate::contexts::skill_evolution_system_activity::{domain::*, infrastructure::*};

mod parsing;
use parsing::*;

impl SkillEvolutionSystemActivityApi {
    pub(crate) fn query_timeline(&self, input: Value) -> Result<Value, String> {
        let input: ActivityTimelineQueryInput =
            serde_json::from_value(input).map_err(|_| invalid())?;
        let query = build_query(&input)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        match repository.query_timeline(&query).map_err(map_error)? {
            ActivityTimelineQueryResult::Page(page) => Ok(json!({
                "kind": "page",
                "activeGenerationId": page.active_generation_id,
                "entries": page
                    .entries
                    .iter()
                    .map(|entry| {
                        Ok(json!({
                            "sequence": entry.sequence,
                            "envelope": to_value(&entry.envelope)?,
                            "detailUnavailableReason": entry
                                .detail_unavailable_reason
                                .map(enum_value)
                                .transpose()?,
                        }))
                    })
                    .collect::<Result<Vec<_>, String>>()?,
                "nextCursor": page.next_cursor,
                "complete": page.complete,
            })),
            ActivityTimelineQueryResult::StaleGeneration {
                requested_generation_id,
                active_generation_id,
            } => Ok(json!({
                "kind": "staleGeneration",
                "requestedGenerationId": requested_generation_id,
                "activeGenerationId": active_generation_id,
            })),
        }
    }

    pub(crate) fn dashboard(
        &self,
        scope_kind: &str,
        canonical_scope_id: &str,
    ) -> Result<Value, String> {
        let scope = parse_scope(scope_kind)?;
        let connection = self.database.connection().map_err(|_| storage())?;
        let mut statement = connection
            .prepare(
                "SELECT d.materialization_kind,d.state_json,d.last_event_id,d.updated_at_ms
                 FROM evolution_activity_dashboard_state d
                 JOIN evolution_system_activity_sessions s
                   ON s.scope_kind=d.scope_kind AND s.canonical_scope_id=d.canonical_scope_id
                  AND s.active_generation_id=d.generation_id
                 WHERE d.scope_kind=?1 AND d.canonical_scope_id=?2
                 ORDER BY d.materialization_kind",
            )
            .map_err(|_| storage())?;
        let scope_text = enum_value(scope)?
            .as_str()
            .map(str::to_owned)
            .ok_or_else(storage)?;
        let rows = statement
            .query_map([scope_text.as_str(), canonical_scope_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        let summaries: Vec<Value> = rows
            .into_iter()
            .map(|(kind, state_json, last_event_id, updated_at_ms)| {
                let state: Value = serde_json::from_str(&state_json).map_err(|_| storage())?;
                Ok(json!({
                    "materializationKind": kind,
                    "state": state,
                    "lastEventId": last_event_id,
                    "updatedAtMs": updated_at_ms,
                }))
            })
            .collect::<Result<_, String>>()?;
        Ok(json!({ "summaries": summaries }))
    }

    pub(crate) fn health(&self) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let lease = repository.lease().map_err(map_error)?;
        let mut domains = Vec::new();
        for domain in EvolutionSourceDomain::ALL {
            if let Some(cursor) = repository.cursor(*domain).map_err(map_error)? {
                domains.push(to_value(&cursor)?);
            }
        }
        let last_completed_at_ms: Option<i64> = connection
            .query_row(
                "SELECT MAX(last_success_at_ms) FROM evolution_activity_domain_cursors",
                [],
                |row| row.get(0),
            )
            .map_err(|_| storage())?;
        let rebuilds = self.latest_rebuilds(&connection)?;
        Ok(json!({
            "leaseOwner": lease.as_ref().map(|lease| lease.owner_id.clone()),
            "domains": domains,
            "lastCompletedAtMs": last_completed_at_ms,
            "rebuilds": rebuilds,
        }))
    }

    pub(crate) fn begin_rebuild(
        &self,
        scope_kind: &str,
        canonical_scope_id: &str,
        item_budget: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let rebuild = repository
            .begin_rebuild(
                parse_scope(scope_kind)?,
                canonical_scope_id,
                item_budget,
                now_ms,
            )
            .map_err(map_error)?;
        to_value(&rebuild)
    }

    pub(crate) fn advance_rebuild(
        &self,
        rebuild_id: &str,
        batch_limit: u64,
        now_ms: i64,
    ) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let step = repository
            .advance_rebuild(rebuild_id, batch_limit, now_ms)
            .map_err(map_error)?;
        Ok(step_value(step))
    }

    pub(crate) fn validate_rebuild(&self, rebuild_id: &str, now_ms: i64) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let step = repository
            .validate_rebuild(rebuild_id, now_ms)
            .map_err(map_error)?;
        Ok(step_value(step))
    }

    pub(crate) fn activate_rebuild(&self, rebuild_id: &str, now_ms: i64) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let step = repository
            .activate_rebuild(rebuild_id, now_ms)
            .map_err(map_error)?;
        Ok(step_value(step))
    }

    pub(crate) fn cancel_rebuild(&self, rebuild_id: &str, now_ms: i64) -> Result<Value, String> {
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        repository
            .cancel_rebuild(rebuild_id, now_ms)
            .map_err(map_error)?;
        Ok(json!({ "cancelled": true }))
    }

    /// Writes a sanitized export to the user-selected path. The path must be absolute, contain no
    /// parent traversal, and its directory must already exist — the app never invents locations.
    /// Exported files live outside automatic retention, which the response discloses.
    pub(crate) fn export(&self, input: Value, now_ms: i64) -> Result<Value, String> {
        let input: ActivityExportInput = serde_json::from_value(input).map_err(|_| invalid())?;
        validate_export_target(&input.target_path)?;
        let target = Path::new(&input.target_path);
        let query_input: ActivityTimelineQueryInput =
            serde_json::from_value(input.query).map_err(|_| invalid())?;
        let query = build_query(&query_input)?;
        let format: ActivityExportFormat =
            serde_json::from_value(Value::String(input.format)).map_err(|_| invalid())?;
        let request = ActivityExportRequest {
            export_id: input.export_id,
            query,
            format,
            locale: input.locale,
            locale_labels: input.locale_labels,
            item_limit: input.item_limit.unwrap_or(1_000),
            size_limit_bytes: input.size_limit_bytes.unwrap_or(10 * 1024 * 1024),
            created_at_ms: now_ms,
        };
        let connection = self.database.connection().map_err(|_| storage())?;
        let repository = SqliteActivityProjectionRepository::new(&connection);
        let document = repository
            .export_activity(&request, &(|| false))
            .map_err(map_error)?;
        std::fs::write(target, &document.content)
            .map_err(|_| "system-activity-export-write-failed".to_owned())?;
        let mut record = to_value(&document.record)?;
        if let Some(record) = record.as_object_mut() {
            record.insert("targetPath".into(), json!(input.target_path));
            record.insert("outsideAutomaticRetention".into(), json!(true));
        }
        Ok(record)
    }

    fn latest_rebuilds(&self, connection: &rusqlite::Connection) -> Result<Vec<Value>, String> {
        let mut statement = connection
            .prepare(
                "SELECT rebuild_id,scope_kind,canonical_scope_id,status,processed_items,
                        item_budget,updated_at_ms
                 FROM evolution_activity_rebuilds ORDER BY updated_at_ms DESC LIMIT 10",
            )
            .map_err(|_| storage())?;
        let rows = statement
            .query_map([], |row| {
                Ok(json!({
                    "rebuildId": row.get::<_, String>(0)?,
                    "scopeKind": row.get::<_, String>(1)?,
                    "canonicalScopeId": row.get::<_, String>(2)?,
                    "status": row.get::<_, String>(3)?,
                    "processedItems": row.get::<_, i64>(4)?,
                    "itemBudget": row.get::<_, i64>(5)?,
                    "updatedAtMs": row.get::<_, i64>(6)?,
                }))
            })
            .map_err(|_| storage())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| storage())?;
        Ok(rows)
    }
}
