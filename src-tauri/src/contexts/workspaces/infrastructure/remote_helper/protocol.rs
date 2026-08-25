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
            Self::RequestTooLarge => "remote_helper_request_too_large",
            Self::ResponseTooLarge => "remote_helper_response_too_large",
            Self::MalformedResponse => "remote_helper_malformed_response",
            Self::VersionMismatch => "remote_helper_version_mismatch",
            Self::Refused(code) => code,
        }
    }
}
