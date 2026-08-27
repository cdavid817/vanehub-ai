use super::definition::{CliParameterControl, CliParameterDefinition, CliParameterOrdering};
use super::error::CliParameterDomainError;
use super::selection::{CliParameterSelection, CliParameterValue};
use regex::Regex;

/// Bidirectional formatting controls can make a rendered token display differently from the bytes
/// that reach the provider, so they are rejected everywhere a user value is accepted.
const BIDI_FORMATTING: [char; 9] = [
    '\u{202a}', '\u{202b}', '\u{202c}', '\u{202d}', '\u{202e}', '\u{2066}', '\u{2067}', '\u{2068}',
    '\u{2069}',
];

fn has_unsafe_characters(value: &str) -> bool {
    value.chars().any(|character| {
        character.is_control()
            || BIDI_FORMATTING.contains(&character)
            || matches!(character, '\u{200e}' | '\u{200f}' | '\u{061c}')
    })
}

fn matches_pattern(value: &str, pattern: &str) -> Result<bool, String> {
    let compiled = Regex::new(pattern).map_err(|error| error.to_string())?;
    Ok(compiled.is_match(value))
}

fn invalid(definition: &CliParameterDefinition, reason: &str) -> CliParameterDomainError {
    CliParameterDomainError::invalid_value(&definition.agent_id, &definition.id, reason)
}

fn normalize_text(
    definition: &CliParameterDefinition,
    value: &str,
) -> Result<String, CliParameterDomainError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(invalid(definition, "empty"));
    }
    if has_unsafe_characters(trimmed) {
        return Err(invalid(definition, "unsafe-characters"));
    }
    if let Some(max_length) = definition.constraints.max_length {
        if trimmed.chars().count() > max_length {
            return Err(invalid(definition, "max-length"));
        }
    }
    let known = definition
        .options
        .iter()
        .any(|option| option.value == trimmed);
    if !known {
        if !definition.allows_custom_values() {
            return Err(invalid(definition, "not-an-allowed-value"));
        }
        if let Some(pattern) = &definition.constraints.pattern {
            if !matches_pattern(trimmed, pattern).map_err(|error| {
                CliParameterDomainError::catalog_invalid(format!(
                    "invalid pattern for {}: {error}",
                    definition.id
                ))
            })? {
                return Err(invalid(definition, "pattern"));
            }
        }
    }
    Ok(trimmed.to_string())
}

fn normalize_list_entry(
    definition: &CliParameterDefinition,
    entry: &str,
) -> Result<String, CliParameterDomainError> {
    let trimmed = if definition.control == CliParameterControl::PathList {
        entry.trim().trim_end_matches(['/', '\\'])
    } else {
        entry.trim()
    };
    if trimmed.is_empty() {
        return Err(invalid(definition, "empty-item"));
    }
    if has_unsafe_characters(trimmed) {
        return Err(invalid(definition, "unsafe-characters"));
    }
    if let Some(max_length) = definition.constraints.item_max_length {
        if trimmed.chars().count() > max_length {
            return Err(invalid(definition, "item-max-length"));
        }
    }
    let known = definition
        .options
        .iter()
        .any(|option| option.value == trimmed);
    if !known {
        if definition.control == CliParameterControl::MultiEnum {
            return Err(invalid(definition, "not-an-allowed-value"));
        }
        if let Some(pattern) = &definition.constraints.item_pattern {
            if !matches_pattern(trimmed, pattern).map_err(|error| {
                CliParameterDomainError::catalog_invalid(format!(
                    "invalid item pattern for {}: {error}",
                    definition.id
                ))
            })? {
                return Err(invalid(definition, "item-pattern"));
            }
        }
    }
    Ok(trimmed.to_string())
}

fn normalize_list(
    definition: &CliParameterDefinition,
    entries: &[String],
) -> Result<Option<Vec<String>>, CliParameterDomainError> {
    let mut normalized = Vec::with_capacity(entries.len());
    for entry in entries {
        let entry = normalize_list_entry(definition, entry)?;
        if definition.constraints.dedupe && normalized.contains(&entry) {
            continue;
        }
        normalized.push(entry);
    }
    if normalized.is_empty() {
        return Ok(None);
    }
    let exclusive = &definition.constraints.exclusive_values;
    if !exclusive.is_empty() {
        let selected_exclusive = normalized.iter().any(|entry| exclusive.contains(entry));
        if selected_exclusive && normalized.len() > 1 {
            return Err(CliParameterDomainError::new(
                super::error::CliParameterErrorCode::Conflict,
            )
            .for_agent(&definition.agent_id)
            .for_parameter(&definition.id)
            .with_detail("reason", "exclusive-value"));
        }
    }
    if let Some(max_items) = definition.constraints.max_items {
        if normalized.len() > max_items {
            return Err(invalid(definition, "max-items"));
        }
    }
    if definition.constraints.ordering == Some(CliParameterOrdering::Catalog) {
        let order = definition.option_values();
        normalized.sort_by_key(|entry| {
            order
                .iter()
                .position(|value| value == entry)
                .unwrap_or(usize::MAX)
        });
    }
    Ok(Some(normalized))
}

