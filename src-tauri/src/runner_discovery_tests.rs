use crate::contexts::sessions::api::{optional_session_metadata, SessionsError};

#[test]
fn missing_optional_session_target_preserves_local_discovery() {
    let target = optional_session_metadata::<()>(Err(SessionsError::SessionNotFound(
        "session-1".to_string(),
    )))
    .expect("optional target failure remains fail-soft");

    assert!(target.is_none());
}

#[test]
fn storage_failure_remains_visible_to_runner_discovery() {
    let result = optional_session_metadata::<()>(Err(SessionsError::Repository(
        "sensitive storage detail".to_string(),
    )));

    assert!(matches!(
        result,
        Err(SessionsError::Repository(message)) if message == "sensitive storage detail"
    ));
}
