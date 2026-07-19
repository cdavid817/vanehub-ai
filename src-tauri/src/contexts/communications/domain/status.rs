use super::delivery::ConnectorErrorClass;
use super::{CommunicationsDomainError, ConnectorKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConnectorLifecycle {
    Unconfigured,
    Disabled,
    Connecting,
    Connected,
    Reconnecting,
    AuthorizationExpired,
    Error,
}

impl ConnectorLifecycle {
    fn as_str(self) -> &'static str {
        match self {
            Self::Unconfigured => "unconfigured",
            Self::Disabled => "disabled",
            Self::Connecting => "connecting",
            Self::Connected => "connected",
            Self::Reconnecting => "reconnecting",
            Self::AuthorizationExpired => "authorization-expired",
            Self::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorHealth {
    pub(crate) kind: ConnectorKind,
    pub(crate) lifecycle: ConnectorLifecycle,
    pub(crate) generation: u64,
    pub(crate) safe_error_code: Option<String>,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorStatus {
    lifecycle: ConnectorLifecycle,
    generation: u64,
    safe_error_code: Option<String>,
}

impl ConnectorStatus {
    pub(crate) fn disabled() -> Self {
        Self {
            lifecycle: ConnectorLifecycle::Disabled,
            generation: 0,
            safe_error_code: None,
        }
    }

    pub(crate) fn begin_start(&mut self) -> Result<u64, CommunicationsDomainError> {
        if !matches!(
            self.lifecycle,
            ConnectorLifecycle::Unconfigured
                | ConnectorLifecycle::Disabled
                | ConnectorLifecycle::AuthorizationExpired
                | ConnectorLifecycle::Error
        ) {
            return Err(CommunicationsDomainError::InvalidConnectorTransition {
                from: self.lifecycle.as_str(),
                to: ConnectorLifecycle::Connecting.as_str(),
            });
        }
        self.generation = self.generation.saturating_add(1);
        self.lifecycle = ConnectorLifecycle::Connecting;
        self.safe_error_code = None;
        Ok(self.generation)
    }

    pub(crate) fn mark_connected(
        &mut self,
        generation: u64,
    ) -> Result<bool, CommunicationsDomainError> {
        self.transition(
            generation,
            &[
                ConnectorLifecycle::Connecting,
                ConnectorLifecycle::Reconnecting,
            ],
            ConnectorLifecycle::Connected,
            None,
        )
    }

    pub(crate) fn mark_reconnecting(
        &mut self,
        generation: u64,
        safe_error_code: String,
    ) -> Result<bool, CommunicationsDomainError> {
        self.transition(
            generation,
            &[
                ConnectorLifecycle::Connecting,
                ConnectorLifecycle::Connected,
                ConnectorLifecycle::Reconnecting,
            ],
            ConnectorLifecycle::Reconnecting,
            Some(safe_error_code),
        )
    }

    pub(crate) fn finish(&mut self, generation: u64) -> Result<bool, CommunicationsDomainError> {
        self.transition(
            generation,
            &[
                ConnectorLifecycle::Connecting,
                ConnectorLifecycle::Connected,
                ConnectorLifecycle::Reconnecting,
            ],
            ConnectorLifecycle::Disabled,
            None,
        )
    }

    pub(crate) fn fail(
        &mut self,
        generation: u64,
        class: ConnectorErrorClass,
        safe_error_code: String,
    ) -> Result<bool, CommunicationsDomainError> {
        let lifecycle = if class == ConnectorErrorClass::AuthorizationExpired {
            ConnectorLifecycle::AuthorizationExpired
        } else {
            ConnectorLifecycle::Error
        };
        self.transition(
            generation,
            &[
                ConnectorLifecycle::Connecting,
                ConnectorLifecycle::Connected,
                ConnectorLifecycle::Reconnecting,
            ],
            lifecycle,
            Some(safe_error_code),
        )
    }

    pub(crate) fn disable(&mut self) {
        self.lifecycle = ConnectorLifecycle::Disabled;
        self.safe_error_code = None;
    }

    pub(crate) fn shutdown_timeout(&mut self) {
        self.lifecycle = ConnectorLifecycle::Error;
        self.safe_error_code = Some("shutdown-timeout".to_string());
    }

    pub(crate) fn is_generation(&self, generation: u64) -> bool {
        self.generation == generation
    }

    pub(crate) fn health(&self, kind: ConnectorKind, updated_at: String) -> ConnectorHealth {
        ConnectorHealth {
            kind,
            lifecycle: self.lifecycle,
            generation: self.generation,
            safe_error_code: self.safe_error_code.clone(),
            updated_at,
        }
    }

    fn transition(
        &mut self,
        generation: u64,
        allowed: &[ConnectorLifecycle],
        next: ConnectorLifecycle,
        safe_error_code: Option<String>,
    ) -> Result<bool, CommunicationsDomainError> {
        if self.generation != generation {
            return Ok(false);
        }
        if !allowed.contains(&self.lifecycle) {
            return Err(CommunicationsDomainError::InvalidConnectorTransition {
                from: self.lifecycle.as_str(),
                to: next.as_str(),
            });
        }
        self.lifecycle = next;
        self.safe_error_code = safe_error_code;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_status_controls_generation_and_reconnect_transitions() {
        let mut status = ConnectorStatus::disabled();
        let generation = status.begin_start().expect("start");
        assert_eq!(generation, 1);
        assert!(status.mark_connected(generation).expect("connected"));
        assert!(status
            .mark_reconnecting(generation, "telegram-http-503".to_string())
            .expect("reconnecting"));
        assert!(status.mark_connected(generation).expect("reconnected"));
        assert!(status.finish(generation).expect("finished"));
        let health = status.health(ConnectorKind::Telegram, "now".to_string());
        assert_eq!(health.lifecycle, ConnectorLifecycle::Disabled);
        assert_eq!(health.generation, 1);
        assert_eq!(health.safe_error_code, None);
    }

    #[test]
    fn stale_workers_cannot_replace_current_status() {
        let mut status = ConnectorStatus::disabled();
        let stale = status.begin_start().expect("first start");
        status.disable();
        let current = status.begin_start().expect("second start");
        assert!(!status.mark_connected(stale).expect("stale ignored"));
        assert!(status.mark_connected(current).expect("current connected"));
    }

    #[test]
    fn failures_have_dedicated_authorization_and_shutdown_states() {
        let mut status = ConnectorStatus::disabled();
        let generation = status.begin_start().expect("start");
        status
            .fail(
                generation,
                ConnectorErrorClass::AuthorizationExpired,
                "wechat-authorization-expired".to_string(),
            )
            .expect("expired");
        assert_eq!(
            status
                .health(ConnectorKind::WeChat, "now".to_string())
                .lifecycle,
            ConnectorLifecycle::AuthorizationExpired
        );

        status.shutdown_timeout();
        let health = status.health(ConnectorKind::WeChat, "later".to_string());
        assert_eq!(health.lifecycle, ConnectorLifecycle::Error);
        assert_eq!(health.safe_error_code.as_deref(), Some("shutdown-timeout"));
    }

    #[test]
    fn invalid_lifecycle_changes_return_typed_errors() {
        let mut status = ConnectorStatus::disabled();
        assert_eq!(
            status.mark_connected(0),
            Err(CommunicationsDomainError::InvalidConnectorTransition {
                from: "disabled",
                to: "connected",
            })
        );
        status.begin_start().expect("start");
        assert_eq!(
            status.begin_start(),
            Err(CommunicationsDomainError::InvalidConnectorTransition {
                from: "connecting",
                to: "connecting",
            })
        );
    }
}
