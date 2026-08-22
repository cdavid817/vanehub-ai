// The install flow that publishes these lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! One immutable set of installed bytes, and the pointer that says which one is live.
//!
//! Publication is two writes to two different kinds of storage — content to a filesystem, a
//! pointer to a database — and they cannot be made one transaction. What can be arranged is that
//! every ordering leaves something recoverable:
//!
//! * **Content is published first, and content is immutable and content-addressed.** Bytes that
//!   land and are never pointed at are garbage, which startup reconciliation collects. Bytes that
//!   are pointed at and missing would be an installation that cannot run.
//! * **The pointer moves last, in one guarded write.** It either moves or it does not.
//! * **The previous snapshot is retained on every failure path**, and recorded on the successful
//!   one, so a rollback target always exists.

use super::{ExtensionId, InstallationId, ManifestDigest, PackageHash, SnapshotId};
use semver::Version;

/// One published set of bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotRecord {
    pub(crate) snapshot: SnapshotId,
    pub(crate) extension: ExtensionId,
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) manifest_digest: ManifestDigest,
    pub(crate) created_at: String,
}

/// Which snapshot an installation is currently running, and which one it came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SnapshotPointer {
    pub(crate) installation: InstallationId,
    pub(crate) extension: ExtensionId,
    pub(crate) active: SnapshotId,
    /// The rollback target. `None` only for a first install, which has nowhere to roll back to.
    pub(crate) previous: Option<SnapshotId>,
    pub(crate) revision: i64,
    pub(crate) updated_at: String,
}

/// What happened to the content half of a publication.
///
/// `AlreadyPresent` is a success, not a conflict. Content is addressed by its own digest, so a
/// destination that already exists holds exactly the bytes being published — including when it
/// exists because a concurrent install of the same package won the race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ContentPublication {
    Published,
    AlreadyPresent,
}

/// Whether the staged content was cleaned up after a failure.
///
/// Reported rather than swallowed: bytes left in quarantine after a failed install are not a
/// correctness problem, but they are a fact an operator may need, and startup reconciliation is
/// what eventually collects them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StagedRecovery {
    Clean,
    /// The staged content could not be removed. It is unreferenced and safe to leave.
    Abandoned,
}

/// Why a publication did not happen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SnapshotPublicationError {
    /// The content could not be written. Nothing was pointed at it, so nothing changed.
    Content(String),
    /// Someone else moved the pointer since the caller read it.
    StaleRevision { expected: i64, actual: i64 },
    /// The pointer write failed. The content is published and unreferenced.
    Pointer {
        reason: String,
        recovery: StagedRecovery,
    },
}

impl SnapshotPublicationError {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Content(_) => "snapshot_content_failure",
            Self::StaleRevision { .. } => "snapshot_stale_revision",
            Self::Pointer { .. } => "snapshot_pointer_failure",
        }
    }
}

pub(crate) fn all_snapshot_publication_errors() -> Vec<SnapshotPublicationError> {
    vec![
        SnapshotPublicationError::Content(String::new()),
        SnapshotPublicationError::StaleRevision {
            expected: 0,
            actual: 0,
        },
        SnapshotPublicationError::Pointer {
            reason: String::new(),
            recovery: StagedRecovery::Clean,
        },
    ]
}
