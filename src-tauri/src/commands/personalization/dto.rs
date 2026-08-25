//! The wire shapes the personalization screens speak.
//!
//! Deliberately not the domain types. A DTO is a contract with a UI that ships separately, so it
//! carries strings where the domain carries closed enums and flat fields where the domain nests:
//! a frontend that had to reconstruct a `MemoryScope` would be reimplementing a rule that lives
//! here, and would drift the first time the rule changed.
//!
//! What no shape below carries: a memory body in a list, an absolute path, a legacy folder, a
//! remote URI, a core prompt, or any raw persistence error. Those are absent by construction
//! rather than filtered, which is what makes their absence checkable.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalizationPolicyView {
    /// `global`, `agent`, `workspace`, or `workspace-agent`.
    pub(crate) scope_kind: String,
    /// The scope's own key: the Agent id, the workspace key, or both joined. Empty for global.
    pub(crate) scope_key: String,
    pub(crate) revision: u64,
    pub(crate) instruction_merge_mode: String,
    pub(crate) about_user: String,
    pub(crate) style_rules: String,
    pub(crate) memory_read_mode: String,
    pub(crate) explicit_save_mode: String,
    pub(crate) automatic_extraction_mode: String,
    pub(crate) global_memory_access_mode: String,
}

/// One layer's edit. Every field is optional because a screen posts what the user touched: a page
/// that sent all of them would republish the ones they did not, which is how one screen's stale
/// copy silently reverts another's.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalizationPolicyPatchInput {
    pub(crate) scope_kind: String,
    #[serde(default)]
    pub(crate) agent_id: Option<String>,
    #[serde(default)]
    pub(crate) workspace_key: Option<String>,
    /// Absent creates the layer; present requires it to still be at that revision.
    #[serde(default)]
    pub(crate) expected_revision: Option<u64>,
    #[serde(default)]
    pub(crate) instruction_merge_mode: Option<String>,
    #[serde(default)]
    pub(crate) about_user: Option<String>,
    #[serde(default)]
    pub(crate) style_rules: Option<String>,
    #[serde(default)]
    pub(crate) memory_read_mode: Option<String>,
    #[serde(default)]
    pub(crate) explicit_save_mode: Option<String>,
    #[serde(default)]
    pub(crate) automatic_extraction_mode: Option<String>,
    #[serde(default)]
    pub(crate) global_memory_access_mode: Option<String>,
}

/// What one Agent can actually consume, so a screen renders controls from the Agent rather than
/// from a list of Agent ids it carries itself.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentCapabilityView {
    pub(crate) agent_id: String,
    pub(crate) display_name: String,
    pub(crate) supports_custom_instructions: bool,
    pub(crate) supports_memory_index: bool,
    pub(crate) supports_selected_memory_bodies: bool,
    pub(crate) supports_automatic_extraction: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectivePreviewInput {
    pub(crate) agent_id: String,
    pub(crate) session_id: String,
    #[serde(default)]
    pub(crate) workspace_key: Option<String>,
    #[serde(default)]
    pub(crate) workspace_display_path: Option<String>,
    /// `standard`, `project-only`, or `temporary`.
    #[serde(default)]
    pub(crate) session_mode: Option<String>,
}

