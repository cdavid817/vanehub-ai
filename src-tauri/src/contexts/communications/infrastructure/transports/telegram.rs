use super::http::{require_success, HttpMethod, HttpRequest, HttpTransport};
use super::protocol::normalize_fixture;
use super::runtime::{submit_inbound, ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
use crate::contexts::communications::domain::{ConnectorKind, NormalizedInbound, OutboundText};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroizing;

const TELEGRAM_API_BASE: &str = "https://api.telegram.org";

pub trait TelegramCheckpoint: Send + Sync {
    fn load_offset(&self) -> Result<i64, ConnectorRuntimeError>;
    fn save_offset(&self, offset: i64) -> Result<(), ConnectorRuntimeError>;
}

pub struct TelegramAdapter {
    token: Zeroizing<String>,
    api_base: String,
    transport: Arc<dyn HttpTransport>,
    checkpoint: Arc<dyn TelegramCheckpoint>,
}

impl TelegramAdapter {
    pub fn new(
        token: &str,
        transport: Arc<dyn HttpTransport>,
        checkpoint: Arc<dyn TelegramCheckpoint>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let token = normalize_token(token)?;
        Ok(Self {
            token: Zeroizing::new(token),
            api_base: TELEGRAM_API_BASE.to_string(),
            transport,
            checkpoint,
        })
    }

    #[cfg(test)]
    fn with_api_base(mut self, api_base: &str) -> Self {
        self.api_base = api_base.trim_end_matches('/').to_string();
        self
    }

    fn method_url(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base, self.token.as_str(), method)
    }

    async fn call(
        &self,
        method: &str,
        body: Option<Value>,
    ) -> Result<Value, ConnectorRuntimeError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method: if body.is_some() {
                    HttpMethod::Post
                } else {
                    HttpMethod::Get
                },
                url: self.method_url(method),
                headers: BTreeMap::new(),
                body,
            })
            .await?;
        require_success(&response)?;
        if response.body.get("ok").and_then(Value::as_bool) != Some(true) {
            let code = response
                .body
                .get("error_code")
                .and_then(Value::as_i64)
                .unwrap_or_default();
            return Err(ConnectorRuntimeError::new(format!("telegram-api-{code}")));
        }
        Ok(response.body)
    }

    async fn poll_once(
        &self,
        offset: i64,
    ) -> Result<(Vec<NormalizedInbound>, i64), ConnectorRuntimeError> {
        let response = self
            .call(
                "getUpdates",
                Some(json!({
                    "offset": offset,
                    "timeout": 25,
                    "allowed_updates": ["message"]
                })),
            )
            .await?;
        let updates = response
            .get("result")
            .and_then(Value::as_array)
            .ok_or_else(|| ConnectorRuntimeError::new("telegram-result-invalid"))?;
        let mut inbound = Vec::new();
        let mut next_offset = offset;
        for update in updates {
            if let Some(update_id) = update.get("update_id").and_then(Value::as_i64) {
                next_offset = next_offset.max(update_id + 1);
            }
            let payload = serde_json::to_string(update)
                .map_err(|_| ConnectorRuntimeError::new("telegram-update-invalid"))?;
            if let Ok(message) = normalize_fixture(ConnectorKind::Telegram, &payload) {
                if message.direct && !message.text.trim().is_empty() {
                    inbound.push(message);
                }
            }
        }
        Ok((inbound, next_offset))
    }
}

#[async_trait]
impl ConnectorAdapter for TelegramAdapter {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Telegram
    }

    fn max_outbound_chars(&self) -> usize {
        4_096
    }

    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
        self.call("getMe", None).await?;
        let webhook = self.call("getWebhookInfo", None).await?;
        let webhook_url = webhook
            .pointer("/result/url")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !webhook_url.is_empty() {
            return Err(ConnectorRuntimeError::new("telegram-webhook-conflict"));
        }
        Ok(())
    }

    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        mut shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let mut ready = Some(ready);
        loop {
            if *shutdown.borrow() {
                return Ok(());
            }
            let offset = self.checkpoint.load_offset()?;
            let poll = self.poll_once(offset);
            tokio::select! {
                changed = shutdown.changed() => {
                    if changed.is_err() || *shutdown.borrow() {
                        return Ok(());
                    }
                }
                result = poll => {
                    match result {
                        Ok((messages, next_offset)) => {
                            if let Some(ready) = ready.take() {
                                let _ = ready.send(());
                            }
                            for message in messages {
                                submit_inbound(&inbound, message).await?;
                            }
                            if next_offset != offset {
                                self.checkpoint.save_offset(next_offset)?;
                            }
                        }
                        Err(error) if error.safe_code == "telegram-api-409" => {
                            return Err(ConnectorRuntimeError::new("telegram-polling-conflict"));
                        }
                        Err(error) => return Err(error),
                    }
                }
            }
        }
    }

    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        self.call(
            "sendMessage",
            Some(json!({
                "chat_id": outbound.chat_id,
                "text": outbound.text
            })),
        )
        .await?;
        Ok(())
    }
}

