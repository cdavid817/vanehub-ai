use super::{CuratorCandidateState, CuratorDraftMutationInput, CuratorStalenessReason};
use serde::{Deserialize, Serialize};

pub(crate) const CURATOR_PREVIEW_TTL_MS: i64 = 15 * 60 * 1_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDiffText {
    pub(crate) content: String,
    pub(crate) total_characters: usize,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDiffHunk {
    pub(crate) label: String,
    pub(crate) before: CuratorDiffText,
    pub(crate) after: CuratorDiffText,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDiffProjection {
    pub(crate) from_hash: String,
    pub(crate) to_hash: String,
    pub(crate) added_characters: usize,
    pub(crate) removed_characters: usize,
    pub(crate) hunks: Vec<CuratorDiffHunk>,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPreviewDiffs {
    pub(crate) base_to_current: CuratorDiffProjection,
    pub(crate) current_to_proposed: CuratorDiffProjection,
    pub(crate) base_to_proposed: CuratorDiffProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPreviewWitnesses {
    pub(crate) candidate_hash: String,
    pub(crate) draft_hash: String,
    pub(crate) assessment_hash: String,
    pub(crate) target_revision: String,
    pub(crate) base_instruction_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) current_effective_hash: String,
    pub(crate) proposed_effective_hash: String,
    pub(crate) overlay_revision: Option<u64>,
    pub(crate) pin_witness: String,
    pub(crate) trust_witness: String,
    pub(crate) conflict_witness: String,
    pub(crate) scanner_version: String,
    pub(crate) policy_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPreviewValidation {
    pub(crate) scan_passed: bool,
    pub(crate) can_commit: bool,
    pub(crate) pinned: bool,
    pub(crate) trusted: bool,
    pub(crate) conflict_count: usize,
    pub(crate) conflicts_complete: bool,
    pub(crate) safe_rule_ids: Vec<String>,
    pub(crate) rules_complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorPreview {
    pub(crate) preview_id: String,
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) draft_id: String,
    pub(crate) draft_revision: u64,
    pub(crate) assessment_id: String,
    pub(crate) witness_hash: String,
    pub(crate) effective_diff_hash: String,
    pub(crate) witnesses: CuratorPreviewWitnesses,
    pub(crate) diffs: CuratorPreviewDiffs,
    pub(crate) validation: CuratorPreviewValidation,
    pub(crate) issued_at_ms: i64,
    pub(crate) expires_at_ms: i64,
    pub(crate) invalidated_at_ms: Option<i64>,
}

impl CuratorPreview {
    pub(crate) fn is_current(&self, now_ms: i64) -> bool {
        self.invalidated_at_ms.is_none()
            && now_ms >= self.issued_at_ms
            && now_ms < self.expires_at_ms
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorPreviewBinding {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) candidate_hash: String,
    pub(crate) policy_hash: String,
    pub(crate) state: CuratorCandidateState,
    pub(crate) workspace_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) overlay_scope: String,
    pub(crate) draft_id: String,
    pub(crate) draft_revision: u64,
    pub(crate) draft_hash: String,
    pub(crate) mutation: CuratorDraftMutationInput,
    pub(crate) base_instruction_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) current_effective_hash: String,
    pub(crate) overlay_revision: Option<u64>,
    pub(crate) pin_witness: String,
    pub(crate) trust_witness: String,
    pub(crate) conflict_witness: String,
    pub(crate) assessment_id: String,
    pub(crate) assessment_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorOverlayPreviewReceipt {
    pub(crate) witnesses: CuratorPreviewWitnesses,
    pub(crate) diffs: CuratorPreviewDiffs,
    pub(crate) validation: CuratorPreviewValidation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CuratorPreviewRequest<'a> {
    pub(crate) candidate_id: &'a str,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) expected_draft_revision: u64,
    pub(crate) expected_assessment_id: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDiffPage {
    pub(crate) hunks: Vec<CuratorDiffHunk>,
    pub(crate) next_cursor: Option<usize>,
    pub(crate) complete: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorPreviewInvalidation {
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) reason: CuratorStalenessReason,
    pub(crate) occurred_at_ms: i64,
}
