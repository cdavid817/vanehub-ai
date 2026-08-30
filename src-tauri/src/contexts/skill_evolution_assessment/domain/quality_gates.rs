use super::{
    AssessmentRisk, AssessmentRoute, DuplicateAssessment, DuplicateClassification, LessonShape,
    QualityCheck, QualityCheckKind, QualityCheckResult, RankedTarget, TargetLifecycle,
};

pub(crate) const QUALITY_CHECK_ORDER_V1: [QualityCheckKind; 9] = [
    QualityCheckKind::PrivacyResidue,
    QualityCheckKind::EvidenceSufficiency,
    QualityCheckKind::DuplicateKnowledge,
    QualityCheckKind::TransientIncident,
    QualityCheckKind::GuidanceSpecificity,
    QualityCheckKind::EvidenceConsistency,
    QualityCheckKind::TargetCompatibility,
    QualityCheckKind::ExecutableContentRisk,
    QualityCheckKind::TargetLifecycleMutability,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateInput {
    pub(crate) privacy_residue: bool,
    pub(crate) verified_corrected_feedback: bool,
    pub(crate) independent_nonduplicate_runs: u8,
    pub(crate) duplicate: DuplicateAssessment,
    pub(crate) transient_incident: bool,
    pub(crate) lesson_shape: LessonShape,
    pub(crate) material_contradiction: bool,
    pub(crate) contradiction_is_scoped: bool,
    pub(crate) target: Option<RankedTarget>,
    pub(crate) target_compatible: bool,
    pub(crate) target_revision_current: bool,
    pub(crate) executable_content: bool,
    pub(crate) evidence_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct QualityGateAssessment {
    pub(crate) checks: Vec<QualityCheck>,
    pub(crate) model_evaluation_allowed: bool,
}

pub(crate) fn evaluate_quality_gates(input: &QualityGateInput) -> QualityGateAssessment {
    let privacy = privacy_check(input);
    let privacy_hard_stop = privacy.result == QualityCheckResult::Fail;
    let checks = vec![
        privacy,
        evidence_sufficiency_check(input, privacy_hard_stop),
        duplicate_check(input, privacy_hard_stop),
        transient_check(input, privacy_hard_stop),
        specificity_check(input, privacy_hard_stop),
        consistency_check(input, privacy_hard_stop),
        compatibility_check(input, privacy_hard_stop),
        executable_check(input, privacy_hard_stop),
        lifecycle_check(input, privacy_hard_stop),
    ];
    debug_assert_eq!(
        checks.iter().map(|check| check.kind).collect::<Vec<_>>(),
        QUALITY_CHECK_ORDER_V1
    );
    QualityGateAssessment {
        checks,
        model_evaluation_allowed: !privacy_hard_stop,
    }
}

fn privacy_check(input: &QualityGateInput) -> QualityCheck {
    if input.privacy_residue {
        check(
            QualityCheckKind::PrivacyResidue,
            QualityCheckResult::Fail,
            AssessmentRisk::High,
            "privacy_residue_detected",
            &input.evidence_ids,
            vec![AssessmentRoute::Drop],
        )
    } else {
        pass(QualityCheckKind::PrivacyResidue, "privacy_input_sanitized")
    }
}

fn evidence_sufficiency_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::EvidenceSufficiency, "privacy_hard_stop");
    }
    if input.verified_corrected_feedback {
        pass(
            QualityCheckKind::EvidenceSufficiency,
            "verified_corrected_feedback",
        )
    } else if input.independent_nonduplicate_runs >= 2 {
        pass(
            QualityCheckKind::EvidenceSufficiency,
            "independent_supporting_runs",
        )
    } else {
        check(
            QualityCheckKind::EvidenceSufficiency,
            QualityCheckResult::Fail,
            AssessmentRisk::Medium,
            "insufficient_independent_evidence",
            &input.evidence_ids,
            vec![AssessmentRoute::Drop],
        )
    }
}

fn duplicate_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::DuplicateKnowledge, "privacy_hard_stop");
    }
    match input.duplicate.classification {
        Some(DuplicateClassification::Exact) => check(
            QualityCheckKind::DuplicateKnowledge,
            QualityCheckResult::Fail,
            AssessmentRisk::Low,
            "canonical_exact_duplicate",
            &safe_optional_reference(&input.duplicate.canonical_reference),
            vec![AssessmentRoute::MergeDuplicate],
        ),
        Some(DuplicateClassification::Near) => check(
            QualityCheckKind::DuplicateKnowledge,
            QualityCheckResult::Review,
            AssessmentRisk::Medium,
            "canonical_near_duplicate",
            &safe_optional_reference(&input.duplicate.canonical_reference),
            vec![AssessmentRoute::MergeDuplicate],
        ),
        None if !input.duplicate.risk_references.is_empty() => check(
            QualityCheckKind::DuplicateKnowledge,
            QualityCheckResult::Review,
            AssessmentRisk::Medium,
            "untrusted_guidance_conflict",
            &input.duplicate.risk_references,
            vec![AssessmentRoute::NeedsHumanReview],
        ),
        None => pass(
            QualityCheckKind::DuplicateKnowledge,
            "no_structural_duplicate",
        ),
    }
}

fn transient_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::TransientIncident, "privacy_hard_stop");
    }
    if input.transient_incident {
        check(
            QualityCheckKind::TransientIncident,
            QualityCheckResult::Fail,
            AssessmentRisk::Low,
            "transient_or_local_incident",
            &input.evidence_ids,
            vec![AssessmentRoute::RecordMemoryOnly],
        )
    } else {
        pass(QualityCheckKind::TransientIncident, "durable_signal")
    }
}

