//! Rich blocks the ACP runtime hands to the transcript.
//!
//! The frontend only renders the `RichBlock` vocabulary in `src/types/chat.ts` (`kind`, `id`,
//! `v`); anything else falls through to the "unsupported block" fallback. Every ACP notice is
//! therefore shaped as a `card`, and the machine-readable ACP facts travel in `meta` so tests
//! and future tooling can still tell the notices apart without parsing prose.

use serde_json::{json, Map, Value};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_BLOCK: AtomicU64 = AtomicU64::new(1);

/// Card tones the frontend understands.
#[derive(Clone, Copy)]
pub(crate) enum Tone {
    Info,
    Warning,
    Danger,
}

impl Tone {
    fn as_str(self) -> &'static str {
        match self {
            Tone::Info => "info",
            Tone::Warning => "warning",
            Tone::Danger => "danger",
        }
    }
}

/// Build a frontend-renderable card. `notice` becomes `meta.type`; every other `meta` entry is
/// copied verbatim, so callers keep their structured payloads while the reader sees prose.
pub(crate) fn card(
    notice: &str,
    title: &str,
    body: impl Into<String>,
    tone: Tone,
    fields: Vec<(&str, String)>,
    meta: Value,
) -> Value {
    let sequence = NEXT_BLOCK.fetch_add(1, Ordering::Relaxed);
    let mut merged = Map::new();
    merged.insert("type".to_string(), Value::String(notice.to_string()));
    if let Value::Object(entries) = meta {
        merged.extend(entries);
    }
    let fields: Vec<Value> = fields
        .into_iter()
        .map(|(label, value)| json!({ "label": label, "value": value }))
        .collect();
    json!({
        "id": format!("acp-{notice}-{sequence}"),
        "kind": "card",
        "v": 1,
        "title": title,
        "bodyMarkdown": body.into(),
        "tone": tone.as_str(),
        "fields": fields,
        "meta": Value::Object(merged),
    })
}

/// Compact rendering of an arbitrary JSON payload for a card body: fenced JSON, never truncated
/// here because the caller already bounded it.
pub(crate) fn fenced_json(value: &Value) -> String {
    format!(
        "```json\n{}\n```",
        serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_matches_frontend_rich_block_shape() {
        let block = card(
            "acp_plan",
            "Plan",
            "body",
            Tone::Info,
            vec![("Entries", "2".to_string())],
            json!({"entries": [1, 2]}),
        );
        assert_eq!(block["kind"], json!("card"));
        assert_eq!(block["v"], json!(1));
        assert!(block["id"].as_str().unwrap().starts_with("acp-acp_plan-"));
        assert_eq!(block["meta"]["type"], json!("acp_plan"));
        assert_eq!(block["meta"]["entries"], json!([1, 2]));
        assert_eq!(block["fields"][0]["label"], json!("Entries"));
        assert_eq!(block["tone"], json!("info"));
    }

    #[test]
    fn card_ids_are_unique() {
        let a = card("x", "t", "", Tone::Warning, vec![], json!({}));
        let b = card("x", "t", "", Tone::Warning, vec![], json!({}));
        assert_ne!(a["id"], b["id"]);
    }
}
