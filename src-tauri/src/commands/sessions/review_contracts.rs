use super::review_dto::{ReviewHunkDecisionReceiptDto, ReviewSessionDto};
use crate::contexts::sessions::domain::{
    ReviewDecision, ReviewFile, ReviewHunkDecision, ReviewSession,
};

#[test]
fn review_dto_uses_stable_camel_case_and_explicit_enum_values() {
    let file = ReviewFile::try_new(
        "src/main.rs".into(),
        None,
        "modified".into(),
        Some("old".into()),
        Some("new".into()),
    )
    .unwrap();
    let mut review = ReviewSession::try_new(
        "review-1".into(),
        "session-1".into(),
        "workspace-1".into(),
        Some("base".into()),
        Some("head".into()),
        "snapshot".into(),
        vec![file],
    )
    .unwrap();
    review.set_timestamps("created".into(), "updated".into());
    let value = serde_json::to_value(ReviewSessionDto::from(review)).unwrap();
    assert_eq!(value["sessionId"], "session-1");
    assert_eq!(value["baseRevision"], "base");
    assert_eq!(value["decision"], "pending");
    assert_eq!(value["createdAt"], "created");
    assert_eq!(value["files"][0]["changeType"], "modified");
}

#[test]
fn hunk_decision_receipt_uses_stable_camel_case_and_the_review_s_own_decision_values() {
    let recorded = ReviewHunkDecision::try_new(
        "src/main.rs".into(),
        "hunk-1".into(),
        "snapshot-a".into(),
        ReviewDecision::ChangesRequested,
        "2026-08-27T00:00:00Z".into(),
    )
    .unwrap();
    let value = serde_json::to_value(ReviewHunkDecisionReceiptDto::recorded(
        "review-1".into(),
        recorded,
    ))
    .unwrap();

    assert_eq!(value["reviewId"], "review-1");
    assert_eq!(value["relativePath"], "src/main.rs");
    assert_eq!(value["hunkFingerprint"], "hunk-1");
    // The same spelling the review-level decision uses. Two spellings for one concept is how a
    // frontend ends up matching one of them and silently ignoring the other.
    assert_eq!(value["decision"], "changes-requested");
    // False on this side, always. The field distinguishes a decision that reached a store from one
    // that lives in the Web fixture's memory.
    assert_eq!(value["simulated"], false);
    // The moment it was recorded is not echoed: a caller that rendered it would be showing a clock
    // read from another machine, and nothing here needs it.
    assert!(value.get("decidedAt").is_none());
}
