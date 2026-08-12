#![cfg_attr(not(test), allow(dead_code))]

use super::{OverlayLearnBlock, OverlayMutationState, OverlayScope};

pub(crate) const LEARNED_GUIDANCE_START_MARKER: &str =
    "<!-- vane:learned-guidance-overlay:start -->";
pub(crate) const LEARNED_GUIDANCE_END_MARKER: &str = "<!-- vane:learned-guidance-overlay:end -->";
pub(crate) const LEARNED_GUIDANCE_HEADING: &str = "## User-learned guidance overlay";

#[derive(Clone, Copy)]
pub(crate) struct ScopedLearnedGuidance<'a> {
    scope: OverlayScope,
    blocks: &'a [OverlayLearnBlock],
}

impl<'a> ScopedLearnedGuidance<'a> {
    pub(crate) fn new(scope: OverlayScope, blocks: &'a [OverlayLearnBlock]) -> Self {
        Self { scope, blocks }
    }
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct LearnedGuidanceReplay {
    content: String,
    applied_block_ids: Vec<String>,
}

impl LearnedGuidanceReplay {
    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn applied_block_ids(&self) -> &[String] {
        &self.applied_block_ids
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LearnedGuidanceConflictReason {
    DelimiterAlreadyPresent,
    DelimiterInjection,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LearnedGuidanceConflict {
    pub(crate) block_id: Option<String>,
    pub(crate) reason: LearnedGuidanceConflictReason,
}

pub(crate) fn render_learned_guidance(
    patched_content: &str,
    scoped_guidance: &[ScopedLearnedGuidance<'_>],
) -> Result<LearnedGuidanceReplay, LearnedGuidanceConflict> {
    if contains_delimiter(patched_content) {
        return Err(LearnedGuidanceConflict {
            block_id: None,
            reason: LearnedGuidanceConflictReason::DelimiterAlreadyPresent,
        });
    }

    let mut scopes = scoped_guidance.iter().collect::<Vec<_>>();
    scopes.sort_by_key(|scope| scope.scope);
    let mut active_blocks = Vec::new();
    for scope in scopes {
        for block in scope.blocks {
            if block.state() == OverlayMutationState::Active {
                if contains_delimiter(&block.guidance) {
                    return Err(LearnedGuidanceConflict {
                        block_id: Some(block.id.clone()),
                        reason: LearnedGuidanceConflictReason::DelimiterInjection,
                    });
                }
                active_blocks.push(block);
            }
        }
    }

    if active_blocks.is_empty() {
        return Ok(LearnedGuidanceReplay {
            content: patched_content.to_string(),
            applied_block_ids: Vec::new(),
        });
    }

    let mut content = patched_content.to_string();
    content.push_str("\n\n");
    content.push_str(LEARNED_GUIDANCE_START_MARKER);
    content.push('\n');
    content.push_str(LEARNED_GUIDANCE_HEADING);
    let mut applied_block_ids = Vec::with_capacity(active_blocks.len());
    for block in active_blocks {
        content.push_str("\n\n");
        content.push_str(&block.guidance);
        applied_block_ids.push(block.id.clone());
    }
    content.push('\n');
    content.push_str(LEARNED_GUIDANCE_END_MARKER);

    Ok(LearnedGuidanceReplay {
        content,
        applied_block_ids,
    })
}

fn contains_delimiter(value: &str) -> bool {
    value.contains(LEARNED_GUIDANCE_START_MARKER)
        || value.contains(LEARNED_GUIDANCE_END_MARKER)
        || value.contains(LEARNED_GUIDANCE_HEADING)
}
