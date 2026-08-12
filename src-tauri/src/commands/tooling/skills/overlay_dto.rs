use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayTargetInput {
    pub(crate) skill_id: String,
    pub(crate) scope: String,
    pub(crate) workspace_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayWitnessesInput {
    pub(crate) expected_overlay_revision: Option<u64>,
    pub(crate) expected_base_instruction_hash: String,
    pub(crate) expected_base_package_hash: String,
    pub(crate) expected_payload_hash: Option<String>,
    pub(crate) expected_pinned: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayPatchInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) old_string: String,
    pub(crate) new_string: String,
    pub(crate) replace_all: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayGuidanceInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) guidance: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayFileInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) content: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayMutationStateInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) mutation_id: String,
    pub(crate) mutation_kind: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayPreviewInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) mutation: OverlayMutationInput,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum OverlayMutationInput {
    ExactPatch {
        old_string: String,
        new_string: String,
        replace_all: bool,
    },
    LearnedGuidance {
        guidance: String,
    },
    SupportingFile {
        logical_path: String,
        media_type: String,
        content: Vec<u8>,
    },
    Disable {
        mutation_id: String,
    },
    Revert {
        mutation_id: String,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayImportInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) source_name: String,
    pub(crate) archive: Vec<u8>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayPromotionInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) reviewed_revision: u64,
    pub(crate) reviewed_document_hash: String,
    pub(crate) reviewed_scan: OverlayScanResult,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayHistoryInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayConflictResolutionInput {
    pub(crate) conflict_id: String,
    pub(crate) resolution: String,
    pub(crate) old_string: Option<String>,
    pub(crate) new_string: Option<String>,
    pub(crate) replace_all: Option<bool>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayReconciliationInput {
    pub(crate) target: OverlayTargetInput,
    pub(crate) witnesses: OverlayWitnessesInput,
    pub(crate) choices: Vec<OverlayConflictResolutionInput>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayBoundedText {
    pub(crate) content: String,
    pub(crate) total_characters: usize,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayScopeSummary {
    pub(crate) scope: String,
    pub(crate) revision: u64,
    pub(crate) trust: String,
    pub(crate) status: String,
    pub(crate) active_mutation_count: usize,
    pub(crate) conflict_count: usize,
    pub(crate) base_hash_changed: bool,
    pub(crate) needs_reconcile: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlaySummary {
    pub(crate) canonical_skill_id: String,
    pub(crate) base_layer: String,
    pub(crate) status: String,
    pub(crate) needs_reconcile: bool,
    pub(crate) pinned: bool,
    pub(crate) base_instruction_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) last_healthy_scope: Option<String>,
    pub(crate) scopes: Vec<OverlayScopeSummary>,
    pub(crate) scopes_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayDiffHunk {
    pub(crate) label: String,
    pub(crate) before: OverlayBoundedText,
    pub(crate) after: OverlayBoundedText,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayDiff {
    pub(crate) base_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) added_characters: usize,
    pub(crate) removed_characters: usize,
    pub(crate) hunks: Vec<OverlayDiffHunk>,
    pub(crate) hunks_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayScopeDiff {
    pub(crate) scope: String,
    pub(crate) revision: u64,
    pub(crate) input_hash: String,
    pub(crate) output_hash: String,
    pub(crate) diff: OverlayDiff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayMutationSummary {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) scope: String,
    pub(crate) state: String,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayConflictSummary {
    pub(crate) id: String,
    pub(crate) mutation_id: String,
    pub(crate) safe_reason: String,
    pub(crate) state: String,
    pub(crate) resolution_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayResourceShadow {
    pub(crate) scope: Option<String>,
    pub(crate) base_layer: Option<String>,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayResourceSummary {
    pub(crate) mutation_id: String,
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
    pub(crate) effective_scope: String,
    pub(crate) state: String,
    pub(crate) shadowed: Vec<OverlayResourceShadow>,
    pub(crate) shadowed_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayDetail {
    pub(crate) summary: OverlaySummary,
    pub(crate) base_instructions: OverlayBoundedText,
    pub(crate) effective_instructions: OverlayBoundedText,
    pub(crate) diff: OverlayDiff,
    pub(crate) scope_diffs: Vec<OverlayScopeDiff>,
    pub(crate) scope_diffs_truncated: bool,
    pub(crate) mutations: Vec<OverlayMutationSummary>,
    pub(crate) mutations_truncated: bool,
    pub(crate) resources: Vec<OverlayResourceSummary>,
    pub(crate) resources_truncated: bool,
    pub(crate) conflicts: Vec<OverlayConflictSummary>,
    pub(crate) conflicts_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OverlayScanResult {
    pub(crate) scanner_version: String,
    pub(crate) passed: bool,
    pub(crate) safe_rule_ids: Vec<String>,
    pub(crate) rule_ids_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayPreview {
    pub(crate) witnesses: OverlayWitnessesInputOutput,
    pub(crate) tentative_revision: u64,
    pub(crate) scan: OverlayScanResult,
    pub(crate) diff: OverlayDiff,
    pub(crate) conflicts: Vec<OverlayConflictSummary>,
    pub(crate) conflicts_truncated: bool,
    pub(crate) can_commit: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayWitnessesInputOutput {
    pub(crate) expected_overlay_revision: Option<u64>,
    pub(crate) expected_base_instruction_hash: String,
    pub(crate) expected_base_package_hash: String,
    pub(crate) expected_payload_hash: Option<String>,
    pub(crate) expected_pinned: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayMutationOutcome {
    pub(crate) summary: OverlaySummary,
    pub(crate) committed_revision: u64,
    pub(crate) diff: OverlayDiff,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayImportReview {
    pub(crate) source_summary: String,
    pub(crate) revision: u64,
    pub(crate) document_hash: String,
    pub(crate) scan: OverlayScanResult,
    pub(crate) diff: OverlayDiff,
    pub(crate) mutations: Vec<OverlayMutationSummary>,
    pub(crate) mutations_truncated: bool,
    pub(crate) resources: Vec<OverlayResourceSummary>,
    pub(crate) resources_truncated: bool,
    pub(crate) conflicts: Vec<OverlayConflictSummary>,
    pub(crate) conflicts_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayHistoryEntry {
    pub(crate) event_id: String,
    pub(crate) canonical_skill_id: String,
    pub(crate) scope: String,
    pub(crate) prior_revision: Option<u64>,
    pub(crate) next_revision: u64,
    pub(crate) actor: String,
    pub(crate) action: String,
    pub(crate) timestamp: String,
    pub(crate) prior_document_hash: Option<String>,
    pub(crate) next_document_hash: String,
    pub(crate) scanner_version: String,
    pub(crate) safe_outcome: String,
    pub(crate) prior_event_hash: Option<String>,
    pub(crate) event_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayHistoryPage {
    pub(crate) entries: Vec<OverlayHistoryEntry>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) integrity: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayReconciliationBaseSnapshot {
    pub(crate) base_identity: String,
    pub(crate) base_layer: String,
    pub(crate) instruction_hash: String,
    pub(crate) package_hash: String,
    pub(crate) instructions: Option<OverlayBoundedText>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayReconciliationProposedResult {
    pub(crate) effective_hash: String,
    pub(crate) instructions: OverlayBoundedText,
    pub(crate) resources: Vec<OverlayResourceSummary>,
    pub(crate) resources_truncated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayReconciliationConflictChoice {
    pub(crate) conflict: OverlayConflictSummary,
    pub(crate) selected_resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OverlayReconciliationPreview {
    pub(crate) witnesses: OverlayWitnessesInputOutput,
    pub(crate) witnessed_base: OverlayReconciliationBaseSnapshot,
    pub(crate) current_base: OverlayReconciliationBaseSnapshot,
    pub(crate) proposed_effective: OverlayReconciliationProposedResult,
    pub(crate) conflict_choices: Vec<OverlayReconciliationConflictChoice>,
    pub(crate) conflicts_truncated: bool,
    pub(crate) final_diff: OverlayDiff,
    pub(crate) final_diff_complete: bool,
    pub(crate) can_commit: bool,
}
