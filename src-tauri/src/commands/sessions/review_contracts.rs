use super::review_dto::ReviewSessionDto;
use crate::contexts::sessions::domain::{ReviewFile, ReviewSession};

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
