use super::feishu::{FeishuFrame, FeishuLongConnection};
use crate::im::runtime::ConnectorRuntimeError;
use crate::network_proxy;
use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use prost::Message as ProstMessage;
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, oneshot, watch, Mutex};
use tokio_tungstenite::tungstenite::Message;

const CONFIG_URL: &str = "https://open.feishu.cn/callback/ws/endpoint";

#[derive(Clone, PartialEq, ProstMessage)]
struct Header {
    #[prost(string, required, tag = "1")]
    key: String,
    #[prost(string, required, tag = "2")]
    value: String,
}

#[derive(Clone, PartialEq, ProstMessage)]
struct WireFrame {
    #[prost(uint64, required, tag = "1")]
    seq_id: u64,
    #[prost(uint64, required, tag = "2")]
    log_id: u64,
    #[prost(int32, required, tag = "3")]
    service: i32,
    #[prost(int32, required, tag = "4")]
    method: i32,
    #[prost(message, repeated, tag = "5")]
    headers: Vec<Header>,
    #[prost(string, optional, tag = "6")]
    payload_encoding: Option<String>,
    #[prost(string, optional, tag = "7")]
    payload_type: Option<String>,
    #[prost(bytes, optional, tag = "8")]
    payload: Option<Vec<u8>>,
    #[prost(string, optional, tag = "9")]
    log_id_new: Option<String>,
}

#[derive(Deserialize)]
struct EndpointEnvelope {
    code: i64,
    data: Option<EndpointData>,
}

#[derive(Deserialize)]
struct EndpointData {
    #[serde(rename = "URL")]
    url: String,
    #[serde(rename = "ClientConfig")]
    client_config: ClientConfig,
}

#[derive(Deserialize)]
struct ClientConfig {
    #[serde(rename = "PingInterval")]
    ping_interval: u64,
}

struct ChunkAssembly {
    parts: Vec<Option<Vec<u8>>>,
    response_frame: WireFrame,
}

pub struct RawFeishuLongConnection {
    outbound: Arc<Mutex<Option<mpsc::Sender<WireFrame>>>>,
    pending: Arc<Mutex<HashMap<String, WireFrame>>>,
    chunks: Arc<Mutex<HashMap<String, ChunkAssembly>>>,
}

