use crate::contexts::tooling::prompt_hooks::application::PromptHookApplicationService;

pub(crate) use crate::contexts::tooling::prompt_hooks::application::{
    EffectivePromptRequest, PromptAssemblyResult, PromptHookApplicationError as PromptHookError,
    PromptHookCreateRequest, PromptHookDraft, PromptHookExecutionObservation,
    PromptHookExecutionOutcome, PromptHookGovernance, PromptHookListResult, PromptHookPreview,
    PromptHookPreviewRequest, PromptHookRecord, PromptHookSnapshot, PromptHookTrace,
    PromptHookUpdateRequest, PromptHookVariable, PromptHookVersion, PromptHookVersionHistory,
    PublishPromptHookRequest, RollbackPromptHookRequest, SavePromptHookDraftRequest,
};
pub(crate) use crate::contexts::tooling::prompt_hooks::domain::{
    ManagedCliAgentId, PromptHookBindings, PromptHookCategory, PromptHookId, PromptHookManifest,
    PromptHookSource, PromptHookStage,
};

#[derive(Clone)]
pub(crate) struct PromptHookApi {
    service: PromptHookApplicationService,
}

impl PromptHookApi {
    pub(crate) fn new(service: PromptHookApplicationService) -> Self {
        Self { service }
    }

    pub(crate) fn list(&self) -> Result<PromptHookListResult, PromptHookError> {
        self.service.list_hooks()
    }

    pub(crate) fn create(
        &self,
        request: PromptHookCreateRequest,
    ) -> Result<PromptHookRecord, PromptHookError> {
        self.service.create_hook(request)
    }

    pub(crate) fn update(
        &self,
        request: PromptHookUpdateRequest,
    ) -> Result<PromptHookRecord, PromptHookError> {
        self.service.update_hook(request)
    }

    pub(crate) fn delete(&self, hook_id: PromptHookId) -> Result<(), PromptHookError> {
        self.service.delete_hook(hook_id)
    }

    pub(crate) fn set_enabled(
        &self,
        hook_id: PromptHookId,
        enabled: bool,
    ) -> Result<PromptHookRecord, PromptHookError> {
        self.service.set_enabled(hook_id, enabled)
    }

    pub(crate) fn set_bindings(
        &self,
        hook_id: PromptHookId,
        bindings: PromptHookBindings,
    ) -> Result<PromptHookRecord, PromptHookError> {
        self.service.set_bindings(hook_id, bindings)
    }

    pub(crate) fn preview(
        &self,
        request: PromptHookPreviewRequest,
    ) -> Result<PromptHookPreview, PromptHookError> {
        self.service.preview_hook(request)
    }

    pub(crate) fn effective_prompt(
        &self,
        agent_id: &str,
        session_id: Option<&str>,
        user_prompt: &str,
    ) -> Result<PromptAssemblyResult, PromptHookError> {
        let agent_id = ManagedCliAgentId::parse(agent_id).map_err(PromptHookError::from)?;
        self.service.assemble_prompt(EffectivePromptRequest {
            agent_id,
            session_id: session_id.map(str::to_string),
            user_prompt: user_prompt.to_string(),
        })
    }

    pub(crate) fn list_traces(&self, limit: i64) -> Result<Vec<PromptHookTrace>, PromptHookError> {
        self.service.list_traces(limit)
    }

    pub(crate) fn list_variables(&self) -> Vec<PromptHookVariable> {
        self.service.list_variables()
    }

    pub(crate) fn save_draft(
        &self,
        request: SavePromptHookDraftRequest,
    ) -> Result<PromptHookDraft, PromptHookError> {
        self.service.save_draft(request)
    }

    pub(crate) fn publish(
        &self,
        request: PublishPromptHookRequest,
    ) -> Result<PromptHookVersion, PromptHookError> {
        self.service.publish(request)
    }

    pub(crate) fn version_history(
        &self,
        hook_id: PromptHookId,
    ) -> Result<PromptHookVersionHistory, PromptHookError> {
        self.service.version_history(hook_id)
    }

    pub(crate) fn rollback(
        &self,
        request: RollbackPromptHookRequest,
    ) -> Result<PromptHookVersion, PromptHookError> {
        self.service.rollback(request)
    }

    pub(crate) fn record_execution_observations(
        &self,
        observations: &[PromptHookExecutionObservation],
    ) -> Result<(), PromptHookError> {
        self.service.record_execution_observations(observations)
    }
}
