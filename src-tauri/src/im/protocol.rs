use super::models::{ConnectorKind, NormalizedInbound};
use crate::AppError;
use serde_json::Value;

pub fn normalize_fixture(
    kind: ConnectorKind,
    payload: &str,
) -> Result<NormalizedInbound, AppError> {
    let value: Value = serde_json::from_str(payload)
        .map_err(|error| AppError::Validation(format!("invalid connector payload: {error}")))?;
    match kind {
        ConnectorKind::Feishu => normalize_feishu(&value),
        ConnectorKind::Telegram => normalize_telegram(&value),
        ConnectorKind::DingTalk => normalize_dingtalk(&value),
        ConnectorKind::WeCom => normalize_wecom(&value),
        ConnectorKind::WeChat => normalize_wechat(&value),
    }
}

fn string_at(value: &Value, pointer: &str) -> Result<String, AppError> {
    value
        .pointer(pointer)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| AppError::Validation(format!("missing connector field: {pointer}")))
}

fn normalize_feishu(value: &Value) -> Result<NormalizedInbound, AppError> {
    let content = string_at(value, "/event/message/content")?;
    let content: Value = serde_json::from_str(&content).map_err(|error| {
        AppError::Validation(format!("invalid Feishu message content: {error}"))
    })?;
    Ok(NormalizedInbound {
        connector: ConnectorKind::Feishu,
        event_id: string_at(value, "/header/event_id")?,
        chat_id: string_at(value, "/event/message/chat_id")?,
        sender_id: string_at(value, "/event/sender/sender_id/open_id")?,
        text: string_at(&content, "/text")?,
        direct: string_at(value, "/event/message/chat_type")? == "p2p",
        reply_context: value
            .pointer("/event/message/message_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn normalize_telegram(value: &Value) -> Result<NormalizedInbound, AppError> {
    let chat_type = string_at(value, "/message/chat/type")?;
    Ok(NormalizedInbound {
        connector: ConnectorKind::Telegram,
        event_id: value
            .get("update_id")
            .and_then(Value::as_i64)
            .map(|id| id.to_string())
            .ok_or_else(|| AppError::Validation("missing connector field: /update_id".into()))?,
        chat_id: value
            .pointer("/message/chat/id")
            .and_then(Value::as_i64)
            .map(|id| id.to_string())
            .ok_or_else(|| {
                AppError::Validation("missing connector field: /message/chat/id".into())
            })?,
        sender_id: value
            .pointer("/message/from/id")
            .and_then(Value::as_i64)
            .map(|id| id.to_string())
            .ok_or_else(|| {
                AppError::Validation("missing connector field: /message/from/id".into())
            })?,
        text: string_at(value, "/message/text")?,
        direct: chat_type == "private",
        reply_context: value
            .pointer("/message/message_id")
            .and_then(Value::as_i64)
            .map(|id| id.to_string()),
    })
}

fn normalize_dingtalk(value: &Value) -> Result<NormalizedInbound, AppError> {
    let direct = string_at(value, "/data/conversationType")? == "1";
    let sender_id = string_at(value, "/data/senderStaffId")?;
    Ok(NormalizedInbound {
        connector: ConnectorKind::DingTalk,
        event_id: string_at(value, "/headers/messageId")?,
        chat_id: if direct { sender_id.clone() } else { string_at(value, "/data/openConversationId")? },
        sender_id,
        text: string_at(value, "/data/text/content")?,
        direct,
        reply_context: value
            .pointer("/data/sessionWebhook")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn normalize_wecom(value: &Value) -> Result<NormalizedInbound, AppError> {
    Ok(NormalizedInbound {
        connector: ConnectorKind::WeCom,
        event_id: string_at(value, "/headers/req_id")?,
        chat_id: string_at(value, "/body/chatid")?,
        sender_id: string_at(value, "/body/from/userid")?,
        text: string_at(value, "/body/text/content")?,
        direct: string_at(value, "/body/chattype")? == "single",
        reply_context: value
            .pointer("/headers/req_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

fn normalize_wechat(value: &Value) -> Result<NormalizedInbound, AppError> {
    Ok(NormalizedInbound {
        connector: ConnectorKind::WeChat,
        event_id: string_at(value, "/message_id")?,
        chat_id: string_at(value, "/conversation_id")?,
        sender_id: string_at(value, "/sender_id")?,
        text: string_at(value, "/text")?,
        direct: value.get("conversation_type").and_then(Value::as_str) == Some("direct"),
        reply_context: value
            .get("context_token")
            .and_then(Value::as_str)
            .map(str::to_owned),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURES: [(ConnectorKind, &str); 5] = [
        (
            ConnectorKind::Feishu,
            include_str!("fixtures/feishu-direct-text.json"),
        ),
        (
            ConnectorKind::Telegram,
            include_str!("fixtures/telegram-direct-text.json"),
        ),
        (
            ConnectorKind::DingTalk,
            include_str!("fixtures/dingtalk-direct-text.json"),
        ),
        (
            ConnectorKind::WeCom,
            include_str!("fixtures/wecom-direct-text.json"),
        ),
        (
            ConnectorKind::WeChat,
            include_str!("fixtures/wechat-direct-text.json"),
        ),
    ];

    #[test]
    fn normalizes_all_recorded_direct_text_fixtures() {
        for (kind, payload) in FIXTURES {
            let inbound = normalize_fixture(kind, payload).unwrap();
            assert_eq!(inbound.connector, kind);
            assert!(inbound.direct);
            assert_eq!(inbound.text, "status please");
            assert!(!inbound.event_id.is_empty());
            assert!(!payload.to_ascii_lowercase().contains("secret"));
            assert!(!payload.to_ascii_lowercase().contains("bearer "));
            assert!(!payload.to_ascii_lowercase().contains("bot_token"));
            assert!(!payload.to_ascii_lowercase().contains("app_secret"));
        }
    }
}
