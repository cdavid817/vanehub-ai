#![allow(dead_code)]

use super::context_policy_corpus::{
    context_policy_regression_corpus, ContextPolicyRegressionCase, CorpusAdjustment,
    CONTEXT_POLICY_CORPUS_VERSION,
};
use super::context_policy_evaluation_support::{
    adjusted_snapshot, apply_candidate_mutation, apply_policy_mutation, evaluation_budget,
    failure_label, project_reinjection_fingerprints, resolve_reinjections,
};
use super::context_projection::project_request;
use super::context_reduction::reconstruct_candidate;
use crate::contexts::agent_runtime::domain::{
    build_optimization_plan, verify_optimization_candidate, OptimizationOutcome,
    CONTEXT_OPTIMIZER_VERSION, CONTEXT_SNAPSHOT_VERSION, CONTEXT_VERIFIER_VERSION,
};
use std::collections::BTreeMap;

const STRUCTURAL_SUMMARY: &str = "## PRIMARY INTENT\nfixture intent\n## TECHNICAL CONSTRAINTS\nfixture constraints\n## DECISIONS\nfixture decisions\n## FILES AND CODE AREAS\nfixture areas\n## ERRORS AND FIXES\nfixture fixes\n## COMPLETED WORK\nfixture completed\n## PENDING WORK\nfixture pending\n## IMMEDIATE NEXT ACTION\nfixture next";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContextPolicyMutation {
    None,
    DropFirstProtected,
    BreakProtocol,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextPolicyProfile {
    pub(crate) version: &'static str,
    pub(crate) target_basis_points: u16,
    pub(crate) mutation: ContextPolicyMutation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextPolicyCaseResult {
    pub(crate) case_id: &'static str,
    pub(crate) passed: bool,
    pub(crate) saved_characters: u64,
    pub(crate) saved_tokens: Option<u64>,
    pub(crate) fallback: Option<&'static str>,
    pub(crate) invariant_failures: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextPolicyEvaluationReport {
    pub(crate) corpus_version: &'static str,
    pub(crate) active_policy_version: &'static str,
    pub(crate) candidate_policy_version: &'static str,
    pub(crate) optimizer_version: &'static str,
    pub(crate) verifier_version: &'static str,
    pub(crate) snapshot_version: &'static str,
    pub(crate) active: Vec<ContextPolicyCaseResult>,
    pub(crate) candidate: Vec<ContextPolicyCaseResult>,
    pub(crate) active_passed: u32,
    pub(crate) candidate_passed: u32,
    pub(crate) regressions: u32,
    pub(crate) active_saved_characters: u64,
    pub(crate) candidate_saved_characters: u64,
    pub(crate) active_fallbacks: BTreeMap<&'static str, u32>,
    pub(crate) candidate_fallbacks: BTreeMap<&'static str, u32>,
    pub(crate) candidate_activated: bool,
}

pub(crate) fn evaluate_context_policies(
    active: &ContextPolicyProfile,
    candidate: &ContextPolicyProfile,
) -> Result<ContextPolicyEvaluationReport, &'static str> {
    validate_policy(active)?;
    validate_policy(candidate)?;
    let corpus = context_policy_regression_corpus();
    let active_results = evaluate_policy(&corpus, active);
    let candidate_results = evaluate_policy(&corpus, candidate);
    let regressions = active_results
        .iter()
        .zip(&candidate_results)
        .filter(|(active, candidate)| active.passed && !candidate.passed)
        .count()
        .min(u32::MAX as usize) as u32;
    Ok(ContextPolicyEvaluationReport {
        corpus_version: CONTEXT_POLICY_CORPUS_VERSION,
        active_policy_version: active.version,
        candidate_policy_version: candidate.version,
        optimizer_version: CONTEXT_OPTIMIZER_VERSION,
        verifier_version: CONTEXT_VERIFIER_VERSION,
        snapshot_version: CONTEXT_SNAPSHOT_VERSION,
        active_passed: passed_count(&active_results),
        candidate_passed: passed_count(&candidate_results),
        regressions,
        active_saved_characters: saved_characters(&active_results),
        candidate_saved_characters: saved_characters(&candidate_results),
        active_fallbacks: fallback_counts(&active_results),
        candidate_fallbacks: fallback_counts(&candidate_results),
        active: active_results,
        candidate: candidate_results,
        candidate_activated: false,
    })
}

fn validate_policy(policy: &ContextPolicyProfile) -> Result<(), &'static str> {
    if policy.version.is_empty()
        || policy.version.len() > 64
        || !policy
            .version
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        || !(1..10_000).contains(&policy.target_basis_points)
    {
        return Err("invalid context policy profile");
    }
    Ok(())
}

fn evaluate_policy(
    corpus: &[ContextPolicyRegressionCase],
    policy: &ContextPolicyProfile,
) -> Vec<ContextPolicyCaseResult> {
    corpus
        .iter()
        .map(|case| evaluate_case(case, policy))
        .collect()
}

fn evaluate_case(
    case: &ContextPolicyRegressionCase,
    policy: &ContextPolicyProfile,
) -> ContextPolicyCaseResult {
    let original = adjusted_snapshot(project_request(&case.body, case.shape), case.adjustment);
    let mut planning = original.clone();
    apply_policy_mutation(&mut planning, policy.mutation);
    let budget = evaluation_budget(&planning, policy.target_basis_points);
    let Ok(plan) = build_optimization_plan(&planning, budget) else {
        return failed_case(case.id, "invalid-plan", Vec::new());
    };
    if plan.outcome == OptimizationOutcome::InsufficientReclaimableContext {
        return failed_case(case.id, "insufficient-context", Vec::new());
    }
    let (reinjections, required) = resolve_reinjections(case);
    let summary = plan.summary_boundary.as_ref().map(|_| STRUCTURAL_SUMMARY);
    let Ok(candidate_body) =
        reconstruct_candidate(&case.body, case.shape, &plan, summary, &reinjections)
    else {
        return failed_case(case.id, "reconstruction", Vec::new());
    };
    let mut candidate = adjusted_snapshot(
        project_request(&candidate_body, case.shape),
        if case.adjustment == CorpusAdjustment::TokensUnavailable {
            CorpusAdjustment::TokensUnavailable
        } else {
            CorpusAdjustment::None
        },
    );
    apply_candidate_mutation(&mut candidate, policy.mutation);
    project_reinjection_fingerprints(&mut candidate, &required);
    let verification = verify_optimization_candidate(&original, &candidate, &plan, &required);
    ContextPolicyCaseResult {
        case_id: case.id,
        passed: verification.accepted,
        saved_characters: original.characters.saturating_sub(candidate.characters),
        saved_tokens: original
            .tokens
            .zip(candidate.tokens)
            .map(|(before, after)| before.saturating_sub(after)),
        fallback: verification.fallback_reason.map(|_| "verification"),
        invariant_failures: verification
            .failures
            .into_iter()
            .map(failure_label)
            .collect(),
    }
}

fn failed_case(
    case_id: &'static str,
    fallback: &'static str,
    invariant_failures: Vec<&'static str>,
) -> ContextPolicyCaseResult {
    ContextPolicyCaseResult {
        case_id,
        passed: false,
        saved_characters: 0,
        saved_tokens: None,
        fallback: Some(fallback),
        invariant_failures,
    }
}

fn passed_count(results: &[ContextPolicyCaseResult]) -> u32 {
    results
        .iter()
        .filter(|result| result.passed)
        .count()
        .min(u32::MAX as usize) as u32
}

fn saved_characters(results: &[ContextPolicyCaseResult]) -> u64 {
    results.iter().fold(0_u64, |total, result| {
        total.saturating_add(result.saved_characters)
    })
}

fn fallback_counts(results: &[ContextPolicyCaseResult]) -> BTreeMap<&'static str, u32> {
    let mut counts = BTreeMap::new();
    for fallback in results.iter().filter_map(|result| result.fallback) {
        let value = counts.entry(fallback).or_insert(0_u32);
        *value = value.saturating_add(1);
    }
    counts
}

#[cfg(test)]
#[path = "context_policy_evaluation_tests.rs"]
mod tests;
