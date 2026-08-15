mod configuration_schema;
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

pub(crate) use configuration_schema::apply_skill_configuration_schema;
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