/// Normalizes one submitted selection or rejects it with a structured field error. An empty list
/// and a rejected one-way `false` both collapse to inheritance rather than to a silent default.
pub(crate) fn normalize_selection(
    definition: &CliParameterDefinition,
    selection: &CliParameterSelection,
) -> Result<CliParameterSelection, CliParameterDomainError> {
    let Some(value) = selection.as_value() else {
        return Ok(CliParameterSelection::Inherit);
    };
    if !definition.renderer.accepts(value) {
        return Err(invalid(definition, "value-kind-mismatch"));
    }
    match value {
        CliParameterValue::Text(text) => Ok(CliParameterSelection::text(normalize_text(
            definition, text,
        )?)),
        CliParameterValue::Boolean(flag) => {
            if !flag && !definition.renderer.supports_explicit_false() {
                return Ok(CliParameterSelection::Inherit);
            }
            Ok(CliParameterSelection::boolean(*flag))
        }
        CliParameterValue::TextList(entries) => match normalize_list(definition, entries)? {
            Some(values) => Ok(CliParameterSelection::text_list(values)),
            None => Ok(CliParameterSelection::Inherit),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::super::testing::{
        boolean_definition, custom_text_definition, enum_definition, list_definition,
        tri_state_definition,
    };
    use super::*;

    #[test]
    fn empty_and_whitespace_custom_text_is_rejected() {
        let definition = custom_text_definition();
        for raw in ["", "   ", "\t"] {
            let error = normalize_selection(&definition, &CliParameterSelection::text(raw))
                .expect_err("must reject");
            assert_eq!(error.code_str(), "CLI_PARAMETER_INVALID_VALUE");
        }
    }

    #[test]
    fn custom_text_is_trimmed_and_preserved() {
        let definition = custom_text_definition();
        let normalized = normalize_selection(
            &definition,
            &CliParameterSelection::text("  deepseek-chat  "),
        )
        .expect("accepts");
        assert_eq!(normalized, CliParameterSelection::text("deepseek-chat"));
    }

    #[test]
    fn control_and_bidirectional_characters_are_rejected() {
        let definition = custom_text_definition();
        for raw in ["a\nb", "a\u{0}b", "a\u{202e}b", "a\u{2066}b", "a\u{200f}b"] {
            assert!(normalize_selection(&definition, &CliParameterSelection::text(raw)).is_err());
        }
    }

    #[test]
    fn an_enum_rejects_a_value_outside_its_options() {
        let definition = enum_definition();
        assert!(normalize_selection(&definition, &CliParameterSelection::text("high")).is_ok());
        let error = normalize_selection(&definition, &CliParameterSelection::text("ultra"))
            .expect_err("must reject");
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("not-an-allowed-value")
        );
    }

    #[test]
    fn a_literal_default_option_is_accepted_as_a_provider_value() {
        let mut definition = enum_definition();
        definition.options[0].value = "default".to_string();
        let normalized = normalize_selection(&definition, &CliParameterSelection::text("default"))
            .expect("accepts");
        assert_eq!(normalized, CliParameterSelection::text("default"));
        assert!(!normalized.is_inherit());
    }

    #[test]
    fn a_mismatched_value_kind_is_a_structured_error() {
        let definition = custom_text_definition();
        let error = normalize_selection(&definition, &CliParameterSelection::boolean(true))
            .expect_err("must reject");
        assert_eq!(
            error.details.get("reason").map(String::as_str),
            Some("value-kind-mismatch")
        );
    }

    #[test]
    fn a_one_way_flag_treats_false_as_inheritance_and_a_tri_state_keeps_it() {
        assert_eq!(
            normalize_selection(
                &boolean_definition(),
                &CliParameterSelection::boolean(false)
            )
            .expect("normalizes"),
            CliParameterSelection::Inherit
        );
        assert_eq!(
            normalize_selection(
                &tri_state_definition(),
                &CliParameterSelection::boolean(false)
            )
            .expect("normalizes"),
            CliParameterSelection::boolean(false)
        );
    }

    #[test]
    fn lists_dedupe_bound_and_collapse_to_inheritance_when_empty() {
        let definition = list_definition();
        assert_eq!(
            normalize_selection(&definition, &CliParameterSelection::text_list(Vec::new()))
                .expect("normalizes"),
            CliParameterSelection::Inherit
        );
        let deduped = normalize_selection(
            &definition,
            &CliParameterSelection::text_list(vec![
                "alpha".to_string(),
                " alpha ".to_string(),
                "beta".to_string(),
            ]),
        )
        .expect("normalizes");
        assert_eq!(
            deduped,
            CliParameterSelection::text_list(vec!["alpha".to_string(), "beta".to_string()])
        );
        let too_many = (0..9)
            .map(|index| format!("item{index}"))
            .collect::<Vec<_>>();
        assert!(
            normalize_selection(&definition, &CliParameterSelection::text_list(too_many)).is_err()
        );
    }

    #[test]
    fn an_exclusive_list_value_conflicts_with_any_other_entry() {
        let definition = list_definition();
        let error = normalize_selection(
            &definition,
            &CliParameterSelection::text_list(vec!["none".to_string(), "alpha".to_string()]),
        )
        .expect_err("must reject");
        assert_eq!(error.code_str(), "CLI_PARAMETER_CONFLICT");
        assert!(normalize_selection(
            &definition,
            &CliParameterSelection::text_list(vec!["none".to_string()])
        )
        .is_ok());
    }

    #[test]
    fn inherit_stays_inherit() {
        for definition in [
            custom_text_definition(),
            boolean_definition(),
            list_definition(),
        ] {
            assert_eq!(
                normalize_selection(&definition, &CliParameterSelection::Inherit).expect("ok"),
                CliParameterSelection::Inherit
            );
        }
    }
}
