use super::error::CliParameterApplicationError;
use super::models::CliParameterFieldView;
use super::ports::CliParameterDirectoryPort;
use crate::contexts::tooling::cli_parameters::domain::catalog::CliParameterCatalog;
use crate::contexts::tooling::cli_parameters::domain::compatibility::{
    evaluate_definition, evaluate_option, CliInstallationSnapshot, CliParameterSupport,
    CliVersionComparator,
};
use crate::contexts::tooling::cli_parameters::domain::definition::{
    CliParameterControl, CliParameterDefinition, CliParameterPlatform,
};
use crate::contexts::tooling::cli_parameters::domain::diagnostic::{
    CliParameterDiagnostic, CliParameterDiagnosticCode,
};
use crate::contexts::tooling::cli_parameters::domain::error::CliParameterDomainError;
use crate::contexts::tooling::cli_parameters::domain::selection::{
    CliParameterSelection, CliParameterSelectionMap,
};
use crate::contexts::tooling::cli_parameters::domain::validation::normalize_selection;
use std::collections::BTreeMap;

pub(crate) fn evaluate_support(
    definitions: &[&CliParameterDefinition],
    snapshot: &CliInstallationSnapshot,
    platform: CliParameterPlatform,
    comparator: &dyn CliVersionComparator,
) -> BTreeMap<String, CliParameterSupport> {
    definitions
        .iter()
        .map(|definition| {
            (
                definition.id.clone(),
                evaluate_definition(definition, snapshot, platform, comparator),
            )
        })
        .collect()
}

pub(crate) fn build_field_views(
    definitions: &[&CliParameterDefinition],
    snapshot: &CliInstallationSnapshot,
    platform: CliParameterPlatform,
    comparator: &dyn CliVersionComparator,
) -> Vec<CliParameterFieldView> {
    definitions
        .iter()
        .map(|definition| CliParameterFieldView {
            definition: (*definition).clone(),
            support: evaluate_definition(definition, snapshot, platform, comparator),
            option_support: definition
                .options
                .iter()
                .map(|option| {
                    (
                        option.value.clone(),
                        evaluate_option(definition, option, snapshot, platform, comparator),
                    )
                })
                .collect(),
        })
        .collect()
}

/// Accepts only editable parameter ids and normalizes each submitted selection. Missing entries
/// fall back to the registry default rather than to a previously stored value.
pub(crate) fn normalize_submitted(
    catalog: &CliParameterCatalog,
    agent_id: &str,
    submitted: &CliParameterSelectionMap,
) -> Result<CliParameterSelectionMap, CliParameterApplicationError> {
    let definitions = catalog.editable_definitions(agent_id)?;
    let editable_ids = definitions
        .iter()
        .map(|definition| definition.id.as_str())
        .collect::<Vec<_>>();
    for parameter_id in submitted.keys() {
        if !editable_ids.contains(&parameter_id.as_str()) {
            return Err(CliParameterDomainError::unknown_parameter(agent_id, parameter_id).into());
        }
    }
    let mut normalized = CliParameterSelectionMap::new();
    for definition in &definitions {
        let selection = submitted
            .get(&definition.id)
            .cloned()
            .unwrap_or_else(|| definition.default_selection.clone());
        normalized.insert(
            definition.id.clone(),
            normalize_selection(definition, &selection)?,
        );
    }
    Ok(normalized)
}

/// Rejects a newly introduced value for a parameter whose compatibility cannot be confirmed. An
/// already stored value is preserved for repair instead, so loading a profile never fails.
pub(crate) fn reject_unsupported_new_values(
    agent_id: &str,
    candidate: &CliParameterSelectionMap,
    baseline: &CliParameterSelectionMap,
    support: &BTreeMap<String, CliParameterSupport>,
) -> Result<(), CliParameterApplicationError> {
    for (parameter_id, selection) in candidate {
        if selection.is_inherit() {
            continue;
        }
        if baseline.get(parameter_id) == Some(selection) {
            continue;
        }
        let Some(state) = support.get(parameter_id) else {
            continue;
        };
        if state.blocks_new_value() {
            return Err(CliParameterDomainError::new(
                crate::contexts::tooling::cli_parameters::domain::error::CliParameterErrorCode::UnsupportedVersion,
            )
            .for_agent(agent_id)
            .for_parameter(parameter_id)
            .with_detail("support", serde_json::to_string(state).unwrap_or_default())
            .into());
        }
    }
    Ok(())
}

