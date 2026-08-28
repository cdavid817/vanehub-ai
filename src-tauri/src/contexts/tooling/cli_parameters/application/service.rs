use super::error::CliParameterApplicationError;
use super::models::{
    CliParameterPreview, CliParameterProfileView, CliParameterSavedPreviews,
    PreviewCliParameterProfileInput, ReplaceCliParameterProfile, ResetCliParameterProfileInput,
    SaveCliParameterProfileInput,
};
use super::ports::{
    CliInstallationSnapshotPort, CliParameterCatalogPort, CliParameterDiagnosticsPort,
    CliParameterDirectoryPort, CliParameterProfileRepository,
};
use super::support::{
    build_field_views, declared_diagnostics, evaluate_support, installation_diagnostics,
    missing_directory_ids, normalize_submitted, reject_unsupported_new_values,
};
use crate::contexts::tooling::cli_parameters::domain::catalog::{
    render_segments, CliParameterCatalog,
};
use crate::contexts::tooling::cli_parameters::domain::compatibility::CliVersionComparator;
use crate::contexts::tooling::cli_parameters::domain::definition::{
    CliLaunchScope, CliParameterDefinition, CliParameterPlatform,
};
use crate::contexts::tooling::cli_parameters::domain::dependency;
use crate::contexts::tooling::cli_parameters::domain::diagnostic::{
    CliParameterDiagnostic, CliParameterDiagnosticCode,
};
use crate::contexts::tooling::cli_parameters::domain::profile::migrate_stored_profile;
use std::collections::BTreeSet;
use std::sync::Arc;

#[derive(Clone)]
pub(crate) struct CliParameterApplicationService {
    pub(crate) catalog: Arc<dyn CliParameterCatalogPort>,
    pub(crate) repository: Arc<dyn CliParameterProfileRepository>,
    pub(crate) installations: Arc<dyn CliInstallationSnapshotPort>,
    pub(crate) directories: Arc<dyn CliParameterDirectoryPort>,
    pub(crate) diagnostics: Arc<dyn CliParameterDiagnosticsPort>,
    pub(crate) comparator: Arc<dyn CliVersionComparator>,
    pub(crate) platform: CliParameterPlatform,
}

pub(crate) struct LoadedProfile {
    pub(crate) view: CliParameterProfileView,
}

impl CliParameterApplicationService {
    fn catalog(&self) -> Result<Arc<CliParameterCatalog>, CliParameterApplicationError> {
        self.catalog.catalog()
    }

    fn require_catalog_version(
        catalog: &CliParameterCatalog,
        agent_id: &str,
        submitted: &str,
    ) -> Result<(), CliParameterApplicationError> {
        if catalog.catalog_version == submitted {
            return Ok(());
        }
        Err(CliParameterApplicationError::CatalogMismatch {
            agent_id: agent_id.to_string(),
            expected_catalog_version: catalog.catalog_version.clone(),
            actual_catalog_version: submitted.to_string(),
        })
    }

    fn editable_definitions(
        catalog: &CliParameterCatalog,
        agent_id: &str,
    ) -> Result<Vec<CliParameterDefinition>, CliParameterApplicationError> {
        Ok(catalog
            .editable_definitions(agent_id)?
            .into_iter()
            .cloned()
            .collect())
    }

    fn emit(&self, diagnostics: &[CliParameterDiagnostic]) {
        let mut seen = BTreeSet::new();
        for diagnostic in diagnostics {
            if seen.insert(diagnostic.dedup_key()) {
                self.diagnostics.emit(diagnostic);
            }
        }
    }

