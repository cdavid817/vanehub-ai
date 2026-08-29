use std::collections::BTreeMap;

use rusqlite::params;
use sha2::{Digest, Sha256};

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

pub(crate) const ACTIVITY_EXPORT_REDACTION_VERSION: &str = "activity-redaction-v1";

/// A bounded export request. `locale_labels` maps event codes to already-localized titles for the
/// Markdown format; a missing key falls back to the safe code itself, so no persisted envelope is
/// ever rewritten for a locale. Filters reuse the timeline query so an export can never see more
/// than the timeline shows.
#[derive(Debug, Clone)]
pub(crate) struct ActivityExportRequest {
    pub(crate) export_id: String,
    pub(crate) query: ActivityTimelineQuery,
    pub(crate) format: ActivityExportFormat,
    pub(crate) locale: String,
    pub(crate) locale_labels: BTreeMap<String, String>,
    pub(crate) item_limit: u32,
    pub(crate) size_limit_bytes: u64,
    pub(crate) created_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ActivityExportDocument {
    pub(crate) record: ActivityExport,
    pub(crate) content: String,
}

impl SqliteActivityProjectionRepository<'_> {
    /// Renders a deterministic, sanitized export of the filtered timeline and persists its
    /// manifest row. `cancelled` is polled between pages; a cancelled export writes nothing.
    /// Only canonical envelopes are read: navigation descriptors are included as inert data and
    /// never followed, so no dossier, evidence, diff, draft, Overlay, or source record can leak.
    pub(crate) fn export_activity(
        &self,
        request: &ActivityExportRequest,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<ActivityExportDocument, ActivityProjectionRepositoryError> {
        if request.created_at_ms < 0
            || request.item_limit == 0
            || request.size_limit_bytes == 0
            || sanitize_text(&request.export_id, "export.id", 160).is_err()
            || sanitize_text(&request.locale, "export.locale", 35).is_err()
        {
            return Err(ActivityProjectionRepositoryError::InvalidInput);
        }
        let mut query = request.query.clone();
        query.cursor = None;
        query.page_size = MAX_ACTIVITY_PAGE_SIZE;
        let mut entries: Vec<ActivityTimelineEntry> = Vec::new();
        let mut generation_id: Option<String> = None;
        let mut truncated = false;
        loop {
            if cancelled() {
                return Err(ActivityProjectionRepositoryError::Cancelled);
            }
            let page = match self.query_timeline(&query)? {
                ActivityTimelineQueryResult::Page(page) => page,
                ActivityTimelineQueryResult::StaleGeneration { .. } => {
                    return Err(ActivityProjectionRepositoryError::Conflict);
                }
            };
            // The generation is stable across pages: a rebuild activating mid-export surfaces as
            // a stale-generation result above rather than a silently mixed document.
            if generation_id.is_none() {
                generation_id = Some(page.active_generation_id.clone());
            }
            for entry in page.entries {
                if entries.len() >= request.item_limit as usize {
                    truncated = true;
                    break;
                }
                entries.push(entry);
            }
            match (truncated, page.next_cursor) {
                (false, Some(cursor)) => query.cursor = Some(cursor),
                (true, Some(_)) => break,
                (_, None) => break,
            }
        }
        let manifest = ExportManifest {
            schema_version: 1,
            export_id: request.export_id.clone(),
            session_id: request.query.session_id.clone(),
            generation_id: generation_id
                .clone()
                .ok_or(ActivityProjectionRepositoryError::Storage)?,
            format: request.format,
            locale: request.locale.clone(),
            filters: filters_summary(&request.query),
            item_count: entries.len() as u32,
            complete: !truncated,
            redaction_version: ACTIVITY_EXPORT_REDACTION_VERSION.into(),
            created_at_ms: request.created_at_ms,
        };
        let mut content = match request.format {
            ActivityExportFormat::Json => render_json(&manifest, &entries)?,
            ActivityExportFormat::Markdown => {
                render_markdown(&manifest, &entries, &request.locale_labels)
            }
        };
        let mut complete = manifest.complete;
        if content.len() as u64 > request.size_limit_bytes {
            // Re-render with fewer items until the document fits; the manifest then reports the
            // export as incomplete rather than silently oversized.
            let mut kept = entries.len();
            while content.len() as u64 > request.size_limit_bytes && kept > 0 {
                if cancelled() {
                    return Err(ActivityProjectionRepositoryError::Cancelled);
                }
                kept = kept.saturating_sub((kept / 4).max(1));
                let mut reduced = manifest.clone();
                reduced.item_count = kept as u32;
                reduced.complete = false;
                content = match request.format {
                    ActivityExportFormat::Json => render_json(&reduced, &entries[..kept])?,
                    ActivityExportFormat::Markdown => {
                        render_markdown(&reduced, &entries[..kept], &request.locale_labels)
                    }
                };
                complete = false;
            }
            if content.len() as u64 > request.size_limit_bytes {
                return Err(ActivityProjectionRepositoryError::InvalidInput);
            }
        }
        // The hash binds generation, filters, and rendered items — not the export id or creation
        // time — so two exports of the same selection verify as identical content.
        let canonical_kept = count_rendered_items(&content, request.format) as usize;
        let hash_input = serde_json::to_string(&serde_json::json!({
            "generationId": generation_id,
            "filters": manifest.filters,
            "format": request.format,
            "locale": request.locale,
            "items": entries[..canonical_kept.min(entries.len())]
                .iter()
                .map(|entry| &entry.envelope.content_hash)
                .collect::<Vec<_>>(),
        }))
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
        let content_hash = format!(
            "sha256:{}",
            hex_bytes(&Sha256::digest(hash_input.as_bytes()))
        );
        let item_count = if complete {
            manifest.item_count
        } else {
            count_rendered_items(&content, request.format)
        };
        self.connection.execute(
            "INSERT INTO evolution_activity_exports
             (export_id,session_id,generation_id,format,filters_json,item_count,byte_count,
              complete,redaction_version,content_hash,created_at_ms)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                request.export_id,
                request.query.session_id,
                manifest.generation_id,
                enum_text(request.format)?,
                serde_json::to_string(&manifest.filters)
                    .map_err(|_| ActivityProjectionRepositoryError::Storage)?,
                i64::from(item_count),
                content.len() as i64,
                complete as i64,
                ACTIVITY_EXPORT_REDACTION_VERSION,
                content_hash,
                request.created_at_ms,
            ],
        )?;
        Ok(ActivityExportDocument {
            record: ActivityExport {
                export_id: request.export_id.clone(),
                session_id: request.query.session_id.clone(),
                generation_id: generation_id.ok_or(ActivityProjectionRepositoryError::Storage)?,
                format: request.format,
                item_count,
                byte_count: content.len() as u64,
                complete,
                redaction_version: ACTIVITY_EXPORT_REDACTION_VERSION.into(),
                content_hash,
            },
            content,
        })
    }
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportManifest {
    schema_version: u8,
    export_id: String,
    session_id: String,
    generation_id: String,
    format: ActivityExportFormat,
    locale: String,
    filters: BTreeMap<String, serde_json::Value>,
    item_count: u32,
    complete: bool,
    redaction_version: String,
    created_at_ms: i64,
}

fn filters_summary(query: &ActivityTimelineQuery) -> BTreeMap<String, serde_json::Value> {
    let mut filters = BTreeMap::new();
    let mut set = |key: &str, value: serde_json::Value| {
        if !matches!(&value, serde_json::Value::Array(items) if items.is_empty())
            && !value.is_null()
        {
            filters.insert(key.to_owned(), value);
        }
    };
    set(
        "committedFromMs",
        serde_json::json!(query.committed_from_ms),
    );
    set("committedToMs", serde_json::json!(query.committed_to_ms));
    set("severities", serde_json::json!(query.severities));
    set("sourceDomains", serde_json::json!(query.source_domains));
    set("statuses", serde_json::json!(query.statuses));
    set("skillId", serde_json::json!(query.skill_id));
    set("runId", serde_json::json!(query.run_id));
    set("curatorStates", serde_json::json!(query.curator_states));
    set("attentionKinds", serde_json::json!(query.attention_kinds));
    filters
}

fn render_json(
    manifest: &ExportManifest,
    entries: &[ActivityTimelineEntry],
) -> Result<String, ActivityProjectionRepositoryError> {
    let items: Vec<serde_json::Value> = entries
        .iter()
        .map(|entry| {
            serde_json::to_value(&entry.envelope).map(|envelope| {
                serde_json::json!({
                    "sequence": entry.sequence,
                    "envelope": envelope,
                })
            })
        })
        .collect::<Result<_, _>>()
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?;
    serde_json::to_string_pretty(&serde_json::json!({
        "manifest": manifest,
        "items": items,
    }))
    .map_err(|_| ActivityProjectionRepositoryError::Storage)
}

fn render_markdown(
    manifest: &ExportManifest,
    entries: &[ActivityTimelineEntry],
    locale_labels: &BTreeMap<String, String>,
) -> String {
    let mut lines = Vec::new();
    lines.push("# Skill Evolution Activity Export".to_owned());
    lines.push(String::new());
    lines.push(format!("- export: `{}`", manifest.export_id));
    lines.push(format!("- session: `{}`", manifest.session_id));
    lines.push(format!("- generation: `{}`", manifest.generation_id));
    lines.push(format!("- locale: `{}`", manifest.locale));
    lines.push(format!("- items: {}", manifest.item_count));
    lines.push(format!("- complete: {}", manifest.complete));
    lines.push(format!("- redaction: `{}`", manifest.redaction_version));
    for entry in entries {
        let envelope = &entry.envelope;
        let code = envelope_code_text(envelope);
        let title = locale_labels.get(&code).cloned().unwrap_or_else(|| {
            // Documented fallback: the safe code stays visible for diagnosis when a locale
            // string is missing, and the persisted envelope is never rewritten.
            code.clone()
        });
        lines.push(String::new());
        lines.push(format!("## {}. {title}", entry.sequence));
        lines.push(format!(
            "- code: `{code}` · severity: `{severity}` · status: `{status}`",
            severity = enum_text(envelope.severity).unwrap_or_default(),
            status = enum_text(envelope.status).unwrap_or_default(),
        ));
        lines.push(format!(
            "- source: `{}` `{}` rev `{}` · committed at {}",
            envelope.source_domain,
            envelope.source_id,
            envelope.source_revision,
            envelope.committed_at_ms,
        ));
        if !envelope.reason_codes.is_empty() {
            let reasons: Vec<String> = envelope
                .reason_codes
                .iter()
                .filter_map(|reason| enum_text(*reason).ok())
                .collect();
            lines.push(format!("- reasons: `{}`", reasons.join("`, `")));
        }
    }
    lines.push(String::new());
    lines.join("\n")
}

fn envelope_code_text(envelope: &EvolutionActivityEnvelopeV1) -> String {
    enum_text(envelope.event_code).unwrap_or_default()
}

fn count_rendered_items(content: &str, format: ActivityExportFormat) -> u32 {
    match format {
        ActivityExportFormat::Json => serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|value| {
                value
                    .get("items")
                    .and_then(|items| items.as_array().map(Vec::len))
            })
            .unwrap_or(0) as u32,
        ActivityExportFormat::Markdown => content
            .lines()
            .filter(|line| line.starts_with("## "))
            .count() as u32,
    }
}

fn enum_text(value: impl serde::Serialize) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}

fn hex_bytes(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
