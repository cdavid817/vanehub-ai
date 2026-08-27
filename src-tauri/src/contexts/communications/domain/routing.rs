use super::{CommunicationsDomainError, ConnectorKind};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum BindingState {
    Active,
    Paused,
}

impl BindingState {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "paused" => Some(Self::Paused),
            _ => None,
        }
    }
}

fn required(
    value: impl Into<String>,
    kind: &'static str,
) -> Result<String, CommunicationsDomainError> {
    let value = value.into();
    let normalized = value.trim();
    if normalized.is_empty() {
        return Err(CommunicationsDomainError::RequiredValue(kind));
    }
    if normalized.chars().any(char::is_control) {
        return Err(CommunicationsDomainError::ControlCharacters(kind));
    }
    Ok(normalized.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RoutingSettings {
    pub(crate) agent_id: String,
    pub(crate) project_path: String,
}

impl RoutingSettings {
    pub(crate) fn new(
        agent_id: impl Into<String>,
        project_path: impl Into<String>,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            agent_id: required(agent_id, "Routing agent id")?,
            project_path: required(project_path, "Routing project path")?,
        })
    }

    pub(crate) fn normalized(&self) -> Result<Self, CommunicationsDomainError> {
        Self::new(self.agent_id.clone(), self.project_path.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatBindingKey {
    connector: ConnectorKind,
    external_chat_id: String,
}

impl ChatBindingKey {
    pub(crate) fn new(
        connector: ConnectorKind,
        external_chat_id: impl Into<String>,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            connector,
            external_chat_id: required(external_chat_id, "External chat id")?,
        })
    }

    pub(crate) fn connector(&self) -> ConnectorKind {
        self.connector
    }

    pub(crate) fn external_chat_id(&self) -> &str {
        &self.external_chat_id
    }
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatBinding {
    key: ChatBindingKey,
    session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionBinding {
    pub(crate) connector: ConnectorKind,
    pub(crate) session_id: String,
    pub(crate) state: BindingState,
    pub(crate) completion_notifications: bool,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionConnectorAccess {
    pub(crate) session_id: String,
    pub(crate) connector: ConnectorKind,
    pub(crate) enabled: bool,
    pub(crate) updated_at: String,
}

impl SessionConnectorAccess {
    pub(crate) fn disabled(session_id: impl Into<String>, connector: ConnectorKind) -> Self {
        Self {
            session_id: session_id.into(),
            connector,
            enabled: false,
            updated_at: "1970-01-01T00:00:00Z".to_string(),
        }
    }
}

impl SessionBinding {
    pub(crate) fn is_active(&self) -> bool {
        self.state == BindingState::Active
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PairingIntent {
    pub(crate) id: String,
    pub(crate) connector: ConnectorKind,
    pub(crate) session_id: String,
    pub(crate) code_hash: String,
    pub(crate) salt: String,
    pub(crate) expires_at: String,
    pub(crate) created_at: String,
    pub(crate) replace_existing: bool,
}

impl PairingIntent {
    pub(crate) fn new(
        id: impl Into<String>,
        connector: ConnectorKind,
        session_id: impl Into<String>,
        digest: (impl Into<String>, impl Into<String>),
        window: (impl Into<String>, impl Into<String>),
        replace_existing: bool,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            id: required(id, "Pairing intent id")?,
            connector,
            session_id: required(session_id, "Pairing session id")?,
            code_hash: required(digest.0, "Pairing code hash")?,
            salt: required(digest.1, "Pairing salt")?,
            expires_at: required(window.0, "Pairing expiry")?,
            created_at: required(window.1, "Pairing creation time")?,
            replace_existing,
        })
    }
}

#[cfg(test)]
impl ChatBinding {
    pub(crate) fn new(
        key: ChatBindingKey,
        session_id: impl Into<String>,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            key,
            session_id: required(session_id, "Bound session id")?,
        })
    }

    pub(crate) fn key(&self) -> &ChatBindingKey {
        &self.key
    }

    pub(crate) fn session_id(&self) -> &str {
        &self.session_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InboundEventIdentity {
    connector: ConnectorKind,
    event_id: String,
}

impl InboundEventIdentity {
    pub(crate) fn new(
        connector: ConnectorKind,
        event_id: impl Into<String>,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            connector,
            event_id: required(event_id, "Inbound event id")?,
        })
    }

    pub(crate) fn connector(&self) -> ConnectorKind {
        self.connector
    }

    pub(crate) fn event_id(&self) -> &str {
        &self.event_id
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckpointKey {
    connector: ConnectorKind,
    name: String,
}

impl CheckpointKey {
    pub(crate) fn new(
        connector: ConnectorKind,
        name: impl Into<String>,
    ) -> Result<Self, CommunicationsDomainError> {
        Ok(Self {
            connector,
            name: required(name, "Checkpoint key")?,
        })
    }

    pub(crate) fn connector(&self) -> ConnectorKind {
        self.connector
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ConnectorCheckpoint {
    key: CheckpointKey,
    value: String,
}

impl ConnectorCheckpoint {
    pub(crate) fn new(key: CheckpointKey, value: impl Into<String>) -> Self {
        Self {
            key,
            value: value.into(),
        }
    }

    pub(crate) fn key(&self) -> &CheckpointKey {
        &self.key
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn routing_is_trimmed_and_requires_both_targets() {
        assert_eq!(
            RoutingSettings::new(" codex-cli ", " C:/repo ").expect("routing"),
            RoutingSettings {
                agent_id: "codex-cli".to_string(),
                project_path: "C:/repo".to_string(),
            }
        );
        assert_eq!(
            RoutingSettings::new(" ", "C:/repo"),
            Err(CommunicationsDomainError::RequiredValue("Routing agent id"))
        );
    }

    #[test]
    fn bindings_are_scoped_by_connector_and_require_stable_ids() {
        let key = ChatBindingKey::new(ConnectorKind::WeCom, " chat-1 ").expect("key");
        let binding = ChatBinding::new(key, " session-1 ").expect("binding");
        assert_eq!(binding.key().connector(), ConnectorKind::WeCom);
        assert_eq!(binding.key().external_chat_id(), "chat-1");
        assert_eq!(binding.session_id(), "session-1");
        assert!(ChatBindingKey::new(ConnectorKind::WeCom, "\n").is_err());
    }

    #[test]
    fn pairing_intents_require_safe_non_empty_metadata() {
        let intent = PairingIntent::new(
            "pair-1",
            ConnectorKind::Telegram,
            "session-1",
            ("hash", "salt"),
            ("2026-08-12T01:05:00Z", "2026-08-12T01:00:00Z"),
            false,
        )
        .expect("intent");
        assert_eq!(intent.connector, ConnectorKind::Telegram);
        assert!(PairingIntent::new(
            " ",
            ConnectorKind::Telegram,
            "session-1",
            ("hash", "salt"),
            ("2026-08-12T01:05:00Z", "2026-08-12T01:00:00Z"),
            false,
        )
        .is_err());
    }

    #[test]
    fn deduplication_and_checkpoint_keys_are_connector_scoped() {
        let event = InboundEventIdentity::new(ConnectorKind::Feishu, " event-1 ").expect("event");
        assert_eq!(event.connector(), ConnectorKind::Feishu);
        assert_eq!(event.event_id(), "event-1");

        let key = CheckpointKey::new(ConnectorKind::Telegram, " offset ").expect("checkpoint");
        let checkpoint = ConnectorCheckpoint::new(key, "");
        assert_eq!(checkpoint.key().connector(), ConnectorKind::Telegram);
        assert_eq!(checkpoint.key().name(), "offset");
        assert_eq!(checkpoint.value(), "");
        assert!(CheckpointKey::new(ConnectorKind::Telegram, " ").is_err());
    }
}
