use std::sync::Arc;

use super::error::PersonalizationApplicationError;
use super::ports::SecretRedactionPort;
use super::resolve_policy::{PolicyResolutionService, ResolutionRequest};
use crate::contexts::personalization::domain::{
    EffectiveMemoryAccess, EffectivePersonalizationSnapshot, ExcludedInstructionSegment,
    InstructionField, InstructionMergeAction, InstructionMergeMode, MemoryDeliveryMode,
    MemoryExclusionCount, PersonalizationWarning,
};

type Result<T> = std::result::Result<T, PersonalizationApplicationError>;

/// The estimator's own version, so a number recorded today is not silently compared against one a
/// later build produced by different rules.
const ESTIMATOR_VERSION: &str = "personalization-context-estimate-v1";

/// Rough characters per token. Deliberately coarse and named: this is an estimate for a settings
/// screen, not an accounting figure, and pretending otherwise by tokenizing here would tie the
/// preview to whichever tokenizer one provider happens to use.
const CHARACTERS_PER_TOKEN: usize = 4;

/// What one instruction field looks like in a preview.
///
/// The text is redacted through the same rule the logs use. A user may have pasted a token into
/// their own instructions, and echoing it back into a screen — which is screenshotted, pasted into
/// issues, and read over shoulders — would be handing it out again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreviewInstructionSegment {
    pub(crate) field: InstructionField,
    pub(crate) scope_kind: &'static str,
    pub(crate) scope_key: String,
    pub(crate) policy_revision: u64,
    pub(crate) merge_action: InstructionMergeAction,
    pub(crate) redacted_text: String,
    pub(crate) characters: usize,
}

/// How much of the context window VaneHub's own personalization accounts for.
///
/// Scoped narrowly and stated so, because an estimate whose boundaries are unclear is worse than
/// none: a user who reads "2,000 tokens" and finds their turn costs 40,000 has been misled by the
/// omission rather than the number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextSizeEstimate {
    pub(crate) known_characters: usize,
    pub(crate) known_utf8_bytes: usize,
    pub(crate) approximate_tokens: usize,
    /// The ceiling selected bodies may add, when the runtime takes them. Nothing has been selected
    /// at preview time, so a single number here would be invented; the budget is the honest bound.
    pub(crate) selected_body_budget_max: usize,
    /// Everything deliberately outside the count, named rather than implied.
    pub(crate) excluded_surfaces: Vec<&'static str>,
    pub(crate) estimator_version: &'static str,
}

/// What the settings screen shows about one resolution.
///
/// Deliberately not the snapshot: a snapshot carries instruction text and memory refs verbatim
/// because a runtime needs them, and a preview is read by a human on a screen. What is safe to act
/// on and what is safe to display are different sets, and giving them one type would make the
/// difference a convention instead of a boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EffectivePreview {
    pub(crate) revision_token: String,
    pub(crate) instruction_mode: InstructionMergeMode,
    pub(crate) included_instructions: Vec<PreviewInstructionSegment>,
    pub(crate) excluded_instructions: Vec<ExcludedInstructionSegment>,
    pub(crate) memory_access: EffectiveMemoryAccess,
    pub(crate) delivery: MemoryDeliveryMode,
    pub(crate) automatic_extraction: bool,
    pub(crate) eligible_memory_count: usize,
    pub(crate) considered_memory_count: usize,
    pub(crate) memory_exclusions: Vec<MemoryExclusionCount>,
    pub(crate) warnings: Vec<PersonalizationWarning>,
    pub(crate) context_estimate: ContextSizeEstimate,
    /// Always false, and reported rather than assumed: VaneHub does not manage a CLI's internal
    /// context, and a screen that stayed silent about it would leave a user thinking the number
    /// above covers their whole session.
    pub(crate) cli_internal_compaction_managed: bool,
}

/// Renders one resolution for a human.
///
/// Everything it returns comes from the snapshot the runtime would actually get, so the screen
/// cannot drift from the behaviour. What it removes is what a screen must not carry: memory bodies,
/// the raw folder a legacy record recorded, display paths, remote URIs, credentials, and any core
/// or system prompt — none of which the snapshot models in the first place, which is what makes
/// "the preview cannot leak them" a property rather than a filter that has to be maintained.
pub(crate) struct PersonalizationPreviewService {
    resolver: Arc<PolicyResolutionService>,
    redaction: Arc<dyn SecretRedactionPort>,
}

