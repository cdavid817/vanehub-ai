//! What travels between this process and the remote helper.
//!
//! One request in, one response out, both bounded, both versioned. The version is on the wire
//! rather than assumed because the helper is shipped with the client and the client is upgraded
//! independently of nothing at all — but a stale helper can still be running from a previous
//! invocation, and a response parsed under the wrong version is worse than no response.
//!
//! Nothing user-controlled goes into the command. The request is JSON on stdin, which is the whole
//! reason the command can be a constant: a path interpolated into a shell command is a shell
//! injection with extra steps, and no amount of quoting makes that a rule somebody cannot forget.

use serde::{Deserialize, Serialize};

/// The protocol version this client speaks.
///
/// A response carrying anything else is refused rather than interpreted. The fields might happen to
/// line up, and acting on a payload whose meaning is not the meaning that was intended is exactly
/// the failure a version exists to prevent.
pub(crate) const HELPER_VERSION: u32 = 1;

/// How large a request may be before it is refused here.
///
/// Refused rather than sent: the remote reads its whole stdin into memory, so an unbounded request
/// is an unbounded allocation on a machine this process does not administer.
pub(crate) const MAX_HELPER_REQUEST_BYTES: usize = 64 * 1024;

/// How much response this client will hold.
///
/// The helper bounds its own output too, and both bounds exist for different reasons: the helper's
/// stops it building a huge string, and this one stops a helper that ignored its own bound — or a
/// remote program that is not the helper at all — from filling this process's memory.
pub(crate) const MAX_HELPER_RESPONSE_BYTES: usize = 1024 * 1024;

/// How long one round trip may take.
///
/// Wall-clock rather than per-read, because the failure to bound is "the remote accepted the
/// connection and then never answered": every individual read succeeds, and only the total says
/// something is wrong.
pub(crate) const HELPER_TIMEOUT_SECONDS: u64 = 20;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperRequest {
    pub(crate) version: u32,
    /// The configured remote root. Resolved to a real path *on the remote host*, because this
    /// machine cannot tell a symlink there from a directory.
    pub(crate) root: String,
    pub(crate) operation: HelperOperation,
}

