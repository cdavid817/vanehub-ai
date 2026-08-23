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
//!
//! ## Who is allowed to claim
//!
//! The publisher a claim is filed under is **never the manifest's `publisher` field**. That field
//! is a string inside a file the claimant wrote: anyone able to produce a package the installer
//! reads can put `acme` in it, and a claim filed under it would let a local build take the version
//! binding for a real publisher and make every later genuine 1.2.0 a conflict. Squatting a
//! competitor's version numbers is a cheap denial of service, and the manifest is exactly the
//! wrong place to learn who someone is.
//!
//! So a claim carries a `ClaimAuthority`, and there are only two ways to get one:
//!
//! * `VerifiedPublisher` — established by signature verification against a trusted key. The
//!   publisher here comes from the *stored key record*, not from the envelope and not from the
//!   manifest, both of which the package supplies.
//! * `LocalDeveloper` — assigned by the host to content with no provenance. It files under the
//!   reserved namespace `local:`, which contains a colon and is therefore unrepresentable as a
//!   `PublisherId`: no verified publisher can ever collide with it, and no manifest can ask to be
//!   filed there.

use super::{ExtensionId, PackageHash, PublisherId, PublisherKeyRecord};
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

/// The reserved namespace unsigned content is filed under.
///
/// Contains a colon, which `PublisherId` cannot, so it can never collide with a real publisher.
pub(crate) const LOCAL_DEVELOPER_NAMESPACE: &str = "local:developer";

/// Who a claim is filed under, and how that was established.
///
/// Constructed only from a verified key record or by the host. There is deliberately no
/// constructor taking a manifest's publisher field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClaimAuthority {
    /// Established by signature verification against a key this installation trusts.
    VerifiedPublisher(PublisherId),
    /// Content with no provenance, filed under the host's reserved namespace.
    LocalDeveloper,
}

impl ClaimAuthority {
    /// Established from the *stored key record*, which is what verification matched against.
    ///
    /// Takes the record rather than the envelope or the manifest: both of those are supplied by
    /// the package, and only the record is something this installation decided to trust.
    pub(crate) fn of_verified_key(key: &PublisherKeyRecord) -> Self {
        Self::VerifiedPublisher(key.publisher.clone())
    }

    /// The string a claim is filed under.
    pub(crate) fn as_str(&self) -> &str {
        match self {
            Self::VerifiedPublisher(publisher) => publisher.as_str(),
            Self::LocalDeveloper => LOCAL_DEVELOPER_NAMESPACE,
        }
    }

    /// Reads an authority back out of storage.
    ///
    /// The reserved namespace round-trips as `LocalDeveloper`; anything else must parse as a
    /// publisher, so a hand-edited row naming something that is neither is refused rather than
    /// treated as a publisher nobody vetted.
    pub(crate) fn parse(value: &str) -> Option<Self> {
        if value == LOCAL_DEVELOPER_NAMESPACE {
            return Some(Self::LocalDeveloper);
        }
        PublisherId::parse(value).ok().map(Self::VerifiedPublisher)
    }

    pub(crate) const fn is_verified(&self) -> bool {
        matches!(self, Self::VerifiedPublisher(_))
    }
}

/// What one version of one extension is bound to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VersionClaim {
    pub(crate) authority: ClaimAuthority,
    pub(crate) extension: ExtensionId,
    pub(crate) version: Version,
    pub(crate) package_hash: PackageHash,
    pub(crate) provenance: ClaimProvenance,
    pub(crate) first_claimed_at: String,
}

/// What claiming a version would mean, given whatever already holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ClaimOutcome {
    /// A verified publisher tried to claim a version in someone else's namespace.
    ///
    /// See `decide_claim`. Decided before anything is compared against the held claim: a claim
    /// nobody was entitled to make is not a conflict with the incumbent, it is not a claim.
    NamespaceMismatch(NamespaceMismatch),
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
            Self::NamespaceMismatch(_) => "extension_namespace_mismatch",
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

/// A verified publisher claiming an extension id that is not theirs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NamespaceMismatch {
    /// Who the signature established.
    pub(crate) authority: String,
    /// Who the extension id says owns it.
    pub(crate) namespace: PublisherId,
}

impl NamespaceMismatch {
    pub(crate) const fn code(&self) -> &'static str {
        "extension_namespace_mismatch"
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
    // Entitlement first. `acme.git-guardian` is owned by `acme`, and a key this installation
    // trusts for `other` is still not a key for `acme`. Without this, any trusted publisher could
    // bind a version in a competitor's namespace and every genuine release of that version
    // afterwards would be refused as a conflict -- the same denial of service the authority model
    // exists to stop, reached from inside the trusted set instead of outside it.
    //
    // Only `VerifiedPublisher` is bound this way. Developer Mode exists precisely to build
    // `acme.git-guardian` before there is a signature for it, and unsigned content files under the
    // host's reserved namespace, where it can neither collide with nor displace the real
    // publisher's binding.
    let namespace = offered.extension.publisher();
    if let ClaimAuthority::VerifiedPublisher(publisher) = &offered.authority {
        if publisher != &namespace {
            return ClaimOutcome::NamespaceMismatch(NamespaceMismatch {
                authority: publisher.as_str().to_string(),
                namespace,
            });
        }
    }

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
