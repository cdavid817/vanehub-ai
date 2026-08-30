use std::collections::BTreeSet;

use sha2::{Digest, Sha256};
use unicode_normalization::UnicodeNormalization;

use super::LessonShape;

const MAX_GUIDANCE_FIELD_CHARS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LessonTrigger {
    CorrectedFeedback,
    VerificationFailure,
    ToolFailure,
    PermissionDenial,
    ProviderFailure,
    ProcessFailure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LessonBehavior {
    InspectEvidence,
    RetryWithConstraint,
    ValidateBeforeAction,
    PreserveState,
    RequestReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LessonProhibition {
    SkipValidation,
    RepeatUnsafeAction,
    ExposeSensitiveData,
    ExpandSideEffects,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LessonVerification {
    TestPasses,
    CommandSucceeds,
    StatePersists,
    HumanConfirmation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LessonEnvironment {
    Project,
    User,
    Remote,
    System,
    CrossEnvironment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum LessonContentKind {
    Guidance,
    Reference,
    Template,
    ToolDeclaration,
    Executable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StructuredLessonEvidence {
    pub(crate) trigger: Option<LessonTrigger>,
    pub(crate) required_behavior: Option<LessonBehavior>,
    pub(crate) prohibited_behavior: Option<LessonProhibition>,
    pub(crate) verification: Option<LessonVerification>,
    pub(crate) environment: Option<LessonEnvironment>,
    pub(crate) content_kinds: Vec<LessonContentKind>,
}

pub(crate) fn derive_lesson_shape(evidence: &StructuredLessonEvidence) -> LessonShape {
    let mut kinds = evidence.content_kinds.clone();
    kinds.sort();
    kinds.dedup();
    LessonShape {
        trigger: evidence.trigger.map(trigger_name).map(str::to_string),
        required_behavior: evidence
            .required_behavior
            .map(behavior_name)
            .map(str::to_string),
        prohibited_behavior: evidence
            .prohibited_behavior
            .map(prohibition_name)
            .map(str::to_string),
        verification: evidence
            .verification
            .map(verification_name)
            .map(str::to_string),
        environment: evidence
            .environment
            .map(environment_name)
            .map(str::to_string),
        content_kinds: kinds
            .into_iter()
            .map(content_kind_name)
            .map(str::to_string)
            .collect(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum GuidanceSourceKind {
    EffectiveSkill,
    TrustedOverlay,
    UntrustedOverlay,
    PendingAssessment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuidanceUnitInput {
    pub(crate) source: GuidanceSourceKind,
    pub(crate) reference: String,
    pub(crate) shape: LessonShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GuidanceUnit {
    pub(crate) source: GuidanceSourceKind,
    pub(crate) reference: String,
    pub(crate) trigger: Option<String>,
    pub(crate) action: Option<String>,
    pub(crate) constraint: Option<String>,
    pub(crate) verification: Option<String>,
    pub(crate) environment: Option<String>,
    pub(crate) fingerprint: String,
    pub(crate) canonical: bool,
}

pub(crate) fn build_guidance_units(inputs: &[GuidanceUnitInput]) -> Vec<GuidanceUnit> {
    let mut units = inputs
        .iter()
        .map(|input| {
            let fields = [
                normalized_optional(input.shape.trigger.as_deref()),
                normalized_optional(input.shape.required_behavior.as_deref()),
                normalized_optional(input.shape.prohibited_behavior.as_deref()),
                normalized_optional(input.shape.verification.as_deref()),
                normalized_optional(input.shape.environment.as_deref()),
            ];
            GuidanceUnit {
                source: input.source,
                reference: bounded(&input.reference),
                trigger: fields[0].clone(),
                action: fields[1].clone(),
                constraint: fields[2].clone(),
                verification: fields[3].clone(),
                environment: fields[4].clone(),
                fingerprint: structural_hash(&fields),
                canonical: input.source != GuidanceSourceKind::UntrustedOverlay,
            }
        })
        .collect::<Vec<_>>();
    units.sort_by(|left, right| left.reference.cmp(&right.reference));
    units
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum DuplicateClassification {
    Exact,
    Near,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DuplicateAssessment {
    pub(crate) classification: Option<DuplicateClassification>,
    pub(crate) canonical_reference: Option<String>,
    pub(crate) risk_references: Vec<String>,
}

pub(crate) fn assess_duplicate(
    candidate: &GuidanceUnit,
    existing: &[GuidanceUnit],
) -> DuplicateAssessment {
    let mut matches = Vec::new();
    let mut risk_references = Vec::new();
    for unit in existing {
        let classification = duplicate_classification(candidate, unit);
        if unit.canonical {
            if let Some(classification) = classification {
                matches.push((classification, unit.reference.clone()));
            }
        } else if classification.is_some() {
            risk_references.push(unit.reference.clone());
        }
    }
    matches.sort();
    risk_references.sort();
    risk_references.dedup();
    DuplicateAssessment {
        classification: matches.first().map(|item| item.0),
        canonical_reference: matches.first().map(|item| item.1.clone()),
        risk_references,
    }
}

fn duplicate_classification(
    candidate: &GuidanceUnit,
    existing: &GuidanceUnit,
) -> Option<DuplicateClassification> {
    if candidate.fingerprint == existing.fingerprint {
        return Some(DuplicateClassification::Exact);
    }
    if candidate.environment != existing.environment
        || !similar(candidate.trigger.as_deref(), existing.trigger.as_deref())
        || !similar(candidate.action.as_deref(), existing.action.as_deref())
    {
        return None;
    }
    let compatible_detail = similar(
        candidate.constraint.as_deref(),
        existing.constraint.as_deref(),
    ) || similar(
        candidate.verification.as_deref(),
        existing.verification.as_deref(),
    );
    compatible_detail.then_some(DuplicateClassification::Near)
}

fn similar(left: Option<&str>, right: Option<&str>) -> bool {
    match (left, right) {
        (Some(left), Some(right)) => {
            let left = words(left);
            let right = words(right);
            let union = left.union(&right).count();
            union > 0 && left.intersection(&right).count() * 4 >= union * 3
        }
        (None, None) => true,
        _ => false,
    }
}

fn words(value: &str) -> BTreeSet<&str> {
    value.split_whitespace().collect()
}

fn normalized_optional(value: Option<&str>) -> Option<String> {
    value.map(normalized).filter(|value| !value.is_empty())
}

fn normalized(value: &str) -> String {
    bounded(value)
        .nfkc()
        .flat_map(char::to_lowercase)
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn bounded(value: &str) -> String {
    value.chars().take(MAX_GUIDANCE_FIELD_CHARS).collect()
}

fn structural_hash(fields: &[Option<String>; 5]) -> String {
    let mut hasher = Sha256::new();
    for field in fields {
        let value = field.as_deref().unwrap_or_default().as_bytes();
        hasher.update((value.len() as u64).to_le_bytes());
        hasher.update(value);
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn trigger_name(value: LessonTrigger) -> &'static str {
    match value {
        LessonTrigger::CorrectedFeedback => "corrected_feedback",
        LessonTrigger::VerificationFailure => "verification_failure",
        LessonTrigger::ToolFailure => "tool_failure",
        LessonTrigger::PermissionDenial => "permission_denial",
        LessonTrigger::ProviderFailure => "provider_failure",
        LessonTrigger::ProcessFailure => "process_failure",
    }
}

fn behavior_name(value: LessonBehavior) -> &'static str {
    match value {
        LessonBehavior::InspectEvidence => "inspect_evidence",
        LessonBehavior::RetryWithConstraint => "retry_with_constraint",
        LessonBehavior::ValidateBeforeAction => "validate_before_action",
        LessonBehavior::PreserveState => "preserve_state",
        LessonBehavior::RequestReview => "request_review",
    }
}

fn prohibition_name(value: LessonProhibition) -> &'static str {
    match value {
        LessonProhibition::SkipValidation => "skip_validation",
        LessonProhibition::RepeatUnsafeAction => "repeat_unsafe_action",
        LessonProhibition::ExposeSensitiveData => "expose_sensitive_data",
        LessonProhibition::ExpandSideEffects => "expand_side_effects",
    }
}

fn verification_name(value: LessonVerification) -> &'static str {
    match value {
        LessonVerification::TestPasses => "test_passes",
        LessonVerification::CommandSucceeds => "command_succeeds",
        LessonVerification::StatePersists => "state_persists",
        LessonVerification::HumanConfirmation => "human_confirmation",
    }
}

fn environment_name(value: LessonEnvironment) -> &'static str {
    match value {
        LessonEnvironment::Project => "project",
        LessonEnvironment::User => "user",
        LessonEnvironment::Remote => "remote",
        LessonEnvironment::System => "system",
        LessonEnvironment::CrossEnvironment => "cross_environment",
    }
}

fn content_kind_name(value: LessonContentKind) -> &'static str {
    match value {
        LessonContentKind::Guidance => "guidance",
        LessonContentKind::Reference => "reference",
        LessonContentKind::Template => "template",
        LessonContentKind::ToolDeclaration => "tool_declaration",
        LessonContentKind::Executable => "executable",
    }
}
