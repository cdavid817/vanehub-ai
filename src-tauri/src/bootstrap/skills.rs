use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::skills::api::SkillApi;
use crate::contexts::tooling::skills::application::{
    OverlayAppliedSkillSnapshotResolver, SkillApplicationService, SkillOverlayApplicationService,
};
use crate::contexts::tooling::skills::infrastructure::{
    CachedEffectiveSkillCatalog, CatalogOverlayEffectiveSnapshot, CurrentWorkspaceSelection,
    DeterministicOverlayContentScanner, EffectiveSkillDerivedCache,
    EffectiveSkillRuntimeCacheInvalidator, EmptyRegistrySkillProvider,
    FilesystemOverlayHistoryRepository, FilesystemOverlayImportParser,
    FilesystemOverlayManifestRepository, FilesystemOverlayTransactionExecutor,
    FilesystemSkillLayerProvider, FilesystemSkillUsageRepository, LayeredSkillPackageReader,
    ManagedSkillFilesystem, OverlayPayloadStore, SqliteSkillRepository, SystemSkillClock,
    SystemSkillDerivedCache, SystemSkillPackages, UnifiedSkillLoggingAdapter,
};
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;
use std::sync::Arc;

pub(crate) fn assemble_skill_api(database: NativeDatabase, fallback_log_dir: PathBuf) -> SkillApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    let skill_logging = Arc::new(UnifiedSkillLoggingAdapter::new(logging));
    let skill_clock = Arc::new(SystemSkillClock);
    let repository = Arc::new(SqliteSkillRepository::new(database));
    let system_packages = Arc::new(SystemSkillPackages);
    let effective_catalog = Arc::new(CachedEffectiveSkillCatalog::new(vec![
        Arc::new(FilesystemSkillLayerProvider::project()),
        Arc::new(FilesystemSkillLayerProvider::user()),
        Arc::new(EmptyRegistrySkillProvider),
        system_packages.clone(),
    ]));
    let system_materializer = Arc::new(SystemSkillDerivedCache::new(system_packages.clone()));
    let package_reader = Arc::new(LayeredSkillPackageReader::new(system_packages.clone()));
    let overlay_manifests = Arc::new(FilesystemOverlayManifestRepository::new());
    let overlay_payloads = Arc::new(OverlayPayloadStore::new());
    let overlay_snapshots = Arc::new(
        OverlayAppliedSkillSnapshotResolver::new(
            Arc::new(CatalogOverlayEffectiveSnapshot::new(
                effective_catalog.clone(),
                package_reader.clone(),
            )),
            overlay_manifests.clone(),
            overlay_payloads.clone(),
        )
        .with_runtime_diagnostics(skill_logging.clone(), skill_clock.clone()),
    );
    let effective_materializer =
        Arc::new(EffectiveSkillDerivedCache::new(overlay_snapshots.clone()));
    let usage_repository = Arc::new(FilesystemSkillUsageRepository::new());
    let overlay_history = Arc::new(FilesystemOverlayHistoryRepository::new());
    let overlay_transactions = Arc::new(FilesystemOverlayTransactionExecutor::new());
    let overlay_runtime_cache = Arc::new(EffectiveSkillRuntimeCacheInvalidator::new(
        effective_catalog.clone(),
        effective_materializer.clone(),
    ));
    let overlay_service = SkillOverlayApplicationService::new(
        overlay_manifests,
        Arc::new(CatalogOverlayEffectiveSnapshot::new(
            effective_catalog.clone(),
            package_reader.clone(),
        )),
        usage_repository.clone(),
        skill_clock.clone(),
    )
    .with_mutation_ports(
        overlay_history,
        usage_repository.clone(),
        overlay_transactions,
    )
    .with_promotion_ports(
        overlay_payloads,
        Arc::new(DeterministicOverlayContentScanner),
    )
    .with_import_parser(Arc::new(FilesystemOverlayImportParser::new()))
    .with_runtime_cache(overlay_runtime_cache);
    let filesystem = Arc::new(ManagedSkillFilesystem::new());
    SkillApi::new(
        SkillApplicationService::new(
            repository.clone(),
            repository.clone(),
            filesystem.clone(),
            Arc::new(CurrentWorkspaceSelection),
            skill_clock,
            skill_logging,
        )
        .with_effective_catalog(effective_catalog)
        .with_system_materializer(system_materializer)
        .with_effective_materializer(effective_materializer)
        .with_builtin_reconciliation(system_packages, repository, filesystem)
        .with_effective_package_reader(package_reader)
        .with_overlay_applied_snapshots(overlay_snapshots)
        .with_usage_repository(usage_repository),
    )
    .with_overlay_service(overlay_service)
}
