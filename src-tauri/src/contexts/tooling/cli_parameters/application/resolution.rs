use super::error::CliParameterApplicationError;
use super::models::{ResolveCliLaunchParametersInput, ResolvedCliLaunchParameters};
use super::service::CliParameterApplicationService;
use super::support::{drop_missing_directories, evaluate_support, normalize_submitted};
use crate::contexts::tooling::cli_parameters::domain::catalog::render_segments;
use crate::contexts::tooling::cli_parameters::domain::definition::{
    CliParameterDefinition, CliParameterOwnership,
};
use crate::contexts::tooling::cli_parameters::domain::dependency;
use crate::contexts::tooling::cli_parameters::domain::profile::migrate_stored_profile;
use crate::contexts::tooling::cli_parameters::domain::selection::CliParameterSelectionMap;
use crate::contexts::tooling::cli_parameters::domain::validation::normalize_selection;

impl CliParameterApplicationService {
    /// The only launch-facing entry point. It returns validated token segments and never exposes
    /// persistence, catalog internals, or unvalidated stored values.
    pub(crate) fn resolve_launch_parameters(
        &self,
        input: &ResolveCliLaunchParametersInput,
    ) -> Result<ResolvedCliLaunchParameters, CliParameterApplicationError> {
        let catalog = self.catalog.catalog()?;
        let all_definitions = catalog.definitions(&input.agent_id)?.to_vec();
        let editable = all_definitions
            .iter()
            .filter(|definition| definition.is_user_editable())
            .cloned()
            .collect::<Vec<_>>();
        let editable_refs = editable.iter().collect::<Vec<_>>();

        let stored = self.repository.load(&input.agent_id)?;
        let migrated = migrate_stored_profile(&editable, &stored);
        let snapshot = self.installations.active_installation(&input.agent_id)?;
        let support = evaluate_support(
            &editable_refs,
            &snapshot,
            self.platform,
            self.comparator.as_ref(),
        );

        // Ordinary precedence: an explicit per-message value beats the saved profile, and the
        // saved profile beats inherited provider behaviour.
        let mut ordinary = migrated.selections.clone();
        let overrides = normalize_submitted(&catalog, &input.agent_id, &input.message_overrides)?;
        for (parameter_id, selection) in overrides {
            if input.message_overrides.contains_key(&parameter_id) {
                ordinary.insert(parameter_id, selection);
            }
        }
        drop_missing_directories(&editable_refs, &mut ordinary, self.directories.as_ref());

        let mut diagnostics = migrated.diagnostics.clone();
        diagnostics.extend(dependency::evaluate(&input.agent_id, &editable, &ordinary));

        // A dependency that is unsatisfied at launch drops the dependent value rather than
        // failing an otherwise valid launch.
        let unsatisfied = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.blocking)
            .filter_map(|diagnostic| diagnostic.parameter_id.clone())
            .collect::<Vec<_>>();
        for parameter_id in unsatisfied {
            ordinary.insert(
                parameter_id,
                crate::contexts::tooling::cli_parameters::domain::selection::CliParameterSelection::Inherit,
            );
        }

        let policy = normalize_policy_overrides(&all_definitions, &input.policy_overrides)?;
        let mut combined = ordinary;
        for (parameter_id, selection) in policy {
            combined.insert(parameter_id, selection);
        }

        let (segments, render_diagnostics) = render_segments(
            &input.agent_id,
            &all_definitions,
            &combined,
            input.scope,
            &support,
        );
        diagnostics.extend(render_diagnostics);
        // Associate every launch diagnostic with the operation that triggered it, when there is
        // one. An Agent Terminal launch has none, and that is not an error.
        if let Some(operation_id) = &input.execution_context.operation_id {
            for diagnostic in &mut diagnostics {
                *diagnostic = diagnostic.clone().with_detail("operationId", operation_id);
            }
        }
        self.emit_runtime_diagnostics(&diagnostics, stored.revision);

        Ok(ResolvedCliLaunchParameters {
            global_tokens: segments.global_values(),
            invocation_tokens: segments.invocation_values(),
            selections: combined,
            diagnostics,
            profile_revision: stored.revision,
            catalog_version: catalog.catalog_version.clone(),
        })
    }

    fn emit_runtime_diagnostics(
        &self,
        diagnostics: &[crate::contexts::tooling::cli_parameters::domain::diagnostic::CliParameterDiagnostic],
        revision: i64,
    ) {
        let mut seen = std::collections::BTreeSet::new();
        for diagnostic in diagnostics {
            let key = format!("{}|{revision}", diagnostic.dedup_key());
            if seen.insert(key) {
                self.diagnostics.emit(diagnostic);
            }
        }
    }
}

/// Policy-governed values come only from the policy projection. They are still validated against
/// the registry so a policy cannot introduce an unrenderable or unknown token.
fn normalize_policy_overrides(
    definitions: &[CliParameterDefinition],
    overrides: &CliParameterSelectionMap,
) -> Result<CliParameterSelectionMap, CliParameterApplicationError> {
    let mut normalized = CliParameterSelectionMap::new();
    for (parameter_id, selection) in overrides {
        let Some(definition) = definitions.iter().find(|entry| &entry.id == parameter_id) else {
            return Err(
                crate::contexts::tooling::cli_parameters::domain::error::CliParameterDomainError::unknown_parameter(
                    definitions.first().map(|entry| entry.agent_id.as_str()).unwrap_or_default(),
                    parameter_id,
                )
                .into(),
            );
        };
        if definition.ownership == CliParameterOwnership::UserEditable {
            return Err(
                crate::contexts::tooling::cli_parameters::domain::error::CliParameterDomainError::unknown_parameter(
                    &definition.agent_id,
                    parameter_id,
                )
                .into(),
            );
        }
        normalized.insert(
            parameter_id.clone(),
            normalize_selection(definition, selection)?,
        );
    }
    Ok(normalized)
}
