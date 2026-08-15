use crate::contexts::tooling::skills::application::{
    SkillApplicationService, SkillOverlayApplicationService,
};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) use crate::contexts::tooling::skills::application::{
    OverlayApplicationError as OverlayError, OverlayDetail, OverlayHistoryPage,
    OverlayHistoryQuery, OverlayImportRequest, OverlayImportReview, OverlayKey,
    OverlayMutationOutcome, OverlayMutationRequest, OverlayPreview, OverlayPromotionRequest,
    OverlayReconciliationPreview, OverlayReconciliationRequest, OverlaySummary, SkillAccessRefusal,
    SkillAgentKind, SkillAgentMountPath, SkillApplicationError as SkillError, SkillBackupEntry,
    SkillCreateRequest, SkillDelegationSummary, SkillDiscoveryRequest, SkillDiscoveryResult,
    SkillDriftReport, SkillFailure, SkillImportRequest, SkillListResult, SkillLoadOutcome,
    SkillLoadRequest, SkillMountMigrationReport, SkillOverview, SkillPreview, SkillPromptForAgent,
    SkillRecord, SkillResourceEntry, SkillResourceIndex, SkillResourceReadOutcome,
    SkillResourceReadRequest, SkillScopeQuery, SkillShadowSummary, SkillSyncResult,
    SkillUpdateRequest, UtilitySkillResolutionOutcome,
};
pub(crate) use crate::contexts::tooling::skills::domain::{
    SkillAvailability, SkillDelegationCapabilityId, SkillDelivery, SkillDomainError,
    SkillDriftIssueType, SkillId, SkillKey, SkillLayer, SkillLocation, SkillMetadata,
    SkillMountPath, SkillOrigin, SkillScope, SkillSource, SkillTrust, SkillType,
};

#[derive(Clone)]
pub(crate) struct SkillApi {
    service: SkillApplicationService,
    overlays: Option<SkillOverlayApplicationService>,
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
        }
    }

    pub(crate) fn with_overlay_service(mut self, overlays: SkillOverlayApplicationService) -> Self {
        self.overlays = Some(overlays);
        self
    }

    fn overlays(&self) -> Result<&SkillOverlayApplicationService, SkillError> {
        self.overlays
            .as_ref()
            .ok_or_else(|| SkillError::Repository("Overlay service is unavailable".to_string()))
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
