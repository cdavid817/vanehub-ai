use crate::contexts::skill_evolution_curation::domain::*;

const MAX_SHAPE_FIELD_CHARS: usize = 512;

pub(crate) fn project_draft_lesson(binding: &CuratorDraftReviewBinding) -> CuratorDraftLessonShape {
    let verification = match binding.draft_kind.as_str() {
        "learn_block" => "overlay_learn_block_preview",
        "exact_patch" => "single_exact_match_preview",
        _ => "unsupported_draft_kind",
    };
    CuratorDraftLessonShape {
        trigger: bounded(&binding.rationale),
        required_behavior: bounded(&binding.expected_effective_change),
        prohibited_behavior: "preserve_assessed_target_and_scope".to_string(),
        verification: verification.to_string(),
        environment: binding.target_skill_id.clone(),
        content_kinds: vec!["guidance".to_string()],
    }
}

pub(crate) fn validate_quality_receipt(
    input: &CuratorDraftQualityInput,
    receipt: &CuratorDraftQualityReceipt,
) -> Result<bool, &'static str> {
    if receipt.candidate_witness_hash != input.candidate_witness_hash
        || receipt.target_skill_id != input.target_skill_id
        || receipt.target_revision != input.target_revision
        || receipt.draft_hash != input.draft_hash
    {
        return Err("draft_quality_witness_mismatch");
    }
    if receipt.checks.len() != CURATOR_DRAFT_CHECK_ORDER_V1.len()
        || receipt
            .checks
            .iter()
            .zip(CURATOR_DRAFT_CHECK_ORDER_V1)
            .any(|(check, expected)| check.code != expected)
    {
        return Err("draft_quality_check_set_invalid");
    }
    if receipt.model_consulted && !receipt.model_evaluation_allowed {
        return Err("draft_quality_model_consent_invalid");
    }
    let blocked = receipt.checks.iter().any(|check| {
        check.result == CuratorCheckResult::Fail
            || matches!(
                check.code.as_str(),
                "target_compatibility" | "executable_content_risk" | "target_lifecycle_mutability"
            ) && check.result != CuratorCheckResult::Pass
            || matches!(
                check.reason_code.as_str(),
                "draft_materially_changes_lesson"
                    | "draft_target_changed"
                    | "draft_executable_content"
                    | "draft_unsupported_content"
                    | "privacy_hard_stop"
            )
    });
    Ok(receipt.deterministic_approvable && !blocked)
}

fn bounded(value: &str) -> String {
    value.trim().chars().take(MAX_SHAPE_FIELD_CHARS).collect()
}
