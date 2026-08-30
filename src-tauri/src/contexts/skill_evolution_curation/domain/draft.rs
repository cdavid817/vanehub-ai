use super::{CuratorCandidateState, CuratorDraftKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CuratorDraftRequestV1 {
    pub(crate) schema_version: u16,
    pub(crate) candidate_id: String,
    pub(crate) expected_candidate_revision: u64,
    pub(crate) target_skill_id: Option<String>,
    pub(crate) target_revision: Option<String>,
    pub(crate) overlay_scope: Option<String>,
    pub(crate) mutation: CuratorDraftMutationInput,
    pub(crate) rationale: String,
    pub(crate) expected_effective_change: String,
    #[serde(default)]
    pub(crate) supporting_files: Vec<String>,
    #[serde(default)]
    pub(crate) requested_permissions: Vec<String>,
    #[serde(default)]
    pub(crate) commands: Vec<String>,
    #[serde(default)]
    pub(crate) direct_base_edit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum CuratorDraftMutationInput {
    LearnedGuidance {
        guidance: String,
    },
    ExactPatch {
        old_string: String,
        new_string: String,
        #[serde(default)]
        replace_all: bool,
    },
}

impl CuratorDraftMutationInput {
    pub(crate) fn kind(&self) -> CuratorDraftKind {
        match self {
            Self::LearnedGuidance { .. } => CuratorDraftKind::LearnBlock,
            Self::ExactPatch { .. } => CuratorDraftKind::ExactPatch,
        }
    }

    pub(crate) fn text_parts(&self) -> Vec<&str> {
        match self {
            Self::LearnedGuidance { guidance } => vec![guidance],
            Self::ExactPatch {
                old_string,
                new_string,
                ..
            } => vec![old_string, new_string],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorDraftCandidateBinding {
    pub(crate) candidate_id: String,
    pub(crate) candidate_revision: u64,
    pub(crate) state: CuratorCandidateState,
    pub(crate) target_skill_id: String,
    pub(crate) target_revision: String,
    pub(crate) overlay_scope: String,
    pub(crate) workspace_id: String,
    pub(crate) evidence_ids: Vec<String>,
    pub(crate) next_draft_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CuratorOverlayValidationReceipt {
    pub(crate) scanner_version: String,
    pub(crate) base_hash: String,
    pub(crate) base_package_hash: String,
    pub(crate) effective_hash: String,
    pub(crate) overlay_revision: Option<u64>,
    pub(crate) pin_witness: String,
    pub(crate) trust_witness: String,
    pub(crate) conflict_witness: String,
}
