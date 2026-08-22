// The install flow that claims a version lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! One version of one extension names exactly one set of bytes, forever.
//!
//! Without this, "install version 1.2.0" is not a statement about any particular content. A
//! publisher — or anyone who can produce a package the installer accepts — could ship 1.2.0 twice
//! with different bytes, and a machine that installed the first would have no way to notice it now
//! runs the second. Recording both as separate snapshots does not help: it makes the ambiguity
//! durable rather than refusing it.
//!
//! So a claim is immutable and the key is `(publisher, extension_id, version)`. The first package
//! to claim a version binds it. A later package claiming the same version with the same hash is
//! the same package and is idempotent; with a different hash it is refused, and the hash it
//! offered is kept as evidence rather than thrown away — "this version was claimed twice with
//! different bytes" is exactly the thing an operator needs to be able to see afterwards.
//!
//! `publisher` is stored even though `extension_id` already begins with it. The binding must not
//! depend on the id grammar staying what it is today; if `ExtensionId` ever admits a different
//! shape, a claim written under the old rule still says who made it.

use super::{ExtensionId, PackageHash, PublisherId};
use semver::Version;

/// Whether the package making a claim had provenance.
///
/// Recorded, not consulted. The rule is the same either way: Developer Mode does not permit
/// overwriting a version in place, because a build loop that reuses a version number is how an
/// unreviewed change reaches an installed extension. Keeping the distinction means an operator can
/// see whether a conflicting claim came from a signed package or a local build.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ClaimProvenance {
    Signed,
    Unsigned,
}

impl ClaimProvenance {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Signed => "signed",
            Self::Unsigned => "unsigned",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "signed" => Some(Self::Signed),
            "unsigned" => Some(Self::Unsigned),
            _ => None,
        }
    }
}

/// What one version of one extension is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionClaim {
    pub(crate) publisher: PublisherId,
    pub(crate) extension: ExtensionId,
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) provenance: ClaimProvenance,
    pub(crate) first_claimed_at: String,
}

/// What claiming a version would mean, given whatever already holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClaimOutcome {
    /// Nothing holds this version. The claim binds it.
    Bound,
    /// The same bytes, claimed again. Reinstalling the identical package is not a conflict.
    AlreadyBound,
    /// The version is held by different bytes. No activatable snapshot may be created for it.
    Conflict(VersionContentConflict),
}

impl ClaimOutcome {
    pub(crate) const fn code(&self) -> &'static str {
        match self {
            Self::Bound => "version_bound",
            Self::AlreadyBound => "version_already_bound",
            Self::Conflict(_) => "version_content_conflict",
        }
    }

    /// Whether an installation may proceed from here.
    pub(crate) const fn admits_snapshot(&self) -> bool {
        matches!(self, Self::Bound | Self::AlreadyBound)
    }
}

/// The same version, twice, with different bytes.
///
/// Carries both hashes because that is the whole content of the finding: which bytes hold the
/// version now, and which bytes were offered for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionContentConflict {
    pub(crate) bound_hash: PackageHash,
    pub(crate) offered_hash: PackageHash,
    pub(crate) bound_provenance: ClaimProvenance,
    pub(crate) bound_at: String,
}

impl VersionContentConflict {
    pub(crate) const fn code(&self) -> &'static str {
        "version_content_conflict"
    }
}

/// Decides what a claim means against whatever already holds the version.
///
/// A pure comparison, so the rule is one place and the repository only has to say what it found.
pub(crate) fn decide_claim(offered: &VersionClaim, held: Option<&VersionClaim>) -> ClaimOutcome {
    let Some(held) = held else {
        return ClaimOutcome::Bound;
    };
    if held.package_hash == offered.package_hash {
        return ClaimOutcome::AlreadyBound;
    }
    ClaimOutcome::Conflict(VersionContentConflict {
        bound_hash: held.package_hash.clone(),
        offered_hash: offered.package_hash.clone(),
        bound_provenance: held.provenance,
        bound_at: held.first_claimed_at.clone(),
    })
}