impl Default for RawFeishuLongConnection {
    fn default() -> Self {
        Self {
            outbound: Arc::new(Mutex::new(None)),
            pending: Arc::new(Mutex::new(HashMap::new())),
            chunks: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RawFeishuLongConnection {
    async fn endpoint(&self, app_id: &str, app_secret: &str) -> Result<EndpointData, ConnectorRuntimeError> {
        let client = network_proxy::http_client(Duration::from_secs(15))
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-client-failed"))?;
        let response = client.post(CONFIG_URL)
            .header("locale", "zh")
            .json(&json!({"AppID": app_id, "AppSecret": app_secret}))
            .send().await
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-config-failed"))?;
        if !response.status().is_success() {
            return Err(ConnectorRuntimeError::new(format!("feishu-ws-config-http-{}", response.status().as_u16())));
        }
        let envelope = response.json::<EndpointEnvelope>().await
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-config-invalid"))?;
        if envelope.code != 0 {
            return Err(ConnectorRuntimeError::new(format!("feishu-ws-config-api-{}", envelope.code)));
        }
        envelope.data.ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-config-missing"))
    }

    async fn connect(&self, app_id: &str, app_secret: &str) -> Result<(
        tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<network_proxy::BoxedAsyncIo>>,
        u64,
        i32,
    ), ConnectorRuntimeError> {
        let endpoint = self.endpoint(app_id, app_secret).await?;
        let target = url::Url::parse(&endpoint.url)
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-url-invalid"))?;
        let service_id = target.query_pairs().find(|(key, _)| key == "service_id")
            .and_then(|(_, value)| value.parse::<i32>().ok())
            .ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-service-id-missing"))?;
        let stream = network_proxy::websocket_stream(&target).await
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-proxy-connect-failed"))?;
        let socket = tokio_tungstenite::client_async_tls_with_config(endpoint.url, stream, None, None).await
            .map(|(socket, _)| socket)
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-connect-failed"))?;
        Ok((socket, endpoint.client_config.ping_interval.max(10), service_id))
    }

    async fn accept_event(&self, frame: WireFrame) -> Result<Option<(String, String)>, ConnectorRuntimeError> {
        let headers = frame.headers.iter().map(|header| (header.key.as_str(), header.value.as_str()))
            .collect::<HashMap<_, _>>();
        if headers.get("type").copied() != Some("event") { return Ok(None); }
        let message_id = headers.get("message_id").copied().unwrap_or_default().to_string();
        if message_id.is_empty() { return Err(ConnectorRuntimeError::new("feishu-ws-message-id-missing")); }
        let sum = headers.get("sum").and_then(|value| value.parse::<usize>().ok()).unwrap_or(1).max(1);
        let seq = headers.get("seq").and_then(|value| value.parse::<usize>().ok()).unwrap_or(0);
        if seq >= sum { return Err(ConnectorRuntimeError::new("feishu-ws-chunk-invalid")); }
        let payload = frame.payload.clone().unwrap_or_default();
        let mut chunks = self.chunks.lock().await;
        let assembly = chunks.entry(message_id.clone()).or_insert_with(|| ChunkAssembly {
            parts: vec![None; sum],
            response_frame: frame.clone(),
        });
        if assembly.parts.len() != sum { return Err(ConnectorRuntimeError::new("feishu-ws-chunk-count-changed")); }
        assembly.parts[seq] = Some(payload);
        if assembly.parts.iter().any(Option::is_none) { return Ok(None); }
        let assembly = chunks.remove(&message_id)
            .ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-chunk-missing"))?;
        let bytes = assembly.parts.into_iter().flatten().flatten().collect::<Vec<_>>();
        let payload = String::from_utf8(bytes)
            .map_err(|_| ConnectorRuntimeError::new("feishu-ws-payload-invalid"))?;
        self.pending.lock().await.insert(message_id.clone(), assembly.response_frame);
        Ok(Some((message_id, payload)))
    }

    async fn run_session(
        &self,
        app_id: &str,
        app_secret: &str,
        frames: &mpsc::Sender<FeishuFrame>,
        outbound: &mut mpsc::Receiver<WireFrame>,
        shutdown: &mut watch::Receiver<bool>,
        ready: &mut Option<oneshot::Sender<()>>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (socket, ping_seconds, service_id) = self.connect(app_id, app_secret).await?;
        if let Some(ready) = ready.take() {
            let _ = ready.send(());
        }
        let (mut writer, mut reader) = socket.split();
        let mut heartbeat = tokio::time::interval(Duration::from_secs(ping_seconds));
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            tokio::select! {
                _ = shutdown.changed() => { let _ = writer.close().await; return Ok(()); }
                _ = heartbeat.tick() => {
                    let ping = WireFrame { seq_id: 0, log_id: 0, service: service_id, method: 0,
                        headers: vec![Header { key: "type".into(), value: "ping".into() }],
                        payload_encoding: None, payload_type: None, payload: None, log_id_new: None };
                    writer.send(Message::Binary(ping.encode_to_vec().into())).await
                        .map_err(|_| ConnectorRuntimeError::new("feishu-ws-ping-failed"))?;
                }
                outgoing = outbound.recv() => {
                    let Some(outgoing) = outgoing else { return Ok(()); };
                    writer.send(Message::Binary(outgoing.encode_to_vec().into())).await
                        .map_err(|_| ConnectorRuntimeError::new("feishu-ws-ack-send-failed"))?;
                }
                incoming = reader.next() => {
                    let incoming = incoming.ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-closed"))?
                        .map_err(|_| ConnectorRuntimeError::new("feishu-ws-frame-failed"))?;
                    if incoming.is_ping() { writer.send(Message::Pong(incoming.into_data())).await
                        .map_err(|_| ConnectorRuntimeError::new("feishu-ws-pong-failed"))?; continue; }
                    if incoming.is_close() { return Err(ConnectorRuntimeError::new("feishu-ws-closed")); }
                    let bytes = match incoming { Message::Binary(bytes) => bytes, _ => continue };
                    let frame = WireFrame::decode(bytes)
                        .map_err(|_| ConnectorRuntimeError::new("feishu-ws-frame-invalid"))?;
                    if frame.method != 1 { continue; }
                    if let Some((acknowledgement_id, payload)) = self.accept_event(frame).await? {
                        frames.send(FeishuFrame { acknowledgement_id, payload }).await
                            .map_err(|_| ConnectorRuntimeError::new("feishu-frame-channel-closed"))?;
                    }
                }
            }
        }
    }
}

#[async_trait]
impl FeishuLongConnection for RawFeishuLongConnection {
    async fn test(&self, app_id: &str, app_secret: &str) -> Result<(), ConnectorRuntimeError> {
        let (mut socket, _, _) = self.connect(app_id, app_secret).await?;
        socket.close(None).await.map_err(|_| ConnectorRuntimeError::new("feishu-ws-test-close-failed"))
    }

    async fn run(&self, app_id: &str, app_secret: &str, frames: mpsc::Sender<FeishuFrame>, mut shutdown: watch::Receiver<bool>, ready: oneshot::Sender<()>) -> Result<(), ConnectorRuntimeError> {
        let (sender, mut receiver) = mpsc::channel(64);
        let mut ready = Some(ready);
        *self.outbound.lock().await = Some(sender);
        let result = self
            .run_session(app_id, app_secret, &frames, &mut receiver, &mut shutdown, &mut ready)
            .await;
        *self.outbound.lock().await = None;
        result
    }

    async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError> {
        let mut frame = self.pending.lock().await.remove(acknowledgement_id)
            .ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-ack-frame-missing"))?;
        frame.headers.push(Header { key: "biz_rt".into(), value: "0".into() });
        frame.payload = Some(b"{\"code\":200}".to_vec());
        let sender = self.outbound.lock().await.clone()
            .ok_or_else(|| ConnectorRuntimeError::new("feishu-ws-not-connected"))?;
        sender.send(frame).await.map_err(|_| ConnectorRuntimeError::new("feishu-ws-ack-closed"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reassembles_event_chunks_before_ack_registration() {
        let connection = RawFeishuLongConnection::default();
        let base = |seq: usize, payload: &[u8]| WireFrame {
            seq_id: 1, log_id: 2, service: 3, method: 1,
            headers: vec![
                Header { key: "type".into(), value: "event".into() },
                Header { key: "message_id".into(), value: "message-1".into() },
                Header { key: "sum".into(), value: "2".into() },
                Header { key: "seq".into(), value: seq.to_string() },
            ], payload_encoding: None, payload_type: None, payload: Some(payload.to_vec()), log_id_new: None,
        };
        assert!(connection.accept_event(base(0, b"{\"ok\":" )).await.unwrap().is_none());
        let merged = connection.accept_event(base(1, b"true}" )).await.unwrap().unwrap();
        assert_eq!(merged.1, "{\"ok\":true}");
        assert!(connection.pending.lock().await.contains_key("message-1"));
    }
}
