use super::runtime::ConnectorRuntimeError;
use super::wecom::{WeComFrame, WeComLongConnection};
use crate::platform::network as network_proxy;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;

const WECOM_ENDPOINT: &str = "wss://openws.work.weixin.qq.com";

pub struct RawWeComLongConnection {
    endpoint: String,
    outbound: Arc<Mutex<Option<mpsc::Sender<Value>>>>,
}

impl Default for RawWeComLongConnection {
    fn default() -> Self {
        Self {
            endpoint: WECOM_ENDPOINT.to_string(),
            outbound: Arc::new(Mutex::new(None)),
        }
    }
}

impl RawWeComLongConnection {
    async fn connect_and_authenticate(
        &self,
        bot_id: &str,
        secret: &str,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<network_proxy::BoxedAsyncIo>,
        >,
        ConnectorRuntimeError,
    > {
        let target = url::Url::parse(&self.endpoint)
            .map_err(|_| ConnectorRuntimeError::new("wecom-websocket-url-invalid"))?;
        let stream = network_proxy::websocket_stream(&target)
            .await
            .map_err(|_| ConnectorRuntimeError::new("wecom-websocket-proxy-connect-failed"))?;
        let (mut socket, _) =
            tokio_tungstenite::client_async_tls_with_config(&self.endpoint, stream, None, None)
                .await
                .map_err(|_| ConnectorRuntimeError::new("wecom-websocket-connect-failed"))?;
        let request_id = uuid::Uuid::new_v4().to_string();
        socket
            .send(Message::Text(
                json!({
                    "cmd": "aibot_subscribe",
                    "headers": {"req_id": request_id},
                    "body": {"bot_id": bot_id, "secret": secret}
                })
                .to_string()
                .into(),
            ))
            .await
            .map_err(|_| ConnectorRuntimeError::new("wecom-subscribe-send-failed"))?;
        let response = tokio::time::timeout(Duration::from_secs(8), socket.next())
            .await
            .map_err(|_| ConnectorRuntimeError::new("wecom-auth-timeout"))?
            .ok_or_else(|| ConnectorRuntimeError::new("wecom-auth-closed"))?
            .map_err(|_| ConnectorRuntimeError::new("wecom-auth-frame-failed"))?;
        let value = message_json(response)?;
        let error_code = value
            .pointer("/headers/errcode")
            .or_else(|| value.pointer("/body/errcode"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        if error_code != 0 {
            return Err(ConnectorRuntimeError::new(format!(
                "wecom-auth-{error_code}"
            )));
        }
        Ok(socket)
    }

    async fn run_session(
        &self,
        bot_id: &str,
        secret: &str,
        frames: &mpsc::Sender<WeComFrame>,
        outbound: &mut mpsc::Receiver<Value>,
        shutdown: &mut watch::Receiver<bool>,
        ready: &mut Option<oneshot::Sender<()>>,
    ) -> Result<(), ConnectorRuntimeError> {
        let socket = self.connect_and_authenticate(bot_id, secret).await?;
        if let Some(ready) = ready.take() {
            let _ = ready.send(());
        }
        let (mut writer, mut reader) = socket.split();
        let mut heartbeat = tokio::time::interval(Duration::from_secs(30));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    let _ = writer.close().await;
                    return Ok(());
                }
                _ = heartbeat.tick() => {
                    writer.send(Message::Text(json!({
                        "cmd": "ping",
                        "headers": {"req_id": uuid::Uuid::new_v4().to_string()}
                    }).to_string().into())).await
                        .map_err(|_| ConnectorRuntimeError::new("wecom-ping-failed"))?;
                }
                outbound_message = outbound.recv() => {
                    let Some(outbound_message) = outbound_message else {
                        return Ok(());
                    };
                    writer.send(Message::Text(outbound_message.to_string().into())).await
                        .map_err(|_| ConnectorRuntimeError::new("wecom-send-failed"))?;
                }
                incoming = reader.next() => {
                    let incoming = incoming
                        .ok_or_else(|| ConnectorRuntimeError::new("wecom-websocket-closed"))?
                        .map_err(|_| ConnectorRuntimeError::new("wecom-frame-failed"))?;
                    if incoming.is_ping() {
                        writer.send(Message::Pong(incoming.into_data())).await
                            .map_err(|_| ConnectorRuntimeError::new("wecom-pong-failed"))?;
                        continue;
                    }
                    if incoming.is_close() {
                        return Err(ConnectorRuntimeError::new("wecom-websocket-closed"));
                    }
                    let value = message_json(incoming)?;
                    let command = value.get("cmd")
                        .or_else(|| value.pointer("/headers/cmd"))
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    if !matches!(command, "aibot_msg_callback" | "aibot_event_callback") {
                        continue;
                    }
                    let acknowledgement_id = value.pointer("/headers/req_id")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    frames.send(WeComFrame {
                        acknowledgement_id,
                        payload: value.to_string(),
                    }).await.map_err(|_| ConnectorRuntimeError::new("wecom-frame-channel-closed"))?;
                }
            }
        }
    }
}

#[async_trait]
impl WeComLongConnection for RawWeComLongConnection {
    async fn test(&self, bot_id: &str, secret: &str) -> Result<(), ConnectorRuntimeError> {
        let mut socket = self.connect_and_authenticate(bot_id, secret).await?;
        socket
            .close(None)
            .await
            .map_err(|_| ConnectorRuntimeError::new("wecom-test-close-failed"))
    }

    async fn run(
        &self,
        bot_id: &str,
        secret: &str,
        frames: mpsc::Sender<WeComFrame>,
        mut shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (sender, mut receiver) = mpsc::channel(64);
        let mut ready = Some(ready);
        *self.outbound.lock().await = Some(sender);
        let result = self
            .run_session(
                bot_id,
                secret,
                &frames,
                &mut receiver,
                &mut shutdown,
                &mut ready,
            )
            .await;
        *self.outbound.lock().await = None;
        result
    }

    async fn acknowledge(&self, _acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError> {
        Ok(())
    }

    async fn send_text(
        &self,
        chat_id: &str,
        request_context: &str,
        text: &str,
    ) -> Result<(), ConnectorRuntimeError> {
        let sender = self
            .outbound
            .lock()
            .await
            .clone()
            .ok_or_else(|| ConnectorRuntimeError::new("wecom-not-connected"))?;
        sender
            .send(json!({
                "cmd": "aibot_respond_msg",
                "headers": {"req_id": request_context},
                "body": {
                    "chatid": chat_id,
                    "msgtype": "stream",
                    "stream": {
                        "id": format!("vanehub-{}", uuid::Uuid::new_v4()),
                        "finish": true,
                        "content": text
                    }
                }
            }))
            .await
            .map_err(|_| ConnectorRuntimeError::new("wecom-outbound-closed"))
    }
}

fn message_json(message: Message) -> Result<Value, ConnectorRuntimeError> {
    match message {
        Message::Text(text) => serde_json::from_str(text.as_str())
            .map_err(|_| ConnectorRuntimeError::new("wecom-frame-invalid")),
        Message::Binary(bytes) => serde_json::from_slice(&bytes)
            .map_err(|_| ConnectorRuntimeError::new("wecom-frame-invalid")),
        _ => Err(ConnectorRuntimeError::new("wecom-frame-unsupported")),
    }
}
