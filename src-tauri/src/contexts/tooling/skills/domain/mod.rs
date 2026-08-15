mod binding;
mod catalog;
mod classification;
mod config_document;
mod config_schema;
mod config_state;
mod drift;
mod error;
mod identity;
mod metadata;
mod mount;
mod mutation;
mod overlay;
mod overlay_guidance;
mod overlay_limits;
mod overlay_media;
mod overlay_path;
mod overlay_replay;
mod overlay_resources;
mod overlay_scope_replay;
mod overlay_text_scanner;
mod source;

#[cfg(test)]
mod config_document_tests;
#[cfg(test)]
mod config_schema_tests;
#[cfg(test)]
mod config_state_tests;
#[cfg(test)]
mod overlay_guidance_tests;
#[cfg(test)]
mod overlay_limits_tests;
#[cfg(test)]
mod overlay_media_tests;
#[cfg(test)]
mod overlay_path_tests;
#[cfg(test)]
mod overlay_replay_tests;
#[cfg(test)]
mod overlay_resource_tests;
#[cfg(test)]
mod overlay_scope_replay_tests;
#[cfg(test)]
mod overlay_tests;
#[cfg(test)]
mod overlay_text_scanner_tests;

pub(crate) use binding::{plan_binding_change, plan_enablement, SkillBindingPlan};
pub(crate) use catalog::{builtin_definition, builtin_definitions, BuiltinSkillDefinition};
pub(crate) use classification::{
    resolve_skill_identity, SkillAvailability, SkillCompatibilityDefaults, SkillDelivery,
    SkillIdentityCandidate, SkillLayer, SkillLookupOutcome, SkillOrigin, SkillTrust, SkillType,
};
#[allow(unused_imports)]
pub(crate) use config_schema::{
    parse_config_schema, validate_value, SkillConfigField, SkillConfigSchema,
    SkillConfigSchemaError, SkillConfigValue,
};
#[allow(unused_imports)]
pub(crate) use config_state::{
    classify_drift, readiness_for, resolve_effective, RedactedSecret, SkillConfigDrift,
    SkillConfigProperty, SkillConfigProvenance, SkillConfigReadiness, SkillConfigRevision,
    SkillConfigScope, SkillConfigSnapshot, SkillSecretIntent, SkillSecretState,
};
pub(crate) use drift::{
    detect_drift, RegisteredSkillInspection, SkillBindingInspection, SkillDriftInspection,
    SkillDriftIssue, SkillDriftIssueType, SkillMountObservation, SkillSourceInspection,
    UnregisteredSkillInspection,
};
pub(crate) use error::SkillDomainError;
pub(crate) use identity::{SkillId, SkillKey, SkillLocation, SkillScope};
pub(crate) use metadata::SkillMetadata;
pub(crate) use mount::{default_mount_path, SkillMountPath};
pub(crate) use mutation::{
    builtin_restore_plan, deletion_policy, source_for_user_create, validate_create_identity,
    validate_update_identity,
};
#[cfg_attr(not(test), allow(unused_imports))]
pub(crate) use overlay::{
    OverlayBaseWitness, OverlayConflict, OverlayConflictState, OverlayDocument, OverlayFile,
    OverlayLearnBlock, OverlayMutationState, OverlayOrigin, OverlayPatch, OverlayScope,
    OverlayTrust, OverlayTrustState, OVERLAY_SCHEMA_VERSION,
};
#[allow(unused_imports)]
pub(crate) use overlay_guidance::{
    render_learned_guidance, LearnedGuidanceConflict, LearnedGuidanceConflictReason,
    LearnedGuidanceReplay, ScopedLearnedGuidance, LEARNED_GUIDANCE_END_MARKER,
    LEARNED_GUIDANCE_HEADING, LEARNED_GUIDANCE_START_MARKER,
};
#[allow(unused_imports)]
pub(crate) use overlay_limits::{OverlayLimits, DEFAULT_OVERLAY_LIMITS};
#[allow(unused_imports)]
pub(crate) use overlay_media::{
    validate_overlay_media, OverlayContentKind, OverlayMediaError, ValidatedOverlayMedia,
};
#[allow(unused_imports)]
pub(crate) use overlay_path::{
    validate_overlay_link_target, validate_overlay_path, OverlayPathError, ValidatedOverlayPath,
};
#[allow(unused_imports)]
pub(crate) use overlay_replay::{
    replay_exact_patches, ExactPatchApplication, ExactPatchConflict, ExactPatchConflictReason,
    ExactPatchReplay,
};
#[allow(unused_imports)]
pub(crate) use overlay_resources::{
    apply_overlay_resource_scope, merge_overlay_resources, BaseSkillResource,
    EffectiveResourceSource, EffectiveSkillResource, OverlayResourceReplay, ResourceShadowSummary,
    ScopedOverlayFiles,
};
#[allow(unused_imports)]
pub(crate) use overlay_scope_replay::{
    replay_overlay_scope_chain, EffectiveOverlaySnapshot, OverlayIntegrityFailure,
    OverlayScopeConflict, OverlayScopeReplay, OverlayScopeReplayInput, OverlayScopeReplayResult,
    OverlayScopeReplayStatus,
};
#[allow(unused_imports)]
pub(crate) use overlay_text_scanner::{
    scan_overlay_text, OverlayTextRuleId, OverlayTextScan, OVERLAY_TEXT_SCANNER_VERSION,
};
pub(crate) use source::SkillSource;
