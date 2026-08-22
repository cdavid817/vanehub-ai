// No production caller yet; see `identity.rs`.
#![cfg_attr(not(test), allow(dead_code))]

//! Every stable failure identity this context can produce.
//!
//! Callers branch on codes, so the invariant that matters is global: two failures meaning
//! different things must never present the same identity, across error types written months apart
//! by people each looking at one file. Collecting them here is what makes that checkable — the
//! alternative is a set of per-type uniqueness tests that are individually green and collectively
//! wrong.
//!
//! The identity is what a caller actually reads. For a path or origin failure that is the pair
//! `code` plus `reason_code`, not the reason alone: `empty` is a reason for both, and only the
//! outer code says which was empty. Registering the pair keeps that honest instead of forcing two
//! subsystems to invent uglier words for the same idea.
//!
//! Kinds arriving with later task groups — package, dependency resolution, lifecycle, runtime,
//! Hook dispatch, rule compilation, connector, stale witness — register here as they land, so the
//! collision check is never retrofitted onto a codebase that already has one.

use super::runtime_generation::all_runtime_generation_errors;
use super::{
    all_developer_mode_errors, all_publisher_key_errors, all_snapshot_publication_errors,
    DecodeReason, ExtensionPlatformFeature, FeatureGateError, IdentifierKind, IntegrityReason,
    OriginRejection, PathRejection, ALL_ADMISSION_REFUSALS, ALL_IDENTIFIER_KINDS,
    ALL_PUBLISHER_KEY_REJECTIONS, ALL_SIGNATURE_REJECTIONS,
};

/// Which subsystem a failure belongs to. Present so a reader can see which groups have landed and
/// which are still to come.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum ErrorArea {
    CapabilityGate,
    Identity,
    PackagePath,
    NetworkOrigin,
    ManifestDecode,
    ManifestIntegrity,
    PackageSignature,
    PublisherKeyManagement,
    PackageAdmission,
    SnapshotPublication,
    RuntimeGeneration,
}

