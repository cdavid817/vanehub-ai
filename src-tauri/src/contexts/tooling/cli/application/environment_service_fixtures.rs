//! Shared harness for the CLI application tests.
//!
//! Assembles the service from in-memory doubles. Nothing here touches SQLite, the filesystem, the
//! network, Tauri, or a live process, so every assertion is about policy rather than about the
//! machine the suite happens to run on.

use std::sync::Arc;

use super::environment_error::CliEnvironmentError;
use super::environment_service::{CliEnvironmentPorts, CliEnvironmentService};
use super::environment_test_doubles::{
    source_id, timestamp, tool_id, FakeClock, FakeCoordinator, FakeDiagnostics, FakeDiscovery,
    FakeIds, FakeOperations, FakeProbes, FakeRepository, FakeSource, FakeSourceRegistry,
};
use crate::contexts::tooling::cli::domain::catalog::{
    CliCatalogStatus, CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::ids::CliInstallationId;
use crate::contexts::tooling::cli::domain::installation::{CliEnvironmentOrigin, CliInstallation};
use crate::contexts::tooling::cli::domain::source::{CliSourceConfidence, CliSourceKind};
use crate::contexts::tooling::cli::domain::status::CliExecutableStatus;
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

pub(super) struct Harness {
    pub(super) service: CliEnvironmentService,
    pub(super) discovery: Arc<FakeDiscovery>,
    pub(super) probes: Arc<FakeProbes>,
    pub(super) registry: Arc<FakeSourceRegistry>,
    pub(super) repository: Arc<FakeRepository>,
    pub(super) operations: Arc<FakeOperations>,
    pub(super) coordinator: Arc<FakeCoordinator>,
    pub(super) diagnostics: Arc<FakeDiagnostics>,
    pub(super) clock: Arc<FakeClock>,
}

impl Harness {
    pub(super) fn new() -> Self {
        let discovery = FakeDiscovery::new("fingerprint-a");
        let probes = FakeProbes::new();
        let registry = FakeSourceRegistry::new();
        let repository = FakeRepository::new();
        let operations = FakeOperations::new();
        let coordinator = FakeCoordinator::new();
        let diagnostics = FakeDiagnostics::new();
        let clock = FakeClock::at(1_000);
        let ids = FakeIds::new();

        let service = CliEnvironmentService::new(CliEnvironmentPorts {
            discovery: Arc::clone(&discovery) as Arc<_>,
            probes: Arc::clone(&probes) as Arc<_>,
            sources: Arc::clone(&registry) as Arc<_>,
            repository: Arc::clone(&repository) as Arc<_>,
            operations: Arc::clone(&operations) as Arc<_>,
            coordinator: Arc::clone(&coordinator) as Arc<_>,
            diagnostics: Arc::clone(&diagnostics) as Arc<_>,
            clock: Arc::clone(&clock) as Arc<_>,
            ids: ids as Arc<_>,
        });

        Self {
            service,
            discovery,
            probes,
            registry,
            repository,
            operations,
            coordinator,
            diagnostics,
            clock,
        }
    }

    /// Registers an npm adapter that answers with `catalog`.
    pub(super) fn register_npm_source(&self, catalog: CliVersionCatalog) -> Arc<FakeSource> {
        let source = FakeSource::new("npm");
        source.set_catalog(catalog);
        self.registry.register(Arc::clone(&source) as Arc<_>);
        source
    }

    /// Registers an npm adapter whose catalog query fails, as an offline registry would.
    pub(super) fn register_failing_npm_source(&self) -> Arc<FakeSource> {
        let source = FakeSource::new("npm");
        *source.catalog_error.lock().expect("catalog error") =
            Some(CliEnvironmentError::CatalogUnavailable {
                source_id: "npm".to_string(),
                reason: CliCatalogUnavailableReason::QueryFailed,
            });
        self.registry.register(Arc::clone(&source) as Arc<_>);
        source
    }

    pub(super) fn register_source(&self, source: Arc<FakeSource>) -> Arc<FakeSource> {
        self.registry.register(Arc::clone(&source) as Arc<_>);
        source
    }
}

/// A runnable npm-owned installation on PATH.
pub(super) fn healthy_npm_installation(id: &str, path: &str) -> CliInstallation {
    CliInstallation {
        id: CliInstallationId::new(id).expect("installation id"),
        executable_path: path.to_string(),
        canonical_path: None,
        reported_version: None,
        source_id: Some(source_id("npm")),
        source_kind: CliSourceKind::Npm,
        // A path heuristic is inferred, never verified -- and inferred is enough to manage.
        source_confidence: CliSourceConfidence::Inferred,
        path_priority: Some(0),
        environment_origin: CliEnvironmentOrigin::Path,
        executable_status: CliExecutableStatus::Unknown,
    }
}

/// An available npm catalog valid for fifteen minutes from the harness clock's start.
pub(super) fn npm_catalog(agent_id: &str, versions: &[&str], latest: &str) -> CliVersionCatalog {
    CliVersionCatalog {
        agent_id: tool_id(agent_id),
        source_id: source_id("npm"),
        channel: Some("stable".to_string()),
        versions: versions
            .iter()
            .map(|version| NormalizedCliVersion::parse(*version))
            .collect(),
        latest: Some(NormalizedCliVersion::parse(latest)),
        fetched_at: timestamp(1_000),
        expires_at: timestamp(1_900),
        status: CliCatalogStatus::Available,
    }
}
