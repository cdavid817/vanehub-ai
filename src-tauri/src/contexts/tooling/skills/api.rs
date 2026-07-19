use crate::contexts::tooling::skills::application::SkillApplicationService;

pub(crate) use crate::contexts::tooling::skills::application::{
    SkillAgentMountPath, SkillApplicationError as SkillError, SkillBackupEntry, SkillCreateRequest,
    SkillDriftReport, SkillFailure, SkillImportRequest, SkillListResult, SkillMountMigrationReport,
    SkillPreview, SkillRecord, SkillScopeQuery, SkillSyncResult, SkillUpdateRequest,
};
pub(crate) use crate::contexts::tooling::skills::domain::{
    SkillDomainError, SkillDriftIssueType, SkillId, SkillKey, SkillLocation, SkillMetadata,
    SkillMountPath, SkillScope, SkillSource,
};

#[derive(Clone)]
pub(crate) struct SkillApi {
    service: SkillApplicationService,
}

impl SkillApi {
    pub(crate) fn new(service: SkillApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list(&self, query: SkillScopeQuery) -> Result<SkillListResult, SkillError> {
        self.service.list_skills(query)
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

    pub(crate) fn preview(&self, key: SkillKey) -> Result<SkillPreview, SkillError> {
        self.service.preview_skill(key)
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
