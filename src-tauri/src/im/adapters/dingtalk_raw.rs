use super::dingtalk::{DingTalkFrame, DingTalkStream};
use crate::im::runtime::ConnectorRuntimeError;
use crate::network_proxy;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;

const GATEWAY_URL: &str = "https://api.dingtalk.com/v1.0/gateway/connections/open";
const ROBOT_TOPIC: &str = "/v1.0/im/bot/messages/get";

#[derive(Deserialize)]
struct ConnectionConfig {
    endpoint: String,
    ticket: String,
}

pub struct RawDingTalkStream {
    outbound: Arc<Mutex<Option<mpsc::Sender<Value>>>>,
}

impl Default for RawDingTalkStream {
    fn default() -> Self {
        Self {
            outbound: Arc::new(Mutex::new(None)),
        }
    }
}

impl RawDingTalkStream {
    async fn connection_config(
        &self,
        app_key: &str,
        app_secret: &str,
    ) -> Result<ConnectionConfig, ConnectorRuntimeError> {
        let client = network_proxy::http_client(Duration::from_secs(15))
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-client-failed"))?;
        let response = client
            .post(GATEWAY_URL)
            .header("accept", "application/json")
            .json(&json!({
                "clientId": app_key,
                "clientSecret": app_secret,
                "ua": "VaneHub AI",
                "subscriptions": [
                    {"type": "EVENT", "topic": "*"},
                    {"type": "CALLBACK", "topic": ROBOT_TOPIC}
                ]
            }))
            .send()
            .await
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-config-failed"))?;
        if !response.status().is_success() {
            return Err(ConnectorRuntimeError::new(format!(
                "dingtalk-stream-config-http-{}",
                response.status().as_u16()
            )));
        }
        response
            .json::<ConnectionConfig>()
            .await
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-config-invalid"))
    }

    async fn connect(
        &self,
        app_key: &str,
        app_secret: &str,
    ) -> Result<tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<network_proxy::BoxedAsyncIo>>, ConnectorRuntimeError> {
        let config = self.connection_config(app_key, app_secret).await?;
        let separator = if config.endpoint.contains('?') { '&' } else { '?' };
        let endpoint = format!("{}{separator}ticket={}", config.endpoint, urlencoding(&config.ticket));
        let target = url::Url::parse(&endpoint)
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-url-invalid"))?;
        let stream = network_proxy::websocket_stream(&target)
            .await
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-proxy-connect-failed"))?;
        tokio_tungstenite::client_async_tls_with_config(endpoint, stream, None, None)
            .await
            .map(|(socket, _)| socket)
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-connect-failed"))
    }

    async fn run_session(
        &self,
        app_key: &str,
        app_secret: &str,
        frames: &mpsc::Sender<DingTalkFrame>,
        outbound: &mut mpsc::Receiver<Value>,
        shutdown: &mut watch::Receiver<bool>,
        ready: &mut Option<oneshot::Sender<()>>,
    ) -> Result<(), ConnectorRuntimeError> {
        let socket = self.connect(app_key, app_secret).await?;
        if let Some(ready) = ready.take() {
            let _ = ready.send(());
        }
        let (mut writer, mut reader) = socket.split();
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    let _ = writer.close().await;
                    return Ok(());
                }
                outbound_message = outbound.recv() => {
                    let Some(outbound_message) = outbound_message else { return Ok(()); };
                    writer.send(Message::Text(outbound_message.to_string().into())).await
                        .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-ack-send-failed"))?;
                }
                incoming = reader.next() => {
                    let incoming = incoming
                        .ok_or_else(|| ConnectorRuntimeError::new("dingtalk-stream-closed"))?
                        .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-frame-failed"))?;
                    if incoming.is_ping() {
                        writer.send(Message::Pong(incoming.into_data())).await
                            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-pong-failed"))?;
                        continue;
                    }
                    if incoming.is_close() {
                        return Err(ConnectorRuntimeError::new("dingtalk-stream-closed"));
                    }
                    let Some(value) = message_json(incoming) else { continue; };
                    let kind = value.get("type").and_then(Value::as_str).unwrap_or_default();
                    let topic = value.pointer("/headers/topic").and_then(Value::as_str).unwrap_or_default();
                    if kind == "SYSTEM" && topic == "ping" {
                        writer.send(Message::Text(json!({
                            "code": 200,
                            "headers": value.get("headers").cloned().unwrap_or_else(|| json!({})),
                            "message": "OK",
                            "data": value.get("data").cloned().unwrap_or(Value::Null)
                        }).to_string().into())).await
                            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-ping-ack-failed"))?;
                        continue;
                    }
                    if kind != "CALLBACK" || topic != ROBOT_TOPIC { continue; }
                    let acknowledgement_id = value.pointer("/headers/messageId")
                        .and_then(Value::as_str).unwrap_or_default().to_string();
                    let data = value.get("data").and_then(Value::as_str)
                        .and_then(|data| serde_json::from_str::<Value>(data).ok())
                        .unwrap_or(Value::Null);
                    frames.send(DingTalkFrame {
                        acknowledgement_id,
                        payload: json!({"headers": value.get("headers"), "data": data}).to_string(),
                    }).await.map_err(|_| ConnectorRuntimeError::new("dingtalk-frame-channel-closed"))?;
                }
            }
        }
    }
}

#[async_trait]
impl DingTalkStream for RawDingTalkStream {
    async fn test(&self, app_key: &str, app_secret: &str) -> Result<(), ConnectorRuntimeError> {
        let mut socket = self.connect(app_key, app_secret).await?;
        socket.close(None).await
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-test-close-failed"))
    }

    async fn run(
        &self,
        app_key: &str,
        app_secret: &str,
        frames: mpsc::Sender<DingTalkFrame>,
        mut shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (sender, mut receiver) = mpsc::channel(64);
        let mut ready = Some(ready);
        *self.outbound.lock().await = Some(sender);
        let result = self
            .run_session(app_key, app_secret, &frames, &mut receiver, &mut shutdown, &mut ready)
            .await;
        *self.outbound.lock().await = None;
        result
    }

    async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError> {
        if acknowledgement_id.is_empty() { return Ok(()); }
        let sender = self.outbound.lock().await.clone()
            .ok_or_else(|| ConnectorRuntimeError::new("dingtalk-stream-not-connected"))?;
        sender.send(json!({
            "code": 200,
            "headers": {"contentType": "application/json", "messageId": acknowledgement_id},
            "message": "OK",
            "data": "{\"response\":\"SUCCESS\"}"
        })).await.map_err(|_| ConnectorRuntimeError::new("dingtalk-stream-ack-closed"))
    }
}

fn message_json(message: Message) -> Option<Value> {
    match message {
        Message::Text(text) => serde_json::from_str(text.as_str()).ok(),
        Message::Binary(bytes) => serde_json::from_slice(&bytes).ok(),
        _ => None,
    }
}

fn urlencoding(value: &str) -> String {
    url::form_urlencoded::byte_serialize(value.as_bytes()).collect()
}