fn specificity_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::GuidanceSpecificity, "privacy_hard_stop");
    }
    if input.lesson_shape.trigger.is_some()
        && input.lesson_shape.required_behavior.is_some()
        && input.lesson_shape.verification.is_some()
    {
        pass(
            QualityCheckKind::GuidanceSpecificity,
            "bounded_testable_guidance",
        )
    } else {
        check(
            QualityCheckKind::GuidanceSpecificity,
            QualityCheckResult::Fail,
            AssessmentRisk::Medium,
            "vague_or_untestable_guidance",
            &input.evidence_ids,
            vec![AssessmentRoute::Drop],
        )
    }
}

fn consistency_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::EvidenceConsistency, "privacy_hard_stop");
    }
    if input.material_contradiction && !input.contradiction_is_scoped {
        check(
            QualityCheckKind::EvidenceConsistency,
            QualityCheckResult::Review,
            AssessmentRisk::High,
            "material_contradiction",
            &input.evidence_ids,
            vec![AssessmentRoute::NeedsHumanReview],
        )
    } else if input.material_contradiction {
        pass(
            QualityCheckKind::EvidenceConsistency,
            "scoped_evidence_consistent",
        )
    } else {
        pass(QualityCheckKind::EvidenceConsistency, "evidence_consistent")
    }
}

fn compatibility_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::TargetCompatibility, "privacy_hard_stop");
    }
    let Some(target) = &input.target else {
        return check(
            QualityCheckKind::TargetCompatibility,
            QualityCheckResult::Fail,
            AssessmentRisk::Medium,
            "target_missing",
            &input.evidence_ids,
            vec![AssessmentRoute::Drop],
        );
    };
    if !input.target_compatible {
        check(
            QualityCheckKind::TargetCompatibility,
            QualityCheckResult::Fail,
            AssessmentRisk::Medium,
            "target_incompatible",
            &input.evidence_ids,
            vec![AssessmentRoute::Drop],
        )
    } else if target.attribution_uncertain {
        check(
            QualityCheckKind::TargetCompatibility,
            QualityCheckResult::Review,
            AssessmentRisk::Medium,
            "target_attribution_uncertain",
            &input.evidence_ids,
            vec![AssessmentRoute::NeedsHumanReview],
        )
    } else {
        pass(QualityCheckKind::TargetCompatibility, "target_compatible")
    }
}

fn executable_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(QualityCheckKind::ExecutableContentRisk, "privacy_hard_stop");
    }
    let shape_is_executable = input.lesson_shape.content_kinds.iter().any(|kind| {
        matches!(
            kind.as_str(),
            "executable" | "tool_declaration" | "script" | "command"
        )
    });
    if input.executable_content || shape_is_executable {
        check(
            QualityCheckKind::ExecutableContentRisk,
            QualityCheckResult::Review,
            AssessmentRisk::High,
            "executable_or_side_effect_expansion",
            &input.evidence_ids,
            vec![AssessmentRoute::NeedsHumanReview],
        )
    } else {
        pass(
            QualityCheckKind::ExecutableContentRisk,
            "non_executable_guidance",
        )
    }
}

fn lifecycle_check(input: &QualityGateInput, stopped: bool) -> QualityCheck {
    if stopped {
        return not_applicable(
            QualityCheckKind::TargetLifecycleMutability,
            "privacy_hard_stop",
        );
    }
    if !input.target_revision_current {
        return lifecycle_drop("target_revision_changed", input);
    }
    match input.target.as_ref().map(|target| target.witness.lifecycle) {
        Some(TargetLifecycle::Active) => pass(
            QualityCheckKind::TargetLifecycleMutability,
            "target_active_and_mutable",
        ),
        Some(TargetLifecycle::Pinned) => check(
            QualityCheckKind::TargetLifecycleMutability,
            QualityCheckResult::Fail,
            AssessmentRisk::Medium,
            "target_pinned",
            &input.evidence_ids,
            vec![AssessmentRoute::RecordMemoryOnly],
        ),
        Some(TargetLifecycle::Archived) => lifecycle_drop("target_archived", input),
        Some(TargetLifecycle::Missing) | None => lifecycle_drop("target_missing", input),
        Some(TargetLifecycle::Malformed) => lifecycle_drop("target_malformed", input),
    }
}

fn lifecycle_drop(reason: &str, input: &QualityGateInput) -> QualityCheck {
    check(
        QualityCheckKind::TargetLifecycleMutability,
        QualityCheckResult::Fail,
        AssessmentRisk::Medium,
        reason,
        &input.evidence_ids,
        vec![AssessmentRoute::Drop],
    )
}

fn pass(kind: QualityCheckKind, reason: &str) -> QualityCheck {
    check(
        kind,
        QualityCheckResult::Pass,
        AssessmentRisk::Low,
        reason,
        &[],
        Vec::new(),
    )
}

fn not_applicable(kind: QualityCheckKind, reason: &str) -> QualityCheck {
    check(
        kind,
        QualityCheckResult::NotApplicable,
        AssessmentRisk::Low,
        reason,
        &[],
        Vec::new(),
    )
}

fn check(
    kind: QualityCheckKind,
    result: QualityCheckResult,
    severity: AssessmentRisk,
    reason: &str,
    evidence_ids: &[String],
    route_constraints: Vec<AssessmentRoute>,
) -> QualityCheck {
    let mut evidence_ids = evidence_ids.to_vec();
    evidence_ids.sort();
    evidence_ids.dedup();
    QualityCheck {
        kind,
        result,
        severity,
        reason_code: reason.to_string(),
        evidence_ids,
        route_constraints,
    }
}

fn safe_optional_reference(reference: &Option<String>) -> Vec<String> {
    reference.iter().cloned().collect()
}