pub(crate) fn installation_diagnostics(
    agent_id: &str,
    snapshot: &CliInstallationSnapshot,
) -> Vec<CliParameterDiagnostic> {
    let mut diagnostics = Vec::new();
    if !snapshot.installed || !snapshot.runnable {
        diagnostics.push(CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::CliNotInstalled,
            agent_id,
            None,
        ));
    } else if snapshot.version.is_none() {
        diagnostics.push(CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::VersionUnknown,
            agent_id,
            None,
        ));
    }
    if snapshot.conflict {
        diagnostics.push(CliParameterDiagnostic::new(
            CliParameterDiagnosticCode::ActiveInstallationConflict,
            agent_id,
            None,
        ));
    }
    diagnostics
}

/// A directory that disappeared after it was saved becomes a warning, not an invalid profile.
pub(crate) fn missing_directory_ids(
    definitions: &[&CliParameterDefinition],
    selections: &CliParameterSelectionMap,
    directories: &dyn CliParameterDirectoryPort,
) -> Vec<(String, usize)> {
    let mut missing = Vec::new();
    for definition in definitions {
        if definition.control != CliParameterControl::PathList {
            continue;
        }
        let Some(CliParameterSelection::Value { value }) = selections.get(&definition.id) else {
            continue;
        };
        let Some(entries) = value.as_text_list() else {
            continue;
        };
        let count = entries
            .iter()
            .filter(|entry| !directories.directory_exists(entry))
            .count();
        if count > 0 {
            missing.push((definition.id.clone(), count));
        }
    }
    missing
}

/// Removes the paths that no longer exist so a moved folder cannot fail an otherwise valid launch.
pub(crate) fn drop_missing_directories(
    definitions: &[&CliParameterDefinition],
    selections: &mut CliParameterSelectionMap,
    directories: &dyn CliParameterDirectoryPort,
) {
    for definition in definitions {
        if definition.control != CliParameterControl::PathList {
            continue;
        }
        let Some(CliParameterSelection::Value { value }) = selections.get(&definition.id) else {
            continue;
        };
        let Some(entries) = value.as_text_list() else {
            continue;
        };
        let kept = entries
            .iter()
            .filter(|entry| directories.directory_exists(entry))
            .cloned()
            .collect::<Vec<_>>();
        if kept.len() == entries.len() {
            continue;
        }
        let replacement = if kept.is_empty() {
            CliParameterSelection::Inherit
        } else {
            CliParameterSelection::text_list(kept)
        };
        selections.insert(definition.id.clone(), replacement);
    }
}

/// Advisory diagnostics a definition declares for itself, such as model-dependent value support.
pub(crate) fn declared_diagnostics(
    agent_id: &str,
    definitions: &[&CliParameterDefinition],
    selections: &CliParameterSelectionMap,
) -> Vec<CliParameterDiagnostic> {
    let mut diagnostics = Vec::new();
    for definition in definitions {
        let explicit = selections
            .get(&definition.id)
            .is_some_and(|selection| !selection.is_inherit());
        if !explicit {
            continue;
        }
        for code in &definition.diagnostics {
            if code == "MODEL_DEPENDENT_VALUE" {
                diagnostics.push(CliParameterDiagnostic::new(
                    CliParameterDiagnosticCode::ModelDependentValue,
                    agent_id,
                    Some(definition.id.clone()),
                ));
            }
        }
    }
    diagnostics
}
