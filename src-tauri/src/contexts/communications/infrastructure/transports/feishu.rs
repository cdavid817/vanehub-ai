use super::http::{require_success, HttpMethod, HttpRequest, HttpTransport};
use super::protocol::normalize_fixture;
use super::runtime::{
    submit_inbound, ConnectorAdapter, ConnectorRuntimeError, InboundDelivery,
    MalformedEventReporter, SafeDiagnosticSink,
};
use super::token_cache::AccessTokenCache;
use crate::contexts::communications::domain::{ConnectorKind, OutboundText};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroizing;

const FEISHU_API_BASE: &str = "https://open.feishu.cn/open-apis";

#[derive(Debug, Clone)]
pub struct FeishuFrame {
    pub acknowledgement_id: String,
    pub payload: String,
}

#[async_trait]
pub trait FeishuLongConnection: Send + Sync {
    async fn test(&self, app_id: &str, app_secret: &str) -> Result<(), ConnectorRuntimeError>;
    async fn run(
        &self,
        app_id: &str,
        app_secret: &str,
        frames: mpsc::Sender<FeishuFrame>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError>;
    async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError>;
}

pub struct FeishuAdapter {
    app_id: String,
    app_secret: Zeroizing<String>,
    api_base: String,
    transport: Arc<dyn HttpTransport>,
    long_connection: Arc<dyn FeishuLongConnection>,
    token_cache: AccessTokenCache,
    malformed_events: MalformedEventReporter,
}

impl FeishuAdapter {
    pub fn new(
        app_id: &str,
        app_secret: &str,
        transport: Arc<dyn HttpTransport>,
        long_connection: Arc<dyn FeishuLongConnection>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let app_id = app_id.trim();
        let app_secret = app_secret.trim();
        if app_id.is_empty() || app_secret.is_empty() {
            return Err(ConnectorRuntimeError::new("feishu-credentials-invalid"));
        }
        Ok(Self {
            app_id: app_id.to_string(),
            app_secret: Zeroizing::new(app_secret.to_string()),
            api_base: FEISHU_API_BASE.to_string(),
            transport,
            long_connection,
            token_cache: AccessTokenCache::default(),
            malformed_events: MalformedEventReporter::new(ConnectorKind::Feishu),
        })
    }

    pub(crate) fn with_diagnostic_sink(mut self, sink: SafeDiagnosticSink) -> Self {
        self.malformed_events = self.malformed_events.with_sink(sink);
        self
    }

    #[cfg(test)]
    fn with_api_base(mut self, api_base: &str) -> Self {
        self.api_base = api_base.trim_end_matches('/').to_string();
        self
    }

    async fn tenant_token(&self) -> Result<String, ConnectorRuntimeError> {
        self.token_cache
            .get_or_refresh(|| self.fetch_tenant_token())
            .await
    }

    async fn fetch_tenant_token(
        &self,
    ) -> Result<(String, std::time::Duration), ConnectorRuntimeError> {
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{}/auth/v3/tenant_access_token/internal", self.api_base),
                headers: BTreeMap::new(),
                body: Some(json!({
                    "app_id": self.app_id,
                    "app_secret": self.app_secret.as_str()
                })),
            })
            .await?;
        require_success(&response)?;
        let code = response
            .body
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if code != 0 {
            return Err(ConnectorRuntimeError::new(format!("feishu-api-{code}")));
        }
        let token = response
            .body
            .get("tenant_access_token")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| ConnectorRuntimeError::new("feishu-token-missing"))?;
        let expires_in = response
            .body
            .get("expire")
            .and_then(Value::as_u64)
            .unwrap_or(3_600);
        Ok((token, std::time::Duration::from_secs(expires_in)))
    }

    async fn send_final(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        let token = self.tenant_token().await?;
        let mut headers = BTreeMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {token}"));
        let content = serde_json::to_string(&json!({"text": outbound.text}))
            .map_err(|_| ConnectorRuntimeError::new("feishu-content-invalid"))?;
        let response = self
            .transport
            .execute(HttpRequest {
                method: HttpMethod::Post,
                url: format!("{}/im/v1/messages?receive_id_type=chat_id", self.api_base),
                headers,
                body: Some(json!({
                    "receive_id": outbound.chat_id,
                    "msg_type": "text",
                    "content": content
                })),
            })
            .await?;
        if let Err(error) = require_success(&response) {
            self.token_cache.invalidate().await;
            return Err(error);
        }
        let code = response
            .body
            .get("code")
            .and_then(Value::as_i64)
            .unwrap_or(-1);
        if code == 0 {
            Ok(())
        } else {
            self.token_cache.invalidate().await;
            Err(ConnectorRuntimeError::new(format!("feishu-api-{code}")))
        }
    }
}

