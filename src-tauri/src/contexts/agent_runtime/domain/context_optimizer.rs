#![allow(dead_code)]

use std::collections::{HashMap, HashSet};

use super::{ContextComponent, ContextSnapshot, ProtocolState, RetentionClass};

pub(crate) const CONTEXT_OPTIMIZER_VERSION: &str = "onepiece-context-optimizer-v1";
pub(crate) const CONTEXT_VERIFIER_VERSION: &str = "onepiece-context-verifier-v1";
const MICROCOMPACTED_CHARACTERS: u64 = 160;
const MICROCOMPACTED_TOKENS: u64 = 40;
const MAX_SAFE_FINGERPRINT_LENGTH: usize = 64;
const MAX_TOOL_REFERENCE_LENGTH: usize = 256;
const TOOL_RESULT_MARKER: &str = "[OnePiece compacted tool result]";

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SafeFingerprint(String);

impl SafeFingerprint {
    pub(crate) fn parse(value: &str) -> Option<Self> {
        (!value.is_empty()
            && value.len() <= MAX_SAFE_FINGERPRINT_LENGTH
            && value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.')))
        .then(|| Self(value.to_owned()))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ContextOptimizationBudget {
    pub(crate) original_characters: u64,
    pub(crate) original_tokens: Option<u64>,
    pub(crate) target_characters: u64,
    pub(crate) target_tokens: Option<u64>,
}

impl ContextOptimizationBudget {
    fn is_met(&self, characters: u64, tokens: Option<u64>) -> bool {
        match (self.target_tokens, tokens) {
            (Some(target), Some(value)) => value <= target,
            _ => characters <= self.target_characters,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum OptimizationActionKind {
    DiscardTransient,
    ReplaceReinjectable,
    MicrocompactToolResult,
    SummarizeRound,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum OptimizationTarget {
    Component(usize),
    Round(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextOptimizationAction {
    pub(crate) kind: OptimizationActionKind,
    pub(crate) target: OptimizationTarget,
    pub(crate) source_fingerprints: Vec<SafeFingerprint>,
    pub(crate) reclaimed_characters: u64,
    pub(crate) reclaimed_tokens: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SummaryBoundary {
    pub(crate) first_round: usize,
    pub(crate) last_round: usize,
    pub(crate) round_count: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptimizationOutcome {
    TargetSatisfied,
    Planned,
    InsufficientReclaimableContext,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextOptimizationPlan {
    pub(crate) version: &'static str,
    pub(crate) budget: ContextOptimizationBudget,
    pub(crate) actions: Vec<ContextOptimizationAction>,
    pub(crate) summary_boundary: Option<SummaryBoundary>,
    pub(crate) projected_characters: u64,
    pub(crate) projected_tokens: Option<u64>,
    pub(crate) outcome: OptimizationOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FallbackReason {
    InvalidPlan,
    InsufficientReclaimableContext,
    ReductionFailed,
    ReinjectionUnavailable,
    SummaryFailed,
    ReconstructionFailed,
    VerificationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerificationFailure {
    CandidateNotSmaller,
    TargetNotMet,
    ProtectedContentChanged,
    VerbatimContentChanged,
    ComponentOrderChanged,
    ProtocolIncomplete,
    ActionMismatch,
    ReinjectionMissing,
    CoverageIncomplete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CandidateEvidence {
    pub(crate) characters: u64,
    pub(crate) tokens: Option<u64>,
    pub(crate) action_count: u32,
    pub(crate) component_count: u32,
    pub(crate) request_fingerprint: SafeFingerprint,
    pub(crate) reduction_basis: ReductionBasis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReductionBasis {
    Tokens,
    Characters,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextOptimizationVerification {
    pub(crate) version: &'static str,
    pub(crate) accepted: bool,
    pub(crate) failures: Vec<VerificationFailure>,
    pub(crate) candidate: Option<CandidateEvidence>,
    pub(crate) fallback_reason: Option<FallbackReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ToolResultOutcome {
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ToolResultReplacement {
    pub(crate) tool_reference: String,
    pub(crate) outcome: ToolResultOutcome,
    pub(crate) source_fingerprint: SafeFingerprint,
    pub(crate) compacted: bool,
}

impl ToolResultReplacement {
    pub(crate) fn new(
        tool_reference: &str,
        failed: bool,
        source_fingerprint: SafeFingerprint,
    ) -> Option<Self> {
        (!tool_reference.is_empty() && tool_reference.len() <= MAX_TOOL_REFERENCE_LENGTH).then(
            || Self {
                tool_reference: tool_reference.to_owned(),
                outcome: if failed {
                    ToolResultOutcome::Failed
                } else {
                    ToolResultOutcome::Completed
                },
                source_fingerprint,
                compacted: true,
            },
        )
    }

    pub(crate) fn marker(&self) -> String {
        let outcome = match self.outcome {
            ToolResultOutcome::Completed => "completed",
            ToolResultOutcome::Failed => "failed",
        };
        format!(
            "{TOOL_RESULT_MARKER} outcome={outcome}; source={}",
            self.source_fingerprint.as_str()
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OptimizationPlanError {
    InvalidBudget,
    UnsafeFingerprint,
    InvalidSnapshot,
}

pub(crate) fn verify_optimization_candidate(
    original: &ContextSnapshot,
    candidate: &ContextSnapshot,
    plan: &ContextOptimizationPlan,
    required_reinjections: &[SafeFingerprint],
) -> ContextOptimizationVerification {
    let mut failures = Vec::new();
    let original_by_sequence: HashMap<_, _> = original
        .components
        .iter()
        .map(|component| (component.sequence, component))
        .collect();
    let candidate_fingerprints: Vec<_> = candidate
        .components
        .iter()
        .map(|component| component.content_fingerprint.as_str())
        .collect();
    verify_retained_class(
        original,
        &candidate_fingerprints,
        RetentionClass::Protected,
        VerificationFailure::ProtectedContentChanged,
        &mut failures,
    );
    verify_retained_class(
        original,
        &candidate_fingerprints,
        RetentionClass::Verbatim,
        VerificationFailure::VerbatimContentChanged,
        &mut failures,
    );
    verify_retained_order(original, &candidate_fingerprints, &mut failures);
    if candidate
        .rounds
        .iter()
        .any(|round| round.protocol_state != ProtocolState::Complete)
    {
        push_failure(&mut failures, VerificationFailure::ProtocolIncomplete);
    }
    verify_action_correspondence(
        original,
        plan,
        &original_by_sequence,
        &candidate_fingerprints,
        &mut failures,
    );
    if required_reinjections
        .iter()
        .any(|required| !candidate_fingerprints.contains(&required.as_str()))
    {
        push_failure(&mut failures, VerificationFailure::ReinjectionMissing);
    }
    let covered = candidate.components.iter().fold(0_u64, |total, component| {
        total.saturating_add(component.characters)
    });
    if covered < candidate.characters || candidate.overflow_count > 0 {
        push_failure(&mut failures, VerificationFailure::CoverageIncomplete);
    }

    let reduction_basis = match (original.tokens, candidate.tokens) {
        (Some(original_tokens), Some(candidate_tokens)) => {
            if candidate_tokens >= original_tokens {
                push_failure(&mut failures, VerificationFailure::CandidateNotSmaller);
            }
            if plan
                .budget
                .target_tokens
                .is_some_and(|target| candidate_tokens > target)
            {
                push_failure(&mut failures, VerificationFailure::TargetNotMet);
            }
            ReductionBasis::Tokens
        }
        _ => {
            if candidate.characters >= original.characters {
                push_failure(&mut failures, VerificationFailure::CandidateNotSmaller);
            }
            if candidate.characters > plan.budget.target_characters {
                push_failure(&mut failures, VerificationFailure::TargetNotMet);
            }
            ReductionBasis::Characters
        }
    };
    let candidate_evidence =
        SafeFingerprint::parse(&candidate.request_fingerprint).map(|value| CandidateEvidence {
            characters: candidate.characters,
            tokens: candidate.tokens,
            action_count: plan.actions.len().min(u32::MAX as usize) as u32,
            component_count: candidate.components.len().min(u32::MAX as usize) as u32,
            request_fingerprint: value,
            reduction_basis,
        });
    if candidate_evidence.is_none() {
        push_failure(&mut failures, VerificationFailure::CoverageIncomplete);
    }
    ContextOptimizationVerification {
        version: CONTEXT_VERIFIER_VERSION,
        accepted: failures.is_empty(),
        fallback_reason: (!failures.is_empty()).then_some(FallbackReason::VerificationFailed),
        failures,
        candidate: candidate_evidence,
    }
}

fn verify_retained_class(
    original: &ContextSnapshot,
    candidate_fingerprints: &[&str],
    retention_class: RetentionClass,
    failure: VerificationFailure,
    failures: &mut Vec<VerificationFailure>,
) {
    if original
        .components
        .iter()
        .filter(|component| component.retention_class == retention_class)
        .any(|component| !candidate_fingerprints.contains(&component.content_fingerprint.as_str()))
    {
        push_failure(failures, failure);
    }
}

fn verify_retained_order(
    original: &ContextSnapshot,
    candidate_fingerprints: &[&str],
    failures: &mut Vec<VerificationFailure>,
) {
    let required: Vec<_> = original
        .components
        .iter()
        .filter(|component| {
            matches!(
                component.retention_class,
                RetentionClass::Protected | RetentionClass::Verbatim
            )
        })
        .map(|component| component.content_fingerprint.as_str())
        .collect();
    let required_set: HashSet<_> = required.iter().copied().collect();
    let actual: Vec<_> = candidate_fingerprints
        .iter()
        .copied()
        .filter(|fingerprint| required_set.contains(fingerprint))
        .collect();
    if actual != required {
        push_failure(failures, VerificationFailure::ComponentOrderChanged);
    }
}

fn verify_action_correspondence(
    original: &ContextSnapshot,
    plan: &ContextOptimizationPlan,
    original_by_sequence: &HashMap<usize, &ContextComponent>,
    candidate_fingerprints: &[&str],
    failures: &mut Vec<VerificationFailure>,
) {
    for action in &plan.actions {
        let expected: Vec<_> = match action.target {
            OptimizationTarget::Component(sequence) => original_by_sequence
                .get(&sequence)
                .map(|component| vec![component.content_fingerprint.as_str()])
                .unwrap_or_default(),
            OptimizationTarget::Round(index) => original
                .rounds
                .iter()
                .find(|round| {
                    round.index == index && round.protocol_state == ProtocolState::Complete
                })
                .map(|round| {
                    round
                        .component_sequences
                        .iter()
                        .filter_map(|sequence| original_by_sequence.get(sequence))
                        .map(|component| component.content_fingerprint.as_str())
                        .collect()
                })
                .unwrap_or_default(),
        };
        let declared: Vec<_> = action
            .source_fingerprints
            .iter()
            .map(SafeFingerprint::as_str)
            .collect();
        if expected.is_empty()
            || expected != declared
            || expected
                .iter()
                .any(|fingerprint| candidate_fingerprints.contains(fingerprint))
        {
            push_failure(failures, VerificationFailure::ActionMismatch);
        }
    }
}

fn push_failure(failures: &mut Vec<VerificationFailure>, failure: VerificationFailure) {
    if !failures.contains(&failure) {
        failures.push(failure);
    }
}

pub(crate) fn build_optimization_plan(
    snapshot: &ContextSnapshot,
    budget: ContextOptimizationBudget,
) -> Result<ContextOptimizationPlan, OptimizationPlanError> {
    validate_inputs(snapshot, budget)?;
    let mut projected_characters = budget.original_characters;
    let mut projected_tokens = budget.original_tokens;
    let mut actions = Vec::new();
    if budget.is_met(projected_characters, projected_tokens) {
        return Ok(plan(
            budget,
            actions,
            None,
            projected_characters,
            projected_tokens,
            OptimizationOutcome::TargetSatisfied,
        ));
    }

    let complete_rounds: HashSet<_> = snapshot
        .rounds
        .iter()
        .filter(|round| round.protocol_state == ProtocolState::Complete)
        .map(|round| round.index)
        .collect();
    for retention_class in [
        RetentionClass::Discardable,
        RetentionClass::Reinjectable,
        RetentionClass::Microcompactable,
    ] {
        for component in snapshot.components.iter().filter(|component| {
            component.retention_class == retention_class
                && component
                    .round
                    .is_none_or(|round| complete_rounds.contains(&round))
        }) {
            let action = component_action(component)?;
            apply_reclaim(&action, &mut projected_characters, &mut projected_tokens);
            actions.push(action);
            if budget.is_met(projected_characters, projected_tokens) {
                return Ok(plan(
                    budget,
                    actions,
                    None,
                    projected_characters,
                    projected_tokens,
                    OptimizationOutcome::Planned,
                ));
            }
        }
    }

    let (summary_actions, boundary) = select_summary_prefix(
        snapshot,
        &actions,
        budget,
        projected_characters,
        projected_tokens,
    )?;
    for action in summary_actions {
        apply_reclaim(&action, &mut projected_characters, &mut projected_tokens);
        actions.push(action);
        if budget.is_met(projected_characters, projected_tokens) {
            break;
        }
    }
    let outcome = if budget.is_met(projected_characters, projected_tokens) {
        OptimizationOutcome::Planned
    } else {
        OptimizationOutcome::InsufficientReclaimableContext
    };
    Ok(plan(
        budget,
        actions,
        boundary,
        projected_characters,
        projected_tokens,
        outcome,
    ))
}

fn validate_inputs(
    snapshot: &ContextSnapshot,
    budget: ContextOptimizationBudget,
) -> Result<(), OptimizationPlanError> {
    if budget.original_characters != snapshot.characters
        || budget.original_tokens != snapshot.tokens
        || budget.target_characters > budget.original_characters
        || matches!((budget.target_tokens, budget.original_tokens), (Some(target), Some(original)) if target > original)
    {
        return Err(OptimizationPlanError::InvalidBudget);
    }
    let mut sequences = HashSet::new();
    if snapshot.components.iter().any(|component| {
        !sequences.insert(component.sequence)
            || SafeFingerprint::parse(&component.content_fingerprint).is_none()
    }) {
        return Err(OptimizationPlanError::UnsafeFingerprint);
    }
    let known: HashSet<_> = snapshot
        .components
        .iter()
        .map(|value| value.sequence)
        .collect();
    if snapshot.rounds.iter().any(|round| {
        round
            .component_sequences
            .iter()
            .any(|sequence| !known.contains(sequence))
    }) {
        return Err(OptimizationPlanError::InvalidSnapshot);
    }
    Ok(())
}

fn component_action(
    component: &ContextComponent,
) -> Result<ContextOptimizationAction, OptimizationPlanError> {
    let kind = match component.retention_class {
        RetentionClass::Discardable => OptimizationActionKind::DiscardTransient,
        RetentionClass::Reinjectable => OptimizationActionKind::ReplaceReinjectable,
        RetentionClass::Microcompactable => OptimizationActionKind::MicrocompactToolResult,
        _ => return Err(OptimizationPlanError::InvalidSnapshot),
    };
    let retained_characters = matches!(kind, OptimizationActionKind::MicrocompactToolResult)
        .then_some(MICROCOMPACTED_CHARACTERS)
        .unwrap_or(0);
    let retained_tokens = matches!(kind, OptimizationActionKind::MicrocompactToolResult)
        .then_some(MICROCOMPACTED_TOKENS)
        .unwrap_or(0);
    Ok(ContextOptimizationAction {
        kind,
        target: OptimizationTarget::Component(component.sequence),
        source_fingerprints: vec![SafeFingerprint::parse(&component.content_fingerprint)
            .ok_or(OptimizationPlanError::UnsafeFingerprint)?],
        reclaimed_characters: component.characters.saturating_sub(retained_characters),
        reclaimed_tokens: component
            .estimated_tokens
            .map(|tokens| tokens.saturating_sub(retained_tokens)),
    })
}

fn select_summary_prefix(
    snapshot: &ContextSnapshot,
    prior_actions: &[ContextOptimizationAction],
    budget: ContextOptimizationBudget,
    mut characters: u64,
    mut tokens: Option<u64>,
) -> Result<(Vec<ContextOptimizationAction>, Option<SummaryBoundary>), OptimizationPlanError> {
    let components: HashMap<_, _> = snapshot
        .components
        .iter()
        .map(|component| (component.sequence, component))
        .collect();
    let previously_selected: HashSet<_> = prior_actions
        .iter()
        .filter_map(|action| match action.target {
            OptimizationTarget::Component(sequence) => Some(sequence),
            OptimizationTarget::Round(_) => None,
        })
        .collect();
    let mut actions = Vec::new();
    for round in &snapshot.rounds {
        if round.protocol_state != ProtocolState::Complete || round.component_sequences.is_empty() {
            break;
        }
        let members: Vec<_> = round
            .component_sequences
            .iter()
            .filter_map(|sequence| components.get(sequence).copied())
            .collect();
        if members.len() != round.component_sequences.len()
            || members.iter().any(|component| {
                component.retention_class != RetentionClass::Summarizable
                    || previously_selected.contains(&component.sequence)
            })
        {
            break;
        }
        let action = ContextOptimizationAction {
            kind: OptimizationActionKind::SummarizeRound,
            target: OptimizationTarget::Round(round.index),
            source_fingerprints: members
                .iter()
                .map(|component| {
                    SafeFingerprint::parse(&component.content_fingerprint)
                        .ok_or(OptimizationPlanError::UnsafeFingerprint)
                })
                .collect::<Result<_, _>>()?,
            reclaimed_characters: members.iter().fold(0_u64, |total, component| {
                total.saturating_add(component.characters)
            }),
            reclaimed_tokens: members.iter().try_fold(0_u64, |total, component| {
                component
                    .estimated_tokens
                    .map(|value| total.saturating_add(value))
            }),
        };
        apply_reclaim(&action, &mut characters, &mut tokens);
        actions.push(action);
        if budget.is_met(characters, tokens) {
            break;
        }
    }
    let boundary = match (actions.first(), actions.last()) {
        (Some(first), Some(last)) => match (&first.target, &last.target) {
            (OptimizationTarget::Round(first), OptimizationTarget::Round(last)) => {
                Some(SummaryBoundary {
                    first_round: *first,
                    last_round: *last,
                    round_count: actions.len().min(u32::MAX as usize) as u32,
                })
            }
            _ => None,
        },
        _ => None,
    };
    Ok((actions, boundary))
}

fn apply_reclaim(
    action: &ContextOptimizationAction,
    characters: &mut u64,
    tokens: &mut Option<u64>,
) {
    *characters = characters.saturating_sub(action.reclaimed_characters);
    *tokens = match (*tokens, action.reclaimed_tokens) {
        (Some(value), Some(reclaimed)) => Some(value.saturating_sub(reclaimed)),
        _ => None,
    };
}

fn plan(
    budget: ContextOptimizationBudget,
    actions: Vec<ContextOptimizationAction>,
    summary_boundary: Option<SummaryBoundary>,
    projected_characters: u64,
    projected_tokens: Option<u64>,
    outcome: OptimizationOutcome,
) -> ContextOptimizationPlan {
    ContextOptimizationPlan {
        version: CONTEXT_OPTIMIZER_VERSION,
        budget,
        actions,
        summary_boundary,
        projected_characters,
        projected_tokens,
        outcome,
    }
}
