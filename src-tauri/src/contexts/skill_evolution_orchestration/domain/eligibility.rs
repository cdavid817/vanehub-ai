use serde::Serialize;

use super::{
    canonical_hash, AutoApplyEligibilityV1, AutoEligibilityResult, AutomaticDraftProvenance,
    EligibilityPredicateV1, EvolutionIntegrityError, EvolutionPolicyMode,
};

pub(crate) const AUTO_ELIGIBILITY_POLICY_V1: &str = "auto-eligibility-all-conditions-v1";
pub(crate) const AUTO_DRAFT_QUALITY_CHECK_ORDER_V1: [&str; 9] = [
    "privacy_residue",
    "evidence_sufficiency",
    "duplicate_knowledge",
    "transient_incident",
    "guidance_specificity",
    "evidence_consistency",
    "target_compatibility",
    "executable_content_risk",
    "target_lifecycle_mutability",
];
const MINIMUM_CONFIDENCE_BASIS_POINTS: u16 = 9_500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomaticMutationKindV1 {
    LearnedGuidance,
    ExactPatch,
    File,
    Script,
    ToolDefinition,
    Command,
    PermissionExpansion,
    SideEffectExpansion,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomaticTargetAttributionV1 {
    Verified,
    Correlated,
    Weak,
    Unattributed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AutomaticTargetLifecycleV1 {
    ActiveMutable,
    Immutable,
    Archived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AutoEligibilityInputV1 {
    pub(crate) eligibility_id: String,
    pub(crate) run_id: String,
    pub(crate) draft_id: String,
    pub(crate) target_skill_id: String,
    pub(crate) mode: EvolutionPolicyMode,
    pub(crate) consent_current: bool,
    pub(crate) skill_allowlisted: bool,
    pub(crate) stable_skill_id: bool,
    pub(crate) assessment_current: bool,
    pub(crate) assessment_route_advance: bool,
    pub(crate) assessment_deterministic: bool,
    pub(crate) target_clear: bool,
    pub(crate) target_attribution: AutomaticTargetAttributionV1,
    pub(crate) target_model_consulted: bool,
    pub(crate) confidence_basis_points: u16,
    pub(crate) risk_low: bool,
    pub(crate) quality_checks: Vec<(String, String)>,
    pub(crate) independent_supporting_runs: u8,
    pub(crate) authorized_verified_correction: bool,
    pub(crate) independent_confirmations: u8,
    pub(crate) draft_current: bool,
    pub(crate) draft_trusted: bool,
    pub(crate) draft_provenance: AutomaticDraftProvenance,
    pub(crate) mutation_kind: AutomaticMutationKindV1,
    pub(crate) target_lifecycle: AutomaticTargetLifecycleV1,
    pub(crate) target_pinned: bool,
    pub(crate) overlay_healthy: bool,
    pub(crate) overlay_trusted: bool,
    pub(crate) overlay_scope_system: bool,
    pub(crate) idle_capacity: bool,
    pub(crate) rate_capacity: bool,
    pub(crate) circuit_breakers_closed: bool,
    pub(crate) evaluated_at_ms: i64,
    pub(crate) revision: u64,
}

pub(crate) fn evaluate_auto_eligibility(
    input: &AutoEligibilityInputV1,
) -> Result<AutoApplyEligibilityV1, EvolutionIntegrityError> {
    let support = input.independent_supporting_runs >= 3
        || (input.authorized_verified_correction && input.independent_confirmations >= 2);
    let conditions = [
        ("policy_mode_active", input.mode != EvolutionPolicyMode::Off),
        ("consent_current", input.consent_current),
        ("skill_allowlisted", input.skill_allowlisted),
        ("stable_skill_id", input.stable_skill_id),
        ("assessment_current", input.assessment_current),
        ("assessment_advance", input.assessment_route_advance),
        ("assessment_deterministic", input.assessment_deterministic),
        ("target_clear", input.target_clear),
        (
            "target_attribution_verified",
            input.target_attribution == AutomaticTargetAttributionV1::Verified,
        ),
        ("target_model_independent", !input.target_model_consulted),
        (
            "confidence_threshold",
            input.confidence_basis_points >= MINIMUM_CONFIDENCE_BASIS_POINTS,
        ),
        ("risk_low", input.risk_low),
        (
            "nine_quality_checks_pass",
            quality_checks_pass(&input.quality_checks),
        ),
        ("independent_support", support),
        ("draft_current", input.draft_current),
        ("draft_trusted", input.draft_trusted),
        ("draft_provenance", input.draft_provenance.eligible()),
        (
            "learned_guidance_only",
            input.mutation_kind == AutomaticMutationKindV1::LearnedGuidance,
        ),
        (
            "target_mutable_active",
            input.target_lifecycle == AutomaticTargetLifecycleV1::ActiveMutable,
        ),
        ("target_unpinned", !input.target_pinned),
        ("overlay_healthy", input.overlay_healthy),
        ("overlay_trusted", input.overlay_trusted),
        ("overlay_scope_bounded", !input.overlay_scope_system),
        ("idle_capacity", input.idle_capacity),
        ("rate_capacity", input.rate_capacity),
        ("circuit_breakers_closed", input.circuit_breakers_closed),
    ];
    let predicates = conditions
        .into_iter()
        .map(|(condition, passed)| predicate(condition, passed))
        .collect::<Result<Vec<_>, _>>()?;
    let all_pass = predicates.iter().all(|predicate| predicate.passed);
    let result = eligibility_result(input.mode, all_pass, &predicates);
    let proof_hash = canonical_hash(&(
        AUTO_ELIGIBILITY_POLICY_V1,
        &input.run_id,
        &input.draft_id,
        &input.target_skill_id,
        result,
        &predicates,
    ))?;
    Ok(AutoApplyEligibilityV1 {
        eligibility_id: input.eligibility_id.clone(),
        run_id: input.run_id.clone(),
        draft_id: input.draft_id.clone(),
        target_skill_id: input.target_skill_id.clone(),
        result,
        predicates,
        proof_hash,
        overlay_preview_hash: None,
        evaluated_at_ms: input.evaluated_at_ms,
        revision: input.revision,
    })
}

fn predicate(
    condition: &'static str,
    passed: bool,
) -> Result<EligibilityPredicateV1, EvolutionIntegrityError> {
    Ok(EligibilityPredicateV1 {
        condition: condition.into(),
        passed,
        safe_reason_code: (!passed).then(|| format!("auto.{condition}.failed")),
        witness_hash: Some(canonical_hash(&(
            AUTO_ELIGIBILITY_POLICY_V1,
            condition,
            passed,
        ))?),
    })
}

fn eligibility_result(
    mode: EvolutionPolicyMode,
    all_pass: bool,
    predicates: &[EligibilityPredicateV1],
) -> AutoEligibilityResult {
    if mode == EvolutionPolicyMode::Off {
        return AutoEligibilityResult::Ineligible;
    }
    if all_pass {
        return if mode == EvolutionPolicyMode::Observe {
            AutoEligibilityResult::WouldApply
        } else {
            AutoEligibilityResult::Eligible
        };
    }
    if predicates.iter().any(|predicate| {
        !predicate.passed
            && matches!(
                predicate.condition.as_str(),
                "idle_capacity" | "rate_capacity"
            )
    }) {
        AutoEligibilityResult::Waiting
    } else {
        AutoEligibilityResult::RoutedToCurator
    }
}

fn quality_checks_pass(checks: &[(String, String)]) -> bool {
    checks.len() == AUTO_DRAFT_QUALITY_CHECK_ORDER_V1.len()
        && checks
            .iter()
            .zip(AUTO_DRAFT_QUALITY_CHECK_ORDER_V1)
            .all(|((code, result), expected)| code == expected && result == "pass")
}
