use std::path::Path;

use serde::Deserialize;
use serde_json::{json, Value};

use super::api::SkillEvolutionSystemActivityApi;
use super::api::{enum_value, invalid, map_error, parse_scope, storage, to_value};
use crate::contexts::skill_evolution_system_activity::{domain::*, infrastructure::*};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields, default)]
pub(crate) struct ActivityTimelineQueryInput {
    pub(crate) session_id: String,
    pub(crate) committed_from_ms: Option<i64>,
    pub(crate) committed_to_ms: Option<i64>,
    pub(crate) severities: Vec<String>,
    pub(crate) source_domains: Vec<String>,
    pub(crate) statuses: Vec<String>,
    pub(crate) skill_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) curator_states: Vec<String>,
    pub(crate) attention_kinds: Vec<String>,
    pub(crate) search_text: Option<String>,
    pub(crate) cursor: Option<String>,
    pub(crate) page_size: Option<u16>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ActivityExportInput {
    pub(crate) export_id: String,
    pub(crate) query: Value,
    pub(crate) format: String,
    pub(crate) locale: String,
    #[serde(default)]
    pub(crate) locale_labels: std::collections::BTreeMap<String, String>,
    pub(crate) target_path: String,
    #[serde(default)]
    pub(crate) item_limit: Option<u32>,
    #[serde(default)]
    pub(crate) size_limit_bytes: Option<u64>,
}

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

/// Exports go only where the user's save dialog pointed: an absolute path with no parent
/// traversal whose directory already exists. Anything else is outside the export boundary.
fn validate_export_target(target_path: &str) -> Result<(), String> {
    let target = Path::new(target_path);
    let traversal = target
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir));
    let parent_exists = target.parent().map(Path::is_dir).unwrap_or(false);
    if !target.is_absolute() || traversal || !parent_exists {
        return Err("system-activity-export-path-outside-boundary".into());
    }
    Ok(())
}

fn step_value(step: ActivityRebuildStep) -> Value {
    match step {
        ActivityRebuildStep::Running { processed_items } => {
            json!({ "step": "running", "processedItems": processed_items })
        }
        ActivityRebuildStep::Validating => json!({ "step": "validating" }),
        ActivityRebuildStep::Ready => json!({ "step": "ready" }),
        ActivityRebuildStep::NeedsCatchUp => json!({ "step": "needsCatchUp" }),
        ActivityRebuildStep::Active => json!({ "step": "active" }),
    }
}

fn build_query(input: &ActivityTimelineQueryInput) -> Result<ActivityTimelineQuery, String> {
    if input.session_id.is_empty() {
        return Err(invalid());
    }
    let search = match &input.search_text {
        Some(text) if !text.trim().is_empty() => Some(parse_safe_search(text)),
        _ => None,
    };
    Ok(ActivityTimelineQuery {
        session_id: input.session_id.clone(),
        committed_from_ms: input.committed_from_ms,
        committed_to_ms: input.committed_to_ms,
        severities: parse_list(&input.severities)?,
        source_domains: parse_list(&input.source_domains)?,
        statuses: parse_list(&input.statuses)?,
        skill_id: input.skill_id.clone(),
        run_id: input.run_id.clone(),
        curator_states: parse_list(&input.curator_states)?,
        attention_kinds: parse_list(&input.attention_kinds)?,
        search,
        cursor: input.cursor.clone(),
        page_size: input.page_size.unwrap_or(50).min(MAX_ACTIVITY_PAGE_SIZE),
    })
}

/// Search text is matched against registered event-code aliases and treated as a safe identity
/// token otherwise; free payload or source text is never indexed or scanned.
fn parse_safe_search(text: &str) -> ActivitySafeSearch {
    let token = text.trim().to_lowercase().replace([' ', '-'], "_");
    let event_alias_codes: Vec<ActivityEventCode> = ActivityEventCode::ALL
        .iter()
        .copied()
        .filter(|code| {
            serde_json::to_value(code)
                .ok()
                .and_then(|value| value.as_str().map(|name| name.contains(token.as_str())))
                .unwrap_or(false)
        })
        .collect();
    // Identity matching accepts only the safe-identity charset; the raw text is reduced to that
    // charset rather than passed through, so a space or quote yields an empty (skipped) token and
    // an alias-only search instead of failing the whole query as invalid input.
    let identity_token: String = text
        .trim()
        .chars()
        .filter(|character| {
            character.is_alphanumeric() || matches!(character, '-' | '_' | '.' | ':' | '@')
        })
        .collect();
    ActivitySafeSearch {
        event_alias_codes,
        identity_tokens: if identity_token.is_empty() {
            Vec::new()
        } else {
            vec![identity_token]
        },
    }
}

fn parse_list<T: serde::de::DeserializeOwned>(values: &[String]) -> Result<Vec<T>, String> {
    values
        .iter()
        .map(|value| serde_json::from_value(Value::String(value.clone())).map_err(|_| invalid()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::validate_export_target;

    #[test]
    fn export_targets_outside_the_user_selected_boundary_are_refused() {
        assert!(validate_export_target("relative/export.json").is_err());
        assert!(validate_export_target("/tmp/../etc/export.json").is_err());
        assert!(validate_export_target("/definitely-missing-dir-x/export.json").is_err());
        let dir = std::env::temp_dir();
        let target = dir.join("system-activity-export-test.json");
        assert!(validate_export_target(target.to_str().expect("utf8 path")).is_ok());
    }
}
