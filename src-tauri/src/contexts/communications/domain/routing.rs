use super::{CommunicationsDomainError, ConnectorKind};
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChatBinding {
    key: ChatBindingKey,
    session_id: String,
}

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
