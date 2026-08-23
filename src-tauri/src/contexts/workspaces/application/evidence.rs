/// What the workspaces context is willing to say about its own work.
///
/// The vocabulary is this context's, not the journal's. Nothing here names an
/// `ExecutionEvidenceEvent`, a payload, or a repository: a bootstrap adapter translates these into
/// whatever the journal accepts, and moving that translation inside this context would make the
/// evidence aggregate a dependency of every shell operation.
///
/// Every field is an identifier or a closed classification. A shell's command text, its output,
/// its working directory, and the remote host it reached are all absent by construction — they
/// cannot be added without adding a field here, which is the review point this shape exists for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceEvidenceSignal {
    ShellOpened {
        session_id: String,
        shell_id: String,
        seat_id: Option<String>,
        /// Which runtime opened it, not where it opened. `local` and `remote` are the whole
        /// vocabulary; a hostname or a path would make this a location record.
        runtime: WorkspaceShellRuntimeKind,
        occurred_at: String,
    },
    /// The shell reached a terminal state. Attach and detach are not this: a client reconnecting
    /// to a live shell has not opened one, and reporting it as a close would end a shell in the
    /// journal that is still running.
    ShellClosed {
        session_id: String,
        shell_id: String,
        seat_id: Option<String>,
        reason: WorkspaceShellCloseReason,
        occurred_at: String,
    },
    /// A file the runtime changed and then confirmed changed.
    ///
    /// Only after a trusted mutation succeeded or a witnessed snapshot comparison saw it. A
    /// rejected write must produce nothing: a record of a change that did not happen is worse than
    /// no record, because nothing downstream can tell the two apart.
    FileMutationObserved {
        session_id: String,
        /// The file's own name, never the directory it sits in. A path would say where the user
        /// works; a basename says which file changed, which is what a reader needs.
        basename: String,
        /// A stable digest of the workspace-relative path. Two changes to one file group by it
        /// without the path itself ever being stored.
        path_fingerprint: String,
        change_kind: WorkspaceFileChangeKind,
        /// What the runtime compared against. A change reported without one cannot be told from a
        /// change someone else made between two reads.
        witness_fingerprint: String,
        observed_directly: bool,
        occurred_at: String,
    },
}

/// Why a shell ended. Closed, because a reader groups by it and free text does not group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceShellCloseReason {
    /// A user or an agent asked for it.
    ExplicitClose,
    /// The process on the other end exited on its own.
    ProcessExit,
    /// A remote workspace's connection dropped.
    RemoteDisconnect,
    /// Reclaimed after going unused.
    IdleCleanup,
    /// The session or the application is going away.
    Shutdown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceFileChangeKind {
    Created,
    Modified,
    Deleted,
    Renamed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceShellRuntimeKind {
    Local,
    Remote,
}

/// Where workspaces hands an observation off.
///
/// `try_publish` returns nothing and cannot fail from the caller's point of view. That is the
/// contract, not an oversight: a shell that opened successfully has opened, whether or not the
/// journal heard about it, and letting the journal's back-pressure change that result would make
/// observation a precondition of the work being observed.
pub(crate) trait WorkspaceEvidencePort: Send + Sync {
    fn try_publish(&self, signal: WorkspaceEvidenceSignal);
}

/// The default. A build with no evidence bridge assembled still runs every shell operation.
pub(crate) struct NoWorkspaceEvidence;

impl WorkspaceEvidencePort for NoWorkspaceEvidence {
    fn try_publish(&self, _signal: WorkspaceEvidenceSignal) {}
}
