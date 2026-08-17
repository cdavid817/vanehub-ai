mod diagnostics;
mod error;
mod identity;
mod lifecycle;
mod limits;
mod manifest;
mod module_inspection;
mod permission_manifest;
mod schema;
mod trust;

#[cfg(test)]
mod manifest_tests;

// Section 1 publishes the whole contract surface; the dispatchers, sandbox, catalog, and
// governance commands that consume the rest of it arrive in sections 3 through 9.
#[allow(unused_imports)]
pub(crate) use diagnostics::{
    SkillToolDiagnostic, SkillToolDiagnosticSeverity, SkillToolDiagnosticSummary,
    MAX_DIAGNOSTIC_DETAIL_CHARACTERS, MAX_DIAGNOSTIC_ENTRIES, MAX_DIAGNOSTIC_SUMMARY_CHARACTERS,
};
#[allow(unused_imports)]
pub(crate) use error::{BoundedSchemaError, SkillToolDomainError};
#[allow(unused_imports)]
pub(crate) use identity::{
    SkillToolId, SkillToolKey, SkillToolOwnerId, SkillToolRevision, SkillToolScope,
    SkillToolSourceScope, MAX_CANONICAL_NAME_CHARACTERS, REVISION_FRAGMENT_CHARACTERS,
};
#[allow(unused_imports)]
pub(crate) use lifecycle::{
    SkillToolAvailability, SkillToolIneligibility, SkillToolLifecycle, SkillToolQuarantine,
    SkillToolValidationState, QUARANTINE_FAILURE_THRESHOLD,
};
#[allow(unused_imports)]
pub(crate) use limits::{
    is_reserved_executable_path, SkillToolLimitOverrides, SkillToolLimits, SkillToolManifestLimits,
    DEFAULT_MANIFEST_LIMITS, DEFAULT_SKILL_TOOL_LIMITS, MANIFEST_PATH, MODULE_DIRECTORY,
    RESERVED_EXECUTABLE_PREFIXES, SUPPORTED_MANIFEST_VERSION,
};
#[allow(unused_imports)]
pub(crate) use manifest::{
    parse_manifest, parse_manifest_bytes, ContentHash, DeclarativeField, DeclarativeFieldSource,
    DeclarativeImplementation, ModuleImplementation, SkillToolCapability, SkillToolDeclaration,
    SkillToolImplementation, SkillToolManifest,
};
#[allow(unused_imports)]
pub(crate) use module_inspection::{
    inspect_module, ModuleInspection, ModuleInspectionError, HOST_IMPORT_MODULE,
};
#[allow(unused_imports)]
pub(crate) use permission_manifest::{
    parse_permission_manifest, SkillFilesystemPermissions, SkillNetworkPermissions,
    SkillProcessCommand, SkillProcessPermissions, SkillProvenanceTrust, SkillToolPermissions,
};
#[allow(unused_imports)]
pub(crate) use schema::{validate_bounded_schema, BoundedJsonSchema, BoundedSchemaMetrics};
#[allow(unused_imports)]
pub(crate) use trust::{
    capability_digest, content_hash_of, declarative_implementation_hash, integrity_for,
    revision_witness, SkillToolIntegrity, SkillToolTrustDecision, SkillToolTrustRecord,
};
