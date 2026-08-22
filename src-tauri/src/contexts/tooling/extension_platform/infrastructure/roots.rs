// The install and runtime flows that use these land with Task Groups 4 and 5; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! The four directories the Extension Platform owns, and nothing else.
//!
//! ```text
//! extensions/
//!   quarantine/<operation-witness>/          unreviewed bytes, during one operation
//!   packages/sha256/<hash>/                  immutable content, named by what it is
//!   scratch/<installation>/<generation>/     one runtime generation's working space
//!   sidecars/<installation>/<generation>/    one sidecar process's working space
//! ```
//!
//! They are separate because their lifetimes are: quarantine belongs to an operation and never
//! outlives it, a package is immutable and outlives everything that points at it, and scratch and
//! sidecar space belong to a generation and are gone when it is. One shared directory would make
//! "is this safe to delete?" a question nobody could answer.
//!
//! Every path is built from validated identifiers rather than from strings, and every segment is
//! re-checked as a portable path before it is used. The identifiers are application-generated and
//! already exclude separators, but "already excludes" is a property of today's rule, and the cost
//! of confirming it here is one comparison against a rule that is written down.

use crate::contexts::tooling::extension_platform::domain::{
    ExtensionRootScope, InstallationId, OperationWitness, PackageHash, PortablePackagePath,
    RuntimeGenerationId, ALL_EXTENSION_ROOT_SCOPES,
};
use crate::platform::filesystem::{
    create_owned_directory, ensure_owned_root, remove_owned_tree, verify_owned, OwnershipError,
};
use std::path::{Path, PathBuf};

const QUARANTINE: &str = "quarantine";
const PACKAGES: &str = "packages";
const PACKAGE_ALGORITHM: &str = "sha256";
const SCRATCH: &str = "scratch";
const SIDECARS: &str = "sidecars";

/// Why a root operation did not happen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootError {
    Ownership(OwnershipError),
    /// An identifier that cannot be used as a path segment. Application-generated identifiers do
    /// not produce this; a stored one that was edited by hand would.
    UnusableSegment,
}

impl RootError {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Ownership(error) => error.code(),
            Self::UnusableSegment => "unusable_path_segment",
        }
    }
}

impl From<OwnershipError> for RootError {
    fn from(error: OwnershipError) -> Self {
        Self::Ownership(error)
    }
}

/// What each root is called on disk. The scope itself is a domain concept — which lifetimes live
/// where — and only the names belong here.
const fn directory(scope: ExtensionRootScope) -> &'static str {
    match scope {
        ExtensionRootScope::Quarantine => QUARANTINE,
        ExtensionRootScope::Packages => PACKAGES,
        ExtensionRootScope::Scratch => SCRATCH,
        ExtensionRootScope::Sidecars => SIDECARS,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ExtensionRoots {
    base: PathBuf,
}

impl ExtensionRoots {
    /// `base` is the `extensions` directory under application data.
    pub(crate) fn new(base: PathBuf) -> Self {
        Self { base }
    }

    pub(crate) fn base(&self) -> &Path {
        &self.base
    }

    pub(crate) fn root(&self, scope: ExtensionRootScope) -> PathBuf {
        self.base.join(directory(scope))
    }

    /// Creates the four roots. Called once at startup so that later operations fail on their own
    /// path rather than on a missing parent.
    pub(crate) fn prepare(&self) -> Result<(), RootError> {
        ensure_owned_root(&self.base)?;
        for scope in ALL_EXTENSION_ROOT_SCOPES {
            create_owned_directory(&self.base, &self.root(scope))?;
        }
        Ok(())
    }

    /// Where one operation's unreviewed bytes go.
    ///
    /// Keyed by the operation witness rather than by the extension: at quarantine time nothing has
    /// been established about which extension this is, and a directory named after a claim the
    /// package made would be named by the package.
    pub(crate) fn quarantine(&self, operation: &OperationWitness) -> Result<PathBuf, RootError> {
        self.resolve(ExtensionRootScope::Quarantine, &[operation.as_str()])
    }

    /// Where one immutable set of package bytes lives, named by its own digest.
    pub(crate) fn package(&self, hash: &PackageHash) -> Result<PathBuf, RootError> {
        self.resolve(
            ExtensionRootScope::Packages,
            &[PACKAGE_ALGORITHM, hash.as_str()],
        )
    }

    pub(crate) fn scratch(
        &self,
        installation: &InstallationId,
        generation: &RuntimeGenerationId,
    ) -> Result<PathBuf, RootError> {
        self.resolve(
            ExtensionRootScope::Scratch,
            &[installation.as_str(), generation.as_str()],
        )
    }

    pub(crate) fn sidecar(
        &self,
        installation: &InstallationId,
        generation: &RuntimeGenerationId,
    ) -> Result<PathBuf, RootError> {
        self.resolve(
            ExtensionRootScope::Sidecars,
            &[installation.as_str(), generation.as_str()],
        )
    }

    /// Creates a directory this application owns, having confirmed every step of the way to it.
    pub(crate) fn create(&self, path: &Path) -> Result<(), RootError> {
        Ok(create_owned_directory(&self.base, path)?)
    }

    /// Removes a subtree, and only after confirming the application owns the way to it.
    pub(crate) fn discard(&self, path: &Path) -> Result<(), RootError> {
        Ok(remove_owned_tree(&self.base, path)?)
    }

    /// Confirms a path is one of ours. Used before reading a snapshot, so a substituted component
    /// is caught rather than followed.
    pub(crate) fn verify(&self, path: &Path) -> Result<(), RootError> {
        Ok(verify_owned(&self.base, path)?)
    }

    fn resolve(&self, scope: ExtensionRootScope, segments: &[&str]) -> Result<PathBuf, RootError> {
        let mut path = self.root(scope);
        for segment in segments {
            // The portable-path rule, applied to one segment. It refuses separators, traversal,
            // control characters, Windows device names, and trailing dots — the last of which an
            // identifier rule that only lists permitted characters would let through.
            PortablePackagePath::parse(segment).map_err(|_| RootError::UnusableSegment)?;
            path.push(segment);
        }
        Ok(path)
    }
}