impl ErrorArea {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::CapabilityGate => "capability_gate",
            Self::Identity => "identity",
            Self::PackagePath => "package_path",
            Self::NetworkOrigin => "network_origin",
            Self::ManifestDecode => "manifest_decode",
            Self::ManifestIntegrity => "manifest_integrity",
            Self::PackageSignature => "package_signature",
            Self::PublisherKeyManagement => "publisher_key_management",
            Self::PackageAdmission => "package_admission",
            Self::SnapshotPublication => "snapshot_publication",
            Self::RuntimeGeneration => "runtime_generation",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RegisteredFailure {
    pub(crate) area: ErrorArea,
    /// Exactly what a caller matches on. A qualified pair where the type presents one.
    pub(crate) identity: String,
}

/// Every failure identity, in a stable order.
///
/// Built from the enums rather than hand-listed alongside them: a second copy of each variant list
/// would be a second place to forget one, which is the failure this exists to catch.
pub(crate) fn registered_failures() -> Vec<RegisteredFailure> {
    let mut failures = Vec::new();

    let mut push = |area: ErrorArea, identity: String| {
        failures.push(RegisteredFailure { area, identity });
    };

    for error in all_gate_errors() {
        push(ErrorArea::CapabilityGate, error.code().to_string());
    }
    for kind in ALL_IDENTIFIER_KINDS {
        push(ErrorArea::Identity, kind.code().to_string());
    }
    for reason in ALL_PATH_REJECTIONS {
        push(
            ErrorArea::PackagePath,
            format!("invalid_package_path:{}", reason.as_str()),
        );
    }
    for reason in ALL_ORIGIN_REJECTIONS {
        push(
            ErrorArea::NetworkOrigin,
            format!("invalid_network_origin:{}", reason.as_str()),
        );
    }
    for reason in all_decode_reasons() {
        push(ErrorArea::ManifestDecode, reason.code().to_string());
    }
    for reason in all_integrity_reasons() {
        push(ErrorArea::ManifestIntegrity, reason.code().to_string());
    }
    for rejection in ALL_SIGNATURE_REJECTIONS {
        push(ErrorArea::PackageSignature, rejection.code().to_string());
    }
    for rejection in ALL_PUBLISHER_KEY_REJECTIONS {
        push(
            ErrorArea::PublisherKeyManagement,
            rejection.code().to_string(),
        );
    }
    for error in all_publisher_key_errors() {
        push(ErrorArea::PublisherKeyManagement, error.code().to_string());
    }
    for refusal in ALL_ADMISSION_REFUSALS {
        push(ErrorArea::PackageAdmission, refusal.code().to_string());
    }
    for error in all_developer_mode_errors() {
        push(ErrorArea::PackageAdmission, error.code().to_string());
    }
    for error in all_snapshot_publication_errors() {
        push(ErrorArea::SnapshotPublication, error.code().to_string());
    }
    for error in all_runtime_generation_errors() {
        push(ErrorArea::RuntimeGeneration, error.code().to_string());
    }

    failures
}

pub(crate) fn all_gate_errors() -> Vec<FeatureGateError> {
    vec![
        FeatureGateError::FeatureUnavailableInBuild {
            feature: ExtensionPlatformFeature::Catalog,
        },
        FeatureGateError::StaleRevision {
            feature: ExtensionPlatformFeature::Catalog,
            expected: 0,
            actual: 0,
        },
        FeatureGateError::Storage(String::new()),
    ]
}

pub(crate) const ALL_PATH_REJECTIONS: [PathRejection; 16] = [
    PathRejection::Empty,
    PathRejection::TooLong,
    PathRejection::TooDeep,
    PathRejection::NulByte,
    PathRejection::Backslash,
    PathRejection::ControlCharacter,
    PathRejection::DirectionOverride,
    PathRejection::Absolute,
    PathRejection::UncPrefix,
    PathRejection::DrivePrefix,
    PathRejection::AlternateDataStream,
    PathRejection::EmptySegment,
    PathRejection::CurrentDirectorySegment,
    PathRejection::ParentDirectorySegment,
    PathRejection::TrailingDotOrSpace,
    PathRejection::WindowsReservedName,
];

pub(crate) const ALL_ORIGIN_REJECTIONS: [OriginRejection; 11] = [
    OriginRejection::Empty,
    OriginRejection::TooLong,
    OriginRejection::Wildcard,
    OriginRejection::Unparseable,
    OriginRejection::UnsupportedScheme,
    OriginRejection::InsecureRemoteScheme,
    OriginRejection::Userinfo,
    OriginRejection::MissingHost,
    OriginRejection::HasPath,
    OriginRejection::HasQuery,
    OriginRejection::HasFragment,
];

/// Carries data, so a list rather than a const.
pub(crate) fn all_integrity_reasons() -> Vec<IntegrityReason> {
    vec![
        IntegrityReason::UnknownHookReference,
        IntegrityReason::UnknownMcpReference,
        IntegrityReason::CaseInsensitivePathCollision {
            other: String::new(),
        },
        IntegrityReason::UnicodePathCollision {
            other: String::new(),
        },
        IntegrityReason::HandlerWithoutRuntime,
        IntegrityReason::UnreachableActivationEvent,
    ]
}

/// Carries data, so a list rather than a const.
pub(crate) fn all_decode_reasons() -> Vec<DecodeReason> {
    vec![
        DecodeReason::Missing,
        DecodeReason::ExpectedScalar,
        DecodeReason::ExpectedMapping,
        DecodeReason::ExpectedScalarSequence,
        DecodeReason::UnknownField,
        DecodeReason::ListOfRecords,
        DecodeReason::InvalidIdentifier(IdentifierKind::Extension),
        DecodeReason::InvalidPath(PathRejection::Backslash),
        DecodeReason::InvalidOrigin(OriginRejection::Wildcard),
        DecodeReason::InvalidVersion,
        DecodeReason::InvalidVersionRequirement,
        DecodeReason::UnsupportedSchemaVersion { declared: 0 },
        DecodeReason::IncompatibleApplicationVersion {
            required: String::new(),
            running: String::new(),
        },
        DecodeReason::UnknownValue { expected: "" },
        DecodeReason::NotPermitted { detail: "" },
        DecodeReason::TooMany { limit: 0 },
        DecodeReason::Empty,
        DecodeReason::MalformedDocument { code: "" },
    ]
}