impl HelperRequest {
    pub(crate) fn new(root: String, operation: HelperOperation) -> Self {
        Self {
            version: HELPER_VERSION,
            root,
            operation,
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub(crate) enum HelperOperation {
    /// What this host can do. Always answered, even when the answer is "almost nothing": a host
    /// with no Python cannot run the helper at all, but one with no ripgrep is perfectly usable for
    /// everything except search.
    Probe,
    /// One directory, bounded and deterministically ordered. `path` is relative to the root; empty
    /// means the root itself.
    ListDirectory {
        path: String,
        /// The ordering key to resume after, already decoded by the client. Sent as a key rather
        /// than the opaque cursor so the helper never has to know the encoding - and so a cursor
        /// forged on the wire cannot make it resume somewhere nobody paged to.
        after_kind_rank: Option<u8>,
        after_name_key: Option<String>,
        limit: usize,
    },
    /// Whether these directories still look the way they did, all in one round trip.
    ///
    /// Batched because the alternative is a channel and a Python launch per directory, per tick.
    /// The answer is a stat each, never a listing: a poll that enumerated would spend the cost the
    /// whole operation exists to avoid.
    DirectoryFingerprints {
        paths: Vec<String>,
    },
    ReadTextFile {
        path: String,
    },
    /// Quick Open. Ranking happens here rather than on the remote host so both providers order the
    /// same way: a rule implemented twice is one that disagrees first about the ties nobody tests.
    /// The helper returns candidate paths; this side scores and pages them.
    SearchPaths {
        query: String,
        limit: usize,
        /// What the remote walk may spend.
        ///
        /// Sent rather than left to the helper's own constants, so the two sides bound the same walk
        /// by the same numbers. A helper deciding for itself is a second budget policy, and the two
        /// disagree first about the workspace nobody has tried yet.
        limits: HelperWalkLimits,
        /// The shared policy's default exclusions, sent rather than restated on the remote host.
        ///
        /// The helper used to hold its own copy of this list, and the copy had already fallen behind
        /// — a workspace appeared to have a different shape depending on which machine it was on,
        /// which is the one thing a provider-neutral seam exists to prevent. Sending it means there
        /// is one list, and the script that ships with this binary cannot disagree with it.
        excluded_directories: Vec<String>,
    },
    Search {
        query: String,
        max_results: usize,
        excluded_directories: Vec<String>,
    },
    /// Content search. Fixed-string and case-insensitive, matching the local scan exactly: two
    /// different engines can agree about a literal and cannot be made to agree about a pattern
    /// language, and a reader whose query means something else on a remote host has been handed a
    /// puzzle rather than a feature.
    SearchContent {
        query: String,
        max_results: usize,
        excluded_directories: Vec<String>,
    },
    GitStatus,
    GitDiff {
        path: String,
        staged: bool,
    },
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperResponse {
    pub(crate) version: u32,
    pub(crate) ok: bool,
    /// Present when `ok` is false. A stable token, never a message: the helper runs on somebody
    /// else's machine and its exception text would carry remote paths into this process's logs.
    #[serde(default)]
    pub(crate) reason_code: Option<String>,
    #[serde(default)]
    pub(crate) result: Option<HelperResult>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperResult {
    #[serde(default)]
    pub(crate) probe: Option<HelperProbe>,
    #[serde(default)]
    pub(crate) listing: Option<HelperListing>,
    #[serde(default)]
    pub(crate) fingerprints: Option<Vec<HelperFingerprint>>,
    #[serde(default)]
    pub(crate) paths: Option<HelperPathCandidates>,
    #[serde(default)]
    pub(crate) file: Option<HelperFile>,
    #[serde(default)]
    pub(crate) search: Option<HelperSearch>,
    #[serde(default)]
    pub(crate) content: Option<HelperContentMatches>,
    /// Git answers arrive as the bytes git printed.
    ///
    /// Parsed on this side with the same parser the local provider uses, so the
    /// locale-independent classification of a porcelain status and the structure of a unified diff
    /// have one implementation. A helper that parsed them would be a second one, and the two would
    /// disagree first about exactly the cases nobody writes tests for.
    #[serde(default)]
    pub(crate) git: Option<HelperGitOutput>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperListing {
    pub(crate) path: String,
    pub(crate) entries: Vec<HelperEntry>,
    /// Set when the bound cut the listing short, never when the directory simply ended.
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperEntry {
    pub(crate) name: String,
    /// Relative to the root, with forward slashes. Absolute remote paths never cross the wire:
    /// they would put another machine's directory layout into this one's UI and its logs.
    pub(crate) path: String,
    /// `directory` or `file`. Anything else on the remote host is skipped rather than named,
    /// because a socket or a device is not something this panel can open.
    pub(crate) kind: String,
    #[serde(default)]
    pub(crate) size: Option<u64>,
}

/// One directory's answer to "does it still look the same".
///
/// `state` rather than a nullable value, because three outcomes matter and only two of them are a
/// change: a directory that is gone was removed, and one that cannot be read was not.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperFingerprint {
    pub(crate) path: String,
    /// `known`, `missing`, or `unreadable`.
    pub(crate) state: String,
    /// Present only with `known`. Opaque on this side: the helper decides what makes a directory
    /// look different, and this process only ever compares two of them for equality.
    #[serde(default)]
    pub(crate) value: Option<String>,
}

/// What a remote walk found, before this side ranks it.
///
/// `truncated` is the honest half: a walk that stopped at its bound has left part of the workspace
/// unexamined, and a result list that did not say so is how somebody concludes a file is not there.
/// What a remote walk may spend, in the terms the shared budget uses.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperWalkLimits {
    pub(crate) max_entries: u64,
    pub(crate) max_depth: u32,
    pub(crate) max_results: u64,
    /// Seconds. A walk on somebody else's machine needs its own deadline: the transport timeout
    /// ends the *exchange*, and a helper that kept walking after it would hold a remote process for
    /// an answer this side has already given up on.
    pub(crate) deadline_seconds: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperPathCandidates {
    pub(crate) entries: Vec<HelperEntry>,
    pub(crate) truncated: bool,
    /// Which bound stopped the walk, in the shared vocabulary. Absent when nothing did.
    ///
    /// Before this, every remote stop arrived as one boolean and was reported as an entry budget —
    /// which was a guess, and wrong whenever the real bound was depth, results, or a directory the
    /// host could not read.
    #[serde(default)]
    pub(crate) reason_code: Option<String>,
    /// What the walk spent. Absent from a helper that did not count.
    #[serde(default)]
    pub(crate) counts: Option<HelperCounts>,
}

/// What one remote walk actually spent.
///
/// Structural counters only, and no paths: this crosses the wire and lands in a coverage a reader
/// can see, and a count is a fact about effort rather than about what the workspace contains.
#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperCounts {
    #[serde(default)]
    pub(crate) directories_visited: u64,
    #[serde(default)]
    pub(crate) entries_visited: u64,
    #[serde(default)]
    pub(crate) max_depth_reached: u32,
    #[serde(default)]
    pub(crate) unreadable_entries: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperFile {
    pub(crate) path: String,
    pub(crate) name: String,
    /// `text`, `binary`, or `oversized` - the local provider's vocabulary, because a panel must
    /// not need to know which provider answered. The last two are facts about the file rather than
    /// failures: a reader who asked for a 4 GiB core dump needs to be told why there is no preview.
    pub(crate) status: String,
    pub(crate) size: u64,
    #[serde(default)]
    pub(crate) content: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperSearch {
    pub(crate) matches: Vec<HelperEntry>,
    pub(crate) truncated: bool,
}

/// Positions inside remote files.
///
/// The snippet arrives already bounded and control-free: the trimming happens where the line is,
/// because sending a megabyte-long minified line so this side can cut it would put the cost of the
/// bound on the wire the bound exists to protect.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperContentMatches {
    pub(crate) matches: Vec<HelperContentMatch>,
    pub(crate) truncated: bool,
    /// Set when ripgrep is not installed. Distinct from an empty result, which means the query
    /// genuinely matched nothing.
    #[serde(default)]
    pub(crate) unavailable: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperContentMatch {
    pub(crate) path: String,
    pub(crate) line: u32,
    pub(crate) column: u32,
    pub(crate) snippet: String,
    pub(crate) truncated: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperGitOutput {
    /// False when the root is not inside a work tree. Distinct from an empty status, which means
    /// a clean repository.
    pub(crate) is_repository: bool,
    /// Base64, because porcelain `-z` output is NUL-separated and a path on a POSIX host is bytes
    /// rather than text — decoding it to send it would lose exactly the names that need care.
    #[serde(default)]
    pub(crate) stdout_base64: Option<String>,
    /// Set when the bound cut the output short. The caller reports it rather than parsing a
    /// half-diff, which would render as a smaller change.
    pub(crate) truncated: bool,
}

/// What the remote host turned out to be.
///
/// Every field is a fact the helper checked rather than something it was told, because the point of
/// probing is that the configured profile says nothing about whether `git` is installed.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HelperProbe {
    pub(crate) helper_version: u32,
    pub(crate) posix: bool,
    /// `major.minor.micro`, for a diagnostic that says which Python answered rather than that one
    /// did. Never logged with the host it came from.
    pub(crate) python_version: String,
    pub(crate) git: bool,
    pub(crate) ripgrep: bool,
    /// Whether the configured root resolves to a directory the helper can read. False is a normal
    /// answer — a path can be moved or a permission revoked — and it is the one that distinguishes
    /// "this workspace is empty" from "this workspace is not there".
    pub(crate) root_readable: bool,
}

/// Why a round trip did not produce an answer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RemoteHelperError {
    /// The profile changed or its host is no longer trusted. Checked before connecting, so a stale
    /// binding never reaches the network.
    ProfileStale,
    HostUntrusted,
    ConnectionFailed,
    ChannelFailed,
    Timeout,
    /// A cancel reached this exchange while it was waiting on the remote host.
    ///
    /// Its own variant rather than a timeout, because the two are different events and a reader is
    /// told different things: one of them is something they did on purpose. The channel is closed on
    /// this path like every other, so the remote process ends rather than running on for an answer
    /// nobody will read.
    Cancelled,
    /// The request would not fit inside its bound. Refused here rather than sent.
    RequestTooLarge,
    ResponseTooLarge,
    /// The remote answered something this client cannot read: not JSON, or a version it does not
    /// speak. Both mean the same thing to a caller — the answer cannot be trusted — but they are
    /// separate because only one of them is fixed by upgrading.
    MalformedResponse,
    VersionMismatch,
    /// The helper ran and refused, with its own stable code.
    Refused(String),
}

impl RemoteHelperError {
    pub(crate) fn code(&self) -> &str {
        match self {
            Self::ProfileStale => "remote_profile_stale",
            Self::HostUntrusted => "remote_host_untrusted",
            Self::ConnectionFailed => "remote_connection_unavailable",
            Self::ChannelFailed => "remote_channel_failed",
            Self::Timeout => "remote_helper_timeout",
            Self::Cancelled => "remote_helper_cancelled",
            Self::RequestTooLarge => "remote_helper_request_too_large",
            Self::ResponseTooLarge => "remote_helper_response_too_large",
            Self::MalformedResponse => "remote_helper_malformed_response",
            Self::VersionMismatch => "remote_helper_version_mismatch",
            Self::Refused(code) => code,
        }
    }
}