#[async_trait]
impl ConnectorAdapter for FeishuAdapter {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::Feishu
    }

    fn max_outbound_chars(&self) -> usize {
        20_000
    }

    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
        self.tenant_token().await?;
        self.long_connection
            .test(&self.app_id, self.app_secret.as_str())
            .await
    }

    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (frame_sender, mut frame_receiver) = mpsc::channel(64);
        let connection = self.long_connection.run(
            &self.app_id,
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
                        return Err(ConnectorRuntimeError::new("feishu-frame-stream-closed"));
                    };
                    match normalize_fixture(ConnectorKind::Feishu, &frame.payload) {
                        Ok(message) => submit_inbound(&inbound, message).await?,
                        Err(_) => self.malformed_events.report(),
                    }
                    self.long_connection.acknowledge(&frame.acknowledgement_id).await?;
                }
            }
        }
    }

    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        self.send_final(outbound).await
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
    struct FakeLongConnection {
        acknowledgements: Mutex<Vec<String>>,
    }

    #[async_trait]
    impl FeishuLongConnection for FakeLongConnection {
        async fn test(
            &self,
            _app_id: &str,
            _app_secret: &str,
        ) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _app_id: &str,
            _app_secret: &str,
            frames: mpsc::Sender<FeishuFrame>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            let _ = ready.send(());
            frames
                .send(FeishuFrame {
                    acknowledgement_id: "ack-1".to_string(),
                    payload: include_str!("fixtures/feishu-direct-text.json").to_string(),
                })
                .await
                .map_err(|_| ConnectorRuntimeError::new("frame-channel-closed"))?;
            while !*shutdown.borrow() {
                shutdown
                    .changed()
                    .await
                    .map_err(|_| ConnectorRuntimeError::new("shutdown-channel-closed"))?;
            }
            Ok(())
        }

        async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError> {
            self.acknowledgements
                .lock()
                .unwrap()
                .push(acknowledgement_id.to_string());
            Ok(())
        }
    }

    fn http(responses: Vec<Value>) -> Arc<MockHttp> {
        Arc::new(MockHttp(Mutex::new(
            responses
                .into_iter()
                .map(|body| HttpResponse { status: 200, body })
                .collect(),
        )))
    }

    #[tokio::test]
    async fn validates_token_normalizes_frame_acknowledges_and_sends_final() {
        let http = http(vec![
            json!({"code": 0, "tenant_access_token": "fixture-access", "expire": 3600}),
            json!({"code": 0, "data": {}}),
        ]);
        let connection = Arc::new(FakeLongConnection::default());
        let adapter = Arc::new(
            FeishuAdapter::new("cli_fixture", "app-private", http, connection.clone())
                .unwrap()
                .with_api_base("https://feishu.invalid"),
        );
        adapter.test_connection().await.unwrap();
        let (inbound_sender, mut inbound_receiver) = mpsc::channel(1);
        let (shutdown_sender, shutdown_receiver) = watch::channel(false);
        let (ready_sender, ready_receiver) = oneshot::channel();
        let running = {
            let adapter = adapter.clone();
            tokio::spawn(async move {
                adapter
                    .run(inbound_sender, shutdown_receiver, ready_sender)
                    .await
            })
        };
        ready_receiver.await.unwrap();
        let delivery = inbound_receiver.recv().await.unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert!(connection.acknowledgements.lock().unwrap().is_empty());
        let inbound = delivery.accept();
        assert_eq!(inbound.text, "status please");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(
            connection.acknowledgements.lock().unwrap().as_slice(),
            ["ack-1"]
        );
        adapter
            .send_text(OutboundText {
                chat_id: "oc_fixture".to_string(),
                text: "final".to_string(),
                reply_context: None,
            })
            .await
            .unwrap();
        shutdown_sender.send(true).unwrap();
        running.await.unwrap().unwrap();
    }

    #[tokio::test]
    async fn concurrent_sends_share_one_cached_token_refresh() {
        let mut responses =
            vec![json!({"code": 0, "tenant_access_token": "fixture-access", "expire": 3600})];
        responses.extend((0..8).map(|_| json!({"code": 0, "data": {}})));
        let http = http(responses);
        let adapter = Arc::new(
            FeishuAdapter::new(
                "cli_fixture",
                "app-private",
                http.clone(),
                Arc::new(FakeLongConnection::default()),
            )
            .unwrap()
            .with_api_base("https://feishu.invalid"),
        );
        let sends = (0..8).map(|index| {
            let adapter = Arc::clone(&adapter);
            async move {
                adapter
                    .send_text(OutboundText {
                        chat_id: format!("chat-{index}"),
                        text: "final".to_string(),
                        reply_context: None,
                    })
                    .await
            }
        });
        let results = futures_util::future::join_all(sends).await;
        assert!(results.into_iter().all(|result| result.is_ok()));
        assert!(http.0.lock().unwrap().is_empty());
    }
}
