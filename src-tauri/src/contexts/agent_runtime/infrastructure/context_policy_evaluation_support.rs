#![allow(dead_code)]

use super::context_policy_corpus::{ContextPolicyRegressionCase, CorpusAdjustment};
use super::context_policy_evaluation::ContextPolicyMutation;
use super::context_projection::PreparedContextProjection;
use crate::contexts::agent_runtime::application::{
    AuthoritativeContextPort, AuthoritativeContextValue, ContextReinjectionBudget,
    ContextReinjectionFailure, ContextReinjectionResult, ContextReinjectionService,
    ReinjectedContextValue,
};
use crate::contexts::agent_runtime::domain::{
    classify_components, ContextCompactionDecision, ContextOptimizationBudget, ContextSnapshot,
    MeasurementQuality, RetentionClass, SafeFingerprint, VerificationFailure,
    CONTEXT_ESTIMATOR_VERSION, CONTEXT_POLICY_VERSION, CONTEXT_SNAPSHOT_VERSION,
};
use std::collections::HashMap;

pub(super) fn adjusted_snapshot(
    projection: PreparedContextProjection,
    adjustment: CorpusAdjustment,
) -> ContextSnapshot {
    let mut components = projection.components;
    match adjustment {
        CorpusAdjustment::MarkDiscardable => {
            if let Some(component) = components
                .iter_mut()
                .find(|value| value.retention_class == RetentionClass::Summarizable)
            {
                component.retention_class = RetentionClass::Discardable;
            }
        }
        CorpusAdjustment::MarkReinjectable => {
            if let Some(component) = components
                .iter_mut()
                .find(|value| value.retention_class == RetentionClass::Summarizable)
            {
                component.reinjectable = true;
                classify_components(&mut components, &projection.rounds);
            }
        }
        CorpusAdjustment::TokensUnavailable => {
            for component in &mut components {
                component.estimated_tokens = None;
            }
        }
        CorpusAdjustment::ArithmeticBoundary => {
            if let Some(component) = components
                .iter_mut()
                .find(|value| value.retention_class == RetentionClass::Summarizable)
            {
                component.characters = u64::MAX;
                component.estimated_tokens = Some(u64::MAX);
            }
        }
        CorpusAdjustment::None => {}
    }
    let characters = components
        .iter()
        .fold(0_u64, |total, value| total.saturating_add(value.characters));
    let tokens = components.iter().try_fold(0_u64, |total, value| {
        value
            .estimated_tokens
            .map(|tokens| total.saturating_add(tokens))
    });
    ContextSnapshot {
        version: CONTEXT_SNAPSHOT_VERSION,
        estimator_version: CONTEXT_ESTIMATOR_VERSION,
        policy_version: CONTEXT_POLICY_VERSION,
        request_fingerprint: projection.request_fingerprint,
        quality: if tokens.is_some() {
            MeasurementQuality::Estimated
        } else {
            MeasurementQuality::CharactersOnly
        },
        characters,
        tokens,
        components,
        rounds: projection.rounds,
        capacity: None,
        reserved_tokens: None,
        remaining_tokens: None,
        utilization_basis_points: None,
        active_character_compaction: true,
        compaction_decision: ContextCompactionDecision::evaluate(None, None),
        overflow_count: projection.overflow_count,
    }
}

pub(super) fn evaluation_budget(
    snapshot: &ContextSnapshot,
    target_basis_points: u16,
) -> ContextOptimizationBudget {
    let target = |value: u64| {
        ((u128::from(value) * u128::from(target_basis_points)) / 10_000).min(u128::from(u64::MAX))
            as u64
    };
    ContextOptimizationBudget {
        original_characters: snapshot.characters,
        original_tokens: snapshot.tokens,
        target_characters: target(snapshot.characters),
        target_tokens: snapshot.tokens.map(target),
    }
}

pub(super) fn apply_policy_mutation(
    snapshot: &mut ContextSnapshot,
    mutation: ContextPolicyMutation,
) {
    if mutation == ContextPolicyMutation::DropFirstProtected {
        if let Some(component) = snapshot
            .components
            .iter_mut()
            .find(|value| value.retention_class == RetentionClass::Protected)
        {
            component.retention_class = RetentionClass::Discardable;
        }
    }
}

pub(super) fn apply_candidate_mutation(
    snapshot: &mut ContextSnapshot,
    mutation: ContextPolicyMutation,
) {
    if mutation == ContextPolicyMutation::BreakProtocol {
        if let Some(round) = snapshot.rounds.first_mut() {
            round.protocol_state =
                crate::contexts::agent_runtime::domain::ProtocolState::Incomplete;
        }
    }
}

struct FixtureContextSource {
    values: HashMap<
        crate::contexts::agent_runtime::application::ContextReinjectionKind,
        Vec<AuthoritativeContextValue>,
    >,
}

impl AuthoritativeContextPort for FixtureContextSource {
    fn load_current(
        &self,
        kind: crate::contexts::agent_runtime::application::ContextReinjectionKind,
    ) -> Result<Vec<AuthoritativeContextValue>, ContextReinjectionFailure> {
        self.values
            .get(&kind)
            .cloned()
            .ok_or(ContextReinjectionFailure::SourceUnavailable)
    }
}

pub(super) fn resolve_reinjections(
    case: &ContextPolicyRegressionCase,
) -> (Vec<ReinjectedContextValue>, Vec<SafeFingerprint>) {
    let Some(kind) = case.reinjection_kind else {
        return (Vec::new(), Vec::new());
    };
    let source = FixtureContextSource {
        values: HashMap::from([(
            kind,
            vec![AuthoritativeContextValue {
                kind,
                revision: "fixture-r1".to_string(),
                content: "[fixture-authoritative-context]".to_string(),
            }],
        )]),
    };
    let ContextReinjectionResult::Ready(values) =
        ContextReinjectionService::resolve(&source, &[kind], ContextReinjectionBudget::default())
    else {
        return (Vec::new(), Vec::new());
    };
    let required = values
        .iter()
        .filter_map(|value| SafeFingerprint::parse(&value.evidence.source_fingerprint))
        .collect();
    (values, required)
}

pub(super) fn project_reinjection_fingerprints(
    snapshot: &mut ContextSnapshot,
    required: &[SafeFingerprint],
) {
    for (component, fingerprint) in snapshot
        .components
        .iter_mut()
        .filter(|value| value.retention_class == RetentionClass::Summarizable)
        .zip(required)
    {
        component.content_fingerprint = fingerprint.as_str().to_string();
    }
}

pub(super) fn failure_label(failure: VerificationFailure) -> &'static str {
    match failure {
        VerificationFailure::CandidateNotSmaller => "candidate-not-smaller",
        VerificationFailure::TargetNotMet => "target-not-met",
        VerificationFailure::ProtectedContentChanged => "protected-content-changed",
        VerificationFailure::VerbatimContentChanged => "verbatim-content-changed",
        VerificationFailure::ComponentOrderChanged => "component-order-changed",
        VerificationFailure::ProtocolIncomplete => "protocol-incomplete",
        VerificationFailure::ActionMismatch => "action-mismatch",
        VerificationFailure::ReinjectionMissing => "reinjection-missing",
        VerificationFailure::CoverageIncomplete => "coverage-incomplete",
    }
}
