// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! What went wrong, and exactly where.
//!
//! A manifest rejection has to be actionable by whoever wrote the manifest, so every error names
//! a dotted field path rather than "invalid manifest". Field paths are built from known field
//! names plus ids the parser already bounded, so the diagnostic cannot grow with the input.

use super::{
    ExtensionDomainError, ExtensionOriginError, ExtensionPathError, IdentifierKind,
    OriginRejection, PathRejection,
};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ManifestDecodeError {
    /// Dotted path into the document, such as `contributes.tools.git_status.handler`.
    field: String,
    reason: DecodeReason,
}

impl ManifestDecodeError {
    pub(crate) fn new(field: impl Into<String>, reason: DecodeReason) -> Self {
        Self {
            field: field.into(),
            reason,
        }
    }

    pub(crate) fn field(&self) -> &str {
        &self.field
    }

    pub(crate) fn reason(&self) -> &DecodeReason {
        &self.reason
    }

    pub(crate) fn code(&self) -> &'static str {
        self.reason.code()
    }
}

impl fmt::Display for ManifestDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.field, self.reason)
    }
}

impl std::error::Error for ManifestDecodeError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DecodeReason {
    /// A required field is absent.
    Missing,
    /// Present, but the wrong shape. Carries what was expected, because "expected a mapping" is
    /// the whole content of the fix.
    ExpectedScalar,
    ExpectedMapping,
    ExpectedScalarSequence,
    /// A key nothing reads. Refused rather than ignored: a security-relevant field this build does
    /// not know is a build that should not install the package.
    UnknownField,
    /// The author wrote a list of records where an id-keyed mapping belongs. Named specifically
    /// because it is the shape someone arriving from another extension ecosystem writes first.
    ListOfRecords,
    /// A value that should have been an identifier was not.
    InvalidIdentifier(IdentifierKind),
    InvalidPath(PathRejection),
    InvalidOrigin(OriginRejection),
    /// Not parseable as a semantic version, or as a version requirement.
    InvalidVersion,
    InvalidVersionRequirement,
    /// The manifest declares a schema this build does not implement. Not "mostly readable": the
    /// security meaning of fields added later is unknown, and guessing is how a package acquires
    /// authority its author never declared.
    UnsupportedSchemaVersion {
        declared: u32,
    },
    /// The running application does not satisfy `min_vanehub_version`.
    IncompatibleApplicationVersion {
        required: String,
        running: String,
    },
    /// A closed vocabulary — runtime kind, trust profile, failure mode, rule effect — received
    /// something outside it.
    UnknownValue {
        expected: &'static str,
    },
    /// Structurally fine but not admissible: a `.vhext` naming the built-in runtime, a rule
    /// asking for Allow, a runtime entry present without a runtime.
    NotPermitted {
        detail: &'static str,
    },
    /// More entries than the manifest profile admits.
    TooMany {
        limit: usize,
    },
    /// Present but empty where empty means nothing.
    Empty,
    /// The bytes were not a well-formed bounded document at all, so no field can be named. Carries
    /// the parser's own code rather than a message, because that is what a caller branches on.
    MalformedDocument {
        code: &'static str,
    },
}

impl DecodeReason {
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::Missing => "missing_field",
            Self::ExpectedScalar => "expected_scalar",
            Self::ExpectedMapping => "expected_mapping",
            Self::ExpectedScalarSequence => "expected_scalar_sequence",
            Self::UnknownField => "unknown_field",
            Self::ListOfRecords => "list_of_records",
            Self::InvalidIdentifier(_) => "invalid_identifier",
            Self::InvalidPath(_) => "invalid_package_path",
            Self::InvalidOrigin(_) => "invalid_network_origin",
            Self::InvalidVersion => "invalid_version",
            Self::InvalidVersionRequirement => "invalid_version_requirement",
            Self::UnsupportedSchemaVersion { .. } => "unsupported_schema_version",
            Self::IncompatibleApplicationVersion { .. } => "incompatible_application_version",
            Self::UnknownValue { .. } => "unknown_value",
            Self::NotPermitted { .. } => "not_permitted",
            Self::TooMany { .. } => "too_many_entries",
            Self::Empty => "empty_value",
            Self::MalformedDocument { .. } => "malformed_document",
        }
    }
}

impl fmt::Display for DecodeReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => formatter.write_str("is required"),
            Self::ExpectedScalar => formatter.write_str("expects a single value"),
            Self::ExpectedMapping => formatter.write_str("expects a mapping"),
            Self::ExpectedScalarSequence => formatter.write_str("expects a list of values"),
            Self::UnknownField => formatter.write_str("is not a field this version reads"),
            Self::ListOfRecords => formatter.write_str(
                "is a list of records; write one entry per id instead, as `<id>:` with its fields \
                 indented beneath",
            ),
            Self::InvalidIdentifier(kind) => write!(formatter, "is not a valid {}", kind.code()),
            Self::InvalidPath(reason) => {
                write!(formatter, "is not a portable path ({})", reason.as_str())
            }
            Self::InvalidOrigin(reason) => write!(
                formatter,
                "is not a scheme://host origin ({})",
                reason.as_str()
            ),
            Self::InvalidVersion => formatter.write_str("is not a semantic version"),
            Self::InvalidVersionRequirement => {
                formatter.write_str("is not a semantic version requirement")
            }
            Self::UnsupportedSchemaVersion { declared } => write!(
                formatter,
                "declares schema version {declared}, which this build does not implement"
            ),
            Self::IncompatibleApplicationVersion { required, running } => write!(
                formatter,
                "requires VaneHub {required}, and this build is {running}"
            ),
            Self::UnknownValue { expected } => write!(formatter, "expects one of: {expected}"),
            Self::NotPermitted { detail } => formatter.write_str(detail),
            Self::TooMany { limit } => write!(formatter, "exceeds {limit} entries"),
            Self::Empty => formatter.write_str("cannot be empty"),
            Self::MalformedDocument { code } => {
                write!(formatter, "is not a readable document ({code})")
            }
        }
    }
}

/// Lifts an identifier rejection into a field-located decode error, so the caller reports *which*
/// field held the bad id rather than only that some id was bad.
pub(crate) fn identifier_at(
    field: impl Into<String>,
    error: &ExtensionDomainError,
) -> ManifestDecodeError {
    ManifestDecodeError::new(field, DecodeReason::InvalidIdentifier(error.identifier()))
}

pub(crate) fn path_at(field: impl Into<String>, error: &ExtensionPathError) -> ManifestDecodeError {
    ManifestDecodeError::new(field, DecodeReason::InvalidPath(error.reason))
}

pub(crate) fn origin_at(
    field: impl Into<String>,
    error: &ExtensionOriginError,
) -> ManifestDecodeError {
    ManifestDecodeError::new(field, DecodeReason::InvalidOrigin(error.reason))
}