fn normalize_token(token: &str) -> Result<String, ConnectorRuntimeError> {
    let token = token.trim();
    let valid = token.len() >= 10
        && token.contains(':')
        && token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, ':' | '_' | '-'));
    if valid {
        Ok(token.to_string())
    } else {
        Err(ConnectorRuntimeError::new("telegram-token-invalid"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::HttpResponse;
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    #[derive(Default)]
    struct MemoryCheckpoint(Mutex<i64>);

    impl TelegramCheckpoint for MemoryCheckpoint {
        fn load_offset(&self) -> Result<i64, ConnectorRuntimeError> {
            self.0
                .lock()
                .map(|value| *value)
                .map_err(|_| ConnectorRuntimeError::new("checkpoint-lock"))
        }

        fn save_offset(&self, offset: i64) -> Result<(), ConnectorRuntimeError> {
            *self
                .0
                .lock()
                .map_err(|_| ConnectorRuntimeError::new("checkpoint-lock"))? = offset;
            Ok(())
        }
    }

    struct MockHttp {
        responses: Mutex<VecDeque<HttpResponse>>,
        requests: Mutex<Vec<HttpRequest>>,
    }

    #[async_trait]
    impl HttpTransport for MockHttp {
        async fn execute(
            &self,
            request: HttpRequest,
        ) -> Result<HttpResponse, ConnectorRuntimeError> {
            self.requests.lock().unwrap().push(request);
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| ConnectorRuntimeError::new("missing-mock-response"))
        }
    }

    fn mock(responses: Vec<Value>) -> Arc<MockHttp> {
        Arc::new(MockHttp {
            responses: Mutex::new(
                responses
                    .into_iter()
                    .map(|body| HttpResponse { status: 200, body })
                    .collect(),
            ),
            requests: Mutex::new(Vec::new()),
        })
    }

    #[tokio::test]
    async fn validates_bot_without_deleting_webhook() {
        let http = mock(vec![
            json!({"ok": true, "result": {"id": 1}}),
            json!({"ok": true, "result": {"url": "https://existing.invalid/hook"}}),
        ]);
        let adapter = TelegramAdapter::new(
            "123456:fixture_token",
            http.clone(),
            Arc::new(MemoryCheckpoint::default()),
        )
        .unwrap()
        .with_api_base("https://telegram.invalid");

        let error = adapter.test_connection().await.unwrap_err();
        assert_eq!(error.safe_code, "telegram-webhook-conflict");
        let requests = http.requests.lock().unwrap();
        assert_eq!(requests.len(), 2);
        assert!(requests
            .iter()
            .all(|request| !request.url.contains("deleteWebhook")));
    }

    #[tokio::test]
    async fn polls_direct_text_persists_offset_and_sends_final_text() {
        let http = mock(vec![
            json!({
                "ok": true,
                "result": [{
                    "update_id": 41,
                    "message": {
                        "message_id": 10,
                        "from": {"id": 20, "is_bot": false},
                        "chat": {"id": 20, "type": "private"},
                        "text": "hello"
                    }
                }]
            }),
            json!({"ok": true, "result": {"message_id": 11}}),
        ]);
        let checkpoint = Arc::new(MemoryCheckpoint::default());
        let adapter =
            TelegramAdapter::new("123456:fixture_token", http.clone(), checkpoint.clone())
                .unwrap()
                .with_api_base("https://telegram.invalid");

        let (messages, next_offset) = adapter.poll_once(0).await.unwrap();
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "hello");
        assert_eq!(next_offset, 42);
        assert_eq!(checkpoint.load_offset().unwrap(), 0);
        adapter
            .send_text(OutboundText {
                chat_id: "20".to_string(),
                text: "final".to_string(),
                reply_context: None,
            })
            .await
            .unwrap();
        assert!(http.requests.lock().unwrap()[1]
            .url
            .ends_with("/sendMessage"));
    }

    #[test]
    fn rejects_malformed_tokens() {
        assert!(normalize_token("secret without separator").is_err());
        assert_eq!(
            normalize_token(" 123456:fixture_token ").unwrap(),
            "123456:fixture_token"
        );
    }
}
