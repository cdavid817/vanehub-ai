use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorKind {
    Feishu,
    Telegram,
    DingTalk,
    WeCom,
    #[serde(rename = "weixin", alias = "wechat")]
    WeChat,
}

impl ConnectorKind {
    pub const ALL: [Self; 5] = [
        Self::Feishu,
        Self::Telegram,
        Self::DingTalk,
        Self::WeCom,
        Self::WeChat,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Feishu => "feishu",
            Self::Telegram => "telegram",
            Self::DingTalk => "dingtalk",
            Self::WeCom => "wecom",
            Self::WeChat => "weixin",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
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
pub struct ConnectorDescriptor {
    pub kind: ConnectorKind,
    pub supports_qr_authorization: bool,
    pub experimental: bool,
    pub max_outbound_chars: usize,
}

pub fn builtin_descriptors() -> Vec<ConnectorDescriptor> {
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
pub struct ConnectorConfig {
    pub kind: ConnectorKind,
    pub enabled: bool,
    pub display_name: Option<String>,
    pub public_config: serde_json::Value,
    pub credential_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RoutingSettings {
    pub agent_id: String,
    pub project_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NormalizedInbound {
    pub connector: ConnectorKind,
    pub event_id: String,
    pub chat_id: String,
    pub sender_id: String,
    pub text: String,
    pub direct: bool,
    pub reply_context: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ConnectorLifecycle {
    Unconfigured,
    Disabled,
    Connecting,
    Connected,
    Reconnecting,
    AuthorizationExpired,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectorHealth {
    pub kind: ConnectorKind,
    pub lifecycle: ConnectorLifecycle,
    pub generation: u64,
    pub safe_error_code: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutboundText {
    pub chat_id: String,
    pub text: String,
    pub reply_context: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_personal_wechat_with_stable_weixin_id() {
        assert_eq!(serde_json::to_string(&ConnectorKind::WeChat).unwrap(), "\"weixin\"");
        assert_eq!(
            serde_json::from_str::<ConnectorKind>("\"wechat\"").unwrap(),
            ConnectorKind::WeChat
        );
        assert_eq!(ConnectorKind::WeChat.as_str(), "weixin");
    }
}
