use super::ConnectorKind;
use serde::{Deserialize, Serialize};

pub(crate) const MAX_PENDING_PER_CHAT: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NormalizedInbound {
    pub(crate) connector: ConnectorKind,
    pub(crate) event_id: String,
    pub(crate) chat_id: String,
    pub(crate) sender_id: String,
    pub(crate) text: String,
    pub(crate) direct: bool,
    pub(crate) reply_context: Option<String>,
}

impl NormalizedInbound {
    pub(crate) fn disposition(&self) -> InboundDisposition {
        if !self.direct {
            InboundDisposition::IgnoreGroupMessage
        } else if self.text.trim().is_empty() {
            InboundDisposition::IgnoreUnsupportedContent
        } else {
            InboundDisposition::Deliver
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InboundDisposition {
    Deliver,
    IgnoreGroupMessage,
    IgnoreUnsupportedContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OutboundText {
    pub(crate) chat_id: String,
    pub(crate) text: String,
    pub(crate) reply_context: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeduplicationDecision {
    Process,
    IgnoreDuplicate,
}

impl DeduplicationDecision {
    pub(crate) fn from_claimed(claimed: bool) -> Self {
        if claimed {
            Self::Process
        } else {
            Self::IgnoreDuplicate
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DeliveryAdmission {
    Admitted,
    Busy,
}

pub(crate) fn pending_delivery_admission(current_pending: usize) -> DeliveryAdmission {
    if current_pending < MAX_PENDING_PER_CHAT {
        DeliveryAdmission::Admitted
    } else {
        DeliveryAdmission::Busy
    }
}

pub(crate) fn split_text(text: &str, max_chars: usize) -> Vec<String> {
    let max_chars = max_chars.max(1);
    let chars = text.chars().collect::<Vec<_>>();
    chars
        .chunks(max_chars)
        .map(|chunk| chunk.iter().collect::<String>())
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConnectorErrorClass {
    Transient,
    Authentication,
    AuthorizationExpired,
    Permanent,
}

impl ConnectorErrorClass {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Transient => "transient",
            Self::Authentication => "authentication",
            Self::AuthorizationExpired => "authorization-expired",
            Self::Permanent => "permanent",
        }
    }
}

pub(crate) fn classify_safe_code(code: &str) -> ConnectorErrorClass {
    if code.contains("authorization-expired") {
        ConnectorErrorClass::AuthorizationExpired
    } else if code.contains("credentials-invalid")
        || code.contains("token-invalid")
        || code.contains("token-missing")
        || code.contains("api-401")
        || code.contains("http-401")
        || code.contains("http-403")
        || code.contains("authentication-failed")
        || code.starts_with("feishu-api-")
        || code.starts_with("feishu-ws-config-api-")
        || code == "dingtalk-stream-config-http-400"
        || (code.starts_with("wecom-auth-")
            && !matches!(
                code,
                "wecom-auth-timeout" | "wecom-auth-closed" | "wecom-auth-frame-failed"
            ))
    {
        ConnectorErrorClass::Authentication
    } else if code.contains("webhook-conflict")
        || code.contains("polling-conflict")
        || code.contains("payload-invalid")
        || code.contains("field-missing")
    {
        ConnectorErrorClass::Permanent
    } else {
        ConnectorErrorClass::Transient
    }
}

pub(crate) fn safe_platform_status_code(safe_code: &str) -> Option<String> {
    safe_code
        .rsplit('-')
        .next()
        .filter(|value| {
            !value.is_empty() && value.chars().all(|character| character.is_ascii_digit())
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inbound(direct: bool, text: &str) -> NormalizedInbound {
        NormalizedInbound {
            connector: ConnectorKind::Telegram,
            event_id: "event-1".to_string(),
            chat_id: "chat-1".to_string(),
            sender_id: "sender-1".to_string(),
            text: text.to_string(),
            direct,
            reply_context: None,
        }
    }

    #[test]
    fn inbound_policy_only_delivers_nonempty_direct_text() {
        assert_eq!(
            inbound(true, "status").disposition(),
            InboundDisposition::Deliver
        );
        assert_eq!(
            inbound(false, "status").disposition(),
            InboundDisposition::IgnoreGroupMessage
        );
        assert_eq!(
            inbound(true, " \n ").disposition(),
            InboundDisposition::IgnoreUnsupportedContent
        );
    }

    #[test]
    fn delivery_capacity_and_deduplication_have_explicit_decisions() {
        assert_eq!(pending_delivery_admission(7), DeliveryAdmission::Admitted);
        assert_eq!(pending_delivery_admission(8), DeliveryAdmission::Busy);
        assert_eq!(
            DeduplicationDecision::from_claimed(true),
            DeduplicationDecision::Process
        );
        assert_eq!(
            DeduplicationDecision::from_claimed(false),
            DeduplicationDecision::IgnoreDuplicate
        );
    }

    #[test]
    fn chunks_preserve_unicode_scalar_boundaries_and_order() {
        assert_eq!(split_text("ab你cd", 2), vec!["ab", "你c", "d"]);
        assert_eq!(split_text("ab", 0), vec!["a", "b"]);
    }

    #[test]
    fn error_policy_separates_retryable_and_terminal_failures() {
        assert_eq!(
            classify_safe_code("telegram-api-401"),
            ConnectorErrorClass::Authentication
        );
        assert_eq!(
            classify_safe_code("wechat-authorization-expired"),
            ConnectorErrorClass::AuthorizationExpired
        );
        assert_eq!(
            classify_safe_code("telegram-http-503"),
            ConnectorErrorClass::Transient
        );
        assert_eq!(
            safe_platform_status_code("telegram-api-429").as_deref(),
            Some("429")
        );
        assert_eq!(safe_platform_status_code("token-invalid"), None);
    }
}
