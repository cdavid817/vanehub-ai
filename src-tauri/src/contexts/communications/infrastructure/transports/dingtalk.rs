use super::http::{require_success, HttpMethod, HttpRequest, HttpTransport};
use super::protocol::normalize_fixture;
use super::runtime::{submit_inbound, ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
use crate::contexts::communications::domain::{ConnectorKind, OutboundText};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroizing;

const DINGTALK_API_BASE: &str = "https://api.dingtalk.com";

#[derive(Debug, Clone)]
pub struct DingTalkFrame {
    pub acknowledgement_id: String,
    pub payload: String,
}

#[async_trait]
pub trait DingTalkStream: Send + Sync {
    async fn test(&self, app_key: &str, app_secret: &str) -> Result<(), ConnectorRuntimeError>;
    async fn run(
        &self,
        app_key: &str,
        app_secret: &str,
        frames: mpsc::Sender<DingTalkFrame>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError>;
    async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError>;
}

pub struct DingTalkAdapter {
    app_key: String,
    app_secret: Zeroizing<String>,
    robot_code: String,
    api_base: String,
    transport: Arc<dyn HttpTransport>,
    stream: Arc<dyn DingTalkStream>,
}

impl DingTalkAdapter {
    pub fn new(
        app_key: &str,
        app_secret: &str,
        robot_code: Option<&str>,
        transport: Arc<dyn HttpTransport>,
        stream: Arc<dyn DingTalkStream>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let app_key = app_key.trim();
        let app_secret = app_secret.trim();
        if app_key.is_empty() || app_secret.is_empty() {
            return Err(ConnectorRuntimeError::new("dingtalk-credentials-invalid"));
        }
        Ok(Self {
            app_key: app_key.to_string(),
            app_secret: Zeroizing::new(app_secret.to_string()),
            robot_code: robot_code
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or(app_key)
                .to_string(),
            api_base: DINGTALK_API_BASE.to_string(),
            transport,
            stream,
        })
    }

    async fn access_token(&self) -> Result<String, ConnectorRuntimeError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{}/v1.0/oauth2/accessToken", self.api_base),
                headers: BTreeMap::new(),
                body: Some(json!({
                    "appKey": self.app_key,
                    "appSecret": self.app_secret.as_str()
                })),
            })
            .await?;
        require_success(&response)?;
        response
            .body
            .get("accessToken")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ConnectorRuntimeError::new("dingtalk-token-missing"))
    }
}

#[async_trait]
impl ConnectorAdapter for DingTalkAdapter {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::DingTalk
    }

    fn max_outbound_chars(&self) -> usize {
        2_000
    }

    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
        self.access_token().await?;
        self.stream
            .test(&self.app_key, self.app_secret.as_str())
            .await
    }

    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (frame_sender, mut frame_receiver) = mpsc::channel(64);
        let connection = self.stream.run(
            &self.app_key,
            self.app_secret.as_str(),
            frame_sender,
            shutdown,
            ready,
        );
        tokio::pin!(connection);
        loop {
            tokio::select! {
                result = &mut connection => return result,
                frame = frame_receiver.recv() => {
                    let Some(frame) = frame else {
                        return Err(ConnectorRuntimeError::new("dingtalk-frame-stream-closed"));
                    };
                    if let Ok(message) = normalize_fixture(ConnectorKind::DingTalk, &frame.payload) {
                        submit_inbound(&inbound, message).await?;
                    }
                    self.stream.acknowledge(&frame.acknowledgement_id).await?;
                }
            }
        }
    }

    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        let access_token = self.access_token().await?;
        let mut headers = BTreeMap::new();
        headers.insert("x-acs-dingtalk-access-token".to_string(), access_token);
        let msg_param = serde_json::to_string(&json!({"content": outbound.text}))
            .map_err(|_| ConnectorRuntimeError::new("dingtalk-content-invalid"))?;
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{}/v1.0/robot/oToMessages/batchSend", self.api_base),
                headers,
                body: Some(json!({
                    "robotCode": self.robot_code,
                    "userIds": [outbound.chat_id],
                    "msgKey": "sampleText",
                    "msgParam": msg_param
                })),
            })
            .await?;
        require_success(&response)
    }
}

#[cfg(test)]
mod tests {
    use super::super::http::HttpResponse;
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

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

    #[derive(Default)]
    struct FakeStream(Mutex<Vec<String>>);

    #[async_trait]
    impl DingTalkStream for FakeStream {
        async fn test(
            &self,
            _app_key: &str,
            _app_secret: &str,
        ) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _app_key: &str,
            _app_secret: &str,
            frames: mpsc::Sender<DingTalkFrame>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            let _ = ready.send(());
            frames
                .send(DingTalkFrame {
                    acknowledgement_id: "ack-dt".to_string(),
                    payload: include_str!("fixtures/dingtalk-direct-text.json").to_string(),
                })
                .await
                .map_err(|_| ConnectorRuntimeError::new("frame-channel-closed"))?;
            while !*shutdown.borrow() {
                shutdown
                    .changed()
                    .await
                    .map_err(|_| ConnectorRuntimeError::new("shutdown-closed"))?;
            }
            Ok(())
        }

        async fn acknowledge(&self, id: &str) -> Result<(), ConnectorRuntimeError> {
            self.0.lock().unwrap().push(id.to_string());
            Ok(())
        }
    }

    #[tokio::test]
    async fn validates_stream_acknowledges_direct_text_and_sends_final() {
        let http = Arc::new(MockHttp(Mutex::new(
            vec![
                HttpResponse {
                    status: 200,
                    body: json!({"accessToken": "access"}),
                },
                HttpResponse {
                    status: 200,
                    body: json!({"accessToken": "access"}),
                },
                HttpResponse {
                    status: 200,
                    body: json!({"processQueryKey": "ok"}),
                },
            ]
            .into_iter()
            .collect(),
        )));
        let stream = Arc::new(FakeStream::default());
        let adapter = Arc::new(
            DingTalkAdapter::new("app-key", "app-private", None, http, stream.clone()).unwrap(),
        );
        adapter.test_connection().await.unwrap();
        let (sender, mut receiver) = mpsc::channel(1);
        let (stop, shutdown) = watch::channel(false);
        let (ready, connected) = oneshot::channel();
        let worker = {
            let adapter = adapter.clone();
            tokio::spawn(async move { adapter.run(sender, shutdown, ready).await })
        };
        connected.await.unwrap();
        assert_eq!(
            receiver.recv().await.unwrap().accept().text,
            "status please"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(stream.0.lock().unwrap().as_slice(), ["ack-dt"]);
        adapter
            .send_text(OutboundText {
                chat_id: "staff".into(),
                text: "final".into(),
                reply_context: None,
            })
            .await
            .unwrap();
        stop.send(true).unwrap();
        worker.await.unwrap().unwrap();
    }
}
