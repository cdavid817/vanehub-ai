use super::{CompactionTriggerSource, ContextSnapshot, MeasurementQuality};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompactionPath {
    Optimizer,
    Compatibility,
}

impl CompactionPath {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Optimizer => "optimizer",
            Self::Compatibility => "compatibility",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextCompactionEvidence {
    pub(crate) attempt_id: String,
    pub(crate) before_characters: u64,
    pub(crate) after_characters: u64,
    pub(crate) saved_characters: u64,
    pub(crate) before_tokens: Option<u64>,
    pub(crate) after_tokens: Option<u64>,
    pub(crate) saved_tokens: Option<u64>,
    pub(crate) before_quality: &'static str,
    pub(crate) after_quality: &'static str,
    pub(crate) trigger_source: &'static str,
    pub(crate) compaction_path: &'static str,
    pub(crate) policy_version: &'static str,
}

impl ContextCompactionEvidence {
    pub(crate) fn project(
        before: &ContextSnapshot,
        after: &ContextSnapshot,
        trigger_source: CompactionTriggerSource,
        path: CompactionPath,
        attempt_id: String,
    ) -> Self {
        Self {
            attempt_id,
            before_characters: before.characters,
            after_characters: after.characters,
            saved_characters: before.characters.saturating_sub(after.characters),
            before_tokens: before.tokens,
            after_tokens: after.tokens,
            saved_tokens: before
                .tokens
                .zip(after.tokens)
                .map(|(before, after)| before.saturating_sub(after)),
            before_quality: quality_label(before.quality),
            after_quality: quality_label(after.quality),
            trigger_source: trigger_label(trigger_source),
            compaction_path: path.as_str(),
            policy_version: before.policy_version,
        }
    }
}

fn quality_label(quality: MeasurementQuality) -> &'static str {
    match quality {
        MeasurementQuality::Reported => "reported",
        MeasurementQuality::ReportedPlusEstimatedDelta => "reported-plus-estimated-delta",
        MeasurementQuality::Estimated => "estimated",
        MeasurementQuality::CharactersOnly => "characters-only",
    }
}

fn trigger_label(source: CompactionTriggerSource) -> &'static str {
    match source {
        CompactionTriggerSource::TokenAware => "token-aware",
        CompactionTriggerSource::CharacterFallback => "character-fallback",
    }
}
