mod config_overview;
mod effective_catalog;
mod error;
mod models;
mod overlay_logging;
mod overlay_models;
mod overlay_ports;
mod overlay_preparation;
mod overlay_promotion;
mod overlay_reconciliation;
mod overlay_runtime;
mod overlay_service;
mod ports;
mod progressive;
mod service;

#[cfg(test)]
mod config_overview_tests;
#[cfg(test)]
mod overlay_logging_tests;
#[cfg(test)]
mod overlay_promotion_tests;
#[cfg(test)]
mod overlay_service_tests;
#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub(crate) use config_overview::{
    SkillConfigurableState, SkillConfigurationContext, SkillConfigurationOverview,
    SkillScopedConfigSummary,
};
pub(crate) use effective_catalog::{
    normalize_utility_availability, preview_package, resolve_effective_catalog,
};
pub(crate) use error::{OverlayApplicationError, SkillApplicationError};
pub(crate) use models::{
    AgentMountConfiguration, BuiltinCleanupStatus, BuiltinReconciliationOutcome,
    BuiltinReconciliationState, EffectiveCatalogCacheKey, EffectiveSkill, ManagedSkillSource,
    SkillAccessRefusal, SkillAccessRefusalReason, SkillAgentBinding, SkillAgentKind,
    SkillAgentMountPath, SkillBackupEntry, SkillCompatibleAgent, SkillCreateRequest,
    SkillDelegationSummary, SkillDiscoveryEntry, SkillDiscoveryRequest, SkillDiscoveryResult,
    SkillDocument, SkillDriftReport, SkillEffectiveMetadata, SkillFailure,
    SkillFilesystemTransaction, SkillImportRequest, SkillImportedSource, SkillListResult,
    SkillLoadOutcome, SkillLoadRequest, SkillLoadResult, SkillLogAction, SkillLogEvent,
    SkillLogLevel, SkillMountMigrationReport, SkillMountRepair, SkillOverview,
    SkillPackageDescriptor, SkillPackagePreview, SkillPackageResource, SkillPreview,
    SkillPromptForAgent, SkillRecord, SkillResourceDocument, SkillResourceEntry,
    SkillResourceIndex, SkillResourceReadOutcome, SkillResourceReadRequest,
    SkillResourceReadResult, SkillScopeQuery, SkillShadowSummary, SkillSourceRefresh, SkillStats,
    SkillSyncResult, SkillUpdateRequest, SkillUsageActivity, SkillUsageIdentity,
    SkillUsageMutation, SkillUsageRead, SkillUsageSummary, UtilitySkillExecutionSnapshot,
    UtilitySkillResolutionOutcome, BUILTIN_RECONCILIATION_VERSION,
};
#[allow(unused_imports)]
pub(crate) use overlay_logging::{
    OverlayValidationDiagnostic, OverlayValidationReason, OverlayValidationTarget,
};
#[allow(unused_imports)]
pub(crate) use overlay_models::{
    OverlayActor, OverlayBoundedText, OverlayConflictResolution, OverlayConflictSummary,
    OverlayDetail, OverlayDiff, OverlayDiffHunk, OverlayHistoryAction, OverlayHistoryEntry,
    OverlayHistoryPage, OverlayHistoryQuery, OverlayImportRequest, OverlayImportReview,
    OverlayIntegrityCode, OverlayLimitKind, OverlayMutation, OverlayMutationKind,
    OverlayMutationOutcome, OverlayMutationRequest, OverlayMutationSummary, OverlayPageIntegrity,
    OverlayPreview, OverlayPromotionRequest, OverlayReconciliationBaseSnapshot,
    OverlayReconciliationChoice, OverlayReconciliationConflictChoice, OverlayReconciliationPreview,
    OverlayReconciliationPreviewInput, OverlayReconciliationProposedResult,
    OverlayReconciliationRequest, OverlayResourceShadow, OverlayResourceSummary, OverlayScanResult,
    OverlayScopeDiff, OverlayScopeStatus, OverlayScopeSummary, OverlayStatus, OverlaySummary,
    OverlayWitnesses,
};
#[allow(unused_imports)]
pub(crate) use overlay_ports::{
    OverlayContentScannerPort, OverlayEffectivePackageSnapshot, OverlayEffectiveSnapshotPort,
    OverlayHistoryRepository, OverlayImportParserPort, OverlayKey, OverlayManifestRepository,
    OverlayManifestSnapshot, OverlayPayloadRepository, OverlayPayloadWrite, OverlayPinSnapshot,
    OverlayPinStatePort, OverlayPreparedImport, OverlayRuntimeCacheInvalidationPort,
    OverlayTransactionExecutor, OverlayTransactionOutcome, OverlayTransactionPlan,
    OverlayUsageDelta, OverlayUsageSnapshot, OverlayUsageStatePort, OverlayValidatedFile,
};
#[allow(unused_imports)]
pub(crate) use overlay_preparation::{
    build_bounded_diff, build_overlay_diff, prepare_overlay_mutation, replay_conflicts,
    OverlayPreparationInput, OverlayPreparationSnapshots, PreparedOverlayMutation,
};
#[allow(unused_imports)]
pub(crate) use overlay_promotion::{
    rescan_overlay_for_promotion, validate_overlay_promotion, OverlayPromotionValidationInput,
};
#[allow(unused_imports)]
pub(crate) use overlay_reconciliation::{
    prepare_overlay_reconciliation, OverlayReconciliationInput, PreparedOverlayReconciliation,
};
#[allow(unused_imports)]
pub(crate) use overlay_runtime::{
    OverlayAppliedSkillSnapshot, OverlayAppliedSkillSnapshotPort,
    OverlayAppliedSkillSnapshotResolver,
};
#[allow(unused_imports)]
pub(crate) use overlay_service::SkillOverlayApplicationService;
pub(crate) use ports::{
    EffectiveSkillCatalogPort, SkillApiBindingRepository, SkillClockPort, SkillFilesystemPort,
    SkillLayerProvider, SkillLegacySourcePort, SkillLoggingPort, SkillPackageMaterializer,
    SkillPackageReader, SkillReconciliationRepository, SkillRepository, SkillSourceProbe,
    SkillUsageRepository, SkillWorkspaceSelectionPort,
};
#[cfg(test)]
pub(crate) use progressive::MAX_RESOURCE_PATH_CHARACTERS;
pub(crate) use progressive::{
    build_resource_index, logical_base_uri, parse_logical_uri, truncate_chars,
    validate_relative_resource_path, MAX_DISCOVERY_QUERY_CHARACTERS, MAX_DISCOVERY_RESULTS,
    MAX_INLINE_SKILL_CHARACTERS, MAX_RESOURCE_BYTES, MAX_RESOURCE_CHARACTERS,
};
pub(crate) use service::SkillApplicationService;
