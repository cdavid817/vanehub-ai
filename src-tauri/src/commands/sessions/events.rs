use serde::Serialize;
use tauri::{AppHandle, Emitter};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionStateEvent {
    kind: String,
    session_id: Option<String>,
}

pub(super) fn emit_active_session_changed(app: &AppHandle, session_id: Option<&str>) {
    emit(app, "active-session-changed", session_id);
}

pub(super) fn emit_configuration_changed(app: &AppHandle, session_id: &str) {
    emit(app, "configuration-changed", Some(session_id));
}

fn emit(app: &AppHandle, kind: &str, session_id: Option<&str>) {
    let _ = app.emit(
        "session:event",
        SessionStateEvent {
            kind: kind.to_string(),
            session_id: session_id.map(str::to_string),
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_event_payloads_preserve_names_optional_identity_and_transport_shape() {
        for (kind, session_id) in [
            ("active-session-changed", Some("session-1")),
            ("active-session-changed", None),
            ("configuration-changed", Some("session-1")),
        ] {
            let value = serde_json::to_value(SessionStateEvent {
                kind: kind.to_string(),
                session_id: session_id.map(str::to_string),
            })
            .expect("serialize event");

            assert_eq!(value["kind"], kind);
            assert_eq!(
                value["sessionId"],
                session_id
                    .map(serde_json::Value::from)
                    .unwrap_or(serde_json::Value::Null)
            );
            assert!(value.get("session_id").is_none());
        }
    }
}
