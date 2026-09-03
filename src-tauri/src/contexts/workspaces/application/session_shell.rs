//! What a retained Session Shell looks like from the application layer, and what it needs.

use super::session_shell_close::ShellRuntimeCloseOutcome;
use crate::contexts::workspaces::domain::{
    SessionShellError, SessionShellState, ShellAttachmentId, ShellCloseBudget,
    ShellCreateRequestId, ShellForegroundProcessState, ShellGeneration, ShellId, ShellOutputFrame,
    ShellReplayGap, ShellRuntimeDescriptor, ShellStream, ShellTitle, TerminalDimensions,
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
    /// Which life of this Shell id the descriptor describes.
    ///
    /// Carried on the descriptor rather than kept private to the store because every late arrival
    /// that has to be classified as stale — a worker completion, a reaper attempt, a retry from the
    /// UI — is comparing against something a reader was handed.
    pub(crate) generation: ShellGeneration,
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
    /// Where the Shell starts, relative to the workspace root. Absent means the root itself.
    ///
    /// Worth being precise about what this is and is not. A Shell can `cd` anywhere the user's
    /// account can reach the moment it opens, so confining this value is not a sandbox and nothing
    /// here should be read as one. What it prevents is *this application* starting a Shell somewhere
    /// the reader did not pick — a path assembled from a stale tree row, or one that escaped the
    /// root through a symlink nobody noticed.
    pub(crate) working_directory: Option<String>,
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
    /// Stamped on every worker event, route entry, and retained handle this open produces, so a
    /// completion arriving after the Shell was replaced can be told apart from a current one.
    pub(crate) generation: ShellGeneration,
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
/// Every call carries the generation the worker was started for. A reader thread can outlive the
/// Shell it was reading — that is the ordinary case on a close that timed out — and without the
/// generation its next frame is indistinguishable from output belonging to whatever now answers to
/// that id.
pub(crate) trait ShellOutputSink: Send + Sync {
    fn on_output(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        stream: ShellStream,
        bytes: &[u8],
    );

    fn on_state(&self, shell_id: &ShellId, generation: ShellGeneration, state: SessionShellState);
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

    /// Makes one bounded attempt at ending a Shell, and says what it achieved.
    ///
    /// No `Result`, deliberately. Every way this can go wrong leaves the adapter still owning a
    /// child, a channel, or a worker, so "it failed" and "it is still mine" are the same fact — and
    /// a `Result` invites the one line of code that loses a live process, `let _ = close(..)`.
    ///
    /// The adapter MUST NOT wait without a ceiling, MUST NOT join a worker that has not reported
    /// itself complete, and MUST NOT remove its own ownership entry unless it returns `Confirmed`.
    /// A generation it does not hold is `NotHeld`, which is not an error: a registry entry can
    /// outlive its process, and failing on that would make cleanup unreliable exactly where it
    /// matters.
    fn close(
        &self,
        shell_id: &ShellId,
        generation: ShellGeneration,
        budget: ShellCloseBudget,
    ) -> ShellRuntimeCloseOutcome;

    /// What the runtime can honestly say about foreground work. `Unknown` is a real answer.
    fn foreground_process(&self, shell_id: &ShellId) -> ShellForegroundProcessState;
}

/// Where a Shell may be opened, resolved from the registered session rather than from a client.
pub(crate) trait SessionShellWorkspacePort: Send + Sync {
    fn resolve(&self, session_id: &str) -> Result<SessionShellWorkspace, SessionShellError>;

    /// The same workspace, starting in one of its subdirectories.
    ///
    /// A second method rather than an `Option` on the first, so every existing caller keeps the
    /// meaning it had. The relative path is resolved where the filesystem is, because only that
    /// side can tell a symlink from a directory.
    fn resolve_at(
        &self,
        session_id: &str,
        relative_directory: &str,
    ) -> Result<SessionShellWorkspace, SessionShellError>;
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
        /// Which life of the Shell changed. A subscriber that has already replaced this Shell uses
        /// it to discard the notice rather than apply an old ending to a new process.
        generation: ShellGeneration,
        session_id: String,
        state: SessionShellState,
        revision: u64,
        occurred_at: String,
    },
}

pub(crate) trait SessionShellNoticePort: Send + Sync {
    fn publish(&self, notice: SessionShellNotice);
}
