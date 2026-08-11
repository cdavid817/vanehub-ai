#![cfg_attr(not(test), allow(dead_code))]

use super::{OverlayMutationState, OverlayPatch};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactPatchApplication {
    pub(crate) patch_id: String,
    pub(crate) match_count: usize,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct ExactPatchReplay {
    content: String,
    applications: Vec<ExactPatchApplication>,
}

impl ExactPatchReplay {
    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn applications(&self) -> &[ExactPatchApplication] {
        &self.applications
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ExactPatchConflictReason {
    TargetMissing,
    AmbiguousTarget { match_count: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExactPatchConflict {
    pub(crate) patch_id: String,
    pub(crate) reason: ExactPatchConflictReason,
}

pub(crate) fn replay_exact_patches(
    base_content: &str,
    patches: &[OverlayPatch],
) -> Result<ExactPatchReplay, ExactPatchConflict> {
    let mut content = base_content.to_string();
    let mut applications = Vec::new();

    for patch in patches {
        if patch.state() != OverlayMutationState::Active {
            continue;
        }

        let match_count = content.match_indices(&patch.old_string).count();
        if match_count == 0 {
            return Err(ExactPatchConflict {
                patch_id: patch.id.clone(),
                reason: ExactPatchConflictReason::TargetMissing,
            });
        }
        if !patch.replace_all && match_count != 1 {
            return Err(ExactPatchConflict {
                patch_id: patch.id.clone(),
                reason: ExactPatchConflictReason::AmbiguousTarget { match_count },
            });
        }

        content = if patch.replace_all {
            content.replace(&patch.old_string, &patch.new_string)
        } else {
            content.replacen(&patch.old_string, &patch.new_string, 1)
        };
        applications.push(ExactPatchApplication {
            patch_id: patch.id.clone(),
            match_count,
        });
    }

    Ok(ExactPatchReplay {
        content,
        applications,
    })
}
