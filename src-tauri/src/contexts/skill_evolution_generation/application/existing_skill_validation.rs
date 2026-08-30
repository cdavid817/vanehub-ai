use crate::contexts::skill_evolution_generation::{
    application::{canonical_hash, sha256_bytes},
    domain::{
        GeneratedArtifactKind, GenerationValidationCheckV1, GenerationValidationStatus,
        MutationTargetV1, StructuredDraftV1,
    },
};

use super::{
    validate_mutation_plan_against_frozen, ExistingSkillValidationPort,
    ExistingSkillValidationRequestV1, ExistingSkillValidationResultV1, ExpectedGenerationTargetV1,
    GenerationOverlayPreviewReceiptV1, MutationPlanValidationContextV1,
    GENERATION_DRAFT_CHECK_ORDER_V1,
};

pub(crate) fn validate_existing_skill_draft(
    port: &dyn ExistingSkillValidationPort,
    request: &ExistingSkillValidationRequestV1,
) -> ExistingSkillValidationResultV1 {
    let mut checks = Vec::new();
    local_checks(request, &mut checks);
    if checks.iter().all(passed) {
        scan_checks(port, request, &mut checks);
    }
    if checks.iter().all(passed) {
        duplicate_check(port, &request.artifact.content_hash, &mut checks);
    }
    if checks.iter().all(passed) {
        quality_checks(port, request, &mut checks);
    }
    let mut preview_witness_hash = None;
    if checks.iter().all(passed) {
        preview_witness_hash = preview_check(port, request, &mut checks);
    }
    let safe_reason_codes = checks
        .iter()
        .filter(|check| check.status != GenerationValidationStatus::Passed)
        .filter_map(|check| check.reason_code.clone())
        .collect::<Vec<_>>();
    let status = if safe_reason_codes.is_empty() {
        GenerationValidationStatus::Passed
    } else {
        GenerationValidationStatus::Failed
    };
    let report_hash = match canonical_hash(&(
        &request.validation_id,
        &checks,
        &preview_witness_hash,
        status,
    )) {
        Ok(hash) => hash,
        Err(_) => sha256_bytes(b"generation_validation_report_serialization_failed"),
    };
    ExistingSkillValidationResultV1 {
        status,
        checks,
        preview_witness_hash,
        report_hash,
        safe_reason_codes,
    }
}

fn local_checks(
    request: &ExistingSkillValidationRequestV1,
    checks: &mut Vec<GenerationValidationCheckV1>,
) {
    let context = MutationPlanValidationContextV1 {
        expected_target: ExpectedGenerationTargetV1::ExistingSkill {
            skill_id: request.frozen_skill_id.clone(),
            effective_revision: request.frozen_revision.clone(),
            overlay_scope: request.overlay_scope.clone(),
        },
        registered_citations: request.registered_citations.clone(),
    };
    push(
        checks,
        "structured_schema_and_citations",
        validate_mutation_plan_against_frozen(&request.plan, &context).is_ok()
            && artifact_matches_plan(request),
        "generation_structure_invalid",
    );
    let bytes_match = request.artifact.size_bytes as usize == request.artifact.content.len()
        && request.artifact.content_hash == sha256_bytes(request.artifact.content.as_bytes());
    push(
        checks,
        "content_and_token_budget",
        bytes_match
            && request.maximum_tokens > 0
            && request.estimated_tokens <= request.maximum_tokens,
        "generation_budget_invalid",
    );
    push(
        checks,
        "overlay_witness_current",
        !request.frozen_overlay_witness_hash.is_empty()
            && request.frozen_overlay_witness_hash == request.current_overlay_witness_hash,
        "generation_overlay_stale",
    );
    push(
        checks,
        "exact_anchor",
        exact_anchor_valid(request),
        "generation_exact_anchor_invalid",
    );
    push(
        checks,
        "verification_plan",
        !request.plan.verification_steps.is_empty()
            && request.plan.verification_steps.iter().all(|step| {
                !step.action_code.trim().is_empty()
                    && !step.expected_code.trim().is_empty()
                    && !step.citation_ids.is_empty()
            }),
        "generation_verification_incomplete",
    );
}

fn scan_checks(
    port: &dyn ExistingSkillValidationPort,
    request: &ExistingSkillValidationRequestV1,
    checks: &mut Vec<GenerationValidationCheckV1>,
) {
    match port.scan(&request.artifact) {
        Ok(receipt) => {
            let bound = !receipt.sanitizer_version.trim().is_empty()
                && receipt.content_hash == request.artifact.content_hash;
            push(
                checks,
                "privacy_sanitizer",
                bound && receipt.privacy_passed,
                "generation_privacy_rejected",
            );
            push(
                checks,
                "injection_scanner",
                bound && receipt.injection_passed,
                "generation_injection_rejected",
            );
            push(
                checks,
                "prohibited_content",
                bound && receipt.prohibited_content_passed,
                "generation_content_prohibited",
            );
        }
        Err(code) => push(checks, "safety_scan", false, code),
    }
}

