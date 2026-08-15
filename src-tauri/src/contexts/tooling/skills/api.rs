use crate::contexts::tooling::skills::application::{
    SkillApplicationService, SkillOverlayApplicationService,
};
use crate::contexts::tooling::skills::configuration_facade::SkillConfigurationFacade;
use crate::contexts::tooling::skills::infrastructure::require_base_revision;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) use crate::contexts::tooling::skills::application::{
    OverlayApplicationError as OverlayError, OverlayDetail, OverlayHistoryPage,
    OverlayHistoryQuery, OverlayImportRequest, OverlayImportReview, OverlayKey,
    OverlayMutationOutcome, OverlayMutationRequest, OverlayPreview, OverlayPromotionRequest,
    OverlayReconciliationPreview, OverlayReconciliationRequest, OverlaySummary, SkillAccessRefusal,
    SkillAgentKind, SkillAgentMountPath, SkillApplicationError as SkillError, SkillBackupEntry,
    SkillCreateRequest, SkillDiscoveryRequest, SkillDiscoveryResult, SkillDriftReport,
    SkillFailure, SkillImportRequest, SkillListResult, SkillLoadOutcome, SkillLoadRequest,
    SkillMountMigrationReport, SkillOverview, SkillPreview, SkillPromptForAgent, SkillRecord,
    SkillResourceEntry, SkillResourceIndex, SkillResourceReadOutcome, SkillResourceReadRequest,
    SkillScopeQuery, SkillShadowSummary, SkillSyncResult, SkillUpdateRequest,
    UtilitySkillResolutionOutcome,
};
pub(crate) use crate::contexts::tooling::skills::application::{
    SkillConfigurableState, SkillConfigurationOverview,
};
pub(crate) use crate::contexts::tooling::skills::domain::{
    SkillAvailability, SkillConfigDrift, SkillConfigField, SkillConfigFieldType, SkillConfigGroup,
    SkillConfigPresentation, SkillConfigProperty, SkillConfigProvenance, SkillConfigReadiness,
    SkillConfigRevision, SkillConfigScalarType, SkillConfigSchema, SkillConfigScope,
    SkillConfigValue, SkillDelivery, SkillDomainError, SkillDriftIssueType, SkillId, SkillKey,
    SkillLayer, SkillLocation, SkillMetadata, SkillMountPath, SkillOrigin, SkillScope,
    SkillSecretIntent, SkillSecretState, SkillSource, SkillTrust, SkillType,
};
pub(crate) use crate::contexts::tooling::skills::infrastructure::{
    consumption_for_binding, ConfigurationConsumption, DeletionRetention, ObsoleteFieldChoice,
    ReconciliationPlan, ResolvedSkillConfiguration, ScopeConfigurationState, SecretRecovery,
    SkillConfigCleanupState, SkillConfigurationError, SkillConfigurationRequest,
    SkillConfigurationSaveResult, StoredSkillConfiguration,
};

#[derive(Clone)]
pub(crate) struct SkillApi {
    service: SkillApplicationService,
    overlays: Option<SkillOverlayApplicationService>,
    configuration: Option<SkillConfigurationFacade>,
}

