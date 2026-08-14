#![allow(dead_code)]

use serde::Serialize;
use std::collections::BTreeMap;

use super::{
    CompactionBypassReason, CompactionPath, CompactionTriggerSource, FallbackReason,
    MeasurementQuality,
};

pub(crate) const CONTEXT_QUALITY_ASSESSMENT_VERSION: &str =
    "onepiece-context-quality-assessment-v1";
pub(crate) const CONTEXT_QUALITY_HISTORY_HARD_LIMIT: u64 = 10_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ContextAssessmentOutcome {
    Compacted,
    Bypassed,
    Fallback,
    Failed,
}

impl ContextAssessmentOutcome {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Compacted => "compacted",
            Self::Bypassed => "bypassed",
            Self::Fallback => "fallback",
            Self::Failed => "failed",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "compacted" => Some(Self::Compacted),
            "bypassed" => Some(Self::Bypassed),
            "fallback" => Some(Self::Fallback),
            "failed" => Some(Self::Failed),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ContextAssessmentPath {
    Optimizer,
    Compatibility,
}

impl From<CompactionPath> for ContextAssessmentPath {
    fn from(value: CompactionPath) -> Self {
        match value {
            CompactionPath::Optimizer => Self::Optimizer,
            CompactionPath::Compatibility => Self::Compatibility,
        }
    }
}

impl ContextAssessmentPath {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Optimizer => "optimizer",
            Self::Compatibility => "compatibility",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "optimizer" => Some(Self::Optimizer),
            "compatibility" => Some(Self::Compatibility),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ContextAssessmentReason {
    RequestSuppressed,
    UserPreferenceSuppressed,
    Cooldown,
    CircuitOpen,
    InvalidPlan,
    InsufficientReclaimableContext,
    ReductionFailed,
    ReinjectionUnavailable,
    SummaryFailed,
    ReconstructionFailed,
    VerificationFailed,
    ProviderFailure,
    PersistenceFailure,
}

impl From<CompactionBypassReason> for ContextAssessmentReason {
    fn from(value: CompactionBypassReason) -> Self {
        match value {
            CompactionBypassReason::RequestSuppressed => Self::RequestSuppressed,
            CompactionBypassReason::UserPreferenceSuppressed => Self::UserPreferenceSuppressed,
            CompactionBypassReason::Cooldown => Self::Cooldown,
            CompactionBypassReason::CircuitOpen => Self::CircuitOpen,
        }
    }
}

impl From<FallbackReason> for ContextAssessmentReason {
    fn from(value: FallbackReason) -> Self {
        match value {
            FallbackReason::InvalidPlan => Self::InvalidPlan,
            FallbackReason::InsufficientReclaimableContext => Self::InsufficientReclaimableContext,
            FallbackReason::ReductionFailed => Self::ReductionFailed,
            FallbackReason::ReinjectionUnavailable => Self::ReinjectionUnavailable,
            FallbackReason::SummaryFailed => Self::SummaryFailed,
            FallbackReason::ReconstructionFailed => Self::ReconstructionFailed,
            FallbackReason::VerificationFailed => Self::VerificationFailed,
        }
    }
}

impl ContextAssessmentReason {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::RequestSuppressed => "request-suppressed",
            Self::UserPreferenceSuppressed => "user-preference-suppressed",
            Self::Cooldown => "cooldown",
            Self::CircuitOpen => "circuit-open",
            Self::InvalidPlan => "invalid-plan",
            Self::InsufficientReclaimableContext => "insufficient-reclaimable-context",
            Self::ReductionFailed => "reduction-failed",
            Self::ReinjectionUnavailable => "reinjection-unavailable",
            Self::SummaryFailed => "summary-failed",
            Self::ReconstructionFailed => "reconstruction-failed",
            Self::VerificationFailed => "verification-failed",
            Self::ProviderFailure => "provider-failure",
            Self::PersistenceFailure => "persistence-failure",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "request-suppressed" => Some(Self::RequestSuppressed),
            "user-preference-suppressed" => Some(Self::UserPreferenceSuppressed),
            "cooldown" => Some(Self::Cooldown),
            "circuit-open" => Some(Self::CircuitOpen),
            "invalid-plan" => Some(Self::InvalidPlan),
            "insufficient-reclaimable-context" => Some(Self::InsufficientReclaimableContext),
            "reduction-failed" => Some(Self::ReductionFailed),
            "reinjection-unavailable" => Some(Self::ReinjectionUnavailable),
            "summary-failed" => Some(Self::SummaryFailed),
            "reconstruction-failed" => Some(Self::ReconstructionFailed),
            "verification-failed" => Some(Self::VerificationFailed),
            "provider-failure" => Some(Self::ProviderFailure),
            "persistence-failure" => Some(Self::PersistenceFailure),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ContextAssessmentMeasurementQuality {
    Reported,
    ReportedPlusEstimatedDelta,
    Estimated,
    CharactersOnly,
}

impl From<MeasurementQuality> for ContextAssessmentMeasurementQuality {
    fn from(value: MeasurementQuality) -> Self {
        match value {
            MeasurementQuality::Reported => Self::Reported,
            MeasurementQuality::ReportedPlusEstimatedDelta => Self::ReportedPlusEstimatedDelta,
            MeasurementQuality::Estimated => Self::Estimated,
            MeasurementQuality::CharactersOnly => Self::CharactersOnly,
        }
    }
}

impl ContextAssessmentMeasurementQuality {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Reported => "reported",
            Self::ReportedPlusEstimatedDelta => "reported-plus-estimated-delta",
            Self::Estimated => "estimated",
            Self::CharactersOnly => "characters-only",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "reported" => Some(Self::Reported),
            "reported-plus-estimated-delta" => Some(Self::ReportedPlusEstimatedDelta),
            "estimated" => Some(Self::Estimated),
            "characters-only" => Some(Self::CharactersOnly),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ContextAssessmentTriggerSource {
    TokenAware,
    CharacterFallback,
}

impl From<CompactionTriggerSource> for ContextAssessmentTriggerSource {
    fn from(value: CompactionTriggerSource) -> Self {
        match value {
            CompactionTriggerSource::TokenAware => Self::TokenAware,
            CompactionTriggerSource::CharacterFallback => Self::CharacterFallback,
        }
    }
}

impl ContextAssessmentTriggerSource {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::TokenAware => "token-aware",
            Self::CharacterFallback => "character-fallback",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "token-aware" => Some(Self::TokenAware),
            "character-fallback" => Some(Self::CharacterFallback),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextAssessmentInvariants {
    pub(crate) protocol_complete: bool,
    pub(crate) protected_retained: bool,
    pub(crate) verbatim_retained: bool,
    pub(crate) reinjection_complete: bool,
}

impl ContextAssessmentInvariants {
    pub(crate) const fn passed() -> Self {
        Self {
            protocol_complete: true,
            protected_retained: true,
            verbatim_retained: true,
            reinjection_complete: true,
        }
    }

