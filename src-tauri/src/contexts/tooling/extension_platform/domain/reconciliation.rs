// The startup hook that runs this lands with Task Group 4; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What a restart may clean up, and what it must leave exactly where it is.
//!
//! A crash mid-install leaves bytes in places nothing points at. Some of those places are safe to
//! empty on sight — an operation cannot survive a restart, so anything in quarantine belongs to an
//! operation that is over — and some are not, because content is shared and immutable and the row
//! that references it may be about to be written.
//!
//! The rule that makes this safe to run unattended is that **anything unrecognised is left alone
//! and reported**. Reconciliation removes what it can name and explain; a file where a directory
//! belongs, a name that is not an identifier, a directory nobody expected — those are reported to
//! an operator and never deleted. A cleanup that deletes what it does not understand is one nobody
//! can safely run at startup.

use super::PackageHash;
use std::collections::BTreeSet;

/// Which of the four application-owned roots an entry was found under.
///
/// In the domain rather than beside the paths, because what may be collected from each is a rule
/// about lifetimes rather than about directories. Infrastructure decides what each is called on
/// disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ExtensionRootScope {
    Quarantine,
    Packages,
    Scratch,
    Sidecars,
}

pub(crate) const ALL_EXTENSION_ROOT_SCOPES: [ExtensionRootScope; 4] = [
    ExtensionRootScope::Quarantine,
    ExtensionRootScope::Packages,
    ExtensionRootScope::Scratch,
    ExtensionRootScope::Sidecars,
];

impl ExtensionRootScope {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Quarantine => "quarantine",
            Self::Packages => "packages",
            Self::Scratch => "scratch",
            Self::Sidecars => "sidecars",
        }
    }
}

/// Why an entry may go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconciliationReason {
    /// Quarantine belongs to one operation, and no operation survives a restart.
    AbandonedQuarantine,
    /// Content no snapshot row names. Nothing can reach it, and nothing will.
    UnreferencedPackage,
    /// Scratch belongs to a runtime generation, and no generation survives a restart.
    StaleScratch,
    /// Sidecar working space belongs to a generation, and the process that owned it is gone.
    OrphanSidecar,
}

impl ReconciliationReason {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::AbandonedQuarantine => "abandoned_quarantine",
            Self::UnreferencedPackage => "unreferenced_package",
            Self::StaleScratch => "stale_scratch",
            Self::OrphanSidecar => "orphan_sidecar",
        }
    }
}

pub(crate) const ALL_RECONCILIATION_REASONS: [ReconciliationReason; 4] = [
    ReconciliationReason::AbandonedQuarantine,
    ReconciliationReason::UnreferencedPackage,
    ReconciliationReason::StaleScratch,
    ReconciliationReason::OrphanSidecar,
];

/// What reconciliation decided about one entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReconciliationVerdict {
    Collect(ReconciliationReason),
    /// Content a snapshot row names. Kept whether or not anything is currently pointed at it,
    /// because which bytes an installation ran is evidence and rollback needs the bytes.
    RetainReferencedPackage,
    /// Not something this build can name. Left alone and reported.
    Unrecognised,
}

impl ReconciliationVerdict {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Collect(reason) => reason.code(),
            Self::RetainReferencedPackage => "referenced_package",
            Self::Unrecognised => "unrecognised_entry",
        }
    }

    pub(crate) const fn collects(self) -> bool {
        matches!(self, Self::Collect(_))
    }
}

/// Decides what to do with one entry found under a root.
///
/// `segments` are the path components below the root: one for quarantine, two for a package
/// (`sha256/<hash>`), and two for scratch and sidecars (`<installation>/<generation>`). A shape
/// that does not match is unrecognised, which is the answer for a stray file as much as for a
/// directory nobody expected.
pub(crate) fn judge_entry(
    scope: ExtensionRootScope,
    segments: &[&str],
    referenced_hashes: &BTreeSet<String>,
) -> ReconciliationVerdict {
    match (scope, segments) {
        (ExtensionRootScope::Quarantine, [operation]) if is_opaque_segment(operation) => {
            ReconciliationVerdict::Collect(ReconciliationReason::AbandonedQuarantine)
        }
        (ExtensionRootScope::Packages, ["sha256", hash]) if PackageHash::parse(hash).is_ok() => {
            if referenced_hashes.contains(*hash) {
                ReconciliationVerdict::RetainReferencedPackage
            } else {
                ReconciliationVerdict::Collect(ReconciliationReason::UnreferencedPackage)
            }
        }
        (ExtensionRootScope::Scratch, [installation, generation])
            if is_opaque_segment(installation) && is_opaque_segment(generation) =>
        {
            ReconciliationVerdict::Collect(ReconciliationReason::StaleScratch)
        }
        (ExtensionRootScope::Sidecars, [installation, generation])
            if is_opaque_segment(installation) && is_opaque_segment(generation) =>
        {
            ReconciliationVerdict::Collect(ReconciliationReason::OrphanSidecar)
        }
        _ => ReconciliationVerdict::Unrecognised,
    }
}

/// The identifier shape every application-generated path segment has.
///
/// Checked rather than assumed: reconciliation deletes what this admits, so a name that arrived
/// from somewhere other than this application must not match.
fn is_opaque_segment(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.chars().all(|character| {
            character.is_ascii_alphanumeric() || character == '-' || character == '_'
        })
}

/// What one reconciliation run found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct ReconciliationSummary {
    pub(crate) collected: Vec<String>,
    pub(crate) retained: Vec<String>,
    /// Left exactly where they are, and reported. An operator decides what these are.
    pub(crate) unrecognised: Vec<String>,
    /// Recognised, and the removal failed. Not an error for the run: the entry is unreferenced,
    /// and the next start will try again.
    pub(crate) uncollectable: Vec<String>,
}

impl ReconciliationSummary {
    pub(crate) fn is_clean(&self) -> bool {
        self.unrecognised.is_empty() && self.uncollectable.is_empty()
    }
}