/// One instruction field as it would be applied, already redacted.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreviewSegmentView {
    pub(crate) field: String,
    pub(crate) scope_kind: String,
    pub(crate) scope_key: String,
    pub(crate) policy_revision: u64,
    pub(crate) merge_action: String,
    /// Redacted through the same rule the logs use. A settings screen is screenshotted and pasted
    /// into issues, and a token a user pasted into their own instructions must not be handed back.
    pub(crate) redacted_text: String,
    /// The length of the real text, not of the redacted rendering: a user sizing their
    /// instructions needs the real number.
    pub(crate) characters: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ExcludedSegmentView {
    pub(crate) field: String,
    pub(crate) scope_kind: String,
    pub(crate) scope_key: String,
    pub(crate) reason: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectivePreviewView {
    pub(crate) revision_token: String,
    pub(crate) instruction_mode: String,
    pub(crate) included_instructions: Vec<PreviewSegmentView>,
    pub(crate) excluded_instructions: Vec<ExcludedSegmentView>,
    pub(crate) memory_delivery: String,
    pub(crate) memory_read: bool,
    pub(crate) explicit_save: bool,
    pub(crate) automatic_extraction: bool,
    pub(crate) candidate_creation: bool,
    pub(crate) retrieval_write: bool,
    pub(crate) eligible_memory_count: usize,
    pub(crate) considered_memory_count: usize,
    pub(crate) memory_exclusions: Vec<MemoryExclusionView>,
    pub(crate) warnings: Vec<String>,
    pub(crate) approximate_tokens: usize,
    pub(crate) known_characters: usize,
    pub(crate) selected_body_budget_max: usize,
    pub(crate) excluded_surfaces: Vec<String>,
    pub(crate) estimator_version: String,
    /// Always false, and reported rather than assumed: VaneHub does not manage a CLI's internal
    /// context, and a screen that stayed silent about it would leave a user thinking the estimate
    /// covers their whole session.
    pub(crate) cli_internal_compaction_managed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryExclusionView {
    pub(crate) reason: String,
    pub(crate) count: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryQueryInput {
    #[serde(default)]
    pub(crate) text: Option<String>,
    #[serde(default)]
    pub(crate) scope_kind: Option<String>,
    #[serde(default)]
    pub(crate) workspace_key: Option<String>,
    #[serde(default)]
    pub(crate) memory_type: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) source_agent_id: Option<String>,
    #[serde(default)]
    pub(crate) cursor: Option<String>,
    #[serde(default)]
    pub(crate) limit: Option<usize>,
}

/// A list entry. No body: a page that carried every body would read the whole store to render a
/// list of names, and the detail call exists for the one the user opens.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemorySummaryView {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: String,
    pub(crate) scope_kind: String,
    pub(crate) workspace_key: Option<String>,
    pub(crate) status: String,
    pub(crate) source: String,
    pub(crate) sensitivity: String,
    pub(crate) revision: u64,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryPageView {
    pub(crate) items: Vec<MemorySummaryView>,
    pub(crate) next_cursor: Option<String>,
    /// Present only when the store can produce it cheaply; a screen must render without it.
    pub(crate) total_matched: Option<usize>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryDetailView {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: String,
    pub(crate) content: String,
    pub(crate) scope_kind: String,
    pub(crate) workspace_key: Option<String>,
    pub(crate) audience_agent_ids: Option<Vec<String>>,
    pub(crate) status: String,
    pub(crate) source: String,
    pub(crate) sensitivity: String,
    pub(crate) revision: u64,
    pub(crate) source_agent_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateMemoryCommandInput {
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) memory_type: String,
    pub(crate) content: String,
    pub(crate) scope_kind: String,
    #[serde(default)]
    pub(crate) workspace_key: Option<String>,
    #[serde(default)]
    pub(crate) audience_agent_ids: Option<Vec<String>>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateMemoryCommandInput {
    pub(crate) id: String,
    pub(crate) expected_revision: u64,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) memory_type: Option<String>,
    #[serde(default)]
    pub(crate) content: Option<String>,
    #[serde(default)]
    pub(crate) status: Option<String>,
    #[serde(default)]
    pub(crate) sensitivity: Option<String>,
}

/// One proposal awaiting a decision.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MemoryCandidateView {
    pub(crate) id: String,
    /// `create`, `update`, or `archive`.
    pub(crate) kind: String,
    pub(crate) name: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) memory_type: Option<String>,
    pub(crate) content: Option<String>,
    pub(crate) target_id: Option<String>,
    pub(crate) expected_target_revision: Option<u64>,
    pub(crate) source: String,
    pub(crate) source_agent_id: Option<String>,
    pub(crate) source_session_id: Option<String>,
    pub(crate) source_message_id: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewCandidateInput {
    pub(crate) candidate_id: String,
    /// `approve`, `approve-with-edits`, `reject`, `mark-sensitive-and-archive`, or `merge-into`.
    pub(crate) action: String,
    #[serde(default)]
    pub(crate) name: Option<String>,
    #[serde(default)]
    pub(crate) description: Option<String>,
    #[serde(default)]
    pub(crate) content: Option<String>,
    #[serde(default)]
    pub(crate) memory_type: Option<String>,
    #[serde(default)]
    pub(crate) merge_target_id: Option<String>,
    #[serde(default)]
    pub(crate) merge_expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOutcomeView {
    pub(crate) candidate_id: String,
    pub(crate) status: String,
    pub(crate) resulting_memory_id: Option<String>,
    pub(crate) retained_content: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResetScopeInput {
    #[serde(default)]
    pub(crate) scope_kind: Option<String>,
    #[serde(default)]
    pub(crate) workspace_key: Option<String>,
    #[serde(default)]
    pub(crate) include_archived: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResetPreviewView {
    /// Quoted back on execute. It names the scope and statuses the preview counted, so a screen
    /// cannot preview one scope and delete another; it travels with the counts rather than beside
    /// them because holding one without the other is never useful.
    pub(crate) confirmation_token: String,
    pub(crate) matched: usize,
    pub(crate) global: usize,
    pub(crate) workspace: usize,
    pub(crate) candidates: usize,
    /// Files the store could not parse. Counted because a reset removes them too, and a preview
    /// that omitted them would understate what the user is about to lose.
    pub(crate) malformed: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MaintenanceResultView {
    pub(crate) matched: usize,
    pub(crate) deleted_files: usize,
    pub(crate) removed_projection_rows: usize,
    pub(crate) revoked_retrieval_entries: usize,
    pub(crate) quarantined: usize,
    /// Per-surface failures as stable codes. A partial result must say so: a caller told a reset
    /// succeeded when a projection row survived would leave a memory recallable that the user
    /// believes is gone.
    pub(crate) failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PersonalizationHealthView {
    /// `not_started`, `busy`, `migrating`, `rebuilding_derived`, `ready`, `repair_required`, or
    /// `failed`.
    pub(crate) state: String,
    pub(crate) memory_available: bool,
    pub(crate) pending_candidates: usize,
}