/// What the Configuration surface needs in one response: whether this revision is configurable at
/// all, the normalized schema when it is, and the resolved effective state. Assembled here rather
/// than by the caller so a schema and the values validated against it always come from the same
/// read of the winning revision.
pub(crate) struct SkillConfigurationView {
    pub(crate) overview: SkillConfigurationOverview,
    pub(crate) schema: Option<SkillConfigSchema>,
    pub(crate) resolved: Option<ResolvedSkillConfiguration>,
    pub(crate) workspace_identity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliSkillEvidenceEntry {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliSkillEvidenceSnapshot {
    pub(crate) manifest_hash: Option<String>,
    pub(crate) mounted: Vec<CliSkillEvidenceEntry>,
    pub(crate) configured_binding_ids: Vec<String>,
}

impl SkillApi {
    pub(crate) fn new(service: SkillApplicationService) -> Self {
        Self {
            service,
            overlays: None,
            configuration: None,
        }
    }

    pub(crate) fn with_overlay_service(mut self, overlays: SkillOverlayApplicationService) -> Self {
        self.overlays = Some(overlays);
        self
    }

    pub(crate) fn with_configuration(mut self, configuration: SkillConfigurationFacade) -> Self {
        self.configuration = Some(configuration);
        self
    }

    fn overlays(&self) -> Result<&SkillOverlayApplicationService, SkillError> {
        self.overlays
            .as_ref()
            .ok_or_else(|| SkillError::Repository("Overlay service is unavailable".to_string()))
    }

    fn configuration(&self) -> Result<&SkillConfigurationFacade, SkillError> {
        self.configuration.as_ref().ok_or_else(|| {
            SkillError::Repository("Skill configuration service is unavailable".to_string())
        })
    }

    pub(crate) fn overlay_detail(
        &self,
        skill_id: &SkillId,
        workspace: Option<&str>,
    ) -> Result<OverlayDetail, SkillError> {
        self.overlays()?.query(skill_id, workspace)
    }

    pub(crate) fn overlay_summary(
        &self,
        skill_id: &SkillId,
        workspace: Option<&str>,
    ) -> Result<OverlaySummary, SkillError> {
        Ok(self.overlay_detail(skill_id, workspace)?.summary)
    }

    pub(crate) fn overlay_preview(
        &self,
        request: &OverlayMutationRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayPreview, SkillError> {
        self.overlays()?.preview(request, workspace)
    }

    pub(crate) fn overlay_history(
        &self,
        key: &OverlayKey,
        workspace: Option<&str>,
        query: &OverlayHistoryQuery,
    ) -> Result<OverlayHistoryPage, SkillError> {
        self.overlays()?.history(key, workspace, query)
    }

    pub(crate) fn import_overlay(
        &self,
        request: &OverlayImportRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayImportReview, SkillError> {
        self.overlays()?.import_overlay(request, workspace)
    }

    pub(crate) fn promote_overlay(
        &self,
        request: &OverlayPromotionRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillError> {
        self.overlays()?.promote_import(request, workspace)
    }

    pub(crate) fn preview_overlay_reconciliation(
        &self,
        request: &OverlayReconciliationRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayReconciliationPreview, SkillError> {
        self.overlays()?.preview_reconciliation(request, workspace)
    }

    pub(crate) fn reconcile_overlay(
        &self,
        request: &OverlayReconciliationRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillError> {
        self.overlays()?.reconcile(request, workspace)
    }

    pub(crate) fn overlay_mutation(
        &self,
        operation: OverlayMutationOperation,
        request: &OverlayMutationRequest,
        workspace: Option<&str>,
    ) -> Result<OverlayMutationOutcome, SkillError> {
        let overlays = self.overlays()?;
        match operation {
            OverlayMutationOperation::CreatePatch => {
                overlays.create_exact_patch(request, workspace)
            }
            OverlayMutationOperation::CreateGuidance => {
                overlays.create_learned_guidance(request, workspace)
            }
            OverlayMutationOperation::AddFile => overlays.add_supporting_file(request, workspace),
            OverlayMutationOperation::ReplaceFile => {
                overlays.replace_supporting_file(request, workspace)
            }
            OverlayMutationOperation::DisablePatch => {
                overlays.disable_exact_patch(request, workspace)
            }
            OverlayMutationOperation::DisableGuidance => {
                overlays.disable_learned_guidance(request, workspace)
            }
            OverlayMutationOperation::DisableFile => {
                overlays.disable_supporting_file(request, workspace)
            }
            OverlayMutationOperation::RevertPatch => {
                overlays.revert_exact_patch(request, workspace)
            }
            OverlayMutationOperation::RevertGuidance => {
                overlays.revert_learned_guidance(request, workspace)
            }
            OverlayMutationOperation::RevertFile => {
                overlays.revert_supporting_file(request, workspace)
            }
        }
    }

    pub(crate) fn list(&self, query: SkillScopeQuery) -> Result<SkillListResult, SkillError> {
        self.service.list_skills(query)
    }

    pub(crate) fn cli_evidence_snapshot(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<CliSkillEvidenceSnapshot, SkillError> {
        let mut records = BTreeMap::new();
        let global = SkillLocation::new(SkillScope::Global, None)
            .map_err(|error| SkillError::Validation(error.to_string()))?;
        for record in self.list(SkillScopeQuery { location: global })?.skills {
            records.insert(record.key.id.as_str().to_string(), record);
        }
        if let Some(workspace_path) = workspace_path {
            let workspace = SkillLocation::new(SkillScope::Workspace, Some(workspace_path))
                .map_err(|error| SkillError::Validation(error.to_string()))?;
            for record in self
                .list(SkillScopeQuery {
                    location: workspace,
                })?
                .skills
            {
                records.insert(record.key.id.as_str().to_string(), record);
            }
        }
        let mut configured_binding_ids = Vec::new();
        let mut mounted = Vec::new();
        for (skill_id, record) in records {
            if !record.enabled {
                continue;
            }
            if let Some(binding) = record
                .bindings
                .iter()
                .find(|binding| binding.agent_id == agent_id)
            {
                configured_binding_ids.push(skill_id.clone());
                if binding.mounted {
                    mounted.push(CliSkillEvidenceEntry {
                        skill_id,
                        revision: record.managed_source.content_hash,
                    });
                }
            }
        }
        let manifest_hash = (!mounted.is_empty()).then(|| {
            let mut digest = Sha256::new();
            for skill in &mounted {
                digest.update(skill.skill_id.as_bytes());
                digest.update(b":");
                digest.update(skill.revision.as_bytes());
                digest.update(b"\n");
            }
            digest
                .finalize()
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect()
        });
        Ok(CliSkillEvidenceSnapshot {
            manifest_hash,
            mounted,
            configured_binding_ids,
        })
    }

    pub(crate) fn list_mount_paths(&self) -> Result<Vec<SkillAgentMountPath>, SkillError> {
        self.service.list_mount_paths()
    }

    pub(crate) fn update_mount_path(
        &self,
        agent_id: String,
        mount_path: SkillMountPath,
    ) -> Result<SkillMountMigrationReport, SkillError> {
        self.service.update_mount_path(agent_id, mount_path)
    }

    pub(crate) fn create(&self, request: SkillCreateRequest) -> Result<SkillRecord, SkillError> {
        self.service.create_skill(request)
    }

    pub(crate) fn update(&self, request: SkillUpdateRequest) -> Result<SkillRecord, SkillError> {
        self.service.update_skill(request)
    }

    pub(crate) fn delete(&self, key: SkillKey) -> Result<(), SkillError> {
        self.service.delete_skill(key)
    }

    pub(crate) fn restore_builtin(&self, id: SkillId) -> Result<SkillRecord, SkillError> {
        self.service.restore_builtin(id)
    }

    pub(crate) fn set_enabled(
        &self,
        key: SkillKey,
        enabled: bool,
    ) -> Result<SkillRecord, SkillError> {
        self.service.set_enabled(key, enabled)
    }

    pub(crate) fn set_bindings(
        &self,
        key: SkillKey,
        agent_ids: Vec<String>,
    ) -> Result<SkillRecord, SkillError> {
        self.service.set_bindings(key, agent_ids)
    }

    pub(crate) fn overview(&self, query: SkillScopeQuery) -> Result<SkillOverview, SkillError> {
        self.service.skill_overview(query)
    }

    pub(crate) fn bind_to_cli_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<SkillRecord, SkillError> {
        self.service.bind_skill_to_cli_agent(key, agent_id)
    }

    pub(crate) fn unbind_from_cli_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<SkillRecord, SkillError> {
        self.service.unbind_skill_from_cli_agent(key, agent_id)
    }

    pub(crate) fn preview(&self, key: SkillKey) -> Result<SkillPreview, SkillError> {
        self.service.preview_skill(key)
    }

    pub(crate) fn bind_to_api_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<(), SkillError> {
        self.service.bind_skill_to_api_agent(key, agent_id)
    }

    pub(crate) fn unbind_from_api_agent(
        &self,
        key: SkillKey,
        agent_id: String,
    ) -> Result<(), SkillError> {
        self.service.unbind_skill_from_api_agent(key, agent_id)
    }

    pub(crate) fn list_api_agent_bindings(&self, key: SkillKey) -> Result<Vec<String>, SkillError> {
        self.service.list_api_agent_bindings(key)
    }

    pub(crate) fn bound_skill_prompts_for_api_agent(
        &self,
        agent_id: &str,
        workspace_path: Option<&str>,
    ) -> Result<Vec<SkillPromptForAgent>, SkillError> {
        self.service
            .bound_skill_prompts_for_api_agent(agent_id, workspace_path)
    }

    pub(crate) fn list_for_agent(
        &self,
        request: SkillDiscoveryRequest,
    ) -> Result<SkillDiscoveryResult, SkillError> {
        self.service.list_skills_for_agent(request)
    }

    pub(crate) fn load_for_agent(
        &self,
        request: SkillLoadRequest,
    ) -> Result<SkillLoadOutcome, SkillError> {
        self.service.load_skill_for_agent(request)
    }

    pub(crate) fn resolve_utility_for_execution(
        &self,
        id_or_alias: &str,
        workspace_path: Option<&str>,
    ) -> Result<UtilitySkillResolutionOutcome, SkillError> {
        self.service
            .resolve_utility_for_execution(id_or_alias, workspace_path)
    }

    pub(crate) fn read_resource_for_agent(
        &self,
        request: SkillResourceReadRequest,
    ) -> Result<SkillResourceReadOutcome, SkillError> {
        self.service.read_skill_resource_for_agent(request)
    }

    pub(crate) fn import(&self, request: SkillImportRequest) -> Result<SkillRecord, SkillError> {
        self.service.import_skill(request)
    }

    pub(crate) fn detect_drift(
        &self,
        query: SkillScopeQuery,
    ) -> Result<SkillDriftReport, SkillError> {
        self.service.detect_skill_drift(query)
    }

    pub(crate) fn sync_drift(&self, query: SkillScopeQuery) -> Result<SkillSyncResult, SkillError> {
        self.service.sync_skill_drift(query)
    }

    pub(crate) fn select_workspace_directory(&self) -> Result<Option<String>, SkillError> {
        self.service.select_workspace_directory()
    }

    pub(crate) fn configuration_view(
        &self,
        key: SkillKey,
    ) -> Result<SkillConfigurationView, SkillError> {
        let workspace_identity = workspace_identity(&key);
        let context = self.service.configuration_context(&key)?;
        let resolved = match context.schema.as_ref() {
            Some(schema) => Some(self.configuration()?.read(
                schema,
                key.id.as_str(),
                &workspace_identity,
            )?),
            None => None,
        };
        Ok(SkillConfigurationView {
            overview: context.overview,
            schema: context.schema,
            resolved,
            workspace_identity: (!workspace_identity.is_empty()).then_some(workspace_identity),
        })
    }

    pub(crate) fn preview_configuration(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
    ) -> Result<Result<ResolvedSkillConfiguration, SkillConfigurationError>, SkillError> {
        let (schema, base_revision) = self.configuration_schema(key)?;
        if let Err(error) = require_base_revision(&base_revision, request) {
            return Ok(Err(error));
        }
        Ok(self.configuration()?.preview(&schema, request))
    }

    pub(crate) fn save_configuration(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
    ) -> Result<Result<SkillConfigurationSaveResult, SkillConfigurationError>, SkillError> {
        let (schema, base_revision) = self.configuration_schema(key)?;
        if let Err(error) = require_base_revision(&base_revision, request) {
            return Ok(Err(error));
        }
        match self.configuration()?.save(&schema, request) {
            Err(error) => Ok(Err(error)),
            Ok(result) => Ok(Ok(self.rescope_preview(&schema, key, result)?)),
        }
    }

    /// Removes one non-secret property from one scope. A secret is refused here on purpose: its
    /// reset destroys a credential rather than a stored value, so it has to be asked for through
    /// the operation that says so.
    pub(crate) fn reset_configuration_property(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
        property_key: &str,
    ) -> Result<Result<SkillConfigurationSaveResult, SkillConfigurationError>, SkillError> {
        self.reset_configured_property(key, request, property_key, false)
    }

    pub(crate) fn clear_configuration_secret(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
        property_key: &str,
    ) -> Result<Result<SkillConfigurationSaveResult, SkillConfigurationError>, SkillError> {
        self.reset_configured_property(key, request, property_key, true)
    }

    pub(crate) fn reconcile_configuration(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
        plan: &ReconciliationPlan,
    ) -> Result<Result<SkillConfigurationSaveResult, SkillConfigurationError>, SkillError> {
        let (schema, base_revision) = self.configuration_schema(key)?;
        if let Err(error) = require_base_revision(&base_revision, request) {
            return Ok(Err(error));
        }
        match self.configuration()?.reconcile(&schema, request, plan) {
            Err(error) => Ok(Err(error)),
            Ok(result) => Ok(Ok(self.rescope_preview(&schema, key, result)?)),
        }
    }

    pub(crate) fn reset_configuration_scope(
        &self,
        key: &SkillKey,
        scope: SkillConfigScope,
    ) -> Result<Result<ResolvedSkillConfiguration, SkillConfigurationError>, SkillError> {
        let (schema, _) = self.configuration_schema(key)?;
        let configuration = self.configuration()?;
        let workspace_identity = workspace_identity(key);
        // A User record is keyed by the empty workspace identity, so clearing it has to address
        // that row rather than the one belonging to the caller's workspace.
        let record_identity = match scope {
            SkillConfigScope::User => String::new(),
            SkillConfigScope::Project => workspace_identity.clone(),
        };
        match configuration.reset_scope(&schema, key.id.as_str(), scope, &record_identity) {
            Err(error) => Ok(Err(error)),
            Ok(_) => Ok(Ok(configuration.read(
                &schema,
                key.id.as_str(),
                &workspace_identity,
            )?)),
        }
    }

    /// Deliberately does not resolve a schema: retention is decided while a Skill is being deleted,
    /// when its package may already be unreadable, and the stored rows still have to be dealt with.
    pub(crate) fn apply_configuration_retention(
        &self,
        key: &SkillKey,
        retention: DeletionRetention,
    ) -> Result<Result<SecretRecovery, SkillConfigurationError>, SkillError> {
        Ok(self.configuration()?.apply_deletion_retention(
            key.id.as_str(),
            &workspace_identity(key),
            retention,
        ))
    }

    fn reset_configured_property(
        &self,
        key: &SkillKey,
        request: &SkillConfigurationRequest,
        property_key: &str,
        expect_secret: bool,
    ) -> Result<Result<SkillConfigurationSaveResult, SkillConfigurationError>, SkillError> {
        let (schema, base_revision) = self.configuration_schema(key)?;
        if let Err(error) = require_base_revision(&base_revision, request) {
            return Ok(Err(error));
        }
        match schema.field(property_key) {
            None => {
                return Ok(Err(SkillConfigurationError::UnknownProperty {
                    key: property_key.to_string(),
                }))
            }
            Some(field) if field.secret != expect_secret => {
                return Ok(Err(SkillConfigurationError::NotConfigurable {
                    key: property_key.to_string(),
                }))
            }
            Some(_) => {}
        }
        match self
            .configuration()?
            .reset_property(&schema, request, property_key)
        {
            Err(error) => Ok(Err(error)),
            Ok(result) => Ok(Ok(self.rescope_preview(&schema, key, result)?)),
        }
    }

    fn configuration_schema(
        &self,
        key: &SkillKey,
    ) -> Result<(SkillConfigSchema, String), SkillError> {
        let context = self.service.configuration_context(key)?;
        let schema = context.schema.ok_or_else(|| {
            SkillError::Validation(format!(
                "Skill declares no supported configuration schema: {}",
                key.id.as_str()
            ))
        })?;
        let base_revision = context.overview.base_revision.ok_or_else(|| {
            SkillError::Validation(format!(
                "Skill configuration has no effective base revision: {}",
                key.id.as_str()
            ))
        })?;
        Ok((schema, base_revision))
    }

    /// A User-scope write carries no workspace identity, so the preview resolved by the write path
    /// cannot see a Project override. Re-resolving against the caller's workspace is what keeps the
    /// returned effective value the one that will actually apply there.
    fn rescope_preview(
        &self,
        schema: &SkillConfigSchema,
        key: &SkillKey,
        mut result: SkillConfigurationSaveResult,
    ) -> Result<SkillConfigurationSaveResult, SkillError> {
        let workspace_identity = workspace_identity(key);
        if workspace_identity != result.record.workspace_identity {
            result.preview =
                self.configuration()?
                    .read(schema, key.id.as_str(), &workspace_identity)?;
        }
        Ok(result)
    }
}

/// Project records are keyed by the canonical workspace path; the empty string is the User key and
/// also what a Global management scope resolves to.
fn workspace_identity(key: &SkillKey) -> String {
    key.location.workspace_path.clone().unwrap_or_default()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayMutationOperation {
    CreatePatch,
    CreateGuidance,
    AddFile,
    ReplaceFile,
    DisablePatch,
    DisableGuidance,
    DisableFile,
    RevertPatch,
    RevertGuidance,
    RevertFile,
}
