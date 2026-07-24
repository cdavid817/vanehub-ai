use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookCategory {
    Bootstrap,
    Callback,
    Dynamic,
    Law,
    Navigation,
    Routing,
    Static,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookStage {
    SessionInit,
    PerTurn,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum PromptHookSource {
    Builtin,
    User,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookGovernance {
    pub(crate) safety_tier: String,
    pub(crate) transparency_tier: String,
    pub(crate) governance_tier: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHook {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) order: i64,
    pub(crate) version: i64,
    pub(crate) source: PromptHookSource,
    pub(crate) enabled: bool,
    pub(crate) disableable: bool,
    pub(crate) cli_bindings: Vec<String>,
    pub(crate) governance: PromptHookGovernance,
    pub(crate) template_body: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookListResult {
    pub(crate) hooks: Vec<PromptHook>,
    pub(crate) stats: PromptHookStats,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookStats {
    pub(crate) total: usize,
    pub(crate) enabled: usize,
    pub(crate) builtin: usize,
    pub(crate) user: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookMutationInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) order: i64,
    pub(crate) template_body: String,
    pub(crate) enabled: bool,
    pub(crate) cli_bindings: Vec<String>,
    pub(crate) governance: PromptHookGovernance,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookUpdateInput {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) order: i64,
    pub(crate) version: i64,
    pub(crate) template_body: String,
    pub(crate) enabled: bool,
    pub(crate) cli_bindings: Vec<String>,
    pub(crate) governance: PromptHookGovernance,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookPreviewInput {
    pub(crate) hook_id: String,
    pub(crate) agent_id: String,
    pub(crate) sample_input: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptAssemblyPreviewInput {
    pub(crate) agent_id: String,
    pub(crate) sample_input: String,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookTraceSummary {
    pub(crate) id: String,
    pub(crate) hook_id: String,
    pub(crate) category: PromptHookCategory,
    pub(crate) stage: PromptHookStage,
    pub(crate) status: String,
    pub(crate) version: Option<i64>,
    pub(crate) content_hash: Option<String>,
    pub(crate) token_estimate: Option<i64>,
    pub(crate) reason: Option<String>,
    pub(crate) agent_id: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookPreview {
    pub(crate) hook_id: Option<String>,
    pub(crate) agent_id: String,
    pub(crate) rendered_content: String,
    pub(crate) trace: Vec<PromptHookTraceSummary>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SavePromptHookDraftInput {
    pub(crate) hook_id: String,
    pub(crate) expected_revision: Option<i64>,
    pub(crate) draft: PromptHookMutationInput,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PublishPromptHookInput {
    pub(crate) hook_id: String,
    pub(crate) expected_draft_revision: i64,
    pub(crate) expected_published_version: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RollbackPromptHookInput {
    pub(crate) hook_id: String,
    pub(crate) version: i64,
    pub(crate) expected_published_version: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookVariableDefinition {
    pub(crate) name: String,
    pub(crate) token: String,
    pub(crate) description_key: String,
    pub(crate) availability_key: String,
    pub(crate) example: String,
    pub(crate) aliases: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookDraft {
    pub(crate) hook_id: String,
    pub(crate) revision: i64,
    pub(crate) input: PromptHookMutationInput,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookVersion {
    pub(crate) hook_id: String,
    pub(crate) version: i64,
    pub(crate) content_hash: String,
    pub(crate) publication_kind: String,
    pub(crate) rollback_from_version: Option<i64>,
    pub(crate) published_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) template_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookEvaluationSummary {
    pub(crate) hook_id: String,
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

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptHookVersionHistory {
    pub(crate) hook_id: String,
    pub(crate) published_version: Option<i64>,
    pub(crate) draft: Option<PromptHookDraft>,
    pub(crate) versions: Vec<PromptHookVersion>,
    pub(crate) evaluations: Vec<PromptHookEvaluationSummary>,
}
