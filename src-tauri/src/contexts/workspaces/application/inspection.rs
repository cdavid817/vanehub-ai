//! Inspecting a workspace without knowing where it is.
//!
//! Today every read in this context is a function of `session_id` and reaches SQLite and the local
//! filesystem directly, which is why a remote session reports no root at all: the code that would
//! answer only knows how to look on this machine. This module is the seam that makes "where" a
//! parameter — one trait, two implementations, and a target that says which.
//!
//! Two rules shape everything below.
//!
//! A target is *resolved*, never supplied. Every operation takes a `WorkspaceTarget` that came from
//! the registered session, so a caller cannot name a root: an inspection API that accepted an
//! absolute path would be a filesystem browser with the session id as decoration, and the
//! confinement rules underneath it would be guarding a boundary the caller chose.
//!
//! An unavailable capability is a typed answer, not an error. A remote host with no `python3` can
//! still run a Shell, and a panel that failed the whole workspace because one prerequisite is
//! missing would take away the thing the user came for.

use super::error::WorkspaceApplicationError;
use super::models::{
    DirectoryListing, DocumentListing, FileContent, FileSearchListing, GitDiffResult,
    GitDiffSource, GitStatusResult,
};
use async_trait::async_trait;
use std::path::PathBuf;

/// Which workspace an inspection is about.
///
/// Carries the session it was resolved from so a provider can report a refusal against something a
/// reader recognises, and so a target cannot be built out of thin air by a caller who happens to
/// know a path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceTarget {
    Local(LocalWorkspaceTarget),
    Remote(RemoteWorkspaceTarget),
}

impl WorkspaceTarget {
    pub(crate) fn session_id(&self) -> &str {
        match self {
            Self::Local(target) => &target.session_id,
            Self::Remote(target) => &target.session_id,
        }
    }

    /// Which provider answers for this target, as the token the frontend reads.
    pub(crate) fn provider(&self) -> &'static str {
        match self {
            Self::Local(_) => "local",
            Self::Remote(_) => "ssh",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LocalWorkspaceTarget {
    pub(crate) session_id: String,
    /// The canonical root. Already resolved, so a provider confines against a real directory rather
    /// than against a string that might contain `..`.
    pub(crate) root: PathBuf,
}

/// A remote workspace, pinned to the connection revision it was resolved against.
///
/// The revision is the whole reason this is not just a host and a path: a profile can be edited
/// between two reads, and an inspection that reconnected under the new one would answer about a
/// different machine while the reader believed they were still looking at the first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RemoteWorkspaceTarget {
    pub(crate) session_id: String,
    pub(crate) connection_id: String,
    pub(crate) connection_revision: i64,
    /// The remote root as configured. Resolution to a real path happens on the remote host, because
    /// this machine cannot tell a symlink there from a directory.
    pub(crate) root: String,
    pub(crate) display_name: String,
}

/// Whether one capability can be used, and if not, why.
///
/// `reason_code` is a stable token rather than a sentence: the frontend owns the wording, and a
/// message assembled here would arrive untranslated in whatever language this build's sources are
/// written in. `remediation` is a second code naming what would fix it, because "search is
/// unavailable" and "install ripgrep on the remote host" are different pieces of information and a
/// reader can only act on the second.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CapabilityState {
    pub(crate) available: bool,
    pub(crate) reason_code: Option<&'static str>,
    pub(crate) remediation: Option<&'static str>,
}

impl CapabilityState {
    pub(crate) fn available() -> Self {
        Self {
            available: true,
            reason_code: None,
            remediation: None,
        }
    }

    pub(crate) fn unavailable(reason_code: &'static str) -> Self {
        Self {
            available: false,
            reason_code: Some(reason_code),
            remediation: None,
        }
    }

    pub(crate) fn with_remediation(mut self, remediation: &'static str) -> Self {
        self.remediation = Some(remediation);
        self
    }
}

/// How a provider learns that something changed.
///
/// Named rather than boolean because the three that exist behave differently enough that a panel
/// has to know which it has: a native watcher is immediate, polling has a floor on how stale a view
/// can be, and event-derived invalidation only sees changes this application itself caused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WatchMode {
    Native,
    Polling,
    EventDerived,
    None,
}

