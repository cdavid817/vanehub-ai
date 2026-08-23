//! Extension Platform domain model.
//!
//! Task Group 0 establishes only the capability-gate vocabulary. Package, manifest, snapshot,
//! lifecycle, runtime, and contribution models arrive with their own task groups.

mod activation;
mod canonical;
mod decode_error;
mod decode_reader;
mod error;
mod error_catalog;
#[cfg(test)]
mod error_catalog_tests;
mod feature;
mod identity;
mod install_witness;
#[cfg(test)]
mod install_witness_tests;
mod manifest;
mod manifest_decoder;
mod manifest_decoder_contributions;
#[cfg(test)]
mod manifest_decoder_tests;
mod manifest_digest;
#[cfg(test)]
mod manifest_digest_tests;
mod manifest_error;
mod manifest_fields;
mod manifest_integrity;
#[cfg(test)]
mod manifest_integrity_tests;
#[cfg(test)]
mod manifest_schema_tests;
#[cfg(test)]
mod manifest_shape_tests;
#[cfg(test)]
mod manifest_test_support;
#[cfg(test)]
mod manifest_tests;
mod network_origin;
#[cfg(test)]
mod network_origin_tests;
mod package_admission;
#[cfg(test)]
mod package_admission_tests;
mod package_layout;
#[cfg(test)]
mod package_layout_tests;
mod package_path;
#[cfg(test)]
mod package_path_tests;
mod publisher_key;
mod publisher_key_admission;
#[cfg(test)]
mod publisher_key_tests;
mod reconciliation;
#[cfg(test)]
mod reconciliation_tests;
mod runtime_generation;
mod signature_envelope;
#[cfg(test)]
mod signature_test_support;
#[cfg(test)]
mod signature_tests;
mod signature_verification;
mod signed_payload;
mod snapshot;
mod version_claim;
#[cfg(test)]
mod version_claim_tests;

pub(crate) use error::FeatureGateError;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use error_catalog::{
    all_decode_reasons, all_gate_errors, all_integrity_reasons, registered_failures, ErrorArea,
    RegisteredFailure, ALL_ORIGIN_REJECTIONS, ALL_PATH_REJECTIONS,
};
pub(crate) use feature::{
    evaluate_gate, ExtensionPlatformFeature, FeatureGateDegradation, FeatureGateEvaluation,
    FeatureGateFreshness, FeatureGateStatus, PrerequisiteReason, ALL_FEATURES,
};

// The manifest vocabulary, published ahead of the decoder that reads it (Task 1.E). Its own tests
// exercise every type; nothing outside the domain consumes them yet.
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use activation::{ActivationEvent, ActivationTarget};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use decode_error::{
    identifier_at, origin_at, path_at, DecodeReason, ManifestDecodeError,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use identity::{
    is_external_contribution_id, ContributionGlobalId, ContributionKind, ContributionLocalId,
    ExtensionId, InstallationId, OperationWitness, PackageHash, PublisherId, RuntimeGenerationId,
    SnapshotId, ALL_CONTRIBUTION_KINDS, MAX_EXTENSION_ID_CHARACTERS, MIN_EXTENSION_ID_CHARACTERS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest::{
    AuthorizationRuleContribution, CapabilityRequest, ConfigurationContribution,
    ConnectorContribution, ContributedRuleEffect, ContributionManifest, ExtensionDependency,
    ExtensionManifestV1, ExtensionRequirements, HookContribution, HookFailureMode,
    HookHandlerDeclaration, McpContribution, McpTransportDeclaration, ModePresetContribution,
    RuntimeDeclaration, RuntimeKind, SkillContribution, SkillDependency, ToolContribution,
    TransformContribution, TrustProfile, VersionedExtensionManifest, SUPPORTED_SCHEMA_VERSIONS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest_decoder::{
    ExtensionManifestV1Decoder, EXTENSION_MANIFEST_YAML_LIMITS, MAX_ACTIVATION_EVENTS,
    MAX_CONTRIBUTIONS_PER_KIND,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest_digest::{manifest_digest, ManifestDigest};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest_error::{
    ExtensionDomainError, ExtensionOriginError, ExtensionPathError, IdentifierKind,
    ALL_IDENTIFIER_KINDS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest_fields::{field_set, FieldSet, MANIFEST_FIELDS};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use manifest_integrity::{
    check_integrity, global_ids, IntegrityReason, IntegrityViolation,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use network_origin::{NetworkOrigin, OriginRejection, MAX_ORIGIN_CHARACTERS};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_path::{PathRejection, PortablePackagePath, MAX_PACKAGE_PATH_CHARACTERS};

// Package provenance (Task 2.3). Verification is a pure function of bytes and lives here; finding
// the key is a lookup against stored trust and lives behind an application port.
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use install_witness::{
    capability_diff, capability_lines, CapabilityDiff, CompatibilityOutcome, DependencySummary,
    ExtensionInstallWitness, InstallWitnessSubject, InstalledSummary, SignatureSummary,
    StaleWitness, WitnessField, ALL_WITNESS_FIELDS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_admission::{
    activation_eligibility, admit_package, all_developer_mode_errors, ActivationEligibility,
    AdmissionRefusal, AdmittedPackage, DeveloperMode, DeveloperModeError, PackageAdmission,
    PersistentWarning, ALL_ADMISSION_REFUSALS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use package_layout::{
    all_package_layout_rejections, check_manifest_against_layout, inspect_package_layout,
    ExtensionPackageLimits, PackageArchiveEntry, PackageLayout, PackageLayoutRejection,
    PackageLayoutViolation, DEFAULT_EXTENSION_PACKAGE_LIMITS, PACKAGE_DIRECTORIES,
    PACKAGE_MANIFEST_ENTRY, PACKAGE_ROOT_FILES,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use publisher_key::{
    parse_publisher_key_material, PublisherKeyFingerprint, PublisherKeyLabel, PublisherKeyRecord,
    PublisherKeyRejection, PublisherKeySource, PublisherPublicKey, PublisherTrustState,
    TrustedPublisherKey, ALL_PUBLISHER_KEY_REJECTIONS, MAX_PUBLISHER_KEY_LABEL_CHARACTERS,
    PUBLISHER_KEY_BYTES,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use publisher_key_admission::{
    all_publisher_key_errors, decide_admission, PublisherKeyAdmission, PublisherKeyError,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use reconciliation::{
    judge_entry, ExtensionRootScope, ReconciliationReason, ReconciliationSummary,
    ReconciliationVerdict, ALL_EXTENSION_ROOT_SCOPES, ALL_RECONCILIATION_REASONS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use runtime_generation::{
    all_runtime_generation_errors, ActiveGeneration, RuntimeGenerationError,
    RuntimeGenerationRecord,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use signature_envelope::{
    parse_signature_envelope, PackageSignature, SignatureAlgorithm, SignatureEnvelope,
    ENVELOPE_YAML_LIMITS, SIGNATURE_BYTES, SUPPORTED_ENVELOPE_VERSION,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use signature_verification::{
    verify_package_signature, ConfirmedSignature, PackageFacts, SignatureRejection, SignatureState,
    VerifiedSignature, ALL_SIGNATURE_REJECTIONS,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use signed_payload::signed_payload;
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use snapshot::{
    all_snapshot_publication_errors, ContentPublication, SnapshotPointer, SnapshotPublicationError,
    SnapshotRecord, StagedRecovery,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use version_claim::{
    decide_claim, ClaimAuthority, ClaimOutcome, ClaimProvenance, VersionClaim,
    VersionContentConflict, LOCAL_DEVELOPER_NAMESPACE,
};
