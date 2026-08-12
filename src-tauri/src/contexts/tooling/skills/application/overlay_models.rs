#![allow(dead_code)]

use crate::contexts::tooling::skills::domain::{
    OverlayConflictState, OverlayMutationState, OverlayScope, OverlayTrustState, SkillId,
    SkillLayer,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayStatus {
    None,
    Healthy,
    Untrusted,
    NeedsReconciliation,
    Blocked,
    IntegrityFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayScopeStatus {
    Applied,
    Untrusted,
    NeedsReconciliation,
    BlockedByEarlierScope,
    IntegrityFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayMutationKind {
    Patch,
    LearnedGuidance,
    SupportingFile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayLimitKind {
    InstructionCharacters,
    MutationCount,
    PathCharacters,
    PathDepth,
    ArchiveEntries,
    SupportingFileBytes,
    ImportBytes,
    ExpandedImportBytes,
    HistorySegmentBytes,
    PageEntries,
}

impl OverlayLimitKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InstructionCharacters => "instruction-characters",
            Self::MutationCount => "mutation-count",
            Self::PathCharacters => "path-characters",
            Self::PathDepth => "path-depth",
            Self::ArchiveEntries => "archive-entries",
            Self::SupportingFileBytes => "supporting-file-bytes",
            Self::ImportBytes => "import-bytes",
            Self::ExpandedImportBytes => "expanded-import-bytes",
            Self::HistorySegmentBytes => "history-segment-bytes",
            Self::PageEntries => "page-entries",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayIntegrityCode {
    DocumentHashMismatch,
    PayloadMissing,
    PayloadHashMismatch,
    HistoryEventChainBroken,
    HistorySegmentMissing,
    HistorySegmentTruncated,
    UnsupportedSchemaVersion,
}

impl OverlayIntegrityCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::DocumentHashMismatch => "document-hash-mismatch",
            Self::PayloadMissing => "payload-missing",
            Self::PayloadHashMismatch => "payload-hash-mismatch",
            Self::HistoryEventChainBroken => "history-event-chain-broken",
            Self::HistorySegmentMissing => "history-segment-missing",
            Self::HistorySegmentTruncated => "history-segment-truncated",
            Self::UnsupportedSchemaVersion => "unsupported-schema-version",
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayBoundedText {
    pub(crate) content: String,
    pub(crate) total_characters: usize,
    pub(crate) truncated: bool,
}

impl OverlayBoundedText {
    pub(crate) fn from_text(value: &str, maximum_characters: usize) -> Self {
        let total_characters = value.chars().count();
        let content = value.chars().take(maximum_characters).collect();
        Self {
            content,
            total_characters,
            truncated: total_characters > maximum_characters,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayWitnesses {
    pub(crate) expected_overlay_revision: Option<u64>,
    pub(crate) expected_base_instruction_hash: String,
    pub(crate) expected_base_package_hash: String,
    pub(crate) expected_payload_hash: Option<String>,
    pub(crate) expected_pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayScopeSummary {
    pub(crate) scope: OverlayScope,
    pub(crate) revision: u64,
    pub(crate) trust: OverlayTrustState,
    pub(crate) status: OverlayScopeStatus,
    pub(crate) active_mutation_count: usize,
    pub(crate) conflict_count: usize,
    pub(crate) base_hash_changed: bool,
    pub(crate) needs_reconcile: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlaySummary {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) base_layer: SkillLayer,
    pub(crate) status: OverlayStatus,
    pub(crate) needs_reconcile: bool,
    pub(crate) pinned: bool,
    pub(crate) base_instruction_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) last_healthy_scope: Option<OverlayScope>,
    pub(crate) scopes: Vec<OverlayScopeSummary>,
    pub(crate) scopes_truncated: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayDiffHunk {
    pub(crate) label: String,
    pub(crate) before: OverlayBoundedText,
    pub(crate) after: OverlayBoundedText,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayDiff {
    pub(crate) base_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) added_characters: usize,
    pub(crate) removed_characters: usize,
    pub(crate) hunks: Vec<OverlayDiffHunk>,
    pub(crate) hunks_truncated: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayScopeDiff {
    pub(crate) scope: OverlayScope,
    pub(crate) revision: u64,
    pub(crate) input_hash: String,
    pub(crate) output_hash: String,
    pub(crate) diff: OverlayDiff,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayMutationSummary {
    pub(crate) id: String,
    pub(crate) kind: OverlayMutationKind,
    pub(crate) scope: OverlayScope,
    pub(crate) state: OverlayMutationState,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayConflictSummary {
    pub(crate) id: String,
    pub(crate) mutation_id: String,
    pub(crate) safe_reason: String,
    pub(crate) state: OverlayConflictState,
    pub(crate) resolution_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayResourceShadow {
    pub(crate) scope: Option<OverlayScope>,
    pub(crate) base_layer: Option<SkillLayer>,
    pub(crate) content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayResourceSummary {
    pub(crate) mutation_id: String,
    pub(crate) logical_path: String,
    pub(crate) media_type: String,
    pub(crate) size_bytes: u64,
    pub(crate) content_hash: String,
    pub(crate) effective_scope: OverlayScope,
    pub(crate) state: OverlayMutationState,
    pub(crate) shadowed: Vec<OverlayResourceShadow>,
    pub(crate) shadowed_truncated: bool,
}

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayScanResult {
    pub(crate) scanner_version: String,
    pub(crate) passed: bool,
    pub(crate) safe_rule_ids: Vec<String>,
    pub(crate) rule_ids_truncated: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum OverlayMutation {
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

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayMutationRequest {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) witnesses: OverlayWitnesses,
    pub(crate) mutation: OverlayMutation,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayPreview {
    pub(crate) witnesses: OverlayWitnesses,
    pub(crate) tentative_revision: u64,
    pub(crate) scan: OverlayScanResult,
    pub(crate) diff: OverlayDiff,
    pub(crate) conflicts: Vec<OverlayConflictSummary>,
    pub(crate) conflicts_truncated: bool,
    pub(crate) can_commit: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayMutationOutcome {
    pub(crate) summary: OverlaySummary,
    pub(crate) committed_revision: u64,
    pub(crate) diff: OverlayDiff,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayImportRequest {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) source_name: String,
    pub(crate) archive: Vec<u8>,
    pub(crate) witnesses: OverlayWitnesses,
}

#[derive(Clone, PartialEq, Eq)]
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayPromotionRequest {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) reviewed_revision: u64,
    pub(crate) reviewed_document_hash: String,
    pub(crate) reviewed_scan: OverlayScanResult,
    pub(crate) witnesses: OverlayWitnesses,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayActor {
    User,
    System,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlayHistoryAction {
    Create,
    Patch,
    Learn,
    File,
    Import,
    Promote,
    Disable,
    Revert,
    Reconcile,
    Conflict,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayHistoryEntry {
    pub(crate) event_id: String,
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) prior_revision: Option<u64>,
    pub(crate) next_revision: u64,
    pub(crate) actor: OverlayActor,
    pub(crate) action: OverlayHistoryAction,
    pub(crate) timestamp: String,
    pub(crate) prior_document_hash: Option<String>,
    pub(crate) next_document_hash: String,
    pub(crate) scanner_version: String,
    pub(crate) safe_outcome: String,
    pub(crate) prior_event_hash: Option<String>,
    pub(crate) event_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayHistoryQuery {
    pub(crate) cursor: Option<String>,
    pub(crate) limit: usize,
}

impl OverlayHistoryQuery {
    pub(crate) fn bounded(cursor: Option<String>, requested: usize, maximum: usize) -> Self {
        Self {
            cursor,
            limit: requested.max(1).min(maximum.max(1)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlayPageIntegrity {
    Verified,
    Failed(OverlayIntegrityCode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OverlayHistoryPage {
    pub(crate) entries: Vec<OverlayHistoryEntry>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) integrity: OverlayPageIntegrity,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) enum OverlayConflictResolution {
    EditPatch {
        old_string: String,
        new_string: String,
        replace_all: bool,
    },
    Ignore,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationChoice {
    pub(crate) conflict_id: String,
    pub(crate) resolution: OverlayConflictResolution,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationRequest {
    pub(crate) canonical_skill_id: SkillId,
    pub(crate) scope: OverlayScope,
    pub(crate) workspace_identity: Option<String>,
    pub(crate) witnesses: OverlayWitnesses,
    pub(crate) choices: Vec<OverlayReconciliationChoice>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationBaseSnapshot {
    pub(crate) base_identity: String,
    pub(crate) base_layer: SkillLayer,
    pub(crate) instruction_hash: String,
    pub(crate) package_hash: String,
    pub(crate) instructions: Option<OverlayBoundedText>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationProposedResult {
    pub(crate) effective_hash: String,
    pub(crate) instructions: OverlayBoundedText,
    pub(crate) resources: Vec<OverlayResourceSummary>,
    pub(crate) resources_truncated: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationConflictChoice {
    pub(crate) conflict: OverlayConflictSummary,
    pub(crate) selected_resolution: Option<OverlayConflictResolution>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationPreviewInput {
    pub(crate) witnesses: OverlayWitnesses,
    pub(crate) witnessed_base: OverlayReconciliationBaseSnapshot,
    pub(crate) current_base: OverlayReconciliationBaseSnapshot,
    pub(crate) proposed_effective: OverlayReconciliationProposedResult,
    pub(crate) conflict_choices: Vec<OverlayReconciliationConflictChoice>,
    pub(crate) conflicts_truncated: bool,
    pub(crate) final_diff: OverlayDiff,
    pub(crate) final_diff_complete: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct OverlayReconciliationPreview {
    pub(crate) witnesses: OverlayWitnesses,
    pub(crate) witnessed_base: OverlayReconciliationBaseSnapshot,
    pub(crate) current_base: OverlayReconciliationBaseSnapshot,
    pub(crate) proposed_effective: OverlayReconciliationProposedResult,
    pub(crate) conflict_choices: Vec<OverlayReconciliationConflictChoice>,
    pub(crate) conflicts_truncated: bool,
    pub(crate) final_diff: OverlayDiff,
    pub(crate) final_diff_complete: bool,
    pub(crate) can_commit: bool,
}

impl OverlayReconciliationPreview {
    pub(crate) fn from_input(input: OverlayReconciliationPreviewInput) -> Self {
        let all_conflicts_resolved = input
            .conflict_choices
            .iter()
            .all(|choice| choice.selected_resolution.is_some());
        let can_commit = input.final_diff_complete
            && !input.final_diff.hunks_truncated
            && !input.proposed_effective.resources_truncated
            && !input.conflicts_truncated
            && all_conflicts_resolved;
        Self {
            witnesses: input.witnesses,
            witnessed_base: input.witnessed_base,
            current_base: input.current_base,
            proposed_effective: input.proposed_effective,
            conflict_choices: input.conflict_choices,
            conflicts_truncated: input.conflicts_truncated,
            final_diff: input.final_diff,
            final_diff_complete: input.final_diff_complete,
            can_commit,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::tooling::skills::application::OverlayApplicationError;

    #[test]
    fn bounded_text_counts_unicode_characters_without_splitting_them() {
        let text = OverlayBoundedText::from_text("甲乙A", 2);
        assert_eq!(text.content, "甲乙");
        assert_eq!(text.total_characters, 3);
        assert!(text.truncated);
    }

    #[test]
    fn history_queries_are_never_empty_or_unbounded() {
        assert_eq!(OverlayHistoryQuery::bounded(None, 0, 50).limit, 1);
        assert_eq!(OverlayHistoryQuery::bounded(None, 500, 50).limit, 50);
        assert_eq!(OverlayHistoryQuery::bounded(None, 10, 0).limit, 1);
    }

    #[test]
    fn structured_errors_keep_stale_and_integrity_details_machine_readable() {
        let stale = OverlayApplicationError::StaleWitnesses {
            expected_revision: Some(2),
            current_revision: Some(3),
            base_changed: true,
            payload_changed: false,
            pin_changed: false,
        };
        assert_eq!(
            stale.to_string(),
            "Overlay witnesses are stale; reload and preview again"
        );

        let integrity = OverlayApplicationError::Integrity {
            code: OverlayIntegrityCode::PayloadHashMismatch,
        };
        assert_eq!(
            integrity.to_string(),
            "Overlay integrity verification failed: payload-hash-mismatch"
        );
    }

    #[test]
    fn reconciliation_preview_requires_exact_witnesses_choices_and_a_complete_final_diff() {
        let unresolved = reconciliation_preview_input(None, true);
        let preview = OverlayReconciliationPreview::from_input(unresolved);

        assert!(!preview.can_commit);
        assert_eq!(preview.witnesses.expected_overlay_revision, Some(3));
        assert_eq!(preview.witnessed_base.instruction_hash, "instruction-v1");
        assert_eq!(preview.current_base.instruction_hash, "instruction-v2");
        assert_eq!(preview.proposed_effective.effective_hash, "effective-v2");
        assert_eq!(preview.conflict_choices.len(), 1);
        assert!(preview.final_diff_complete);

        let resolved = reconciliation_preview_input(Some(OverlayConflictResolution::Ignore), true);
        let preview = OverlayReconciliationPreview::from_input(resolved);
        assert!(preview.can_commit);

        let incomplete =
            reconciliation_preview_input(Some(OverlayConflictResolution::Ignore), false);
        let preview = OverlayReconciliationPreview::from_input(incomplete);
        assert!(!preview.can_commit);
    }

    #[test]
    fn reconciliation_preview_never_commits_truncated_evidence() {
        let mut truncated_conflicts =
            reconciliation_preview_input(Some(OverlayConflictResolution::Ignore), true);
        truncated_conflicts.conflicts_truncated = true;
        assert!(!OverlayReconciliationPreview::from_input(truncated_conflicts).can_commit);

        let mut truncated_resources =
            reconciliation_preview_input(Some(OverlayConflictResolution::Ignore), true);
        truncated_resources.proposed_effective.resources_truncated = true;
        assert!(!OverlayReconciliationPreview::from_input(truncated_resources).can_commit);

        let mut truncated_diff =
            reconciliation_preview_input(Some(OverlayConflictResolution::Ignore), true);
        truncated_diff.final_diff.hunks_truncated = true;
        assert!(!OverlayReconciliationPreview::from_input(truncated_diff).can_commit);
    }

    fn reconciliation_preview_input(
        selected_resolution: Option<OverlayConflictResolution>,
        final_diff_complete: bool,
    ) -> OverlayReconciliationPreviewInput {
        OverlayReconciliationPreviewInput {
            witnesses: OverlayWitnesses {
                expected_overlay_revision: Some(3),
                expected_base_instruction_hash: "instruction-v2".to_string(),
                expected_base_package_hash: "package-v2".to_string(),
                expected_payload_hash: None,
                expected_pinned: false,
            },
            witnessed_base: reconciliation_base("system:developer", "instruction-v1", "package-v1"),
            current_base: reconciliation_base("user:developer", "instruction-v2", "package-v2"),
            proposed_effective: OverlayReconciliationProposedResult {
                effective_hash: "effective-v2".to_string(),
                instructions: OverlayBoundedText::from_text("Proposed", 64),
                resources: Vec::new(),
                resources_truncated: false,
            },
            conflict_choices: vec![OverlayReconciliationConflictChoice {
                conflict: OverlayConflictSummary {
                    id: "conflict-1".to_string(),
                    mutation_id: "patch-1".to_string(),
                    safe_reason: "exact-patch-target-missing".to_string(),
                    state: OverlayConflictState::Active,
                    resolution_revision: None,
                },
                selected_resolution,
            }],
            conflicts_truncated: false,
            final_diff: OverlayDiff {
                base_hash: "instruction-v2".to_string(),
                effective_hash: "effective-v2".to_string(),
                added_characters: 4,
                removed_characters: 2,
                hunks: Vec::new(),
                hunks_truncated: false,
            },
            final_diff_complete,
        }
    }

    fn reconciliation_base(
        base_identity: &str,
        instruction_hash: &str,
        package_hash: &str,
    ) -> OverlayReconciliationBaseSnapshot {
        OverlayReconciliationBaseSnapshot {
            base_identity: base_identity.to_string(),
            base_layer: SkillLayer::System,
            instruction_hash: instruction_hash.to_string(),
            package_hash: package_hash.to_string(),
            instructions: Some(OverlayBoundedText::from_text("Base", 64)),
        }
    }
}
