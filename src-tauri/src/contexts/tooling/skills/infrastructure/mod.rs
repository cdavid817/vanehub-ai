mod configuration_logging;
mod configuration_repository;
mod configuration_resolution;
mod configuration_schema;
mod configuration_secrets;
mod configuration_service;
mod effective_cache;
mod effective_catalog;
mod filesystem;
mod overlay_runtime;
mod overlay_scanner;
mod package_reader;
mod runtime_support;
mod sqlite_repository;
mod system_cache;
mod system_packages;

#[cfg(test)]
mod effective_cache_tests;
#[cfg(test)]
mod recovery_tests;

#[allow(unused_imports)]
pub(crate) use configuration_logging::{
    configuration_event, record_cleanup, record_drift, record_lifecycle, record_save,
    record_secret_mutation, record_validation_failure,
};
#[allow(unused_imports)]
pub(crate) use configuration_repository::{
    SkillConfigCleanupState, SkillConfigurationSave, SkillConfigurationWrite,
    SqliteSkillConfigurationRepository, StoredSkillConfiguration,
};
#[allow(unused_imports)]
pub(crate) use configuration_resolution::{
    require_canonical_workspace, resolve_from_records, resolve_stored_configuration,
    ResolvedSkillConfiguration, ScopeConfigurationState,
};
pub(crate) use configuration_schema::apply_skill_configuration_schema;
#[allow(unused_imports)]
pub(crate) use configuration_secrets::{
    secret_alias, SecretRecovery, SecretSlotState, SkillConfigurationSecrets, SkillSecretFailure,
    SkillSecretStore, StagedSecrets,
};
#[allow(unused_imports)]
pub(crate) use configuration_service::{
    apply_deletion_retention, preview, reconcile, require_base_revision, reset_property,
    reset_scope, save, validate_request, DeletionRetention, ObsoleteFieldChoice,
    ReconciliationPlan, SkillConfigurationError, SkillConfigurationRequest,
    SkillConfigurationSaveResult, MAX_CONFIGURATION_PAYLOAD_BYTES,
};
pub(crate) use effective_cache::{
    EffectiveSkillDerivedCache, EffectiveSkillRuntimeCacheInvalidator,
};
pub(crate) use effective_catalog::CachedEffectiveSkillCatalog;
pub(crate) use filesystem::{
    EmptyRegistrySkillProvider, FilesystemOverlayHistoryRepository, FilesystemOverlayImportParser,
    FilesystemOverlayManifestRepository, FilesystemOverlayTransactionExecutor,
    FilesystemSkillLayerProvider, FilesystemSkillUsageRepository, ManagedSkillFilesystem,
    OverlayPayloadStore,
};
pub(crate) use overlay_runtime::CatalogOverlayEffectiveSnapshot;
pub(crate) use overlay_scanner::DeterministicOverlayContentScanner;
pub(crate) use package_reader::LayeredSkillPackageReader;
pub(crate) use runtime_support::{
    CurrentWorkspaceSelection, SystemSkillClock, UnifiedSkillLoggingAdapter,
};
pub(crate) use sqlite_repository::{
    apply_effective_runtime_schema, apply_reliability_schema, apply_schema, SqliteSkillRepository,
};
pub(crate) use system_cache::SystemSkillDerivedCache;
pub(crate) use system_packages::SystemSkillPackages;
