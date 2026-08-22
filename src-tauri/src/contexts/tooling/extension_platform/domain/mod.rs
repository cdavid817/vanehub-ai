//! Extension Platform domain model.
//!
//! Task Group 0 establishes only the capability-gate vocabulary. Package, manifest, snapshot,
//! lifecycle, runtime, and contribution models arrive with their own task groups.

mod activation;
mod decode_error;
mod decode_reader;
mod error;
mod error_catalog;
#[cfg(test)]
mod error_catalog_tests;
mod feature;
mod identity;
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
mod manifest_tests;
mod network_origin;
#[cfg(test)]
mod network_origin_tests;
mod package_path;
#[cfg(test)]
mod package_path_tests;

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
