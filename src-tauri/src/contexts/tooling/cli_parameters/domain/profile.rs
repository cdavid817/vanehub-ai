use super::definition::CliParameterDefinition;
use super::diagnostic::{CliParameterDiagnostic, CliParameterDiagnosticCode};
use super::selection::{CliParameterSelection, CliParameterSelectionMap};
use super::validation::normalize_selection;
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) const CURRENT_SELECTION_SCHEMA_VERSION: u32 = 2;

/// One persisted row exactly as stored, before any interpretation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredSelectionRow {
    pub(crate) parameter_id: String,
    pub(crate) value_json: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StoredCliParameterProfile {
    pub(crate) agent_id: String,
    pub(crate) revision: i64,
    pub(crate) selection_schema_version: u32,
    pub(crate) catalog_version: String,
    pub(crate) updated_at: Option<String>,
    pub(crate) rows: Vec<StoredSelectionRow>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MigratedCliParameterProfile {
    pub(crate) selections: CliParameterSelectionMap,
    pub(crate) diagnostics: Vec<CliParameterDiagnostic>,
    /// True when the stored rows are not yet in the current selection schema. The next
    /// successful save or reset rewrites them; startup never mutates user rows eagerly.
    pub(crate) requires_rewrite: bool,
}

fn quarantine(
    agent_id: &str,
    parameter_id: &str,
    reason: &str,
    stored: Option<&str>,
) -> CliParameterDiagnostic {
    let diagnostic = CliParameterDiagnostic::new(
        CliParameterDiagnosticCode::LegacySelectionQuarantined,
        agent_id,
        Some(parameter_id.to_string()),
    )
    .with_detail("reason", reason);
    match stored {
        Some(value) => diagnostic.with_redacted_detail("storedValue", value),
        None => diagnostic,
    }
}

/// Converts one legacy raw JSON value using the definition that owns it. Only unambiguous shapes
/// convert; everything else is quarantined rather than reinterpreted.
pub(crate) enum LegacyConversion {
    Converted(CliParameterSelection),
    /// The stored bytes have more than one defensible reading. Guessing would either drop a real
    /// provider value or start emitting a flag the v1 runtime never emitted.
    Ambiguous(&'static str),
    Unsupported(&'static str),
}

/// The v1 catalog overloaded the literal string `default` as its inheritance sentinel. That is
/// only unambiguous while the v2 registry does *not* also declare `default` as a real provider
/// value for the same definition — `gemini-cli.approvalMode` is exactly such a case.
fn convert_legacy(definition: &CliParameterDefinition, value: &Value) -> LegacyConversion {
    match value {
        Value::String(text) if text == "default" => {
            if definition
                .options
                .iter()
                .any(|option| option.value == "default")
            {
                LegacyConversion::Ambiguous("legacy-default-sentinel-collides-with-provider-value")
            } else {
                LegacyConversion::Converted(CliParameterSelection::Inherit)
            }
        }
        Value::String(text) => {
            LegacyConversion::Converted(CliParameterSelection::text(text.clone()))
        }
        Value::Bool(flag) => {
            if *flag {
                LegacyConversion::Converted(CliParameterSelection::boolean(true))
            } else if definition.renderer.supports_explicit_false() {
                // v1 had no renderer that could emit a negative flag, so a stored `false` on a
                // definition that now can emit one cannot be read as an explicit choice.
                LegacyConversion::Ambiguous("legacy-false-on-a-tri-state-definition")
            } else {
                // A v1 one-way presence flag stored `false` to mean "do not emit".
                LegacyConversion::Converted(CliParameterSelection::Inherit)
            }
        }
        Value::Array(entries) => {
            let texts = entries
                .iter()
                .map(|entry| entry.as_str().map(str::to_string))
                .collect::<Option<Vec<_>>>();
            match texts {
                Some(texts) if texts.is_empty() => {
                    LegacyConversion::Converted(CliParameterSelection::Inherit)
                }
                Some(texts) => LegacyConversion::Converted(CliParameterSelection::text_list(texts)),
                None => LegacyConversion::Unsupported("legacy-list-contains-a-non-string-entry"),
            }
        }
        _ => LegacyConversion::Unsupported("unsupported-legacy-shape"),
    }
}

/// Reads a stored profile into validated selections. Unknown, malformed, and no-longer-valid rows
/// are quarantined with a repair diagnostic; they are never deleted and never rendered.
pub(crate) fn migrate_stored_profile(
    definitions: &[CliParameterDefinition],
    stored: &StoredCliParameterProfile,
) -> MigratedCliParameterProfile {
    let by_id = definitions
        .iter()
        .map(|definition| (definition.id.as_str(), definition))
        .collect::<BTreeMap<_, _>>();

    let mut selections = CliParameterSelectionMap::new();
    let mut diagnostics = Vec::new();
    let mut converted_legacy = false;

    for definition in definitions {
        selections.insert(definition.id.clone(), definition.default_selection.clone());
    }

    for row in &stored.rows {
        let Some(definition) = by_id.get(row.parameter_id.as_str()) else {
            diagnostics.push(quarantine(
                &stored.agent_id,
                &row.parameter_id,
                "unknown-parameter",
                None,
            ));
            continue;
        };
        let Ok(raw) = serde_json::from_str::<Value>(&row.value_json) else {
            diagnostics.push(quarantine(
                &stored.agent_id,
                &row.parameter_id,
                "malformed-json",
                None,
            ));
            continue;
        };
        let candidate = match serde_json::from_value::<CliParameterSelection>(raw.clone()) {
            Ok(selection) => selection,
            Err(_) => match convert_legacy(definition, &raw) {
                LegacyConversion::Converted(selection) => {
                    converted_legacy = true;
                    selection
                }
                LegacyConversion::Ambiguous(reason) | LegacyConversion::Unsupported(reason) => {
                    diagnostics.push(quarantine(
                        &stored.agent_id,
                        &row.parameter_id,
                        reason,
                        None,
                    ));
                    continue;
                }
            },
        };
        match normalize_selection(definition, &candidate) {
            Ok(selection) => {
                selections.insert(definition.id.clone(), selection);
            }
            Err(_) => diagnostics.push(quarantine(
                &stored.agent_id,
                &row.parameter_id,
                "no-longer-valid",
                candidate.as_value().and_then(|value| value.as_text()),
            )),
        }
    }

    let requires_rewrite =
        converted_legacy || stored.selection_schema_version < CURRENT_SELECTION_SCHEMA_VERSION;
    if converted_legacy {
        diagnostics.push(CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::LegacySelectionMigrated,
            &stored.agent_id,
            None,
        ));
    }

    MigratedCliParameterProfile {
        selections,
        diagnostics,
        requires_rewrite,
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        boolean_definition, custom_text_definition, enum_definition, list_definition,
        tri_state_definition,
    };
    use super::*;

    fn stored(schema_version: u32, rows: &[(&str, &str)]) -> StoredCliParameterProfile {
        StoredCliParameterProfile {
            agent_id: "claude-code".to_string(),
            revision: 3,
            selection_schema_version: schema_version,
            catalog_version: "2.0.0".to_string(),
            updated_at: Some("2026-08-22T00:00:00Z".to_string()),
            rows: rows
                .iter()
                .map(|(parameter_id, value_json)| StoredSelectionRow {
                    parameter_id: (*parameter_id).to_string(),
                    value_json: (*value_json).to_string(),
                })
                .collect(),
        }
    }

    fn definitions() -> Vec<CliParameterDefinition> {
        vec![
            custom_text_definition(),
            boolean_definition(),
            list_definition(),
        ]
    }

    #[test]
    fn a_legacy_default_sentinel_becomes_inheritance() {
        let migrated =
            migrate_stored_profile(&definitions(), &stored(1, &[("model", "\"default\"")]));
        assert_eq!(migrated.selections["model"], CliParameterSelection::Inherit);
        assert!(migrated.requires_rewrite);
        assert!(migrated
            .diagnostics
            .iter()
            .any(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionMigrated));
    }

    #[test]
    fn legacy_scalars_and_lists_convert_without_loss() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(
                1,
                &[
                    ("model", "\"sonnet\""),
                    ("search", "true"),
                    ("extensions", "[\"alpha\",\"beta\"]"),
                ],
            ),
        );
        assert_eq!(
            migrated.selections["model"],
            CliParameterSelection::text("sonnet")
        );
        assert_eq!(
            migrated.selections["search"],
            CliParameterSelection::boolean(true)
        );
        assert_eq!(
            migrated.selections["extensions"],
            CliParameterSelection::text_list(vec!["alpha".to_string(), "beta".to_string()])
        );
    }

    #[test]
    fn a_legacy_false_one_way_flag_and_empty_list_become_inheritance() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(1, &[("search", "false"), ("extensions", "[]")]),
        );
        assert_eq!(
            migrated.selections["search"],
            CliParameterSelection::Inherit
        );
        assert_eq!(
            migrated.selections["extensions"],
            CliParameterSelection::Inherit
        );
    }

    #[test]
    fn a_v2_envelope_is_read_without_legacy_conversion() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(2, &[("model", r#"{"state":"value","value":"opus"}"#)]),
        );
        assert_eq!(
            migrated.selections["model"],
            CliParameterSelection::text("opus")
        );
        assert!(!migrated.requires_rewrite);
        assert!(migrated.diagnostics.is_empty());
    }

    #[test]
    fn malformed_unknown_and_invalid_rows_are_quarantined_not_deleted() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(
                1,
                &[
                    ("model", "not-json"),
                    ("removedParameter", "\"x\""),
                    ("model", "\"has space and \\u0000\""),
                ],
            ),
        );
        let reasons = migrated
            .diagnostics
            .iter()
            .filter(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
            .filter_map(|entry| entry.details.get("reason").cloned())
            .collect::<Vec<_>>();
        assert!(reasons.contains(&"malformed-json".to_string()));
        assert!(reasons.contains(&"unknown-parameter".to_string()));
        assert!(reasons.contains(&"no-longer-valid".to_string()));
        assert!(migrated.diagnostics.iter().all(|entry| !entry.blocking));
    }

    #[test]
    fn a_quarantined_row_does_not_stop_other_selections_loading() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(1, &[("removedParameter", "\"x\""), ("search", "true")]),
        );
        assert_eq!(
            migrated.selections["search"],
            CliParameterSelection::boolean(true)
        );
    }

    #[test]
    fn a_legacy_default_is_quarantined_when_the_registry_declares_default_a_real_value() {
        // `gemini-cli.approvalMode` is the real instance: v1 stored `default` to mean "emit
        // nothing", v2 declares `default` as the provider's own ask-every-time mode.
        let mut definition = enum_definition();
        definition.options[0].value = "default".to_string();
        let migrated = migrate_stored_profile(
            &[definition],
            &StoredCliParameterProfile {
                agent_id: "gemini-cli".to_string(),
                revision: 1,
                selection_schema_version: 1,
                catalog_version: String::new(),
                updated_at: None,
                rows: vec![StoredSelectionRow {
                    parameter_id: "effort".to_string(),
                    value_json: "\"default\"".to_string(),
                }],
            },
        );
        assert_eq!(
            migrated.selections["effort"],
            CliParameterSelection::Inherit
        );
        let reason = migrated
            .diagnostics
            .iter()
            .find(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
            .and_then(|entry| entry.details.get("reason"))
            .cloned();
        assert_eq!(
            reason.as_deref(),
            Some("legacy-default-sentinel-collides-with-provider-value")
        );
    }

    #[test]
    fn a_v2_literal_default_survives_even_where_the_legacy_sentinel_was_ambiguous() {
        let mut definition = enum_definition();
        definition.options[0].value = "default".to_string();
        let migrated = migrate_stored_profile(
            &[definition],
            &StoredCliParameterProfile {
                agent_id: "gemini-cli".to_string(),
                revision: 1,
                selection_schema_version: 2,
                catalog_version: "2.0.0".to_string(),
                updated_at: None,
                rows: vec![StoredSelectionRow {
                    parameter_id: "effort".to_string(),
                    value_json: r#"{"state":"value","value":"default"}"#.to_string(),
                }],
            },
        );
        assert_eq!(
            migrated.selections["effort"],
            CliParameterSelection::text("default")
        );
        assert!(migrated.diagnostics.is_empty());
    }

    #[test]
    fn a_legacy_false_on_a_tri_state_definition_is_quarantined_rather_than_guessed() {
        let migrated = migrate_stored_profile(
            &[tri_state_definition()],
            &StoredCliParameterProfile {
                agent_id: "claude-code".to_string(),
                revision: 1,
                selection_schema_version: 1,
                catalog_version: String::new(),
                updated_at: None,
                rows: vec![StoredSelectionRow {
                    parameter_id: "chrome".to_string(),
                    value_json: "false".to_string(),
                }],
            },
        );
        assert_eq!(
            migrated.selections["chrome"],
            CliParameterSelection::Inherit
        );
        assert_eq!(
            migrated
                .diagnostics
                .iter()
                .find(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
                .and_then(|entry| entry.details.get("reason"))
                .map(String::as_str),
            Some("legacy-false-on-a-tri-state-definition")
        );
    }

    #[test]
    fn a_legacy_list_with_a_non_string_entry_is_quarantined() {
        let migrated = migrate_stored_profile(
            &definitions(),
            &stored(1, &[("extensions", "[\"alpha\", 7]")]),
        );
        assert_eq!(
            migrated
                .diagnostics
                .iter()
                .find(|entry| entry.code == CliParameterDiagnosticCode::LegacySelectionQuarantined)
                .and_then(|entry| entry.details.get("reason"))
                .map(String::as_str),
            Some("legacy-list-contains-a-non-string-entry")
        );
    }

    #[test]
    fn migration_is_idempotent_over_repeated_reads() {
        let profile = stored(1, &[("model", "\"sonnet\""), ("search", "true")]);
        let first = migrate_stored_profile(&definitions(), &profile);
        let second = migrate_stored_profile(&definitions(), &profile);
        assert_eq!(first.selections, second.selections);
        assert_eq!(first.requires_rewrite, second.requires_rewrite);
    }
}
