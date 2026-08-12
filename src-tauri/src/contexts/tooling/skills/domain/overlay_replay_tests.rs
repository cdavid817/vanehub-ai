use super::{replay_exact_patches, ExactPatchConflictReason, OverlayMutationState, OverlayPatch};

fn patch(id: &str, old_string: &str, new_string: &str, replace_all: bool) -> OverlayPatch {
    OverlayPatch::new(
        id,
        old_string,
        new_string,
        replace_all,
        "instruction-hash",
        "2026-08-10T10:00:00Z",
    )
    .expect("valid patch")
}

fn replay_error(content: &str, patches: &[OverlayPatch]) -> super::ExactPatchConflict {
    match replay_exact_patches(content, patches) {
        Ok(_) => panic!("expected exact patch conflict"),
        Err(error) => error,
    }
}

#[test]
fn unique_exact_match_is_replaced_once() {
    let result = replay_exact_patches(
        "Write focused tests before implementation.",
        &[patch("patch-1", "focused", "behavioral", false)],
    )
    .expect("unique replay");

    assert_eq!(
        result.content(),
        "Write behavioral tests before implementation."
    );
    assert_eq!(result.applications()[0].patch_id, "patch-1");
    assert_eq!(result.applications()[0].match_count, 1);
}

#[test]
fn zero_matches_fail_closed_without_partial_content() {
    let error = replay_error(
        "Write tests first.",
        &[patch(
            "patch-missing",
            "Deploy first.",
            "Deploy last.",
            false,
        )],
    );

    assert_eq!(error.patch_id, "patch-missing");
    assert_eq!(error.reason, ExactPatchConflictReason::TargetMissing);
}

#[test]
fn multiple_matches_require_replace_all() {
    let error = replay_error(
        "test, test, test",
        &[patch("patch-ambiguous", "test", "check", false)],
    );

    assert_eq!(
        error.reason,
        ExactPatchConflictReason::AmbiguousTarget { match_count: 3 }
    );
}

#[test]
fn replace_all_changes_every_non_overlapping_exact_match() {
    let result = replay_exact_patches(
        "test, test, test",
        &[patch("patch-all", "test", "check", true)],
    )
    .expect("replace all");

    assert_eq!(result.content(), "check, check, check");
    assert_eq!(result.applications()[0].match_count, 3);
}

#[test]
fn unicode_targets_use_exact_string_equality() {
    let result = replay_exact_patches(
        "先写测试，再写实现。",
        &[patch("patch-unicode", "测试", "行为测试", false)],
    )
    .expect("unicode replay");

    assert_eq!(result.content(), "先写行为测试，再写实现。");
}

#[test]
fn newline_styles_are_not_normalized() {
    let error = replay_error(
        "first\r\nsecond",
        &[patch("patch-newline", "first\nsecond", "changed", false)],
    );

    assert_eq!(error.reason, ExactPatchConflictReason::TargetMissing);
}

#[test]
fn patches_replay_in_their_stored_creation_order() {
    let result = replay_exact_patches(
        "alpha",
        &[
            patch("patch-1", "alpha", "beta", false),
            patch("patch-2", "beta", "gamma", false),
        ],
    )
    .expect("ordered replay");

    assert_eq!(result.content(), "gamma");
    assert_eq!(result.applications()[0].patch_id, "patch-1");
    assert_eq!(result.applications()[1].patch_id, "patch-2");
}

#[test]
fn disabled_and_reverted_patches_remain_auditable_but_do_not_replay() {
    let mut disabled = patch("patch-disabled", "base", "disabled", false);
    disabled
        .disable("2026-08-10T11:00:00Z")
        .expect("disable patch");
    let mut reverted = patch("patch-reverted", "base", "reverted", false);
    reverted
        .revert("2026-08-10T11:00:00Z")
        .expect("revert patch");

    assert_eq!(disabled.state(), OverlayMutationState::Disabled);
    assert_eq!(reverted.state(), OverlayMutationState::Reverted);
    let result = replay_exact_patches("base", &[disabled, reverted]).expect("skip inactive");
    assert_eq!(result.content(), "base");
    assert!(result.applications().is_empty());
}