fn duplicate_check(
    port: &dyn ExistingSkillValidationPort,
    artifact_hash: &str,
    checks: &mut Vec<GenerationValidationCheckV1>,
) {
    match port.is_duplicate(artifact_hash) {
        Ok(duplicate) => push(
            checks,
            "structural_duplicate",
            !duplicate,
            "generation_duplicate",
        ),
        Err(code) => push(checks, "structural_duplicate", false, code),
    }
}

fn quality_checks(
    port: &dyn ExistingSkillValidationPort,
    request: &ExistingSkillValidationRequestV1,
    checks: &mut Vec<GenerationValidationCheckV1>,
) {
    let Ok(receipt) = port.quality(request) else {
        push(
            checks,
            "quality_gates",
            false,
            "generation_quality_unavailable",
        );
        return;
    };
    let ordered = receipt.artifact_hash == request.artifact.content_hash
        && receipt.checks.len() == GENERATION_DRAFT_CHECK_ORDER_V1.len()
        && receipt
            .checks
            .iter()
            .zip(GENERATION_DRAFT_CHECK_ORDER_V1)
            .all(|(check, expected)| check.code == expected);
    push(
        checks,
        "quality_gate_contract",
        ordered,
        "generation_quality_contract_invalid",
    );
    if ordered {
        checks.extend(receipt.checks);
        push(
            checks,
            "stricter_model_judge",
            receipt.stricter_judge_passed.unwrap_or(true),
            "generation_model_judge_rejected",
        );
    }
}

fn preview_check(
    port: &dyn ExistingSkillValidationPort,
    request: &ExistingSkillValidationRequestV1,
    checks: &mut Vec<GenerationValidationCheckV1>,
) -> Option<String> {
    let Ok(receipt) = port.preview(request) else {
        push(
            checks,
            "overlay_preview",
            false,
            "generation_preview_unavailable",
        );
        return None;
    };
    let patch_anchor_valid = request.plan.artifact_kind != GeneratedArtifactKind::OverlayExactPatch
        || receipt.exact_anchor_matches == 1;
    let valid = receipt.artifact_hash == request.artifact.content_hash
        && receipt.target_revision == request.frozen_revision
        && receipt.overlay_witness_hash == request.frozen_overlay_witness_hash
        && patch_anchor_valid
        && !receipt.unrelated_deletion
        && receipt.can_commit
        && !request.pinned
        && !receipt.preview_witness_hash.trim().is_empty();
    push(
        checks,
        "overlay_preview",
        valid,
        if request.pinned {
            "generation_target_pinned"
        } else {
            "generation_preview_invalid"
        },
    );
    valid.then_some(receipt.preview_witness_hash)
}

fn artifact_matches_plan(request: &ExistingSkillValidationRequestV1) -> bool {
    matches!(request.plan.target, MutationTargetV1::ExistingSkill { .. })
        && request.plan.artifact_kind == request.artifact.artifact_kind
        && matches!(
            (&request.plan.artifact_kind, &request.draft),
            (
                GeneratedArtifactKind::OverlayLearnBlock,
                StructuredDraftV1::OverlayLearnBlock { .. }
            ) | (
                GeneratedArtifactKind::OverlayExactPatch,
                StructuredDraftV1::OverlayExactPatch { .. }
            )
        )
}

fn exact_anchor_valid(request: &ExistingSkillValidationRequestV1) -> bool {
    match &request.draft {
        StructuredDraftV1::OverlayLearnBlock { .. } => true,
        StructuredDraftV1::OverlayExactPatch {
            old_string,
            new_string,
            replace_all,
        } => {
            !replace_all
                && !old_string.is_empty()
                && old_string != new_string
                && request.frozen_effective_content.matches(old_string).count() == 1
        }
        StructuredDraftV1::NewSkill { .. } => false,
    }
}

fn passed(check: &GenerationValidationCheckV1) -> bool {
    check.status == GenerationValidationStatus::Passed
}

fn push(
    checks: &mut Vec<GenerationValidationCheckV1>,
    code: &str,
    passed: bool,
    failure_code: &str,
) {
    checks.push(GenerationValidationCheckV1 {
        code: code.into(),
        status: if passed {
            GenerationValidationStatus::Passed
        } else {
            GenerationValidationStatus::Failed
        },
        reason_code: (!passed).then(|| failure_code.into()),
    });
}
