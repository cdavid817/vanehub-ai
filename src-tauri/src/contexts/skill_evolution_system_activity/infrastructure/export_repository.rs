use std::collections::BTreeMap;

use rusqlite::params;

use super::{ActivityProjectionRepositoryError, SqliteActivityProjectionRepository};
use crate::contexts::skill_evolution_system_activity::domain::*;

mod rendering;
use rendering::*;

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
        let content_hash = crate::platform::hashing::sha256_tagged(hash_input.as_bytes());
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
