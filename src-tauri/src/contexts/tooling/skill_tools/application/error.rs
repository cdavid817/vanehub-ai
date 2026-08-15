use crate::contexts::tooling::skill_tools::domain::{ModuleInspectionError, SkillToolDomainError};

/// Use-case level failures for Skill tool discovery, integrity, and state persistence.
///
/// Carries a `code()` for the same reason the domain error does: these become unified-log fields
/// and, in section 7, command-boundary error contracts, so the machine-readable reason has to be
/// stable independently of message wording.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolApplicationError {
    Domain(SkillToolDomainError),
    ModuleInspection(ModuleInspectionError),
    /// The manifest declares a Skill id that is not the package it was found in.
    OwnerMismatch {
        declared: String,
        package: String,
    },
    PathEscape(String),
    OversizedFile {
        path: String,
        maximum: u64,
        actual: u64,
    },
    OversizedPackage {
        maximum: u64,
        actual: u64,
    },
    IntegrityMismatch {
        path: String,
    },
    MissingImplementationFile(String),
    /// A revision that changed underneath a read; the caller must re-resolve rather than mix
    /// content from two revisions into one witness.
    StaleRevision,
    ModuleRuntimeUnavailable,
    NotFound(String),
    Storage(String),
    Filesystem(String),
}

impl SkillToolApplicationError {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Domain(error) => error.code(),
            Self::ModuleInspection(error) => error.code(),
            Self::OwnerMismatch { .. } => "manifest-owner-mismatch",
            Self::PathEscape(_) => "path-escape",
            Self::OversizedFile { .. } => "oversized-file",
            Self::OversizedPackage { .. } => "oversized-package",
            Self::IntegrityMismatch { .. } => "integrity-mismatch",
            Self::MissingImplementationFile(_) => "missing-implementation-file",
            Self::StaleRevision => "stale-revision",
            Self::ModuleRuntimeUnavailable => "module-runtime-unavailable",
            Self::NotFound(_) => "not-found",
            Self::Storage(_) => "storage",
            Self::Filesystem(_) => "filesystem",
        }
    }

    /// Whether the failure is a property of the content rather than of this machine.
    ///
    /// Only content failures may mark a revision invalid: a transient storage or filesystem error
    /// must not turn into a persistent "this Skill's tools are broken" verdict.
    pub(crate) fn is_content_failure(&self) -> bool {
        !matches!(
            self,
            Self::Storage(_) | Self::Filesystem(_) | Self::StaleRevision
        )
    }
}

impl From<SkillToolDomainError> for SkillToolApplicationError {
    fn from(error: SkillToolDomainError) -> Self {
        Self::Domain(error)
    }
}

impl From<ModuleInspectionError> for SkillToolApplicationError {
    fn from(error: ModuleInspectionError) -> Self {
        Self::ModuleInspection(error)
    }
}
