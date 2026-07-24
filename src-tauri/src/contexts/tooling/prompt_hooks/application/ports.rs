use super::{
    PromptHookApplicationError, PromptHookDraft, PromptHookEvaluationSummary,
    PromptHookExecutionObservation, PromptHookLogEvent, PromptHookOverride, PromptHookRecord,
    PromptHookTrace, PromptHookVersion,
};
use crate::contexts::tooling::prompt_hooks::domain::{PromptHookBindings, PromptHookId};

pub(crate) trait PromptHookRepository: Send + Sync {
    fn list_user_hooks(&self) -> Result<Vec<PromptHookRecord>, PromptHookApplicationError>;

    fn list_builtin_overrides(&self)
        -> Result<Vec<PromptHookOverride>, PromptHookApplicationError>;

    fn create_user_hook(&self, record: &PromptHookRecord)
        -> Result<(), PromptHookApplicationError>;

    fn create_user_draft(
        &self,
        record: &PromptHookRecord,
        draft: &PromptHookDraft,
    ) -> Result<(), PromptHookApplicationError> {
        self.create_user_hook(record)?;
        self.save_draft(draft, None)
    }

    fn delete_user_hook(&self, hook_id: &PromptHookId) -> Result<(), PromptHookApplicationError>;

    fn set_user_enabled(
        &self,
        hook_id: &PromptHookId,
        enabled: bool,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError>;

    fn set_user_bindings(
        &self,
        hook_id: &PromptHookId,
        bindings: &PromptHookBindings,
        updated_at: &str,
    ) -> Result<(), PromptHookApplicationError>;

    fn save_builtin_override(
        &self,
        override_record: &PromptHookOverride,
    ) -> Result<(), PromptHookApplicationError>;

    fn save_traces(
        &self,
        traces: &[PromptHookTrace],
        retained_limit: usize,
    ) -> Result<(), PromptHookApplicationError>;

    fn list_traces(&self, limit: usize)
        -> Result<Vec<PromptHookTrace>, PromptHookApplicationError>;

    fn get_draft(
        &self,
        hook_id: &PromptHookId,
    ) -> Result<Option<PromptHookDraft>, PromptHookApplicationError> {
        let _ = hook_id;
        Ok(None)
    }

    fn save_draft(
        &self,
        draft: &PromptHookDraft,
        expected_revision: Option<i64>,
    ) -> Result<(), PromptHookApplicationError> {
        let _ = (draft, expected_revision);
        Err(PromptHookApplicationError::Repository(
            "Prompt Hook draft persistence is unavailable.".to_string(),
        ))
    }

    fn publish_draft(
        &self,
        version: &PromptHookVersion,
        expected_draft_revision: i64,
        expected_published_version: Option<i64>,
    ) -> Result<(), PromptHookApplicationError> {
        let _ = (version, expected_draft_revision, expected_published_version);
        Err(PromptHookApplicationError::Repository(
            "Prompt Hook publication persistence is unavailable.".to_string(),
        ))
    }

    fn list_versions(
        &self,
        hook_id: &PromptHookId,
        limit: usize,
    ) -> Result<Vec<PromptHookVersion>, PromptHookApplicationError> {
        let _ = (hook_id, limit);
        Ok(Vec::new())
    }

    fn publish_rollback(
        &self,
        version: &PromptHookVersion,
        expected_published_version: Option<i64>,
    ) -> Result<(), PromptHookApplicationError> {
        let _ = (version, expected_published_version);
        Err(PromptHookApplicationError::Repository(
            "Prompt Hook rollback persistence is unavailable.".to_string(),
        ))
    }

    fn save_execution_observations(
        &self,
        observations: &[PromptHookExecutionObservation],
    ) -> Result<(), PromptHookApplicationError> {
        let _ = observations;
        Ok(())
    }

    fn evaluation_summaries(
        &self,
        hook_id: &PromptHookId,
        limit: usize,
    ) -> Result<Vec<PromptHookEvaluationSummary>, PromptHookApplicationError> {
        let _ = (hook_id, limit);
        Ok(Vec::new())
    }
}

pub(crate) trait PromptHookClockPort: Send + Sync {
    fn now(&self) -> String;
}

pub(crate) trait PromptHookTraceIdPort: Send + Sync {
    fn next_trace_id(&self) -> String;
}

pub(crate) trait PromptHookLoggingPort: Send + Sync {
    fn record(&self, event: &PromptHookLogEvent);
}