    pub(crate) fn load_profile(
        &self,
        agent_id: &str,
    ) -> Result<LoadedProfile, CliParameterApplicationError> {
        let catalog = self.catalog()?;
        let definitions = Self::editable_definitions(&catalog, agent_id)?;
        let refs = definitions.iter().collect::<Vec<_>>();
        let stored = self.repository.load(agent_id)?;
        let migrated = migrate_stored_profile(&definitions, &stored);
        let snapshot = self.installations.active_installation(agent_id)?;
        let support = evaluate_support(&refs, &snapshot, self.platform, self.comparator.as_ref());

        let mut diagnostics = migrated.diagnostics.clone();
        diagnostics.extend(installation_diagnostics(agent_id, &snapshot));
        diagnostics.extend(dependency::evaluate(
            agent_id,
            &definitions,
            &migrated.selections,
        ));
        diagnostics.extend(declared_diagnostics(agent_id, &refs, &migrated.selections));
        for (parameter_id, count) in
            missing_directory_ids(&refs, &migrated.selections, self.directories.as_ref())
        {
            diagnostics.push(
                CliParameterDiagnostic::new(
                    CliParameterDiagnosticCode::MissingDirectory,
                    agent_id,
                    Some(parameter_id),
                )
                .with_detail("missingCount", count.to_string()),
            );
        }

        let (chat, chat_diagnostics) = render_segments(
            agent_id,
            &definitions,
            &migrated.selections,
            CliLaunchScope::Chat,
            &support,
        );
        let (interactive, interactive_diagnostics) = render_segments(
            agent_id,
            &definitions,
            &migrated.selections,
            CliLaunchScope::Interactive,
            &support,
        );
        diagnostics.extend(chat_diagnostics);
        diagnostics.extend(interactive_diagnostics);
        self.emit(&diagnostics);

        let view = CliParameterProfileView {
            agent_id: agent_id.to_string(),
            catalog_version: catalog.catalog_version.clone(),
            revision: stored.revision,
            updated_at: stored.updated_at.clone(),
            installation: snapshot.clone(),
            fields: build_field_views(&refs, &snapshot, self.platform, self.comparator.as_ref()),
            selections: migrated.selections,
            saved_previews: CliParameterSavedPreviews { chat, interactive },
            diagnostics: dedup(diagnostics),
        };
        Ok(LoadedProfile { view })
    }

    pub(crate) fn list_profiles(
        &self,
    ) -> Result<Vec<CliParameterProfileView>, CliParameterApplicationError> {
        let catalog = self.catalog()?;
        let agent_ids = catalog
            .agent_ids()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        agent_ids
            .iter()
            .map(|agent_id| self.load_profile(agent_id).map(|loaded| loaded.view))
            .collect()
    }

    /// Read-only. It never touches the repository, so no revision or timestamp can move.
    pub(crate) fn preview_profile(
        &self,
        input: &PreviewCliParameterProfileInput,
    ) -> Result<CliParameterPreview, CliParameterApplicationError> {
        let catalog = self.catalog()?;
        if !catalog.contains_agent(&input.agent_id) {
            return Err(
                crate::contexts::tooling::cli_parameters::domain::error::CliParameterDomainError::unknown_agent(
                    &input.agent_id,
                )
                .into(),
            );
        }
        Self::require_catalog_version(&catalog, &input.agent_id, &input.catalog_version)?;
        let definitions = Self::editable_definitions(&catalog, &input.agent_id)?;
        let refs = definitions.iter().collect::<Vec<_>>();
        let normalized = normalize_submitted(&catalog, &input.agent_id, &input.selections)?;
        let snapshot = self.installations.active_installation(&input.agent_id)?;
        let support = evaluate_support(&refs, &snapshot, self.platform, self.comparator.as_ref());

        let mut diagnostics = dependency::evaluate(&input.agent_id, &definitions, &normalized);
        diagnostics.extend(declared_diagnostics(&input.agent_id, &refs, &normalized));
        for (parameter_id, count) in
            missing_directory_ids(&refs, &normalized, self.directories.as_ref())
        {
            diagnostics.push(
                CliParameterDiagnostic::new(
                    CliParameterDiagnosticCode::MissingDirectory,
                    &input.agent_id,
                    Some(parameter_id),
                )
                .with_detail("missingCount", count.to_string()),
            );
        }
        let (segments, render_diagnostics) = render_segments(
            &input.agent_id,
            &definitions,
            &normalized,
            input.scope,
            &support,
        );
        diagnostics.extend(render_diagnostics);

        Ok(CliParameterPreview {
            agent_id: input.agent_id.clone(),
            catalog_version: catalog.catalog_version.clone(),
            scope: input.scope,
            request_id: input.request_id.clone(),
            normalized_selections: normalized,
            segments,
            diagnostics: dedup(diagnostics),
        })
    }

