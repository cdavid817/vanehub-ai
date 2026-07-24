use crate::contexts::tooling::prompt_hooks::domain::{
    ManagedCliAgentId, PromptHookBindings, PromptHookCategory, PromptHookId, PromptHookManifest,
    PromptHookSource, PromptHookStage,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookGovernance {
    pub(crate) safety_tier: String,
    pub(crate) transparency_tier: String,
    pub(crate) governance_tier: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookRecord {
    pub(crate) manifest: PromptHookManifest,
    pub(crate) description: String,
    pub(crate) version: i64,
    pub(crate) source: PromptHookSource,
    pub(crate) enabled: bool,
    pub(crate) disableable: bool,
    pub(crate) governance: PromptHookGovernance,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

impl PromptHookRecord {
    pub(crate) fn id(&self) -> &PromptHookId {
        self.manifest.id()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookOverride {
    pub(crate) hook_id: PromptHookId,
    pub(crate) enabled: bool,
    pub(crate) bindings: PromptHookBindings,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct PromptHookStats {
    pub(crate) total: usize,
    pub(crate) enabled: usize,
    pub(crate) builtin: usize,
    pub(crate) user: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookListResult {
    pub(crate) hooks: Vec<PromptHookRecord>,
    pub(crate) stats: PromptHookStats,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookCreateRequest {
    pub(crate) manifest: PromptHookManifest,
    pub(crate) description: String,
    pub(crate) enabled: bool,
    pub(crate) governance: PromptHookGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookUpdateRequest {
    pub(crate) hook_id: PromptHookId,
    pub(crate) manifest: PromptHookManifest,
    pub(crate) description: String,
    pub(crate) version: i64,
    pub(crate) enabled: bool,
    pub(crate) governance: PromptHookGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookSnapshot {
    pub(crate) manifest: PromptHookManifest,
    pub(crate) description: String,
    pub(crate) enabled: bool,
    pub(crate) governance: PromptHookGovernance,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookDraft {
    pub(crate) hook_id: PromptHookId,
    pub(crate) revision: i64,
    pub(crate) snapshot: PromptHookSnapshot,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptHookPublicationKind {
    Publish,
    Rollback,
}

impl PromptHookPublicationKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Publish => "publish",
            Self::Rollback => "rollback",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "publish" => Some(Self::Publish),
            "rollback" => Some(Self::Rollback),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookVersion {
    pub(crate) hook_id: PromptHookId,
    pub(crate) version: i64,
    pub(crate) snapshot: PromptHookSnapshot,
    pub(crate) content_hash: String,
    pub(crate) publication_kind: PromptHookPublicationKind,
    pub(crate) rollback_from_version: Option<i64>,
    pub(crate) published_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptHookExecutionOutcome {
    Succeeded,
    Failed,
    Cancelled,
}

impl PromptHookExecutionOutcome {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookExecutionObservation {
    pub(crate) invocation_id: String,
    pub(crate) hook_id: PromptHookId,
    pub(crate) version: i64,
    pub(crate) outcome: PromptHookExecutionOutcome,
    pub(crate) elapsed_ms: i64,
    pub(crate) agent_id: ManagedCliAgentId,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PromptHookEvaluationSummary {
    pub(crate) hook_id: PromptHookId,
    pub(crate) version: i64,
    pub(crate) execution_count: i64,
    pub(crate) succeeded_count: i64,
    pub(crate) failed_count: i64,
    pub(crate) cancelled_count: i64,
    pub(crate) success_rate: Option<f64>,
    pub(crate) average_elapsed_ms: Option<f64>,
    pub(crate) minimum_elapsed_ms: Option<i64>,
    pub(crate) maximum_elapsed_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PromptHookVersionHistory {
    pub(crate) hook_id: PromptHookId,
    pub(crate) published_version: Option<i64>,
    pub(crate) draft: Option<PromptHookDraft>,
    pub(crate) versions: Vec<PromptHookVersion>,
    pub(crate) evaluations: Vec<PromptHookEvaluationSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SavePromptHookDraftRequest {
    pub(crate) hook_id: PromptHookId,
    pub(crate) expected_revision: Option<i64>,
    pub(crate) snapshot: PromptHookSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublishPromptHookRequest {
    pub(crate) hook_id: PromptHookId,
    pub(crate) expected_draft_revision: i64,
    pub(crate) expected_published_version: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RollbackPromptHookRequest {
    pub(crate) hook_id: PromptHookId,
    pub(crate) version: i64,
    pub(crate) expected_published_version: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookVariable {
    pub(crate) name: String,
    pub(crate) token: String,
    pub(crate) description_key: String,
    pub(crate) availability_key: String,
    pub(crate) example: String,
    pub(crate) aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookPreviewRequest {
    pub(crate) hook_id: PromptHookId,
    pub(crate) agent_id: ManagedCliAgentId,
    pub(crate) sample_input: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePromptRequest {
    pub(crate) agent_id: ManagedCliAgentId,
    pub(crate) session_id: Option<String>,
    pub(crate) user_prompt: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptHookTraceStatus {
    Fired,
    Disabled,
    Skipped,
}

impl PromptHookTraceStatus {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Fired => "fired",
            Self::Disabled => "disabled",
            Self::Skipped => "skipped",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "fired" => Some(Self::Fired),
            "disabled" => Some(Self::Disabled),
            "skipped" => Some(Self::Skipped),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookTrace {
    pub(crate) id: String,
    pub(crate) hook_id: PromptHookId,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) status: PromptHookTraceStatus,
    pub(crate) version: Option<i64>,
    pub(crate) content_hash: Option<String>,
    pub(crate) token_estimate: Option<i64>,
    pub(crate) reason: Option<String>,
    pub(crate) agent_id: Option<ManagedCliAgentId>,
    pub(crate) session_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookPreview {
    pub(crate) hook_id: PromptHookId,
    pub(crate) agent_id: ManagedCliAgentId,
    pub(crate) rendered_content: String,
    pub(crate) trace: Vec<PromptHookTrace>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptAssemblyResult {
    pub(crate) effective_prompt: String,
    pub(crate) trace: Vec<PromptHookTrace>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptHookLogAction {
    Create,
    Update,
    Delete,
    SetEnabled,
    SetBindings,
    Preview,
    Assemble,
}

impl PromptHookLogAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Update => "update",
            Self::Delete => "delete",
            Self::SetEnabled => "set-enabled",
            Self::SetBindings => "set-bindings",
            Self::Preview => "preview",
            Self::Assemble => "assemble",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PromptHookLogLevel {
    Error,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PromptHookLogEvent {
    pub(crate) action: PromptHookLogAction,
    pub(crate) level: PromptHookLogLevel,
    pub(crate) hook_id: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) message: String,
}
