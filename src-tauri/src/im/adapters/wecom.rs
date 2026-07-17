use crate::im::models::{ConnectorKind, OutboundText};
use crate::im::protocol::normalize_fixture;
use crate::im::runtime::{submit_inbound, ConnectorAdapter, ConnectorRuntimeError, InboundDelivery};
use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, watch};
use zeroize::Zeroizing;

#[derive(Debug, Clone)]
pub struct WeComFrame {
    pub acknowledgement_id: String,
    pub payload: String,
}

#[async_trait]
pub trait WeComLongConnection: Send + Sync {
    async fn test(&self, bot_id: &str, secret: &str) -> Result<(), ConnectorRuntimeError>;
    async fn run(
        &self,
        bot_id: &str,
        secret: &str,
        frames: mpsc::Sender<WeComFrame>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError>;
    async fn acknowledge(&self, acknowledgement_id: &str) -> Result<(), ConnectorRuntimeError>;
    async fn send_text(
        &self,
        chat_id: &str,
        request_context: &str,
        text: &str,
    ) -> Result<(), ConnectorRuntimeError>;
}

pub struct WeComAdapter {
    bot_id: String,
    secret: Zeroizing<String>,
    connection: Arc<dyn WeComLongConnection>,
}

impl WeComAdapter {
    pub fn new(
        bot_id: &str,
        secret: &str,
        connection: Arc<dyn WeComLongConnection>,
    ) -> Result<Self, ConnectorRuntimeError> {
        let bot_id = bot_id.trim();
        let secret = secret.trim();
        if bot_id.is_empty() || secret.is_empty() {
            return Err(ConnectorRuntimeError::new("wecom-credentials-invalid"));
        }
        Ok(Self {
            bot_id: bot_id.to_string(),
            secret: Zeroizing::new(secret.to_string()),
            connection,
        })
    }
}

#[async_trait]
impl ConnectorAdapter for WeComAdapter {
    fn kind(&self) -> ConnectorKind {
        ConnectorKind::WeCom
    }

    fn max_outbound_chars(&self) -> usize {
        2_000
    }

    async fn test_connection(&self) -> Result<(), ConnectorRuntimeError> {
        self.connection
            .test(&self.bot_id, self.secret.as_str())
            .await
    }

    async fn run(
        &self,
        inbound: mpsc::Sender<InboundDelivery>,
        shutdown: watch::Receiver<bool>,
        ready: oneshot::Sender<()>,
    ) -> Result<(), ConnectorRuntimeError> {
        let (frame_sender, mut frame_receiver) = mpsc::channel(64);
        let connection =
            self.connection
                .run(&self.bot_id, self.secret.as_str(), frame_sender, shutdown, ready);
        tokio::pin!(connection);
        loop {
            tokio::select! {
                result = &mut connection => return result,
                frame = frame_receiver.recv() => {
                    let Some(frame) = frame else {
                        return Err(ConnectorRuntimeError::new("wecom-frame-stream-closed"));
                    };
                    if let Ok(message) = normalize_fixture(ConnectorKind::WeCom, &frame.payload) {
                        submit_inbound(&inbound, message).await?;
                    }
                    self.connection.acknowledge(&frame.acknowledgement_id).await?;
                }
            }
        }
    }

    async fn send_text(&self, outbound: OutboundText) -> Result<(), ConnectorRuntimeError> {
        let context = outbound
            .reply_context
            .ok_or_else(|| ConnectorRuntimeError::new("wecom-reply-context-missing"))?;
        self.connection
            .send_text(&outbound.chat_id, &context, &outbound.text)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeConnection {
        acknowledgements: Mutex<Vec<String>>,
        outbound: Mutex<Vec<(String, String, String)>>,
    }

    #[async_trait]
    impl WeComLongConnection for FakeConnection {
        async fn test(&self, _bot_id: &str, _secret: &str) -> Result<(), ConnectorRuntimeError> {
            Ok(())
        }

        async fn run(
            &self,
            _bot_id: &str,
            _secret: &str,
            frames: mpsc::Sender<WeComFrame>,
            mut shutdown: watch::Receiver<bool>,
            ready: oneshot::Sender<()>,
        ) -> Result<(), ConnectorRuntimeError> {
            let _ = ready.send(());
            frames
                .send(WeComFrame {
                    acknowledgement_id: "ack-wc".to_string(),
                    payload: include_str!("../fixtures/wecom-direct-text.json").to_string(),
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
            self.acknowledgements.lock().unwrap().push(id.to_string());
            Ok(())
        }

        async fn send_text(
            &self,
            chat_id: &str,
            request_context: &str,
            text: &str,
        ) -> Result<(), ConnectorRuntimeError> {
            self.outbound.lock().unwrap().push((
                chat_id.to_string(),
                request_context.to_string(),
                text.to_string(),
            ));
            Ok(())
        }
    }

    #[tokio::test]
    async fn authenticates_acknowledges_and_replies_with_inbound_context() {
        let connection = Arc::new(FakeConnection::default());
        let adapter =
            Arc::new(WeComAdapter::new("bot-id", "bot-private", connection.clone()).unwrap());
        adapter.test_connection().await.unwrap();
        let (sender, mut receiver) = mpsc::channel(1);
        let (stop, shutdown) = watch::channel(false);
        let (ready, connected) = oneshot::channel();
        let worker = {
            let adapter = adapter.clone();
            tokio::spawn(async move { adapter.run(sender, shutdown, ready).await })
        };
        connected.await.unwrap();
        let message = receiver.recv().await.unwrap().accept();
        adapter
            .send_text(OutboundText {
                chat_id: message.chat_id,
                text: "final".into(),
                reply_context: message.reply_context,
            })
            .await
            .unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        assert_eq!(
            connection.acknowledgements.lock().unwrap().as_slice(),
            ["ack-wc"]
        );
        assert_eq!(connection.outbound.lock().unwrap()[0].1, "wc-fixture-001");
        stop.send(true).unwrap();
        worker.await.unwrap().unwrap();
    }
}
