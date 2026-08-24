//! What a retained Session Shell looks like from the application layer, and what it needs.

use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellAttachmentId, ShellCreateRequestId,
    ShellForegroundProcessState, ShellId, ShellOutputFrame, ShellReplayGap, ShellRuntimeDescriptor,
    ShellStream, ShellTitle, TerminalDimensions,
};

/// How many Shells may exist at once.
///
/// Two ceilings rather than one. A single session opening fifty shells and fifty sessions opening
/// one each are different problems, and a single number would either permit the first or forbid
/// the second.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ShellCapacities {
    pub(crate) per_session: usize,
    pub(crate) total: usize,
}

impl Default for ShellCapacities {
    fn default() -> Self {
        Self {
            per_session: 8,
            total: 32,
        }
    }
}

/// One retained Shell, as a reader sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionShellDescriptor {
    pub(crate) shell_id: ShellId,
    pub(crate) session_id: String,
    /// Which participant owns it. `None` only in a single-seat session.
    pub(crate) seat_id: Option<String>,
    pub(crate) title: ShellTitle,
    pub(crate) runtime: ShellRuntimeDescriptor,
    pub(crate) state: SessionShellState,
    pub(crate) created_at: String,
    pub(crate) last_activity_at: String,
    /// Counts descriptor changes, not frames.
    ///
    /// A view compares revisions to decide whether a state notice is newer than what it holds. A
    /// timestamp cannot answer that when two changes land inside one clock tick, and the output
    /// sequence cannot either — output moves while the descriptor stands still.
    pub(crate) revision: u64,
    pub(crate) foreground_process: ShellForegroundProcessState,
}

/// What an attaching view receives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellAttachSnapshot {
    /// This view's claim on the Shell. Detach, write, and resize carry it back.
    pub(crate) attachment_id: ShellAttachmentId,
    pub(crate) descriptor: SessionShellDescriptor,
    pub(crate) replay: Vec<ShellOutputFrame>,
    pub(crate) next_sequence: u64,
    pub(crate) gap: Option<ShellReplayGap>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateSessionShellRequest {
    pub(crate) session_id: String,
    pub(crate) seat_id: Option<String>,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
    /// The client's idempotency key.
    ///
    /// Absent means "the default Shell for this session and seat", which is what a tab opening for
    /// the first time asks for; two of those racing must produce one Shell. Present means the user
    /// pressed Add, and a retry of that same press must not produce a second one.
    pub(crate) request_id: Option<ShellCreateRequestId>,
    pub(crate) title: Option<ShellTitle>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AttachSessionShellRequest {
    pub(crate) shell_id: ShellId,
    /// The last sequence the view consumed; 0 when it has consumed nothing.
    pub(crate) after_sequence: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellAttachmentScope {
    pub(crate) shell_id: ShellId,
    pub(crate) attachment_id: ShellAttachmentId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WriteSessionShellRequest {
    pub(crate) scope: ShellAttachmentScope,
    pub(crate) content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResizeSessionShellRequest {
    pub(crate) scope: ShellAttachmentScope,
    pub(crate) rows: u16,
    pub(crate) cols: u16,
}

/// What the runtime needs to open one Shell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRuntimeOpen {
    pub(crate) shell_id: ShellId,
    pub(crate) session_id: String,
    pub(crate) root: String,
    pub(crate) dimensions: TerminalDimensions,
    pub(crate) remote: Option<ShellRemoteTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRemoteTarget {
    pub(crate) connection_id: String,
    pub(crate) profile_revision: i64,
    pub(crate) path: String,
}

/// A live Shell's process or channel.
///
/// The runtime owns the handle and the registry owns the descriptor, which is why opening returns a
/// descriptor fragment rather than a process: the registry must never learn what a PTY is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShellRuntimeOpened {
    pub(crate) runtime: ShellRuntimeDescriptor,
    pub(crate) state: SessionShellState,
}

/// Where output and state changes arrive from.
///
/// The runtime calls this from its own worker. It is separate from the notice port because these
/// two have different jobs: this one records into the retained buffer, and the notice port tells a
/// subscriber that something changed. Recording first is what makes a notice safe to drop.
pub(crate) trait ShellOutputSink: Send + Sync {
    fn on_output(&self, shell_id: &ShellId, stream: ShellStream, bytes: &[u8]);

    fn on_state(&self, shell_id: &ShellId, state: SessionShellState);
}

pub(crate) trait SessionShellRuntimePort: Send + Sync {
    fn open(
        &self,
        request: &ShellRuntimeOpen,
        sink: std::sync::Arc<dyn ShellOutputSink>,
    ) -> Result<ShellRuntimeOpened, SessionShellError>;

    fn write(&self, shell_id: &ShellId, content: &str) -> Result<(), SessionShellError>;

    fn resize(
        &self,
        shell_id: &ShellId,
        dimensions: TerminalDimensions,
    ) -> Result<(), SessionShellError>;

    /// Terminates one Shell and joins its workers. Closing a Shell the runtime does not hold is a
    /// success: a registry entry can outlive its process, and a close that failed on that would
    /// make cleanup unreliable exactly when it matters.
    fn close(&self, shell_id: &ShellId) -> Result<(), SessionShellError>;

    /// What the runtime can honestly say about foreground work. `Unknown` is a real answer.
    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState;
}

/// Where a Shell may be opened, resolved from the registered session rather than from a client.
pub(crate) trait SessionShellWorkspacePort: Send + Sync {
    fn resolve(&self, session_id: &str) -> Result<SessionShellWorkspace, SessionShellError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SessionShellWorkspace {
    pub(crate) root: String,
    pub(crate) remote: Option<ShellRemoteTarget>,
    pub(crate) read_only: bool,
    /// How many participants the session has. One means a Shell needs no seat; more than one means
    /// it needs a concrete owner, because a Shell is one interactive channel with one runtime.
    pub(crate) seat_count: usize,
}

pub(crate) trait ShellClockPort: Send + Sync {
    fn now(&self) -> String;

    /// Monotonic milliseconds, for idle arithmetic that a wall-clock adjustment must not disturb.
    fn elapsed_millis(&self) -> u64;
}

pub(crate) trait ShellIdPort: Send + Sync {
    fn next_shell_id(&self) -> String;

    fn next_attachment_id(&self) -> String;
}

/// One bounded notice per change. Replay never travels this way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SessionShellNotice {
    Output {
        shell_id: ShellId,
        session_id: String,
        frame: ShellOutputFrame,
    },
    State {
        shell_id: ShellId,
        session_id: String,
        state: SessionShellState,
        revision: u64,
        occurred_at: String,
    },
}

pub(crate) trait SessionShellNoticePort: Send + Sync {
    fn publish(&self, notice: SessionShellNotice);
}
