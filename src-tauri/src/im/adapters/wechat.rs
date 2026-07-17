use super::http::{require_success, HttpMethod, HttpRequest, HttpTransport};
use crate::im::models::{ConnectorKind, NormalizedInbound, OutboundText};
use crate::im::runtime::{submit_inbound, ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
use async_trait::async_trait;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroizing;

const ILINK_BASE_URL: &str = "https://ilinkai.weixin.qq.com";
const SESSION_EXPIRED: i64 = -14;

pub trait WeChatSessionStore: Send + Sync {
    fn load_cursor(&self) -> Result<String, ConnectorRuntimeError>;
    fn save_cursor(&self, cursor: &str) -> Result<(), ConnectorRuntimeError>;
    fn load_context(&self, chat_id: &str) -> Result<Option<String>, ConnectorRuntimeError>;
    fn save_context(&self, chat_id: &str, context: &str) -> Result<(), ConnectorRuntimeError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeChatQrCode {
    pub qr_url: String,
    pub qr_payload: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case", tag = "status", content = "value")]
pub enum WeChatQrStatus {
    Waiting,
    Scanned,
    Confirmed(WeChatCredentials),
    Expired,
    Error(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct WeChatCredentials {
    pub bot_token: String,
    pub base_url: String,
    pub bot_id: String,
}

pub struct WeChatAdapter {
    bot_token: Zeroizing<String>,
    base_url: String,
    transport: Arc<dyn HttpTransport>,
    session: Arc<dyn WeChatSessionStore>,
}

impl WeChatAdapter {
    pub fn new(
        bot_token: &str,
        transport: Arc<dyn HttpTransport>,
        session: Arc<dyn WeChatSessionStore>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let bot_token = bot_token.trim();
        if bot_token.is_empty() {
            return Err(ConnectorRuntimeError::new("wechat-token-invalid"));
        }
        Self::new_with_base_url(bot_token, ILINK_BASE_URL, transport, session)
    }

    pub fn new_with_base_url(
        bot_token: &str,
        base_url: &str,
        transport: Arc<dyn HttpTransport>,
        session: Arc<dyn WeChatSessionStore>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let bot_token = bot_token.trim();
        let base_url = base_url.trim().trim_end_matches('/');
        if bot_token.is_empty() || !base_url.starts_with("https://") {
            return Err(ConnectorRuntimeError::new("wechat-credentials-invalid"));
        }
        Ok(Self {
            bot_token: Zeroizing::new(bot_token.to_string()),
            base_url: base_url.to_string(),
            transport,
            session,
        })
    }

    fn headers(&self) -> BTreeMap<String, String> {
        let uin = base64::engine::general_purpose::STANDARD
            .encode(rand::random::<u32>().to_string().as_bytes());
        BTreeMap::from([
            (
                "authorization".to_string(),
                format!("Bearer {}", self.bot_token.as_str()),
            ),
            (
                "authorizationtype".to_string(),
                "ilink_bot_token".to_string(),
            ),
            ("x-wechat-uin".to_string(), uin),
            ("ilink-app-id".to_string(), "bot".to_string()),
            ("ilink-app-clientversion".to_string(), "256".to_string()),
        ])
    }

    async fn post(&self, path: &str, body: Value) -> Result<Value, ConnectorRuntimeError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{}{}", self.base_url, path),
                headers: self.headers(),
                body: Some(body),
            })
            .await?;
        require_success(&response)?;
        Ok(response.body)
    }

    async fn poll_once(
        &self,
    ) -> Result<(Vec<NormalizedInbound>, Option<String>), ConnectorRuntimeError> {
        let cursor = self.session.load_cursor()?;
        let body = self
            .post(
                "/ilink/bot/getupdates",
                json!({
                    "get_updates_buf": cursor,
                    "base_info": {"channel_version": "0.1.0", "bot_agent": "VaneHub/0.1.0"}
                }),
            )
            .await?;
        let error_code = body
            .get("errcode")
            .or_else(|| body.get("ret"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if error_code == SESSION_EXPIRED {
            return Err(ConnectorRuntimeError::new("wechat-authorization-expired"));
        }
        if error_code != 0 {
            return Err(ConnectorRuntimeError::new(format!(
                "wechat-api-{error_code}"
            )));
        }
        let next_cursor = body
            .get("get_updates_buf")
            .and_then(Value::as_str)
            .map(str::to_owned);
        let mut result = Vec::new();
        for (index, message) in body
            .get("msgs")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            let Some(sender_id) = message.get("from_user_id").and_then(Value::as_str) else {
                continue;
            };
            let Some(context) = message.get("context_token").and_then(Value::as_str) else {
                continue;
            };
            let Some(item) = message
                .get("item_list")
                .and_then(Value::as_array)
                .and_then(|items| items.first())
            else {
                continue;
            };
            if item.get("type").and_then(Value::as_i64).unwrap_or(1) != 1 {
                continue;
            }
            let Some(text) = item.pointer("/text_item/text").and_then(Value::as_str) else {
                continue;
            };
            self.session.save_context(sender_id, context)?;
            let event_id = message
                .get("message_id")
                .and_then(Value::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| fallback_event_id(sender_id, context, text, index));
            result.push(NormalizedInbound {
                connector: ConnectorKind::WeChat,
                event_id,
                chat_id: sender_id.to_string(),
                sender_id: sender_id.to_string(),
                text: text.to_string(),
                direct: true,
                reply_context: Some(context.to_string()),
            });
        }
        Ok((result, next_cursor))
    }

    pub async fn create_qr_code(
        transport: &dyn HttpTransport,
    ) -> Result<WeChatQrCode, ConnectorRuntimeError> {
        let response = transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{ILINK_BASE_URL}/ilink/bot/get_bot_qrcode?bot_type=3"),
                headers: common_headers(),
                body: Some(json!({"local_token_list": []})),
            })
            .await?;
        require_success(&response)?;
        let code = response
            .body
            .get("errcode")
            .or_else(|| response.body.get("ret"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if code != 0 {
            return Err(ConnectorRuntimeError::new(format!("wechat-qr-{code}")));
        }
        let qr_url = response
            .body
            .get("qrcode_img_content")
            .or_else(|| response.body.get("qrcode_url"))
            .and_then(Value::as_str)
            .ok_or_else(|| ConnectorRuntimeError::new("wechat-qr-url-missing"))?;
        let qr_payload = response
            .body
            .get("qrcode")
            .and_then(Value::as_str)
            .ok_or_else(|| ConnectorRuntimeError::new("wechat-qr-payload-missing"))?;
        Ok(WeChatQrCode {
            qr_url: qr_url.to_string(),
            qr_payload: qr_payload.to_string(),
        })
    }

    pub async fn poll_qr_status(
        transport: &dyn HttpTransport,
        qr_payload: &str,
    ) -> Result<WeChatQrStatus, ConnectorRuntimeError> {
        let encoded =
            url::form_urlencoded::byte_serialize(qr_payload.as_bytes()).collect::<String>();
        let response = transport
            .execute(HttpRequest {
                method: HttpMethod::Get,
                url: format!("{ILINK_BASE_URL}/ilink/bot/get_qrcode_status?qrcode={encoded}"),
                headers: common_headers(),
                body: None,
            })
            .await?;
        require_success(&response)?;
        let status = response.body.get("status");
        Ok(match status {
            Some(Value::Number(value)) if value.as_i64() == Some(0) => WeChatQrStatus::Waiting,
            Some(Value::Number(value)) if value.as_i64() == Some(1) => WeChatQrStatus::Scanned,
            Some(Value::Number(value)) if value.as_i64() == Some(2) => confirmed_credentials(&response.body),
            Some(Value::Number(value)) if value.as_i64() == Some(3) => WeChatQrStatus::Expired,
            Some(Value::String(value)) if value == "wait" => WeChatQrStatus::Waiting,
            Some(Value::String(value)) if value == "scanned" || value == "scaned" => WeChatQrStatus::Scanned,
            Some(Value::String(value)) if value == "confirmed" => confirmed_credentials(&response.body),
            Some(Value::String(value)) if value == "expired" => WeChatQrStatus::Expired,
            _ => WeChatQrStatus::Error("status-invalid".to_string()),
        })
    }
}

#[async_trait]
impl ConnectorAdapter for WeChatAdapter {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::WeChat
    }

    fn max_outbound_chars(&self) -> usize {
        2_000
    }

    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
        self.poll_once().await.map(|_| ())
    }

    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        mut shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let mut ready = Some(ready);
        loop {
            let poll = self.poll_once();
            tokio::select! {
                _ = shutdown.changed() => return Ok(()),
                result = poll => match result {
                    Ok((messages, next_cursor)) => {
                        if let Some(ready) = ready.take() {
                            let _ = ready.send(());
                        }
                        for message in messages {
                            submit_inbound(&inbound, message).await?;
                        }
                        if let Some(next_cursor) = next_cursor {
                            self.session.save_cursor(&next_cursor)?;
                        }
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }

    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        let context = match outbound.reply_context {
            Some(context) => context,
            None => self
                .session
                .load_context(&outbound.chat_id)?
                .ok_or_else(|| ConnectorRuntimeError::new("wechat-context-missing"))?,
        };
        let response = self
            .post(
                "/ilink/bot/sendmessage",
                json!({
                    "msg": {
                        "from_user_id": "",
                        "to_user_id": outbound.chat_id,
                        "client_id": format!("vanehub-{}", uuid::Uuid::new_v4()),
                        "message_type": 2,
                        "context_token": context,
                        "message_state": 2,
                        "item_list": [{"type": 1, "text_item": {"text": outbound.text}}]
                    },
                    "base_info": {"channel_version": "0.1.0", "bot_agent": "VaneHub/0.1.0"}
                }),
            )
            .await?;
        let code = response
            .get("errcode")
            .or_else(|| response.get("ret"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if code == 0 {
            Ok(())
        } else if code == SESSION_EXPIRED {
            Err(ConnectorRuntimeError::new("wechat-authorization-expired"))
        } else {
            Err(ConnectorRuntimeError::new(format!("wechat-api-{code}")))
        }
    }
}

fn common_headers() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("ilink-app-id".to_string(), "bot".to_string()),
        ("ilink-app-clientversion".to_string(), "256".to_string()),
    ])
}

fn confirmed_credentials(body: &Value) -> WeChatQrStatus {
    let Some(bot_token) = body.get("bot_token").and_then(Value::as_str) else {
        return WeChatQrStatus::Error("token-missing".to_string());
    };
    let Some(bot_id) = body.get("ilink_bot_id").and_then(Value::as_str) else {
        return WeChatQrStatus::Error("bot-id-missing".to_string());
    };
    WeChatQrStatus::Confirmed(WeChatCredentials {
        bot_token: bot_token.to_string(),
        base_url: body.get("baseurl").and_then(Value::as_str).unwrap_or(ILINK_BASE_URL).to_string(),
        bot_id: bot_id.to_string(),
    })
}

fn fallback_event_id(sender_id: &str, context: &str, text: &str, index: usize) -> String {
    let mut digest = Sha256::new();
    digest.update(sender_id.as_bytes());
    digest.update([0]);
    digest.update(context.as_bytes());
    digest.update([0]);
    digest.update(text.as_bytes());
    digest.update(index.to_le_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::im::adapters::http::HttpResponse;
    use std::collections::{HashMap, VecDeque};
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemorySession {
        cursor: Mutex<String>,
        contexts: Mutex<HashMap<String, String>>,
    }

    impl WeChatSessionStore for MemorySession {
        fn load_cursor(&self) -> Result<String, ConnectorRuntimeError> {
            Ok(self.cursor.lock().unwrap().clone())
        }
        fn save_cursor(&self, cursor: &str) -> Result<(), ConnectorRuntimeError> {
            *self.cursor.lock().unwrap() = cursor.to_string();
            Ok(())
        }
        fn load_context(&self, chat_id: &str) -> Result<Option<String>, ConnectorRuntimeError> {
            Ok(self.contexts.lock().unwrap().get(chat_id).cloned())
        }
        fn save_context(&self, chat_id: &str, context: &str) -> Result<(), ConnectorRuntimeError> {
            self.contexts
                .lock()
                .unwrap()
                .insert(chat_id.to_string(), context.to_string());
            Ok(())
        }
    }

    struct MockHttp(Mutex<VecDeque<HttpResponse>>);

    #[async_trait]
    impl HttpTransport for MockHttp {
        async fn execute(
            &self,
            _request: HttpRequest,
        ) -> Result<HttpResponse, ConnectorRuntimeError> {
            self.0
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| ConnectorRuntimeError::new("missing-mock-response"))
        }
    }

    fn http(values: Vec<Value>) -> Arc<MockHttp> {
        Arc::new(MockHttp(Mutex::new(
            values
                .into_iter()
                .map(|body| HttpResponse { status: 200, body })
                .collect(),
        )))
    }

    #[tokio::test]
    async fn restores_cursor_normalizes_text_context_and_sends_final() {
        let session = Arc::new(MemorySession::default());
        let http = http(vec![
            json!({"ret": 0, "get_updates_buf": "cursor-2", "msgs": [{
                "message_id": 7,
                "from_user_id": "wx-user",
                "context_token": "context-private",
                "item_list": [{"type": 1, "text_item": {"text": "hello"}}]
            }]}),
            json!({"ret": 0}),
        ]);
        let adapter = WeChatAdapter::new("bot-private", http, session.clone()).unwrap();
        let (messages, next_cursor) = adapter.poll_once().await.unwrap();
        assert_eq!(messages[0].text, "hello");
        assert_eq!(next_cursor.as_deref(), Some("cursor-2"));
        assert_eq!(session.load_cursor().unwrap(), "");
        adapter
            .send_text(OutboundText {
                chat_id: "wx-user".into(),
                text: "final".into(),
                reply_context: None,
            })
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn creates_and_expires_qr_authorization_without_persistence() {
        let http = http(vec![
            json!({"ret": 0, "qrcode": "payload-private", "qrcode_img_content": "https://qr.invalid/code"}),
            json!({"ret": 0, "status": 3}),
        ]);
        let qr = WeChatAdapter::create_qr_code(http.as_ref()).await.unwrap();
        assert_eq!(qr.qr_url, "https://qr.invalid/code");
        assert_eq!(
            WeChatAdapter::poll_qr_status(http.as_ref(), &qr.qr_payload)
                .await
                .unwrap(),
            WeChatQrStatus::Expired
        );
    }

    #[tokio::test]
    async fn accepts_official_scanned_and_confirmed_qr_states() {
        let http = http(vec![
            json!({"ret": 0, "status": "scaned"}),
            json!({
                "ret": 0,
                "status": "confirmed",
                "bot_token": "bot-private",
                "baseurl": "https://ilink.example",
                "ilink_bot_id": "bot-id"
            }),
        ]);
        assert_eq!(
            WeChatAdapter::poll_qr_status(http.as_ref(), "qr-private")
                .await
                .unwrap(),
            WeChatQrStatus::Scanned
        );
        let WeChatQrStatus::Confirmed(credentials) = WeChatAdapter::poll_qr_status(
            http.as_ref(),
            "qr-private",
        )
        .await
        .unwrap() else {
            panic!("expected confirmed credentials");
        };
        assert_eq!(credentials.bot_token, "bot-private");
        assert_eq!(credentials.base_url, "https://ilink.example");
        assert_eq!(credentials.bot_id, "bot-id");
    }

    #[tokio::test]
    async fn reports_authorization_expiry() {
        let adapter = WeChatAdapter::new(
            "bot-private",
            http(vec![json!({"errcode": -14})]),
            Arc::new(MemorySession::default()),
        )
        .unwrap();
        assert_eq!(
            adapter.poll_once().await.unwrap_err().safe_code,
            "wechat-authorization-expired"
        );
    }
}