    pub(crate) fn save_profile(
        &self,
        input: &SaveCliParameterProfileInput,
    ) -> Result<CliParameterProfileView, CliParameterApplicationError> {
        let catalog = self.catalog()?;
        Self::require_catalog_version(&catalog, &input.agent_id, &input.catalog_version)?;
        let definitions = Self::editable_definitions(&catalog, &input.agent_id)?;
        let refs = definitions.iter().collect::<Vec<_>>();
        let normalized = normalize_submitted(&catalog, &input.agent_id, &input.selections)?;

        let snapshot = self.installations.active_installation(&input.agent_id)?;
        let support = evaluate_support(&refs, &snapshot, self.platform, self.comparator.as_ref());
        let stored = self.repository.load(&input.agent_id)?;
        let baseline = migrate_stored_profile(&definitions, &stored).selections;
        reject_unsupported_new_values(&input.agent_id, &normalized, &baseline, &support)?;

        let blocking = dependency::evaluate(&input.agent_id, &definitions, &normalized)
            .into_iter()
            .find(|diagnostic| diagnostic.blocking);
        if let Some(diagnostic) = blocking {
            return Err(blocking_error(&input.agent_id, &diagnostic));
        }

        self.repository
            .replace_if_revision(ReplaceCliParameterProfile {
                agent_id: input.agent_id.clone(),
                expected_revision: input.expected_revision,
                catalog_version: catalog.catalog_version.clone(),
                selections: normalized,
            })?;
        Ok(self.load_profile(&input.agent_id)?.view)
    }

    pub(crate) fn reset_profile(
        &self,
        input: &ResetCliParameterProfileInput,
    ) -> Result<CliParameterProfileView, CliParameterApplicationError> {
        let catalog = self.catalog()?;
        Self::require_catalog_version(&catalog, &input.agent_id, &input.catalog_version)?;
        catalog.editable_definitions(&input.agent_id)?;
        self.repository.reset_if_revision(
            &input.agent_id,
            input.expected_revision,
            &catalog.catalog_version,
        )?;
        Ok(self.load_profile(&input.agent_id)?.view)
    }

    pub(crate) fn catalog_version(&self) -> Result<String, CliParameterApplicationError> {
        Ok(self.catalog()?.catalog_version.clone())
    }
}

fn blocking_error(
    agent_id: &str,
    diagnostic: &CliParameterDiagnostic,
) -> CliParameterApplicationError {
    use crate::contexts::tooling::cli_parameters::domain::error::{
        CliParameterDomainError, CliParameterErrorCode,
    };
    let code = match diagnostic.code {
        CliParameterDiagnosticCode::ConflictingSelection => CliParameterErrorCode::Conflict,
        _ => CliParameterErrorCode::DependencyUnsatisfied,
    };
    let mut error = CliParameterDomainError::new(code).for_agent(agent_id);
    if let Some(parameter_id) = &diagnostic.parameter_id {
        error = error.for_parameter(parameter_id);
    }
    for (key, value) in &diagnostic.details {
        error = error.with_detail(key, value.clone());
    }
    error.into()
}

fn dedup(diagnostics: Vec<CliParameterDiagnostic>) -> Vec<CliParameterDiagnostic> {
    let mut seen = BTreeSet::new();
    diagnostics
        .into_iter()
        .filter(|diagnostic| seen.insert(diagnostic.dedup_key()))
        .collect()
}
