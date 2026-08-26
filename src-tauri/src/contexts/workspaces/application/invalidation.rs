//! Telling a console that what it is showing has changed.
//!
//! Three things can notice a workspace change and none of them notice the same thing. A filesystem
//! watcher sees a path event. A bounded poll sees that a directory's entries no longer match what it
//! last read, without knowing which entry moved. The runtime's own writes are known exactly, down to
//! the file, before anything has to be observed at all.
//!
//! Normalising them is not cosmetic. A consumer that branched on which source spoke would end up
//! with three refresh policies, and the two rarer ones would be the ones nobody tested. So every
//! source produces the same value, and it says only what its source actually established: a poll
//! that cannot name the file says `Directory`, and a source that cannot name the directory says
//! `Workspace`. None of them may guess a path, because a guessed path invalidates the wrong query
//! and leaves the right one showing stale content — the exact failure the notice exists to prevent.
//!
//! The source still travels with the notice, because a reader needs it for a different question:
//! not *what* to refresh but *how much to believe*. "Modified, seen by the watcher" and "modified,
//! inferred by a poll up to thirty seconds ago" are the same instruction and different guarantees.

/// Which mechanism noticed.
///
/// Kept out of the refresh decision on purpose. It answers how stale the observation may be, and
/// nothing about which queries are affected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WorkspaceInvalidationSource {
    /// A filesystem watcher on this machine reported it. As immediate as this platform gets.
    Watch,
    /// A bounded comparison against what the last poll read. True as of that poll, not as of now.
    Poll,
    /// The runtime performed the write and confirmed it. Exact, and limited to changes this
    /// application caused: an edit made in another editor produces nothing here.
    ExecutionEvidence,
}

impl WorkspaceInvalidationSource {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Watch => "watch",
            Self::Poll => "poll",
            Self::ExecutionEvidence => "execution-evidence",
        }
    }
}

/// What happened to a path, when that is known.
///
/// `Unknown` is a real answer rather than a default. A watcher on some platforms reports that a
/// directory entry changed without saying how, and a poll that sees a differing entry list knows
/// something moved but not whether it arrived or left. Calling either of those `Modified` would put
/// a claim in the record that nobody made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WorkspaceInvalidationChange {
    Created,
    Modified,
    Removed,
    Unknown,
}

impl WorkspaceInvalidationChange {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Created => "created",
            Self::Modified => "modified",
            Self::Removed => "removed",
            Self::Unknown => "unknown",
        }
    }
}

/// How much of the workspace this notice is about.
///
/// Ordered from most specific to least, and a producer must use the most specific one it can
/// actually justify. The consumer's refresh cost rises steeply down this list — `Workspace` means
/// every open directory, preview, status and diff query — so a producer that reaches for the broad
/// variant when it knew the path is buying itself simplicity with the user's latency.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum WorkspaceInvalidationScope {
    /// One file or directory, named relative to the workspace root.
    ///
    /// The parent directory is deliberately not carried alongside: it is a prefix of this path, and
    /// two fields that must agree eventually disagree.
    Path {
        relative_path: String,
        change: WorkspaceInvalidationChange,
    },
    /// A directory's entries changed and which entry is not known.
    Directory { relative_path: String },
    /// Something changed and this notice cannot say where.
    ///
    /// The honest answer when observation was lost — a watcher that dropped events, a burst past
    /// what coalescing will hold. Expensive to act on, which is why it is never the shape a
    /// producer reaches for first.
    Workspace,
}

impl WorkspaceInvalidationScope {
    pub(crate) fn token(&self) -> &'static str {
        match self {
            Self::Path { .. } => "path",
            Self::Directory { .. } => "directory",
            Self::Workspace => "workspace",
        }
    }

    /// The path this scope names, when it names one.
    pub(crate) fn relative_path(&self) -> Option<&str> {
        match self {
            Self::Path { relative_path, .. } | Self::Directory { relative_path } => {
                Some(relative_path)
            }
            Self::Workspace => None,
        }
    }
}

/// One normalized change notice.
///
/// Carries workspace-relative paths and never absolute ones. That distinction is the whole reason
/// this is safe to put on the event channel: a relative path is a string the console already asked
/// for and is already rendering, while an absolute path would add the user's home directory, account
/// name and machine layout to a message that exists only to say "refresh this row".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceInvalidationNotice {
    pub(crate) session_id: String,
    pub(crate) source: WorkspaceInvalidationSource,
    pub(crate) scope: WorkspaceInvalidationScope,
    pub(crate) observed_at: String,
    /// Monotonic per session, starting at 1.
    ///
    /// So a consumer can tell "nothing has changed" from "I stopped receiving": a gap in this
    /// sequence is the only evidence available that a notice was lost in transit, and without it a
    /// silent channel and a quiet workspace look identical.
    pub(crate) sequence: u64,
    /// How many further observations this notice stands in for.
    ///
    /// Absent when it stands only for itself — absent rather than zero, because "one observation"
    /// and "a burst that collapsed to one notice" are different facts and a zero would read as the
    /// first while meaning either.
    pub(crate) coalesced: Option<u32>,
}

/// Where notices go.
///
/// A port rather than a direct emit so the dispatcher stays testable without a Tauri app handle,
/// and so a failure to deliver cannot propagate back into whatever was writing the file.
pub(crate) trait WorkspaceInvalidationPublisher: Send + Sync {
    fn publish(&self, notice: &WorkspaceInvalidationNotice);
}

/// How a producer outside this context reports what it saw.
///
/// Narrow on purpose. The runtime knows it wrote a file long before any poll could notice, and this
/// is the entire surface it needs: no dispatcher, no clock, no knowledge of coalescing windows or
/// event channels. Returning nothing is part of it — a write must not be able to fail because a
/// notice could not be delivered.
pub(crate) trait WorkspaceChangeObserverPort: Send + Sync {
    fn observe(
        &self,
        session_id: &str,
        source: WorkspaceInvalidationSource,
        scope: WorkspaceInvalidationScope,
    );
}
