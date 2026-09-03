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

use super::content_search::{WorkspaceContentSearchRequest, WorkspaceContentSearchResult};
use super::error::WorkspaceApplicationError;
use super::inspection_budget::{WorkspaceInspectionBudgetSnapshot, WorkspaceInspectionReason};
use super::inspection_execution::WorkspaceInspectionExecution;
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
    /// The continuation token does not belong to this listing.
    ///
    /// Its own variant because a reader acts differently: a stale cursor is fixed by starting
    /// the listing again, while an unavailable workspace is not fixed by anything they can do.
    InvalidCursor,
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
            Self::InvalidCursor => "workspace_cursor_invalid",
            Self::Timeout => "workspace_inspection_timeout",
            Self::Storage(_) => "workspace_inspection_unavailable",
        }
    }
}

impl From<WorkspaceApplicationError> for WorkspaceInspectionError {
    fn from(error: WorkspaceApplicationError) -> Self {
        match error {
            // A validation refusal from the confined reads is a path that is not there, an
            // unreadable workspace, or a request the operation does not accept. It is
            // deliberately *not* an escape: escapes are classified from the request itself,
            // before the filesystem is reached, so this mapping cannot claim one that did not
            // happen.
            WorkspaceApplicationError::Validation(_) => Self::NotFound,
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
    /// Where to resume, from a previous page's `next_cursor`. A cursor issued for another
    /// directory is refused rather than applied.
    pub(crate) cursor: Option<String>,
    /// How many entries to take. Clamped, never unbounded: the limit arrives from a client and
    /// a listing is enumerated and sorted in full before it is cut.
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ReadTextFileRequest {
    pub(crate) path: String,
}

/// How many directories one fingerprint request may cover.
///
/// The bound exists for the remote case, where the whole batch is one round trip: without it, a
/// console with many directories open would decide how much work a poll does on somebody else's
/// machine.
pub(crate) const MAX_FINGERPRINT_PATHS: usize = 32;

/// Whether a directory's entries still look the way they did.
///
/// Cheap by design. A poll that re-listed every open directory would enumerate and sort thousands
/// of names to answer a yes/no question, and over SSH it would do that once per directory per tick.
/// A value that merely *changes* when the directory changes answers the same question for the cost
/// of a stat.
///
/// What it does not catch is an in-place edit that leaves the directory's own metadata alone. That
/// is a real limit and it is why this is a directory-level signal: it says the listing needs
/// refetching, never that a file's contents did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DirectoryFingerprint {
    pub(crate) relative_path: String,
    pub(crate) state: DirectoryFingerprintState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DirectoryFingerprintState {
    /// A value that changes when the directory's entries change. Opaque: only equality is defined,
    /// so a provider is free to derive it however its filesystem allows.
    Known(String),
    /// The directory is not there. A change in its own right — something removed it.
    Missing,
    /// It is there and could not be read.
    ///
    /// Separate from `Missing` because only one of them means the tree changed. Treating an
    /// unreadable directory as a removed one would announce a deletion every time a permission
    /// tightened or a network share hiccuped.
    Unreadable,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkspaceSearchRequest {
    pub(crate) query: String,
    pub(crate) max_results: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct WorkspacePathSearchRequest {
    /// What the reader typed. Empty is a valid query: it browses rather than searches.
    pub(crate) query: String,
    /// Issued by the caller, and reused for every keystroke from the same panel.
    ///
    /// A path search is cheaper than a content search but it is still a filesystem walk on a
    /// blocking thread, and a reader holding a key down starts one per repeat. Registering under a
    /// stable id is what makes the newest of those cancel the ones it replaced, under the registry's
    /// own lock — rather than leaving a trail of walks whose answers nobody is waiting for.
    pub(crate) search_id: String,
    pub(crate) cursor: Option<String>,
    pub(crate) limit: Option<usize>,
}

/// One thing a path search found.
///
/// Carries its kind because a reader acts on it: opening a file shows a preview, and "opening" a
/// directory means revealing it in the tree. A result list that made them look alike would offer
/// one action for two different things.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspacePathMatch {
    pub(crate) name: String,
    pub(crate) path: String,
    /// `file` or `directory`.
    pub(crate) kind: &'static str,
}

/// How much of the workspace a search actually looked at.
///
/// Separate from `next_cursor`, and the distinction is the point. A cursor says "more matches
/// follow"; coverage says "and some of the workspace was never examined". Paging fixes the first
/// and cannot fix the second, so collapsing them would let a reader page to the end of a result
/// list and conclude they had seen everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkspaceSearchCoverageState {
    Complete,
    Partial,
    Unavailable,
}

impl WorkspaceSearchCoverageState {
    pub(crate) fn token(self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Partial => "partial",
            Self::Unavailable => "unavailable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspaceSearchCoverage {
    pub(crate) state: WorkspaceSearchCoverageState,
    /// Why it is not complete, as a token the frontend translates. Absent when it is.
    ///
    /// One primary reason rather than a list. A UI that had to rank several would rank them
    /// differently from the code that produced them, and a reader only ever acts on one.
    pub(crate) reason_code: Option<&'static str>,
    /// What the inspection actually spent, when it was accounted.
    ///
    /// Absent means "not accounted", never "spent nothing": a remote provider counts on the other
    /// machine and a fixture counts nothing at all, and a zero here would claim a scan that never
    /// happened.
    pub(crate) budget: Option<WorkspaceInspectionBudgetSnapshot>,
}

impl WorkspaceSearchCoverage {
    pub(crate) fn complete() -> Self {
        Self {
            state: WorkspaceSearchCoverageState::Complete,
            reason_code: None,
            budget: None,
        }
    }

    pub(crate) fn partial(reason_code: &'static str) -> Self {
        Self {
            state: WorkspaceSearchCoverageState::Partial,
            reason_code: Some(reason_code),
            budget: None,
        }
    }

    pub(crate) fn unavailable(reason_code: &'static str) -> Self {
        Self {
            state: WorkspaceSearchCoverageState::Unavailable,
            reason_code: Some(reason_code),
            budget: None,
        }
    }

    /// Coverage for a stop reason, with the state that reason implies.
    ///
    /// The mapping lives on the reason rather than at each call site. A provider that decided for
    /// itself whether `inspection_busy` was partial or unavailable would be a second opinion, and
    /// the two would differ exactly where a reader needs them not to.
    pub(crate) fn stopped(reason: WorkspaceInspectionReason) -> Self {
        let state = if reason.is_unavailable() {
            WorkspaceSearchCoverageState::Unavailable
        } else {
            WorkspaceSearchCoverageState::Partial
        };
        Self {
            state,
            reason_code: Some(reason.code()),
            budget: None,
        }
    }

    /// Attaches what the inspection spent.
    pub(crate) fn with_budget(mut self, budget: WorkspaceInspectionBudgetSnapshot) -> Self {
        self.budget = Some(budget);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspacePathSearchResult {
    pub(crate) coverage: WorkspaceSearchCoverage,
    pub(crate) matches: Vec<WorkspacePathMatch>,
    pub(crate) next_cursor: Option<String>,
}

/// A path search answer together with the registration that produced it.
///
/// The same shape content search delivers, for the same reason: a provider is handed a query and
/// returns what it found, and whether that answer is still wanted is a fact about the registry that
/// is only knowable when it comes back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WorkspacePathSearchDelivery {
    /// Which registration under the search id produced this.
    pub(crate) generation: u64,
    pub(crate) result: WorkspacePathSearchResult,
}

/// What a finished path search is allowed to hand back.
///
/// A superseded page loses its matches *and* its cursor. The cursor is the part that would do real
/// damage: it names a rank in an ordering derived from a query the reader has already retyped, so a
/// caller that kept it would page the new query's result list from a position the new ordering never
/// produced.
pub(crate) fn deliver_path_search(
    registration: &super::search_cancellation::SearchRegistration,
    result: WorkspacePathSearchResult,
) -> WorkspacePathSearchDelivery {
    WorkspacePathSearchDelivery {
        generation: registration.generation().value(),
        result: if registration.is_current() {
            result
        } else {
            WorkspacePathSearchResult {
                coverage: WorkspaceSearchCoverage::stopped(
                    super::inspection_budget::WorkspaceInspectionReason::Superseded,
                ),
                matches: Vec::new(),
                next_cursor: None,
            }
        },
    }
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

    /// Whether the named directories still look the way they did, one call for all of them.
    ///
    /// Batched rather than one call per directory because the remote implementation is a round
    /// trip: polling six open directories individually would be six SSH channels and six helper
    /// launches for a question whose whole point is to be cheap enough to ask repeatedly.
    ///
    /// Every requested path gets an answer, including the ones that are gone. A provider that
    /// omitted them would make "not there" indistinguishable from "not asked about", and the
    /// caller compares against what it saw last time — an absent entry would read as no change.
    async fn directory_fingerprints(
        &self,
        target: &WorkspaceTarget,
        paths: &[String],
    ) -> Result<Vec<DirectoryFingerprint>, WorkspaceInspectionError>;

    /// Content search: positions inside files, bounded and interruptible.
    ///
    /// Takes the cancellation token rather than looking one up. A provider that consulted a registry
    /// would be a second place that decides whether a search is still wanted, and the two would
    /// disagree exactly when a reader cancelled at the wrong moment. It is also the only half of a
    /// registration a worker is given: the guard that owns the slot stays with the async caller, so
    /// no walk on the blocking pool can remove a registration — its own or anybody else's.
    async fn search_content(
        &self,
        target: &WorkspaceTarget,
        request: WorkspaceContentSearchRequest,
        execution: WorkspaceInspectionExecution,
    ) -> Result<WorkspaceContentSearchResult, WorkspaceInspectionError>;

    async fn list_documents(
        &self,
        target: &WorkspaceTarget,
        execution: WorkspaceInspectionExecution,
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

    /// Quick Open: relative paths matching what was typed, ranked and paged.
    ///
    /// Its own operation rather than a shape on `search`. That one exists to rank prompt-mention
    /// candidates, so it filters to source extensions and skips directories — right for composing a
    /// message and wrong for a reader trying to reach `package-lock.json` or a folder. Widening it
    /// would change what a mention offers, which nobody asked for.
    ///
    /// Takes the whole execution context rather than a token alone: the generation it runs under,
    /// the budget it may spend, the clock that bounds it, and the rules about where it may look.
    /// Five loose arguments is a shape where supplying four still compiles.
    async fn search_paths(
        &self,
        target: &WorkspaceTarget,
        request: WorkspacePathSearchRequest,
        execution: WorkspaceInspectionExecution,
    ) -> Result<WorkspacePathSearchResult, WorkspaceInspectionError>;

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
