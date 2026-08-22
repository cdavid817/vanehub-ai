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
