use super::review_dto::{
    ReviewFileViewedReceiptDto, ReviewHunkDecisionReceiptDto, ReviewSessionDto,
};
use crate::contexts::sessions::application::{ReviewSummary, ReviewView};
use crate::contexts::sessions::domain::{
    ReviewDecision, ReviewFile, ReviewFileViewState, ReviewHunkDecision, ReviewSession,
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
    let value = serde_json::to_value(ReviewSessionDto::from(ReviewView {
        hunk_decisions: Vec::new(),
        session: review,
        viewed_paths: Vec::new(),
        summary: ReviewSummary {
            changed_files: 1,
            viewed_files: 0,
            unresolved_comments: 0,
            unresolved_findings: 0,
        },
    }))
    .unwrap();
    assert_eq!(value["sessionId"], "session-1");
    assert_eq!(value["baseRevision"], "base");
    assert_eq!(value["decision"], "pending");
    assert_eq!(value["createdAt"], "created");
    assert_eq!(value["files"][0]["changeType"], "modified");
}

#[test]
fn review_summary_crosses_as_four_named_counts() {
    let file =
        ReviewFile::try_new("src/main.rs".into(), None, "modified".into(), None, None).unwrap();
    let mut review = ReviewSession::try_new(
        "review-1".into(),
        "session-1".into(),
        "workspace-1".into(),
        None,
        None,
        "snapshot".into(),
        vec![file],
    )
    .unwrap();
    review.set_timestamps("created".into(), "updated".into());
    let value = serde_json::to_value(ReviewSessionDto::from(ReviewView {
        hunk_decisions: vec![ReviewHunkDecision::try_new(
            "src/main.rs".into(),
            "hunk-1".into(),
            "snapshot".into(),
            ReviewDecision::Accepted,
            "2026-08-27T00:00:00Z".into(),
        )
        .unwrap()],
        session: review,
        viewed_paths: vec!["src/main.rs".into()],
        summary: ReviewSummary {
            changed_files: 8,
            viewed_files: 4,
            unresolved_comments: 2,
            unresolved_findings: 1,
        },
    }))
    .unwrap();

    // Four numbers rather than a rendered sentence: "8 files · 4 unviewed" is a translation
    // problem, and a backend that shipped the string would have to know which locale to write it
    // in.
    assert_eq!(value["summary"]["changedFiles"], 8);
    assert_eq!(value["summary"]["viewedFiles"], 4);
    assert_eq!(value["summary"]["unresolvedComments"], 2);
    assert_eq!(value["summary"]["unresolvedFindings"], 1);
    // Unviewed is the subtraction, and it is the caller's to make. Shipping it as well would be a
    // fifth number that can disagree with the two it came from.
    assert!(value["summary"].get("unviewedFiles").is_none());

    // Matched by fingerprint on the reading side, so a decision survives an edit to a different
    // hunk. The snapshot it was recorded against is not sent: a caller that filtered on it would
    // drop every decision whenever any file in the review moved.
    assert_eq!(value["hunkDecisions"][0]["relativePath"], "src/main.rs");
    assert_eq!(value["hunkDecisions"][0]["hunkFingerprint"], "hunk-1");
    assert_eq!(value["hunkDecisions"][0]["decision"], "accepted");
    assert!(value["hunkDecisions"][0]
        .get("snapshotFingerprint")
        .is_none());
    assert!(value["hunkDecisions"][0].get("decidedAt").is_none());
}

#[test]
fn file_viewed_receipt_carries_the_witness_and_omits_a_time_it_does_not_have() {
    let viewed = ReviewFileViewState::try_new(
        "src/main.rs".into(),
        "snapshot-a".into(),
        "file-witness-1".into(),
        true,
        Some("2026-08-27T00:00:00Z".into()),
    )
    .unwrap();
    let value = serde_json::to_value(ReviewFileViewedReceiptDto::recorded(
        "review-1".into(),
        viewed,
    ))
    .unwrap();
    assert_eq!(value["reviewId"], "review-1");
    assert_eq!(value["relativePath"], "src/main.rs");
    // Not the snapshot fingerprint. A caller comparing this against a later receipt learns whether
    // the file moved, which is what decides whether an old mark still means anything.
    assert_eq!(value["fileWitness"], "file-witness-1");
    assert_eq!(value["viewed"], true);
    assert_eq!(value["viewedAt"], "2026-08-27T00:00:00Z");
    assert_eq!(value["simulated"], false);

    let unviewed = ReviewFileViewState::try_new(
        "src/main.rs".into(),
        "snapshot-a".into(),
        "file-witness-1".into(),
        false,
        None,
    )
    .unwrap();
    let value = serde_json::to_value(ReviewFileViewedReceiptDto::recorded(
        "review-1".into(),
        unviewed,
    ))
    .unwrap();
    // Absent rather than null. A reader that saw the key would have to decide what a null moment
    // means, and every answer to that is wrong.
    assert!(value.get("viewedAt").is_none());
    assert_eq!(value["viewed"], false);
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
