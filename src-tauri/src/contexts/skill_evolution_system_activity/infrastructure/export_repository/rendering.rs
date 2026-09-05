use super::*;

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ExportManifest {
    pub(super) schema_version: u8,
    pub(super) export_id: String,
    pub(super) session_id: String,
    pub(super) generation_id: String,
    pub(super) format: ActivityExportFormat,
    pub(super) locale: String,
    pub(super) filters: BTreeMap<String, serde_json::Value>,
    pub(super) item_count: u32,
    pub(super) complete: bool,
    pub(super) redaction_version: String,
    pub(super) created_at_ms: i64,
}

pub(super) fn filters_summary(
    query: &ActivityTimelineQuery,
) -> BTreeMap<String, serde_json::Value> {
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

pub(super) fn render_json(
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

pub(super) fn render_markdown(
    manifest: &ExportManifest,
    entries: &[ActivityTimelineEntry],
    locale_labels: &BTreeMap<String, String>,
) -> String {
    let mut lines = vec![
        "# Skill Evolution Activity Export".to_owned(),
        String::new(),
        format!("- export: `{}`", manifest.export_id),
        format!("- session: `{}`", manifest.session_id),
        format!("- generation: `{}`", manifest.generation_id),
        format!("- locale: `{}`", manifest.locale),
        format!("- items: {}", manifest.item_count),
        format!("- complete: {}", manifest.complete),
        format!("- redaction: `{}`", manifest.redaction_version),
    ];
    for entry in entries {
        let envelope = &entry.envelope;
        let code = enum_text(envelope.event_code).unwrap_or_default();
        let title = locale_labels
            .get(&code)
            .cloned()
            .unwrap_or_else(|| code.clone());
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

pub(super) fn count_rendered_items(content: &str, format: ActivityExportFormat) -> u32 {
    match format {
        ActivityExportFormat::Json => serde_json::from_str::<serde_json::Value>(content)
            .ok()
            .and_then(|value| value.get("items")?.as_array().map(Vec::len))
            .unwrap_or(0) as u32,
        ActivityExportFormat::Markdown => content
            .lines()
            .filter(|line| line.starts_with("## "))
            .count() as u32,
    }
}

pub(super) fn enum_text(
    value: impl serde::Serialize,
) -> Result<String, ActivityProjectionRepositoryError> {
    serde_json::to_value(value)
        .map_err(|_| ActivityProjectionRepositoryError::Storage)?
        .as_str()
        .map(str::to_owned)
        .ok_or(ActivityProjectionRepositoryError::Storage)
}
