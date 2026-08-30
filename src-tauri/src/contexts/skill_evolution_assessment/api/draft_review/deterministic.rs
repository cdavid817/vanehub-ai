use super::*;

pub(super) fn deterministic_review(
    request: &DraftQualityReviewRequestApi,
) -> Result<DraftQualityReviewResultApi, AssessmentApiError> {
    let original = |code: &str| {
        request
            .original_checks
            .iter()
            .find(|check| check.code == code)
    };
    let material_change = materially_changed(request);
    let target = ranked_target(request);
    let mut gate = evaluate_quality_gates(&QualityGateInput {
        privacy_residue: original_failed(original("privacy_residue")),
        verified_corrected_feedback: original_passed(original("evidence_sufficiency")),
        independent_nonduplicate_runs: u8::from(original_passed(original("evidence_sufficiency")))
            * 2,
        duplicate: duplicate_input(original("duplicate_knowledge")),
        transient_incident: original_failed(original("transient_incident")),
        lesson_shape: internal_shape(&request.draft_lesson_shape),
        material_contradiction: material_change
            || original_reviewed(original("evidence_consistency")),
        contradiction_is_scoped: false,
        target: Some(target),
        target_compatible: request.target_matches,
        target_revision_current: request.target_revision_current,
        executable_content: request
            .draft_lesson_shape
            .content_kinds
            .iter()
            .any(|kind| kind != "guidance"),
        evidence_ids: request.evidence_ids.clone(),
    });
    if material_change {
        replace_reason(
            &mut gate.checks,
            QualityCheckKind::EvidenceConsistency,
            "draft_materially_changes_lesson",
        );
    }
    if !request.target_matches {
        replace_reason(
            &mut gate.checks,
            QualityCheckKind::TargetCompatibility,
            "draft_target_changed",
        );
    }
    let checks = gate.checks.iter().map(api_check).collect::<Vec<_>>();
    let approvable = !material_change
        && checks.iter().all(|check| check.result != "fail")
        && checks.iter().all(|check| {
            !matches!(
                check.code.as_str(),
                "target_compatibility" | "executable_content_risk" | "target_lifecycle_mutability"
            ) || check.result == "pass"
        });
    Ok(DraftQualityReviewResultApi {
        checks,
        deterministic_approvable: approvable,
        model_evaluation_allowed: gate.model_evaluation_allowed,
        model_consulted: false,
        model_fallback_reason: None,
    })
}

pub(super) fn validate_request(
    request: &DraftQualityReviewRequestApi,
) -> Result<(), AssessmentApiError> {
    if request.original_checks.len() != QUALITY_CHECK_ORDER_V1.len()
        || request
            .original_checks
            .iter()
            .zip(QUALITY_CHECK_ORDER_V1)
            .any(|(check, kind)| check.code != check_kind(kind))
        || request.evidence_ids.len() > 64
        || request.target_skill_id.trim().is_empty()
        || request.target_revision.trim().is_empty()
        || shape_fields(request).any(|value| value.is_empty() || value.chars().count() > 512)
    {
        return Err(AssessmentApiError::InvalidRequest);
    }
    Ok(())
}

fn ranked_target(request: &DraftQualityReviewRequestApi) -> RankedTarget {
    RankedTarget {
        witness: EffectiveTargetWitness {
            skill_id: request.target_skill_id.clone(),
            skill_type: "skill".to_string(),
            revision_hash: request.target_revision.clone(),
            scope: TargetScope::User,
            lifecycle: TargetLifecycle::Active,
            trust: TargetTrust::Trusted,
        },
        score: 100,
        attribution_score: 100,
        participation_score: 100,
        compatibility_score: 100,
        lexical_score: 100,
        locality_score: 100,
        matched_feature_classes: Vec::new(),
        exclusions: Vec::new(),
        attribution_uncertain: false,
    }
}

fn shape_fields(request: &DraftQualityReviewRequestApi) -> impl Iterator<Item = &str> {
    [
        request.draft_lesson_shape.trigger.as_str(),
        request.draft_lesson_shape.required_behavior.as_str(),
        request.draft_lesson_shape.prohibited_behavior.as_str(),
        request.draft_lesson_shape.verification.as_str(),
        request.draft_lesson_shape.environment.as_str(),
    ]
    .into_iter()
}

fn materially_changed(request: &DraftQualityReviewRequestApi) -> bool {
    let original = request
        .original_lesson_shape
        .required_behavior
        .trim()
        .to_lowercase();
    let draft = request
        .draft_lesson_shape
        .required_behavior
        .trim()
        .to_lowercase();
    !original.is_empty() && !draft.contains(&original)
}

fn duplicate_input(check: Option<&DraftQualityCheckApi>) -> DuplicateAssessment {
    let classification = match check.map(|value| value.result.as_str()) {
        Some("fail") => Some(DuplicateClassification::Exact),
        Some("review") => Some(DuplicateClassification::Near),
        _ => None,
    };
    DuplicateAssessment {
        classification,
        canonical_reference: classification.map(|_| "sanitized-original-check".to_string()),
        risk_references: Vec::new(),
    }
}

fn original_passed(check: Option<&DraftQualityCheckApi>) -> bool {
    check.is_some_and(|value| value.result == "pass")
}

fn original_failed(check: Option<&DraftQualityCheckApi>) -> bool {
    check.is_some_and(|value| value.result == "fail")
}

fn original_reviewed(check: Option<&DraftQualityCheckApi>) -> bool {
    check.is_some_and(|value| value.result == "review")
}

fn replace_reason(checks: &mut [QualityCheck], kind: QualityCheckKind, reason: &str) {
    if let Some(check) = checks.iter_mut().find(|check| check.kind == kind) {
        check.result = QualityCheckResult::Fail;
        check.reason_code = reason.to_string();
    }
}
