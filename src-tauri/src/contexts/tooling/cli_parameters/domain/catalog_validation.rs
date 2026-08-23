use super::catalog::CliParameterCatalog;
use super::definition::{CliParameterControl, CliParameterDefinition, CliParameterEvidence};
use super::dependency::find_requires_cycle;
use super::error::CliParameterDomainError;
use super::rendering::{CliConfigEncoding, CliParameterRenderer};
use super::selection::{CliParameterSelection, CliParameterValue};
use super::validation::normalize_selection;
use std::collections::BTreeSet;

/// The external managed CLIs this subdomain owns, in settings display order.
pub(crate) const MANAGED_CLI_AGENT_IDS: [&str; 5] = [
    "claude-code",
    "codex-cli",
    "gemini-cli",
    "opencode",
    "antigravity-cli",
];

/// Flags VaneHub constructs itself. Admitting one to the registry would let a profile replace a
/// runtime-owned argument and rely on last-argument-wins behaviour.
const RESERVED_FLAGS: [&str; 16] = [
    "-p",
    "-o",
    "-c",
    "-",
    "--prompt",
    "--print",
    "--output-format",
    "--format",
    "--json",
    "--resume",
    "--session",
    "--session-id",
    "--conversation",
    "--include-partial-messages",
    "--verbose",
    "--append-system-prompt",
];

/// Substrings that identify a secret-bearing or otherwise forbidden concern.
const FORBIDDEN_FLAG_SUBSTRINGS: [&str; 8] = [
    "dangerously",
    "api-key",
    "api_key",
    "token",
    "password",
    "secret",
    "system-prompt",
    "stdin",
];

fn invalid(reason: impl Into<String>) -> CliParameterDomainError {
    CliParameterDomainError::catalog_invalid(reason)
}

fn renderer_matches_control(control: CliParameterControl, renderer: &CliParameterRenderer) -> bool {
    match control {
        CliParameterControl::Enum | CliParameterControl::CustomText => matches!(
            renderer,
            CliParameterRenderer::FlagValue { .. }
                | CliParameterRenderer::ConfigKeyValue {
                    encoding: CliConfigEncoding::TomlString,
                    ..
                }
        ),
        CliParameterControl::BooleanFlag => matches!(
            renderer,
            CliParameterRenderer::PresenceFlag { .. }
                | CliParameterRenderer::ConfigKeyValue {
                    encoding: CliConfigEncoding::TomlBoolean,
                    ..
                }
        ),
        CliParameterControl::TriState => matches!(
            renderer,
            CliParameterRenderer::PositiveNegativeFlag { .. }
                | CliParameterRenderer::ConfigKeyValue {
                    encoding: CliConfigEncoding::TomlBoolean,
                    ..
                }
        ),
        CliParameterControl::MultiEnum
        | CliParameterControl::OrderedStringList
        | CliParameterControl::PathList => matches!(
            renderer,
            CliParameterRenderer::RepeatFlagValue { .. } | CliParameterRenderer::JoinedList { .. }
        ),
    }
}

fn validate_flags(definition: &CliParameterDefinition) -> Result<(), CliParameterDomainError> {
    let flags = definition.renderer.flags();
    for flag in &flags {
        if flag.trim().is_empty() {
            return Err(invalid(format!("{} has an empty flag", definition.id)));
        }
        if RESERVED_FLAGS.contains(flag) {
            return Err(invalid(format!(
                "{} maps to the runtime-reserved flag {flag}",
                definition.id
            )));
        }
        let lowered = flag.to_ascii_lowercase();
        if FORBIDDEN_FLAG_SUBSTRINGS
            .iter()
            .any(|forbidden| lowered.contains(forbidden))
        {
            return Err(invalid(format!(
                "{} maps to the forbidden flag {flag}",
                definition.id
            )));
        }
    }
    if let CliParameterRenderer::PositiveNegativeFlag {
        positive_flag,
        negative_flag,
        ..
    } = &definition.renderer
    {
        if positive_flag == negative_flag {
            return Err(invalid(format!(
                "{} uses the same flag for both tri-state directions",
                definition.id
            )));
        }
    }
    Ok(())
}

fn validate_localization(
    definition: &CliParameterDefinition,
) -> Result<(), CliParameterDomainError> {
    if definition.label_key.trim().is_empty() || definition.description_key.trim().is_empty() {
        return Err(invalid(format!(
            "{} is missing a localization key",
            definition.id
        )));
    }
    for option in &definition.options {
        if option.label_key.trim().is_empty() || option.description_key.trim().is_empty() {
            return Err(invalid(format!(
                "{}.{} is missing a localization key",
                definition.id, option.value
            )));
        }
    }
    Ok(())
}

fn validate_audit(definition: &CliParameterDefinition) -> Result<(), CliParameterDomainError> {
    let audit = &definition.audit;
    if audit.source_id.trim().is_empty()
        || audit.note.trim().is_empty()
        || audit.reviewed_at.len() != 10
    {
        return Err(invalid(format!(
            "{} has an incomplete audit record",
            definition.id
        )));
    }
    if !audit.source_url.starts_with("https://") {
        return Err(invalid(format!(
            "{} has a non-https audit source",
            definition.id
        )));
    }
    if audit.reviewed_state.trim().is_empty() {
        return Err(invalid(format!(
            "{} records no reviewed artefact state",
            definition.id
        )));
    }
    if audit.evidence.is_empty() {
        return Err(invalid(format!(
            "{} records no audit evidence",
            definition.id
        )));
    }
    // `pending-review` means nothing settled it. Listing it beside real evidence would let a
    // parameter read as both audited and unaudited, which is how a laundered verdict starts.
    if audit
        .evidence
        .contains(&CliParameterEvidence::PendingReview)
        && audit.evidence.len() > 1
    {
        return Err(invalid(format!(
            "{} pairs pending-review with other evidence",
            definition.id
        )));
    }
    let mut seen = BTreeSet::new();
    for evidence in &audit.evidence {
        if !seen.insert(*evidence) {
            return Err(invalid(format!(
                "{} repeats an audit evidence kind",
                definition.id
            )));
        }
    }
    Ok(())
}