impl PersonalizationPreviewService {
    pub(crate) fn new(
        resolver: Arc<PolicyResolutionService>,
        redaction: Arc<dyn SecretRedactionPort>,
    ) -> Self {
        Self {
            resolver,
            redaction,
        }
    }

    pub(crate) fn preview(&self, request: ResolutionRequest) -> Result<EffectivePreview> {
        let snapshot = self.resolver.resolve(request)?;
        Ok(self.render(snapshot))
    }

    fn render(&self, snapshot: EffectivePersonalizationSnapshot) -> EffectivePreview {
        let included: Vec<PreviewInstructionSegment> = snapshot
            .instruction_segments
            .iter()
            .map(|segment| {
                let redacted_text = self.redaction.redact(&segment.text);
                PreviewInstructionSegment {
                    field: segment.field,
                    scope_kind: segment.scope_kind,
                    scope_key: segment.scope_key.clone(),
                    policy_revision: segment.policy_revision,
                    // The count is of what will actually be sent, not of the redacted rendering:
                    // a user sizing their instructions needs the real length.
                    characters: segment.text.chars().count(),
                    merge_action: segment.merge_action,
                    redacted_text,
                }
            })
            .collect();

        let context_estimate = Self::estimate(&snapshot, &included);
        EffectivePreview {
            revision_token: snapshot.revision_token,
            instruction_mode: snapshot.effective_instruction_mode,
            included_instructions: included,
            excluded_instructions: snapshot.excluded_instruction_segments,
            delivery: snapshot.memory_access.delivery,
            automatic_extraction: snapshot.memory_access.automatic_extraction,
            memory_access: snapshot.memory_access,
            eligible_memory_count: snapshot.memory.eligible_total,
            considered_memory_count: snapshot.memory.considered,
            memory_exclusions: snapshot.memory.exclusions,
            warnings: snapshot.warnings,
            context_estimate,
            cli_internal_compaction_managed: false,
        }
    }

    /// Counts only what VaneHub itself contributes.
    ///
    /// The resolved user instructions and the memory index it would inject, and nothing else. Not
    /// the core prompt, not the user's message, not Prompt Hooks, not a CLI's own context — every
    /// one of those is either not ours to count or not knowable from here, and including a guess
    /// for any of them would make the total unfalsifiable.
    fn estimate(
        snapshot: &EffectivePersonalizationSnapshot,
        included: &[PreviewInstructionSegment],
    ) -> ContextSizeEstimate {
        let mut characters = 0usize;
        let mut bytes = 0usize;
        for segment in included {
            characters += segment.characters;
            bytes += segment.redacted_text.len();
        }
        // The index is a bounded pointer list: one line per eligible memory, name and description.
        for entry in &snapshot.memory.refs {
            characters += entry.name.chars().count() + entry.description.chars().count();
            bytes += entry.name.len() + entry.description.len();
        }

        ContextSizeEstimate {
            known_characters: characters,
            known_utf8_bytes: bytes,
            approximate_tokens: characters.div_ceil(CHARACTERS_PER_TOKEN),
            selected_body_budget_max: match snapshot.memory_access.delivery {
                MemoryDeliveryMode::IndexWithSelectedBodies => SELECTED_BODY_BUDGET_CHARACTERS,
                MemoryDeliveryMode::IndexOnly | MemoryDeliveryMode::None => 0,
            },
            excluded_surfaces: vec![
                "core_system_prompt",
                "user_message",
                "prompt_hooks",
                "cli_internal_context",
                "provider_tool_definitions",
            ],
            estimator_version: ESTIMATOR_VERSION,
        }
    }
}

/// The ceiling selected memory bodies may add to one turn.
///
/// A bound rather than a measurement: nothing has been selected when a preview is rendered, and
/// reporting a precise figure for a selection that has not happened would be a fabrication.
const SELECTED_BODY_BUDGET_CHARACTERS: usize = 8_000;