    pub(crate) const fn all_passed(self) -> bool {
        self.protocol_complete
            && self.protected_retained
            && self.verbatim_retained
            && self.reinjection_complete
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextQualityAssessmentInput<'a> {
    pub(crate) generation_correlation: &'a str,
    pub(crate) decision_sequence: u64,
    pub(crate) outcome: ContextAssessmentOutcome,
    pub(crate) path: Option<ContextAssessmentPath>,
    pub(crate) reason: Option<ContextAssessmentReason>,
    pub(crate) trigger_source: Option<ContextAssessmentTriggerSource>,
    pub(crate) before_characters: u64,
    pub(crate) after_characters: u64,
    pub(crate) before_tokens: Option<u64>,
    pub(crate) after_tokens: Option<u64>,
    pub(crate) measurement_quality: ContextAssessmentMeasurementQuality,
    pub(crate) invariants: Option<ContextAssessmentInvariants>,
    pub(crate) context_policy_version: &'static str,
    pub(crate) optimizer_version: &'static str,
    pub(crate) verifier_version: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextQualityAssessment {
    pub(crate) version: String,
    pub(crate) attempt_id: String,
    pub(crate) decision_sequence: u64,
    pub(crate) outcome: ContextAssessmentOutcome,
    pub(crate) path: Option<ContextAssessmentPath>,
    pub(crate) reason: Option<ContextAssessmentReason>,
    pub(crate) trigger_source: Option<ContextAssessmentTriggerSource>,
    pub(crate) before_characters: u64,
    pub(crate) after_characters: u64,
    pub(crate) saved_characters: u64,
    pub(crate) before_tokens: Option<u64>,
    pub(crate) after_tokens: Option<u64>,
    pub(crate) saved_tokens: Option<u64>,
    pub(crate) measurement_quality: ContextAssessmentMeasurementQuality,
    pub(crate) invariants: Option<ContextAssessmentInvariants>,
    pub(crate) context_policy_version: String,
    pub(crate) optimizer_version: String,
    pub(crate) verifier_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ContextQualityAssessmentRecord {
    pub(crate) session_correlation: Option<String>,
    pub(crate) recorded_at: String,
    pub(crate) assessment: ContextQualityAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextQualityAssessmentPage {
    pub(crate) items: Vec<ContextQualityAssessmentRecord>,
    pub(crate) next_cursor: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ContextQualitySummary {
    pub(crate) evaluated: u64,
    pub(crate) saved_characters: u64,
    pub(crate) saved_tokens: u64,
    pub(crate) token_measurement_count: u64,
    pub(crate) outcomes: BTreeMap<String, u64>,
    pub(crate) paths: BTreeMap<String, u64>,
    pub(crate) qualities: BTreeMap<String, u64>,
    pub(crate) reasons: BTreeMap<String, u64>,
    pub(crate) policy_versions: BTreeMap<String, u64>,
    pub(crate) earliest_recorded_at: Option<String>,
    pub(crate) latest_recorded_at: Option<String>,
}

impl ContextQualityAssessment {
    pub(crate) fn new(input: ContextQualityAssessmentInput<'_>) -> Self {
        Self {
            version: CONTEXT_QUALITY_ASSESSMENT_VERSION.to_string(),
            attempt_id: stable_attempt_id(input.generation_correlation, input.decision_sequence),
            decision_sequence: input.decision_sequence,
            outcome: input.outcome,
            path: input.path,
            reason: input.reason,
            trigger_source: input.trigger_source,
            before_characters: input.before_characters,
            after_characters: input.after_characters,
            saved_characters: input
                .before_characters
                .saturating_sub(input.after_characters),
            before_tokens: input.before_tokens,
            after_tokens: input.after_tokens,
            saved_tokens: input
                .before_tokens
                .zip(input.after_tokens)
                .map(|(before, after)| before.saturating_sub(after)),
            measurement_quality: input.measurement_quality,
            invariants: input.invariants,
            context_policy_version: input.context_policy_version.to_string(),
            optimizer_version: input.optimizer_version.to_string(),
            verifier_version: input.verifier_version.to_string(),
        }
    }
}

fn stable_attempt_id(generation_correlation: &str, decision_sequence: u64) -> String {
    let hash = generation_correlation
        .bytes()
        .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
    format!("ctxq-{hash:016x}-{decision_sequence}")
}
