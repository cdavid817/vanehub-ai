use super::*;

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

/// Exports go only where the user's save dialog pointed: an absolute path with no parent
/// traversal whose directory already exists. Anything else is outside the export boundary.
pub(super) fn validate_export_target(target_path: &str) -> Result<(), String> {
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

pub(super) fn step_value(step: ActivityRebuildStep) -> Value {
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

pub(super) fn build_query(
    input: &ActivityTimelineQueryInput,
) -> Result<ActivityTimelineQuery, String> {
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
    let event_alias_codes = ActivityEventCode::ALL
        .iter()
        .copied()
        .filter(|code| {
            serde_json::to_value(code)
                .ok()
                .and_then(|value| value.as_str().map(|name| name.contains(token.as_str())))
                .unwrap_or(false)
        })
        .collect();
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
        let target = std::env::temp_dir().join("system-activity-export-test.json");
        assert!(validate_export_target(target.to_str().expect("utf8 path")).is_ok());
    }
}
