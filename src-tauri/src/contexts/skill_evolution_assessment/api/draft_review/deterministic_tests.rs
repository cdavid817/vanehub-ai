use super::deterministic::deterministic_review;
use super::*;

#[test]
fn safe_refinement_reuses_all_nine_checks_and_remains_approvable() {
    let result = deterministic_review(&request("validate_before_action with preflight", true))
        .unwrap_or_else(|error| panic!("review: {error:?}"));

    assert_eq!(result.checks.len(), 9);
    assert!(result.deterministic_approvable);
    assert_eq!(
        result
            .checks
            .iter()
            .map(|check| check.code.as_str())
            .collect::<Vec<_>>(),
        [
            "privacy_residue",
            "evidence_sufficiency",
            "duplicate_knowledge",
            "transient_incident",
            "guidance_specificity",
            "evidence_consistency",
            "target_compatibility",
            "executable_content_risk",
            "target_lifecycle_mutability",
        ]
    );
}

#[test]
fn materially_changed_guidance_and_target_changes_are_hard_blocks() {
    let changed = deterministic_review(&request("run_unrelated_commands", true))
        .unwrap_or_else(|error| panic!("changed review: {error:?}"));
    assert!(!changed.deterministic_approvable);
    assert!(changed.checks.iter().any(|check| {
        check.reason_code == "draft_materially_changes_lesson" && check.result == "fail"
    }));

    let mismatch = deterministic_review(&request("validate_before_action", false))
        .unwrap_or_else(|error| panic!("target review: {error:?}"));
    assert!(!mismatch.deterministic_approvable);
    assert!(mismatch
        .checks
        .iter()
        .any(|check| { check.reason_code == "draft_target_changed" && check.result == "fail" }));
}

#[test]
fn executable_projection_cannot_become_approvable() {
    let mut input = request("validate_before_action", true);
    input.draft_lesson_shape.content_kinds = vec!["executable".to_string()];
    let result =
        deterministic_review(&input).unwrap_or_else(|error| panic!("executable review: {error:?}"));
    assert!(!result.deterministic_approvable);
    assert_eq!(result.checks[7].result, "review");
}

fn request(required_behavior: &str, target_matches: bool) -> DraftQualityReviewRequestApi {
    DraftQualityReviewRequestApi {
        evidence_ids: vec!["evidence-1".to_string()],
        original_checks: checks(),
        original_lesson_shape: shape("validate_before_action"),
        draft_lesson_shape: shape(required_behavior),
        target_skill_id: "skill-1".to_string(),
        target_revision: "revision-1".to_string(),
        target_matches,
        target_revision_current: true,
    }
}

fn shape(required_behavior: &str) -> DraftLessonShapeApi {
    DraftLessonShapeApi {
        trigger: "verification_failure".to_string(),
        required_behavior: required_behavior.to_string(),
        prohibited_behavior: "skip_validation".to_string(),
        verification: "test_passes".to_string(),
        environment: "project".to_string(),
        content_kinds: vec!["guidance".to_string()],
    }
}

fn checks() -> Vec<DraftQualityCheckApi> {
    [
        "privacy_residue",
        "evidence_sufficiency",
        "duplicate_knowledge",
        "transient_incident",
        "guidance_specificity",
        "evidence_consistency",
        "target_compatibility",
        "executable_content_risk",
        "target_lifecycle_mutability",
    ]
    .iter()
    .map(|code| DraftQualityCheckApi {
        code: (*code).to_string(),
        result: "pass".to_string(),
        reason_code: "fixture".to_string(),
    })
    .collect()
}
