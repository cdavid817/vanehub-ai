//! What a retained Shell needs from a remote channel, stated by the side that needs it.
//!
//! `ssh_connections` publishes a concrete `SshExecutionChannel`. Depending on it directly left the
//! whole remote startup and close path — the launch guard, the route commit, the bounded close, the
//! shared-transport isolation — with no way to be tested at all: every one of those branches needs a
//! channel that fails in a specific way at a specific moment, and a real SSH connection cannot be
//! asked to do that. Production code with no reachable test is the worst of both worlds; it looks
//! covered because the file has tests, and the branches that actually carry the risk have none.
//!
//! So the dependency is inverted. `workspaces` declares the two operations it uses, an adapter in
//! its own infrastructure satisfies them over `ssh_connections::api`, and the failure modes become
//! ordinary values a test can produce.
//!
//! The port is deliberately narrower than `SshExecutionChannel`. `send_eof` and `open_exec` exist
//! there for the remote helper protocol; a Shell has no use for either, and a port that offered
//! them would invite one.

use async_trait::async_trait;
use std::sync::Arc;

/// Why a remote Shell could not be opened.
///
/// Two causes, kept apart because a reader acts differently on them: the connection being
/// unavailable is about the host or the profile, and a channel failing on a healthy connection is
/// about this Shell. Collapsing them would make "the server is down" and "you have too many
/// terminals open" the same message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RemoteShellOpenFailure {
    ConnectionUnavailable,
    ChannelUnavailable,
}

/// A remote channel operation failed.
///
/// Opaque on purpose. The underlying error carries a host, a command, or a transport message, and
/// none of those may cross this boundary — the Shell surfaces stable reason codes and this type is
/// what stops a caller from formatting something else instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RemoteShellChannelError;

/// Something the remote end sent.
///
/// `Eof` is kept rather than folded into the end of the stream: a remote program can close its
/// output while the channel stays open and the user keeps typing, and treating that as the end
/// would tear down a live Shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteShellEvent {
    Output(Vec<u8>),
    /// The remote program ended. `None` when the runtime reported a signal or a closed channel
    /// rather than a code — never `0`, which would report an unknown ending as a clean one.
    Exited {
        code: Option<i32>,
    },
    Eof,
}

/// One remote channel, for the lifetime of one Shell.
///
/// `?Send` on the futures, not on the trait object. The connection pool behind the real adapter
/// holds a `std::sync::MutexGuard` across an await, so its futures are not `Send` — and they never
/// need to be: every call here is created and driven to completion on one thread by `block_on`,
/// including the reader worker's own. Requiring `Send` would mean either lying about the pool or
/// rewriting it, and neither is this change's business.
#[async_trait(?Send)]
pub(crate) trait RemoteShellChannel: Send + Sync {
    async fn write(&self, content: &[u8]) -> Result<(), RemoteShellChannelError>;

    async fn resize(&self, columns: u16, rows: u16) -> Result<(), RemoteShellChannelError>;

    /// The next event, or `Ok(None)` when the stream has ended.
    async fn next_event(&self) -> Result<Option<RemoteShellEvent>, RemoteShellChannelError>;

    /// Ends this channel. Never the transport: it may be carrying other Shells.
    async fn close(&self) -> Result<(), RemoteShellChannelError>;
}

/// Opens one channel for one Shell on a pooled transport.
///
/// The pool is not exposed and neither is the lease. A Shell decides nothing about how many
/// connections exist, which is what keeps one closing Shell from taking unrelated terminals with
/// it — and what makes "two Shells on one transport" a thing a test can set up.
///
/// `?Send` for the same reason as [`RemoteShellChannel`].
#[async_trait(?Send)]
pub(crate) trait RemoteShellTransport: Send + Sync {
    async fn open_channel(
        &self,
        connection_id: &str,
        profile_revision: i64,
        columns: u16,
        rows: u16,
    ) -> Result<Arc<dyn RemoteShellChannel>, RemoteShellOpenFailure>;
}
