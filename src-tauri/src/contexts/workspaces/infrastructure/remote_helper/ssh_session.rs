//! The SSH-backed half of the helper session.
//!
//! The only file in this context that touches `ssh_connections`, and it touches only the published
//! API: a lease, an exec channel, and the four things a channel can do. The pool, the transport,
//! and the profile store stay where they are — a workspace inspection that reached into them would
//! make every connection-management change a workspace change too.

use super::protocol::RemoteHelperError;
use super::transport::{
    RemoteHelperChannel, RemoteHelperEvent, RemoteHelperSession, HELPER_BOOTSTRAP_COMMAND,
};
use crate::contexts::ssh_connections::api::{
    SshConnectionsApi, SshExecutionChannel, SshExecutionChannelEvent,
};
use async_trait::async_trait;

pub(crate) struct SshRemoteHelperSession {
    ssh: SshConnectionsApi,
}

impl SshRemoteHelperSession {
    pub(crate) fn new(ssh: SshConnectionsApi) -> Self {
        Self { ssh }
    }
}

#[async_trait]
impl RemoteHelperSession for SshRemoteHelperSession {
    async fn open(
        &self,
        connection_id: &str,
        revision: i64,
    ) -> Result<Box<dyn RemoteHelperChannel>, RemoteHelperError> {
        // Acquiring runs on the blocking pool because the connection pool holds a std mutex across
        // an await, which makes its future non-`Send`. The retained Shell reaches the same code the
        // same way; doing it inline here would make this trait non-`Send` and that would spread to
        // every provider method above it.
        //
        // The lease is dropped when the closure ends and the channel outlives it, which is the
        // arrangement the Shell already depends on: a lease governs acquisition, not the life of
        // what it opened.
        let ssh = self.ssh.clone();
        let connection_id = connection_id.to_string();
        let channel = tauri::async_runtime::spawn_blocking(move || {
            // `acquire_execution` refuses a revision that no longer matches, so a stale binding
            // fails before a channel exists rather than after a request has been sent to a host
            // the user has since reconfigured.
            let lease =
                tauri::async_runtime::block_on(ssh.acquire_execution(&connection_id, revision))
                    .map_err(|_| RemoteHelperError::ConnectionFailed)?;
            tauri::async_runtime::block_on(lease.open_exec(HELPER_BOOTSTRAP_COMMAND.as_bytes()))
                .map_err(|_| RemoteHelperError::ChannelFailed)
        })
        .await
        .map_err(|_| RemoteHelperError::ConnectionFailed)??;
        Ok(Box::new(SshHelperChannel { channel }))
    }
}

struct SshHelperChannel {
    channel: SshExecutionChannel,
}

#[async_trait]
impl RemoteHelperChannel for SshHelperChannel {
    async fn write(&self, bytes: &[u8]) -> Result<(), RemoteHelperError> {
        self.channel
            .write(bytes)
            .await
            .map_err(|_| RemoteHelperError::ChannelFailed)
    }

    async fn send_eof(&self) -> Result<(), RemoteHelperError> {
        self.channel
            .send_eof()
            .await
            .map_err(|_| RemoteHelperError::ChannelFailed)
    }

    async fn next_event(&self) -> Result<Option<RemoteHelperEvent>, RemoteHelperError> {
        let event = self
            .channel
            .next_event()
            .await
            .map_err(|_| RemoteHelperError::ChannelFailed)?;
        Ok(event.map(|event| match event {
            SshExecutionChannelEvent::Output(bytes) => RemoteHelperEvent::Stdout(bytes),
            // The content is dropped here rather than carried and discarded later: the further it
            // travels, the more places have to remember not to log it.
            SshExecutionChannelEvent::ExtendedOutput { .. } => RemoteHelperEvent::Stderr,
            SshExecutionChannelEvent::ExitStatus(code) => RemoteHelperEvent::Exited(code),
            // A signal is not an exit code, and inventing one would report a specific failure the
            // remote never gave. It ends the exchange the same way, which is all the caller needs.
            SshExecutionChannelEvent::ExitSignal(_) => RemoteHelperEvent::Ended,
            SshExecutionChannelEvent::Eof | SshExecutionChannelEvent::Closed => {
                RemoteHelperEvent::Ended
            }
        }))
    }

    async fn close(&self) -> Result<(), RemoteHelperError> {
        self.channel
            .close()
            .await
            .map_err(|_| RemoteHelperError::ChannelFailed)
    }
}
