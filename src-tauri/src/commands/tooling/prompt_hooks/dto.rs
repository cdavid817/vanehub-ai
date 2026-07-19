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

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
