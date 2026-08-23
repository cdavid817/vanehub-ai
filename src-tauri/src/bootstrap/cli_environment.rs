//! Assembles the source-aware CLI environment service.
//!
//! The only place concrete adapters are named. Which sources exist, which database backs the
//! repository, and which log receives diagnostics are all decided here; the application layer sees
//! ports.

use std::sync::Arc;

use crate::contexts::operations::api::OperationsApi;
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::tooling::cli::api::CliEnvironmentApi;
use crate::contexts::tooling::cli::application::environment_service::{
    CliEnvironmentPorts, CliEnvironmentService,
};
use crate::contexts::tooling::cli::infrastructure::environment_discovery::SystemCliDiscovery;
use crate::contexts::tooling::cli::infrastructure::environment_gateway::SystemCommandGateway;
use crate::contexts::tooling::cli::infrastructure::environment_probe::SystemCliProbe;
use crate::contexts::tooling::cli::infrastructure::environment_repository::SqliteCliEnvironmentRepository;
use crate::contexts::tooling::cli::infrastructure::environment_runtime_adapters::{
    CliEnvironmentDiagnosticsAdapter, CliEnvironmentMutationCoordinator,
    CliEnvironmentOperationsAdapter, CliSourceAdapterRegistry, SystemEnvironmentClock,
    UuidCliIdFactory,
};
use crate::contexts::tooling::cli::infrastructure::npm_source::NpmSource;
use crate::contexts::tooling::cli::infrastructure::vendor_downloader::HttpsInstallerDownloader;
use crate::contexts::tooling::cli::infrastructure::vendor_source::VendorSource;
use crate::contexts::tooling::cli::infrastructure::winget_source::WingetSource;
use crate::platform::database::NativeDatabase;
use std::path::PathBuf;

pub(crate) fn assemble_cli_environment_api(
    database: NativeDatabase,
    operations: OperationsApi,
    fallback_log_dir: PathBuf,
) -> CliEnvironmentApi {
    let logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_dir));
    let gateway = Arc::new(SystemCommandGateway);

    // Registered under the id each adapter reports for itself. A tool whose plan names a source
    // that is not here resolves to `source-unavailable` -- a typed refusal, never a fallback onto
    // whichever source happens to be present.
    let sources = CliSourceAdapterRegistry::default()
        .with(Arc::new(NpmSource::new(gateway.clone())))
        .with(Arc::new(WingetSource::new(gateway.clone())))
        .with(Arc::new(VendorSource::new(
            gateway,
            Arc::new(HttpsInstallerDownloader),
        )));

    CliEnvironmentApi::new(CliEnvironmentService::new(CliEnvironmentPorts {
        discovery: Arc::new(SystemCliDiscovery),
        probes: Arc::new(SystemCliProbe),
        sources: Arc::new(sources),
        repository: Arc::new(SqliteCliEnvironmentRepository::new(database)),
        operations: Arc::new(CliEnvironmentOperationsAdapter::new(operations)),
        coordinator: Arc::new(CliEnvironmentMutationCoordinator::default()),
        diagnostics: Arc::new(CliEnvironmentDiagnosticsAdapter::new(logging)),
        clock: Arc::new(SystemEnvironmentClock),
        ids: Arc::new(UuidCliIdFactory::default()),
    }))
}