fn validate_values(definition: &CliParameterDefinition) -> Result<(), CliParameterDomainError> {
    if matches!(
        definition.control,
        CliParameterControl::Enum | CliParameterControl::MultiEnum
    ) && definition.options.is_empty()
    {
        return Err(invalid(format!(
            "{} declares no allowed values",
            definition.id
        )));
    }
    let mut seen = BTreeSet::new();
    for option in &definition.options {
        if !seen.insert(option.value.as_str()) {
            return Err(invalid(format!(
                "{} repeats the option {}",
                definition.id, option.value
            )));
        }
        let probe = match definition.control {
            CliParameterControl::MultiEnum
            | CliParameterControl::OrderedStringList
            | CliParameterControl::PathList => {
                CliParameterSelection::text_list(vec![option.value.clone()])
            }
            _ => CliParameterSelection::text(option.value.clone()),
        };
        normalize_selection(definition, &probe).map_err(|error| {
            invalid(format!(
                "{} declares the invalid option {} ({})",
                definition.id,
                option.value,
                error.details.get("reason").cloned().unwrap_or_default()
            ))
        })?;
    }
    if let Some(value) = definition.default_selection.as_value() {
        if !definition.renderer.accepts(value) {
            return Err(invalid(format!("{} has a mistyped default", definition.id)));
        }
        normalize_selection(definition, &definition.default_selection)
            .map_err(|_| invalid(format!("{} has an invalid default", definition.id)))?;
        if matches!(value, CliParameterValue::Boolean(false))
            && !definition.renderer.supports_explicit_false()
        {
            return Err(invalid(format!(
                "{} defaults to a false value its renderer cannot emit",
                definition.id
            )));
        }
    }
    Ok(())
}

fn validate_compatibility(
    definition: &CliParameterDefinition,
) -> Result<(), CliParameterDomainError> {
    if definition.compatibility.platforms.is_empty() {
        return Err(invalid(format!("{} supports no platform", definition.id)));
    }
    let ranges = std::iter::once(&definition.compatibility).chain(
        definition
            .options
            .iter()
            .filter_map(|o| o.compatibility.as_ref()),
    );
    for range in ranges {
        if let (Some(min), Some(max)) = (&range.min_version, &range.max_version) {
            if min > max {
                return Err(invalid(format!(
                    "{} declares a contradictory version range",
                    definition.id
                )));
            }
        }
    }
    Ok(())
}

pub(crate) fn validate(catalog: &CliParameterCatalog) -> Result<(), CliParameterDomainError> {
    if catalog.catalog_version.trim().is_empty()
        || !catalog
            .catalog_version
            .split('.')
            .all(|part| !part.is_empty() && part.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(invalid("catalogVersion is missing or unparseable"));
    }
    if catalog.selection_schema_version == 0 {
        return Err(invalid("selectionSchemaVersion must be positive"));
    }
    let declared = catalog.agent_ids();
    if declared != MANAGED_CLI_AGENT_IDS.to_vec() {
        return Err(invalid(format!(
            "agent ids must be exactly {MANAGED_CLI_AGENT_IDS:?} in order, found {declared:?}"
        )));
    }
    for agent in &catalog.agents {
        let mut ids = BTreeSet::new();
        let mut flags = BTreeSet::new();
        for definition in &agent.parameters {
            if !ids.insert(definition.id.as_str()) {
                return Err(invalid(format!(
                    "{} repeats the parameter id {}",
                    agent.agent_id, definition.id
                )));
            }
            if definition.launch_scopes.is_empty() {
                return Err(invalid(format!(
                    "{} declares no launch scope",
                    definition.id
                )));
            }
            if !renderer_matches_control(definition.control, &definition.renderer) {
                return Err(invalid(format!(
                    "{} pairs control {:?} with an incompatible renderer",
                    definition.id, definition.control
                )));
            }
            validate_flags(definition)?;
            for flag in definition.renderer.flags() {
                if !flags.insert(flag.to_string()) {
                    return Err(invalid(format!(
                        "{} repeats the flag {flag}",
                        agent.agent_id
                    )));
                }
            }
            validate_localization(definition)?;
            validate_audit(definition)?;
            validate_values(definition)?;
            validate_compatibility(definition)?;
        }
        for definition in &agent.parameters {
            for reference in definition
                .dependencies
                .requires_all
                .iter()
                .map(|condition| condition.parameter_id.as_str())
                .chain(
                    definition
                        .dependencies
                        .conflicts_with
                        .iter()
                        .map(String::as_str),
                )
            {
                if !ids.contains(reference) {
                    return Err(invalid(format!(
                        "{}.{} references the unknown parameter {reference}",
                        agent.agent_id, definition.id
                    )));
                }
            }
        }
        if let Some(cycle) = find_requires_cycle(&agent.parameters) {
            return Err(invalid(format!(
                "{} has a dependency cycle: {cycle}",
                agent.agent_id
            )));
        }
    }
    Ok(())
}
