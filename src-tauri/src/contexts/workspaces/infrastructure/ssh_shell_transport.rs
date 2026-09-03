//! The only place a retained Shell touches `ssh_connections`.
//!
//! Everything the Shell runtime knows about SSH is on the other side of this file. That is what
//! lets the whole remote startup and close path be tested with a deterministic fake, and it is also
//! where the two vocabularies are translated exactly once — a second translation somewhere else is
//! how a transport error would come to mean two different things.

use crate::contexts::ssh_connections::api::{
    SshConnectionsApi, SshExecutionChannel, SshExecutionChannelEvent,
};
use crate::contexts::workspaces::application::{
    RemoteShellChannel, RemoteShellChannelError, RemoteShellEvent, RemoteShellOpenFailure,
    RemoteShellTransport,
};
use async_trait::async_trait;
use std::sync::Arc;

pub(crate) struct SshShellTransport {
    ssh: SshConnectionsApi,
}

impl SshShellTransport {
    pub(crate) fn new(ssh: SshConnectionsApi) -> Self {
        Self { ssh }
    }
}

#[async_trait(?Send)]
impl RemoteShellTransport for SshShellTransport {
    async fn open_channel(
        &self,
        connection_id: &str,
        profile_revision: i64,
        columns: u16,
        rows: u16,
    ) -> Result<Arc<dyn RemoteShellChannel>, RemoteShellOpenFailure> {
        // The revision is checked by `acquire_execution`, which refuses a stale one. A Shell opened
        // against a profile the user has since edited would connect somewhere they did not choose.
        let lease = self
            .ssh
            .acquire_execution(connection_id, profile_revision)
            .await
            .map_err(|_| RemoteShellOpenFailure::ConnectionUnavailable)?;
        let channel = lease
            .open_pty(columns, rows)
            .await
            .map_err(|_| RemoteShellOpenFailure::ChannelUnavailable)?;
        Ok(Arc::new(SshShellChannel { channel }))
    }
}

struct SshShellChannel {
    channel: SshExecutionChannel,
}

#[async_trait(?Send)]
impl RemoteShellChannel for SshShellChannel {
    async fn write(&self, content: &[u8]) -> Result<(), RemoteShellChannelError> {
        self.channel
            .write(content)
            .await
            .map_err(|_| RemoteShellChannelError)
    }

    async fn resize(&self, columns: u16, rows: u16) -> Result<(), RemoteShellChannelError> {
        self.channel
            .resize(columns, rows)
            .await
            .map_err(|_| RemoteShellChannelError)
    }

    async fn next_event(&self) -> Result<Option<RemoteShellEvent>, RemoteShellChannelError> {
        match self.channel.next_event().await {
            Ok(Some(event)) => Ok(shell_event(event)),
            Ok(None) => Ok(None),
            Err(_) => Err(RemoteShellChannelError),
        }
    }

    async fn close(&self) -> Result<(), RemoteShellChannelError> {
        self.channel
            .close()
            .await
            .map_err(|_| RemoteShellChannelError)
    }
}

/// Translates one SSH event, or reports the end of the stream.
///
/// `Closed` becomes `None` rather than an event: from the Shell's side "the channel closed" and
/// "there is nothing more coming" are the same fact, and keeping two spellings of it would mean two
/// branches that have to agree forever.
///
/// A signal exit keeps `code: None`. Substituting `0` there would report a killed process as a
/// clean one, which is the reading a user is least able to recover from.
fn shell_event(event: SshExecutionChannelEvent) -> Option<RemoteShellEvent> {
    match event {
        // One merged stream: an SSH PTY interleaves them, and labelling either separately would
        // claim a separation nobody made.
        SshExecutionChannelEvent::Output(bytes)
        | SshExecutionChannelEvent::ExtendedOutput { content: bytes, .. } => {
            Some(RemoteShellEvent::Output(bytes))
        }
        SshExecutionChannelEvent::ExitStatus(code) => Some(RemoteShellEvent::Exited {
            code: Some(code as i32),
        }),
        SshExecutionChannelEvent::ExitSignal(_) => Some(RemoteShellEvent::Exited { code: None }),
        SshExecutionChannelEvent::Eof => Some(RemoteShellEvent::Eof),
        SshExecutionChannelEvent::Closed => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_ssh_event_maps_to_exactly_one_shell_event() {
        assert_eq!(
            shell_event(SshExecutionChannelEvent::Output(b"hi".to_vec())),
            Some(RemoteShellEvent::Output(b"hi".to_vec()))
        );
        assert_eq!(
            shell_event(SshExecutionChannelEvent::ExtendedOutput {
                stream: 1,
                content: b"err".to_vec(),
            }),
            Some(RemoteShellEvent::Output(b"err".to_vec())),
            "stderr must merge into the one stream a PTY actually produces"
        );
        assert_eq!(
            shell_event(SshExecutionChannelEvent::ExitStatus(3)),
            Some(RemoteShellEvent::Exited { code: Some(3) })
        );
        assert_eq!(
            shell_event(SshExecutionChannelEvent::Eof),
            Some(RemoteShellEvent::Eof)
        );
        // The channel ending is the absence of an event, not an event.
        assert_eq!(shell_event(SshExecutionChannelEvent::Closed), None);
    }

    #[test]
    fn a_signal_exit_never_becomes_a_clean_one() {
        // The runtime saw it end and did not see a code. Substituting `0` would report a killed
        // process as a successful one, which is the reading a user is least able to recover from.
        assert_eq!(
            shell_event(SshExecutionChannelEvent::ExitSignal("TERM".to_string())),
            Some(RemoteShellEvent::Exited { code: None })
        );
    }
}
