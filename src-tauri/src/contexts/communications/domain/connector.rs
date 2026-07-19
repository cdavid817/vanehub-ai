use super::CommunicationsDomainError;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub(crate) enum ConnectorKind {
    Feishu,
    Telegram,
    DingTalk,
    WeCom,
    #[serde(rename = "weixin", alias = "wechat")]
    WeChat,
}

impl ConnectorKind {
    pub(crate) const ALL: [Self; 5] = [
        Self::Feishu,
        Self::Telegram,
        Self::DingTalk,
        Self::WeCom,
        Self::WeChat,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Feishu => "feishu",
            Self::Telegram => "telegram",
            Self::DingTalk => "dingtalk",
            Self::WeCom => "wecom",
            Self::WeChat => "weixin",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "feishu" => Some(Self::Feishu),
            "telegram" => Some(Self::Telegram),
            "dingtalk" => Some(Self::DingTalk),
            "wecom" => Some(Self::WeCom),
            "weixin" | "wechat" => Some(Self::WeChat),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorDescriptor {
    pub(crate) kind: ConnectorKind,
    pub(crate) supports_qr_authorization: bool,
    pub(crate) experimental: bool,
    pub(crate) max_outbound_chars: usize,
}

pub(crate) fn builtin_descriptors() -> Vec<ConnectorDescriptor> {
    ConnectorKind::ALL
        .into_iter()
        .map(|kind| ConnectorDescriptor {
            kind,
            supports_qr_authorization: kind == ConnectorKind::WeChat,
            experimental: kind == ConnectorKind::WeChat,
            max_outbound_chars: match kind {
                ConnectorKind::Telegram => 4_096,
                ConnectorKind::Feishu => 20_000,
                ConnectorKind::DingTalk | ConnectorKind::WeCom | ConnectorKind::WeChat => 2_000,
            },
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConnectorConfig {
    pub(crate) kind: ConnectorKind,
    pub(crate) enabled: bool,
    pub(crate) display_name: Option<String>,
    pub(crate) public_config: Value,
    pub(crate) credential_ref: Option<String>,
}

impl ConnectorConfig {
    pub(crate) fn validate(&self) -> Result<(), CommunicationsDomainError> {
        reject_sensitive_public_config(&self.public_config)
    }
}

fn reject_sensitive_public_config(value: &Value) -> Result<(), CommunicationsDomainError> {
    match value {
        Value::Object(values) => {
            for (key, value) in values {
                let normalized = key.to_ascii_lowercase();
                if normalized.contains("secret")
                    || normalized.contains("token")
                    || normalized.contains("password")
                    || normalized.contains("authorization")
                {
                    return Err(CommunicationsDomainError::SensitivePublicConfigField(
                        key.clone(),
                    ));
                }
                reject_sensitive_public_config(value)?;
            }
        }
        Value::Array(values) => {
            for value in values {
                reject_sensitive_public_config(value)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn connector_identity_keeps_the_stable_weixin_id_and_legacy_alias() {
        assert_eq!(ConnectorKind::parse("weixin"), Some(ConnectorKind::WeChat));
        assert_eq!(ConnectorKind::parse("wechat"), Some(ConnectorKind::WeChat));
        assert_eq!(ConnectorKind::WeChat.as_str(), "weixin");
        assert_eq!(
            serde_json::to_string(&ConnectorKind::WeChat).expect("serialize"),
            "\"weixin\""
        );
        assert_eq!(
            serde_json::from_str::<ConnectorKind>("\"wechat\"").expect("legacy alias"),
            ConnectorKind::WeChat
        );
    }

    #[test]
    fn builtins_define_stable_capabilities_and_delivery_limits() {
        let descriptors = builtin_descriptors();
        assert_eq!(descriptors.len(), ConnectorKind::ALL.len());
        let telegram = descriptors
            .iter()
            .find(|descriptor| descriptor.kind == ConnectorKind::Telegram)
            .expect("telegram");
        assert_eq!(telegram.max_outbound_chars, 4_096);
        let wechat = descriptors
            .iter()
            .find(|descriptor| descriptor.kind == ConnectorKind::WeChat)
            .expect("wechat");
        assert!(wechat.supports_qr_authorization);
        assert!(wechat.experimental);
    }

    #[test]
    fn public_configuration_rejects_sensitive_fields_at_any_depth() {
        let valid = ConnectorConfig {
            kind: ConnectorKind::Telegram,
            enabled: false,
            display_name: None,
            public_config: json!({"apiBase": "https://api.telegram.org"}),
            credential_ref: None,
        };
        assert!(valid.validate().is_ok());

        let invalid = ConnectorConfig {
            public_config: json!({"nested": [{"appSecret": "private"}]}),
            ..valid
        };
        assert_eq!(
            invalid.validate(),
            Err(CommunicationsDomainError::SensitivePublicConfigField(
                "appSecret".to_string()
            ))
        );
    }
}
