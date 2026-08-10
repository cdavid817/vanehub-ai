use serde::Serialize;
use tauri::{AppHandle, Emitter};

use crate::contexts::sessions::application::{
    SessionRecoveryEvent, SessionRecoveryEventKind, SessionRecoveryEventPort,
    SessionsApplicationError,
};

#[derive(Clone)]
pub(crate) struct NativeSessionRecoveryEvents {
    app: AppHandle,
}

impl NativeSessionRecoveryEvents {
    pub(crate) fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
enum SessionEventKind {
    ActiveSessionChanged,
    ConfigurationChanged,
    RecoveryStarted,
    RecoveryCompleted,
    RecoveryActionRequired,
    RecoveryQuarantined,
    RecoveryAcknowledged,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionStateEvent {
    kind: SessionEventKind,
    session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    recovery_revision: Option<u64>,
}

pub(super) fn emit_active_session_changed(app: &AppHandle, session_id: Option<&str>) {
    emit(
        app,
        SessionEventKind::ActiveSessionChanged,
        session_id,
        None,
    );
}

pub(super) fn emit_configuration_changed(app: &AppHandle, session_id: &str) {
    emit(
        app,
        SessionEventKind::ConfigurationChanged,
        Some(session_id),
        None,
    );
}

fn emit(
    app: &AppHandle,
    kind: SessionEventKind,
    session_id: Option<&str>,
    recovery_revision: Option<u64>,
) {
    let _ = app.emit(
        "session:event",
        SessionStateEvent {
            kind,
            session_id: session_id.map(str::to_string),
            recovery_revision,
        },
    );
}

impl SessionRecoveryEventPort for NativeSessionRecoveryEvents {
    fn publish_recovery_event(
        &self,
        event: SessionRecoveryEvent,
    ) -> Result<(), SessionsApplicationError> {
        let kind = match event.kind {
            SessionRecoveryEventKind::Started => SessionEventKind::RecoveryStarted,
            SessionRecoveryEventKind::Completed => SessionEventKind::RecoveryCompleted,
            SessionRecoveryEventKind::ActionRequired => SessionEventKind::RecoveryActionRequired,
            SessionRecoveryEventKind::Quarantined => SessionEventKind::RecoveryQuarantined,
            SessionRecoveryEventKind::Acknowledged => SessionEventKind::RecoveryAcknowledged,
        };
        emit(
            &self.app,
            kind,
            Some(&event.session_id),
            Some(event.recovery_revision),
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_event_payloads_preserve_names_optional_identity_and_transport_shape() {
        for (kind, expected_kind, session_id) in [
            (
                SessionEventKind::ActiveSessionChanged,
                "active-session-changed",
                Some("session-1"),
            ),
            (
                SessionEventKind::ActiveSessionChanged,
                "active-session-changed",
                None,
            ),
            (
                SessionEventKind::ConfigurationChanged,
                "configuration-changed",
                Some("session-1"),
            ),
        ] {
            let value = serde_json::to_value(SessionStateEvent {
                kind,
                session_id: session_id.map(str::to_string),
                recovery_revision: None,
            })
            .expect("serialize event");

            assert_eq!(value["kind"], expected_kind);
            assert_eq!(
                value["sessionId"],
                session_id
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null)
            );
            assert!(value.get("session_id").is_none());
            assert!(value.get("recoveryRevision").is_none());
        }
    }

    #[test]
    fn recovery_event_payloads_carry_session_and_revision() {
        for (kind, expected) in [
            (SessionEventKind::RecoveryStarted, "recovery-started"),
            (SessionEventKind::RecoveryCompleted, "recovery-completed"),
            (
                SessionEventKind::RecoveryActionRequired,
                "recovery-action-required",
            ),
            (
                SessionEventKind::RecoveryQuarantined,
                "recovery-quarantined",
            ),
            (
                SessionEventKind::RecoveryAcknowledged,
                "recovery-acknowledged",
            ),
        ] {
            let value = serde_json::to_value(SessionStateEvent {
                kind,
                session_id: Some("session-1".to_string()),
                recovery_revision: Some(7),
            })
            .expect("serialize recovery event");
            assert_eq!(value["kind"], expected);
            assert_eq!(value["sessionId"], "session-1");
            assert_eq!(value["recoveryRevision"], 7);
        }
    }
}
