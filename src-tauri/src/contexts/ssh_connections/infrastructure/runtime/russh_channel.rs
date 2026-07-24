use super::russh_adapter::HostCheckingHandler;
use crate::contexts::ssh_connections::application::runtime::{
    RemoteSshChannelPort, RemoteSshError, RemoteSshTransportPort,
};
use crate::contexts::ssh_connections::domain::runtime::{RemotePtyRequest, RemoteSshChannelEvent};
use async_trait::async_trait;
use russh::client;
use russh::{ChannelMsg, ChannelReadHalf, ChannelWriteHalf, Disconnect};
use std::sync::Arc;

pub(super) struct RusshTransport {
    pub(super) handle: Arc<client::Handle<HostCheckingHandler>>,
}

#[async_trait]
impl RemoteSshTransportPort for RusshTransport {
    async fn open_pty(
        &self,
        request: RemotePtyRequest,
    ) -> Result<Arc<dyn RemoteSshChannelPort>, RemoteSshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)?;
        channel
            .request_pty(
                true,
                "xterm-256color",
                request.columns,
                request.rows,
                0,
                0,
                &[],
            )
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)?;
        channel
            .request_shell(true)
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)?;
        Ok(Arc::new(RusshChannel::new(channel)))
    }

    async fn open_exec(
        &self,
        command: &[u8],
    ) -> Result<Arc<dyn RemoteSshChannelPort>, RemoteSshError> {
        let channel = self
            .handle
            .channel_open_session()
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)?;
        channel
            .exec(true, command)
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)?;
        Ok(Arc::new(RusshChannel::new(channel)))
    }

    async fn keepalive(&self) -> Result<(), RemoteSshError> {
        self.handle
            .send_ping()
            .await
            .map_err(|_| RemoteSshError::TransportClosed)
    }

    async fn close(&self) -> Result<(), RemoteSshError> {
        self.handle
            .disconnect(Disconnect::ByApplication, "", "en")
            .await
            .map_err(|_| RemoteSshError::TransportClosed)
    }

    fn is_healthy(&self) -> bool {
        !self.handle.is_closed()
    }
}

struct RusshChannel {
    reader: tokio::sync::Mutex<ChannelReadHalf>,
    writer: ChannelWriteHalf<client::Msg>,
}

impl RusshChannel {
    fn new(channel: russh::Channel<client::Msg>) -> Self {
        let (reader, writer) = channel.split();
        Self {
            reader: tokio::sync::Mutex::new(reader),
            writer,
        }
    }
}

#[async_trait]
impl RemoteSshChannelPort for RusshChannel {
    async fn write(&self, content: &[u8]) -> Result<(), RemoteSshError> {
        self.writer
            .data_bytes(content.to_vec())
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)
    }

    async fn resize(&self, request: RemotePtyRequest) -> Result<(), RemoteSshError> {
        self.writer
            .window_change(request.columns, request.rows, 0, 0)
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)
    }

    async fn next_event(&self) -> Result<Option<RemoteSshChannelEvent>, RemoteSshError> {
        let message = self.reader.lock().await.wait().await;
        Ok(message.map(map_channel_event))
    }

    async fn close(&self) -> Result<(), RemoteSshError> {
        self.writer
            .close()
            .await
            .map_err(|_| RemoteSshError::ChannelFailed)
    }
}

fn map_channel_event(message: ChannelMsg) -> RemoteSshChannelEvent {
    match message {
        ChannelMsg::Data { data } => RemoteSshChannelEvent::Output(data.to_vec()),
        ChannelMsg::ExtendedData { data, ext } => RemoteSshChannelEvent::ExtendedOutput {
            stream: ext,
            content: data.to_vec(),
        },
        ChannelMsg::ExitStatus { exit_status } => RemoteSshChannelEvent::ExitStatus(exit_status),
        ChannelMsg::ExitSignal { signal_name, .. } => {
            RemoteSshChannelEvent::ExitSignal(format!("{signal_name:?}"))
        }
        ChannelMsg::Eof => RemoteSshChannelEvent::Eof,
        _ => RemoteSshChannelEvent::Closed,
    }
}
