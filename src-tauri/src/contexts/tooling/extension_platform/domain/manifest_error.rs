// Landed ahead of its consumer; see `identity.rs`. Remove when Task 1.E's decoder lands.
#![cfg_attr(not(test), allow(dead_code))]

//! Domain errors raised while validating an extension's identity and manifest.
//!
//! Separate from `FeatureGateError`: a gate answers "is this capability on?", while these answer
//! "is this package describable?". Callers branch on the code and only humans read the message.
//! Untrusted text is truncated by the constructor that builds the error, so a hostile manifest
//! cannot make a diagnostic unbounded.

use super::package_path::PathRejection;
use std::fmt;

/// Which identifier failed to validate.
///
/// A rejected identifier is always the same event with a different subject, so the subject is a
/// field rather than nine variants with identical payloads. That also keeps the codes derived
/// from one table instead of a match arm that can drift.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum IdentifierKind {
    Extension,
    Publisher,
    Contribution,
    PackageHash,
    Snapshot,
    Installation,
    RuntimeGeneration,
    OperationWitness,
    ActivationEvent,
}

impl IdentifierKind {
    pub(crate) const fn code(self) -> &'static str {
        match self {
            Self::Extension => "invalid_extension_id",
            Self::Publisher => "invalid_publisher_id",
            Self::Contribution => "invalid_contribution_id",
            Self::PackageHash => "invalid_package_hash",
            Self::Snapshot => "invalid_snapshot_id",
            Self::Installation => "invalid_installation_id",
            Self::RuntimeGeneration => "invalid_runtime_generation_id",
            Self::OperationWitness => "invalid_operation_witness",
            Self::ActivationEvent => "invalid_activation_event",
        }
    }
}

pub(crate) const ALL_IDENTIFIER_KINDS: [IdentifierKind; 9] = [
    IdentifierKind::Extension,
    IdentifierKind::Publisher,
    IdentifierKind::Contribution,
    IdentifierKind::PackageHash,
    IdentifierKind::Snapshot,
    IdentifierKind::Installation,
    IdentifierKind::RuntimeGeneration,
    IdentifierKind::OperationWitness,
    IdentifierKind::ActivationEvent,
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionDomainError {
    identifier: IdentifierKind,
    /// The offending text, already truncated where it came from an untrusted source.
    value: String,
}

impl ExtensionDomainError {
    pub(crate) fn new(identifier: IdentifierKind, value: String) -> Self {
        Self { identifier, value }
    }

    pub(crate) const fn identifier(&self) -> IdentifierKind {
        self.identifier
    }

    pub(crate) const fn code(&self) -> &'static str {
        self.identifier.code()
    }

    pub(crate) fn value(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for ExtensionDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.code(), self.value())
    }
}

impl std::error::Error for ExtensionDomainError {}

/// A declared path that is not portable, and which rule it broke.
///
/// Separate from `ExtensionDomainError` because the reason matters to the operator: "this path
/// escapes the package" and "this name is a device on Windows" call for different fixes, and
/// collapsing them into one code would tell a publisher nothing actionable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExtensionPathError {
    pub(crate) path: String,
    pub(crate) reason: PathRejection,
}

impl ExtensionPathError {
    pub(crate) const fn code(&self) -> &'static str {
        "invalid_package_path"
    }

    pub(crate) const fn reason_code(&self) -> &'static str {
        self.reason.as_str()
    }
}

impl fmt::Display for ExtensionPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.code(),
            self.path,
            self.reason_code()
        )
    }
}

impl std::error::Error for ExtensionPathError {}