impl WatchMode {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Native => "native",
            Self::Polling => "polling",
            Self::EventDerived => "event-derived",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceInspectionCapabilities {
    /// `local`, `ssh`, or `simulated`.
    pub(crate) provider: &'static str,
    pub(crate) list_files: CapabilityState,
    pub(crate) read_text_files: CapabilityState,
    pub(crate) search_files: CapabilityState,
    pub(crate) git_status: CapabilityState,
    pub(crate) git_diff: CapabilityState,
    pub(crate) watch_mode: WatchMode,
}

/// Why an inspection could not answer.
///
/// Separate from `WorkspaceApplicationError` because these are the failures a *reader* has to be
/// told apart: a path that escaped its root is a refusal, a missing remote helper is a prerequisite,
/// and a timed-out connection is something to retry. The general error carries a message and
/// collapses all three into "something went wrong".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum WorkspaceInspectionError {
    /// The session has no workspace this provider can reach.
    TargetUnavailable(&'static str),
    /// The requested path resolved outside the root. Never carries the path: a refusal that echoed
    /// what it refused would put a caller-chosen absolute path into a log.
    PathEscaped,
    NotFound,
    /// The provider works, but this operation needs something the host does not have.
    Unsupported(&'static str),
    /// The connection failed, or the remote host answered in a way the helper could not use.
    RemoteUnavailable(&'static str),
    Timeout,
    Storage(String),
}

impl WorkspaceInspectionError {
    /// The stable code a caller translates. Every variant has one, including the ones that carry a
    /// message, because the message is for a diagnostic and the code is for a person.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::TargetUnavailable(code)
            | Self::Unsupported(code)
            | Self::RemoteUnavailable(code) => code,
            Self::PathEscaped => "workspace_path_escaped",
            Self::NotFound => "workspace_path_not_found",
            Self::Timeout => "workspace_inspection_timeout",
            Self::Storage(_) => "workspace_inspection_unavailable",
        }
    }
}

impl From<WorkspaceApplicationError> for WorkspaceInspectionError {
    fn from(error: WorkspaceApplicationError) -> Self {
        match error {
            // A validation failure at this boundary is a path the confinement rules refused, which
            // is the one case a reader must not see as a transient fault to retry.
            WorkspaceApplicationError::Validation(_) => Self::PathEscaped,
            WorkspaceApplicationError::SessionNotFound(_) => {
                Self::TargetUnavailable("workspace_session_not_found")
            }
            other => Self::Storage(other.to_string()),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ListDirectoryRequest {
    /// Relative to the target root. Empty means the root itself.
    pub(crate) path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReadTextFileRequest {
    pub(crate) path: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceSearchRequest {
    pub(crate) query: String,
    pub(crate) max_results: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GitDiffRequest {
    pub(crate) path: String,
    pub(crate) source: GitDiffSource,
}

/// Reading a workspace, wherever it is.
///
/// Async because the remote implementation is network I/O with a timeout, and a synchronous port
/// would force that onto whatever thread happened to call it. The local implementation is disk and
/// SQLite work and moves itself to the blocking pool, which is the same thing the API wrappers
/// already do for the calls this replaces.
///
/// Every method takes the target rather than the provider holding one, so a single provider serves
/// every session of its kind and a caller cannot end up with a provider pointed somewhere other
/// than where it thinks.
#[async_trait]
pub(crate) trait WorkspaceInspectionProvider: Send + Sync {
    /// What this provider can actually do for this target.
    ///
    /// Asked before anything else and answered even when most of it is unavailable: the panel needs
    /// to render the parts that work and explain the parts that do not, and a provider that refused
    /// to describe itself would leave it with nothing to say.
    async fn capabilities(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<WorkspaceInspectionCapabilities, WorkspaceInspectionError>;

    async fn list_directory(
        &self,
        target: &WorkspaceTarget,
        request: ListDirectoryRequest,
    ) -> Result<DirectoryListing, WorkspaceInspectionError>;

    async fn list_documents(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<DocumentListing, WorkspaceInspectionError>;

    async fn read_text_file(
        &self,
        target: &WorkspaceTarget,
        request: ReadTextFileRequest,
    ) -> Result<FileContent, WorkspaceInspectionError>;

    async fn search(
        &self,
        target: &WorkspaceTarget,
        request: WorkspaceSearchRequest,
    ) -> Result<FileSearchListing, WorkspaceInspectionError>;

    async fn git_status(
        &self,
        target: &WorkspaceTarget,
    ) -> Result<GitStatusResult, WorkspaceInspectionError>;

    async fn git_diff(
        &self,
        target: &WorkspaceTarget,
        request: GitDiffRequest,
    ) -> Result<GitDiffResult, WorkspaceInspectionError>;
}

/// Where a target comes from.
///
/// A separate port from the provider because resolving is a different question from inspecting, and
/// keeping them apart is what makes "a caller cannot name a root" checkable: the only thing that
/// produces a `WorkspaceTarget` is an implementation of this, and the only input it takes is a
/// session id.
pub(crate) trait WorkspaceTargetResolver: Send + Sync {
    fn resolve(&self, session_id: &str) -> Result<WorkspaceTarget, WorkspaceInspectionError>;
}
