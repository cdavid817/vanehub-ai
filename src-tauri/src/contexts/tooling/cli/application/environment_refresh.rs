//! Discovering what is installed and what each owning source says about it.
//!
//! Split so the command boundary never waits: `prepare_refresh` creates the operation and returns
//! its id, and `execute_refresh` does the probing on a background thread.
//!
//! The rule a targeted refresh must not break: refreshing one tool touches one tool. Nothing else
//! is re-derived, re-saved, or invalidated, so a single card refreshing cannot blank the page.

use super::environment_error::CliEnvironmentError;
use super::environment_ports::{CliCancellation, CliProbeBudget, CliProbeOutcome};
use super::environment_service::CliEnvironmentService;
use crate::contexts::tooling::cli::domain::action::{
    derive_allowed_actions, CliActionContext, CliAllowedAction,
};
use crate::contexts::tooling::cli::domain::catalog::{
    update_status_for_source, CliCatalogUnavailableReason, CliVersionCatalog,
};
use crate::contexts::tooling::cli::domain::definition::CliToolDefinition;
use crate::contexts::tooling::cli::domain::ids::CliToolId;
use crate::contexts::tooling::cli::domain::installation::{
    conflicts_block_mutation, deduplicate, derive_conflicts, group_launcher_families,
    select_active, ActiveSelection, CliInstallation,
};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::probe_interpretation::{
    interpret_authentication, interpret_doctor, CliDoctorVerdict, CliProbeReading,
};
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::contexts::tooling::cli::domain::source::{
    CliPlatform, CliSourceConfidence, CliSourceSummary,
};
use crate::contexts::tooling::cli::domain::status::{
    CliAuthenticationStatus, CliCompatibilityStatus, CliDiscoveryStatus, CliExecutableStatus,
    CliFreshness, CliUpdateStatus,
};
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

/// A refresh whose operation exists but whose work has not started.
#[derive(Debug, Clone)]
pub(crate) struct PreparedEnvironmentRefresh {
    pub(crate) operation_id: String,
    agent_ids: Vec<String>,
    force_catalog: bool,
}

impl CliEnvironmentService {
    /// Creates the operation and returns immediately.
    ///
    /// `agent_ids` empty means every registered tool. Unknown ids are rejected here rather than
    /// silently skipped later, so a typo surfaces at the call site.
    pub(crate) fn prepare_refresh(
        &self,
        agent_ids: Vec<String>,
        force_catalog: bool,
    ) -> Result<PreparedEnvironmentRefresh, CliEnvironmentError> {
        let targets = self.resolve_refresh_targets(&agent_ids)?;
        let related = (targets.len() == 1).then(|| targets[0].0.clone());
        let message = match related.as_ref() {
            Some(agent) => format!("Refreshing {}", agent.as_str()),
            None => "Refreshing CLI environments".to_string(),
        };
        let operation_id = self.ports.operations.start(related.as_ref(), message)?;
        Ok(PreparedEnvironmentRefresh {
            operation_id,
            agent_ids: targets
                .into_iter()
                .map(|(id, _)| id.as_str().to_string())
                .collect(),
            force_catalog,
        })
    }

    /// Runs the refresh. Intended for a background thread.
    pub(crate) fn execute_refresh(
        &self,
        prepared: PreparedEnvironmentRefresh,
    ) -> Result<(), CliEnvironmentError> {
        let operation_id = prepared.operation_id.clone();
        match self.refresh_all(&prepared) {
            Ok(refreshed) => self
                .ports
                .operations
                .complete(&operation_id, serde_json::json!({ "agentIds": refreshed })),
            Err(error) => {
                let message = error.to_string();
                self.ports
                    .diagnostics
                    .record(&operation_id, None, None, &message);
                self.ports.operations.fail(&operation_id, message)
            }
        }
    }

    fn refresh_all(
        &self,
        prepared: &PreparedEnvironmentRefresh,
    ) -> Result<Vec<String>, CliEnvironmentError> {
        let operation_id = &prepared.operation_id;
        let cancellation = self.ports.operations.cancellation(operation_id)?;
        let fingerprint = self.ports.discovery.environment_fingerprint()?;
        let total = u32::try_from(prepared.agent_ids.len()).unwrap_or(u32::MAX);

        let mut refreshed = Vec::new();
        for (index, agent_id) in prepared.agent_ids.iter().enumerate() {
            if cancellation.is_cancelled() {
                break;
            }
            let (tool_id, definition) = self.resolve_tool(agent_id)?;
            self.ports
                .operations
                .report_phase(operation_id, CliOperationPhase::Preflight, true)?;

            // A mutation may be mid-write on this tool's source. Probing then can read a
            // half-installed tree and persist it as the machine's state, so the tool is left with
            // the snapshot it already has, labelled stale.
            if let Some(busy_key) = self.blocking_mutation_key(definition) {
                self.ports.diagnostics.record(
                    operation_id,
                    Some(&tool_id),
                    None,
                    &format!("skipped detection while {busy_key} is being written"),
                );
                let mut existing = self.snapshot_or_never_scanned(&tool_id, &fingerprint)?;
                existing.mark_stale();
                self.ports.repository.save_snapshot_atomic(&existing)?;
                continue;
            }

            let snapshot = self.refresh_one(
                &tool_id,
                definition,
                &fingerprint,
                prepared.force_catalog,
                operation_id,
                &cancellation,
            )?;
            // Only the targeted tool's snapshot is written. Unrelated snapshots keep whatever they
            // held, including their own freshness.
            self.ports.repository.save_snapshot_atomic(&snapshot)?;
            refreshed.push(tool_id.as_str().to_string());

            let completed = u32::try_from(index + 1).unwrap_or(u32::MAX);
            self.ports
                .operations
                .report_units(operation_id, completed, total)?;
        }

        self.ports
            .operations
            .report_phase(operation_id, CliOperationPhase::Completed, false)?;
        Ok(refreshed)
    }

    /// The mutation key currently blocking detection for this tool, if any.
    ///
    /// Returns `None` when nothing is held or when every affected source declares reads safe
    /// during its own writes.
    fn blocking_mutation_key(&self, definition: &'static CliToolDefinition) -> Option<String> {
        definition
            .actionable_distributions()
            .find_map(|distribution| {
                let source_id = distribution.source_id().ok()?;
                let adapter = self.ports.sources.adapter(&source_id)?;
                let tool_id = definition.tool_id().ok()?;
                let key = adapter.mutation_key(&tool_id);
                (!self.ports.coordinator.may_detect_now(&key)).then(|| key.as_str().to_string())
            })
    }

    /// Also used by post-mutation verification, which needs exactly this: a fresh look at one tool.
    pub(super) fn refresh_one(
        &self,
        tool_id: &CliToolId,
        definition: &'static CliToolDefinition,
        fingerprint: &str,
        force_catalog: bool,
        operation_id: &str,
        cancellation: &CliCancellation,
    ) -> Result<CliEnvironmentSnapshot, CliEnvironmentError> {
        let installations =
            self.discover_and_probe(tool_id, definition, operation_id, cancellation)?;

        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::ResolvingSource,
            true,
        )?;
        let selection = select_active(&installations);
        // Update state, catalogs, and actions all describe the installation VaneHub would act on,
        // which is the recommended one -- not necessarily the one PATH reaches first.
        let active_installation = selection.recommended.map(|index| &installations[index]);
        // Owned rather than borrowed: `installations` moves into the snapshot below, and the
        // compatibility check still needs the version afterwards.
        let active_version = active_installation.and_then(|i| i.reported_version.clone());
        let active_source = active_installation.and_then(|i| i.source_id.clone());

        // Readiness probes run against the installation VaneHub would act on, before the catalog
        // work, so an unauthenticated tool is known to be unauthenticated even if the source is
        // unreachable.
        let readiness_probes =
            self.run_readiness_probes(definition, active_installation, operation_id, cancellation)?;

        self.ports.operations.report_phase(
            operation_id,
            CliOperationPhase::QueryingCatalog,
            true,
        )?;
        let catalogs = self.resolve_catalogs(
            tool_id,
            definition,
            force_catalog,
            operation_id,
            cancellation,
        )?;

        // The update state comes from the catalog of the source that owns the active install --
        // never from whichever source happens to have one.
        let owning = active_source
            .as_ref()
            .and_then(|id| catalogs.iter().find(|catalog| &catalog.source_id == id));
        let update = match (&active_source, active_installation) {
            // Nothing installed: whether an update exists is not a question that applies.
            (_, None) => CliUpdateStatus::Unknown,
            (Some(source_id), Some(_)) => update_status_for_source(
                owning,
                tool_id,
                source_id,
                owning.and_then(|catalog| catalog.channel.as_deref()),
                active_version.as_ref(),
            ),
            // Installed, but nothing establishes which source owns it, so no catalog is
            // authoritative for it. Borrowing any of them is exactly the defect being removed.
            (None, Some(_)) => CliUpdateStatus::CatalogUnavailable,
        };

        // Conflicts are derived before actions, because a conflict that makes the mutation target
        // ambiguous withholds every mutating action regardless of what the source can do.
        let conflicts = derive_conflicts(&installations, selection);
        let blocks_mutation = conflicts_block_mutation(&conflicts);
        let allowed_actions = self.derive_actions(
            definition,
            &installations,
            selection,
            &catalogs,
            blocks_mutation,
        );
        let sources = self.summarize_sources(definition, &catalogs);

        let mut snapshot = self.snapshot_or_never_scanned(tool_id, fingerprint)?;
        snapshot.environment_fingerprint = fingerprint.to_string();
        snapshot.conflicts = conflicts;
        // A scan just ran, so an empty list now means not-found rather than never-looked.
        snapshot.discovery = CliDiscoveryStatus::from_count(installations.len());
        snapshot.installations = installations;
        snapshot.update = update;
        snapshot.sources = sources;
        snapshot.allowed_actions = allowed_actions;
        snapshot.compatibility = compatibility_for(definition, active_version.as_ref());
        snapshot.authentication = readiness_probes.authentication;
        snapshot.freshness = CliFreshness::Fresh;
        snapshot.checked_at = Some(self.ports.clock.now());
        snapshot.last_operation_id = Some(operation_id.to_string());
        // Readiness is derived in the backend from the executable, authentication, compatibility,
        // and Doctor results together -- never assembled by the frontend from separate fields.
        Ok(snapshot.recompute_derived(
            readiness_probes.missing_dependency,
            readiness_probes.doctor.reports_problem(),
        ))
    }

    fn discover_and_probe(
        &self,
        tool_id: &CliToolId,
        definition: &'static CliToolDefinition,
        operation_id: &str,
        cancellation: &CliCancellation,
    ) -> Result<Vec<CliInstallation>, CliEnvironmentError> {
        let discovered = self.ports.discovery.discover(
            tool_id,
            definition.executable_names,
            CliProbeBudget::default(),
            cancellation,
        )?;
        // Deduplicate identical binaries first, then fold platform launcher aliases. Without the
        // second step one npm global install on Windows reports as three competing installations.
        let mut installations = group_launcher_families(deduplicate(discovered));

        for installation in &mut installations {
            if cancellation.is_cancelled() {
                break;
            }
            let outcome = self.ports.probes.run_probe(
                &installation.executable_path,
                definition.probes.version,
                cancellation,
            )?;
            installation.executable_status = if outcome.succeeded() {
                CliExecutableStatus::Healthy
            } else if outcome.timed_out {
                CliExecutableStatus::TimedOut
            } else {
                CliExecutableStatus::Broken
            };
            installation.reported_version = outcome
                .succeeded()
                .then(|| NormalizedCliVersion::from_probe_output(&outcome.stdout))
                .flatten();
            if !outcome.succeeded() {
                self.ports.diagnostics.record(
                    operation_id,
                    Some(tool_id),
                    None,
                    "version probe did not succeed",
                );
            }
        }
        Ok(installations)
    }

    /// Runs the read-only Doctor and authentication probes a tool declares, and normalizes them.
    ///
    /// Nothing here concludes anything from silence: a tool with no declared probe, a timeout, or a
    /// cancellation all leave the state `Unknown`. The probe adapter has already bounded and
    /// redacted the output, and the interpretation returns an enum, so no fragment of what a
    /// provider printed reaches the snapshot.
    fn run_readiness_probes(
        &self,
        definition: &'static CliToolDefinition,
        active: Option<&CliInstallation>,
        operation_id: &str,
        cancellation: &CliCancellation,
    ) -> Result<ReadinessProbes, CliEnvironmentError> {
        let Some(installation) = active.filter(|installation| installation.is_runnable()) else {
            // Nothing runnable to ask. A missing dependency is equally unknowable here.
            return Ok(ReadinessProbes::unknown());
        };

        let mut probes = ReadinessProbes::unknown();
        if let Some(command) = definition.probes.authentication.command() {
            self.ports.operations.report_phase(
                operation_id,
                CliOperationPhase::RunningDoctor,
                true,
            )?;
            let outcome = self.ports.probes.run_probe(
                &installation.executable_path,
                command,
                cancellation,
            )?;
            probes.authentication = interpret_authentication(
                definition.probes.authentication_parser,
                reading_of(&outcome),
            );
        }
        if let Some(command) = definition.probes.doctor.command() {
            self.ports.operations.report_phase(
                operation_id,
                CliOperationPhase::RunningDoctor,
                true,
            )?;
            let outcome = self.ports.probes.run_probe(
                &installation.executable_path,
                command,
                cancellation,
            )?;
            probes.doctor = interpret_doctor(definition.probes.doctor_parser, reading_of(&outcome));
        }
        Ok(probes)
    }

    /// One catalog per actionable distribution, cached until it expires.
    fn resolve_catalogs(
        &self,
        tool_id: &CliToolId,
        definition: &'static CliToolDefinition,
        force: bool,
        operation_id: &str,
        cancellation: &CliCancellation,
    ) -> Result<Vec<CliVersionCatalog>, CliEnvironmentError> {
        let now = self.ports.clock.now();
        let mut catalogs = Vec::new();
        for distribution in definition.actionable_distributions() {
            let Ok(source_id) = distribution.source_id() else {
                continue;
            };
            let channel = distribution.default_channel().map(|channel| channel.id);
            let cached = self
                .ports
                .repository
                .load_catalog(tool_id, &source_id, channel)?;
            if !force {
                if let Some(cached) = cached.filter(|catalog| !catalog.is_expired(now)) {
                    catalogs.push(cached);
                    continue;
                }
            }

            let Some(adapter) = self.ports.sources.adapter(&source_id) else {
                continue;
            };
            let catalog = match adapter.list_versions(tool_id, distribution, channel, cancellation)
            {
                Ok(catalog) => catalog,
                Err(error) => {
                    self.ports.diagnostics.record(
                        operation_id,
                        Some(tool_id),
                        None,
                        &error.to_string(),
                    );
                    // A source that cannot answer contributes an explicit unavailable catalog, so
                    // the snapshot records "we asked and could not tell" rather than nothing.
                    CliVersionCatalog::unavailable(
                        tool_id.clone(),
                        source_id.clone(),
                        channel.map(str::to_string),
                        CliCatalogUnavailableReason::QueryFailed,
                        now,
                        now,
                    )
                }
            };
            self.ports.repository.save_catalog(&catalog)?;
            catalogs.push(catalog);
        }
        Ok(catalogs)
    }

    fn derive_actions(
        &self,
        definition: &'static CliToolDefinition,
        installations: &[CliInstallation],
        selection: ActiveSelection,
        catalogs: &[CliVersionCatalog],
        conflict_blocks_mutation: bool,
    ) -> Vec<CliAllowedAction> {
        let Some(platform) = CliPlatform::current() else {
            return Vec::new();
        };
        let active_installation = selection.recommended.map(|index| &installations[index]);
        let active_version = active_installation.and_then(|i| i.reported_version.as_ref());

        definition
            .actionable_distributions()
            .filter_map(|distribution| {
                let source_id = distribution.source_id().ok()?;
                let catalog = catalogs
                    .iter()
                    .find(|catalog| catalog.source_id == source_id);
                Some(derive_allowed_actions(CliActionContext {
                    distribution,
                    platform,
                    is_installed: !installations.is_empty(),
                    active_version,
                    active_source_matches: active_installation
                        .is_some_and(|i| i.source_id.as_ref() == Some(&source_id)),
                    active_source_confidence: active_installation
                        .map(|i| i.source_confidence)
                        .unwrap_or(CliSourceConfidence::Unknown),
                    active_executable_healthy: active_installation
                        .is_some_and(CliInstallation::is_runnable),
                    catalog_latest: catalog.and_then(|catalog| catalog.latest.as_ref()),
                    catalog_available: catalog.is_some_and(CliVersionCatalog::is_available),
                    // Dynamic capabilities are confirmed during planning, not during a refresh.
                    repair_preflight_passed: false,
                    conflict_blocks_mutation,
                }))
            })
            .flatten()
            .collect()
    }

    fn summarize_sources(
        &self,
        definition: &'static CliToolDefinition,
        catalogs: &[CliVersionCatalog],
    ) -> Vec<CliSourceSummary> {
        definition
            .distributions
            .iter()
            .filter_map(|distribution| {
                let source_id = distribution.source_id().ok()?;
                let catalog = catalogs
                    .iter()
                    .find(|catalog| catalog.source_id == source_id);
                Some(CliSourceSummary {
                    source_id,
                    kind: distribution.kind,
                    capabilities: distribution.capabilities,
                    supported_on_this_platform: distribution.is_actionable_here(),
                    available_version_count: catalog
                        .filter(|catalog| catalog.is_available())
                        .map(|catalog| catalog.versions.len()),
                })
            })
            .collect()
    }

    fn resolve_refresh_targets(
        &self,
        agent_ids: &[String],
    ) -> Result<Vec<(CliToolId, &'static CliToolDefinition)>, CliEnvironmentError> {
        if agent_ids.is_empty() {
            return crate::contexts::tooling::cli::domain::registry::CLI_TOOL_DEFINITIONS
                .iter()
                .map(|definition| {
                    definition
                        .tool_id()
                        .map(|id| (id, definition))
                        .map_err(|error| CliEnvironmentError::Validation(error.to_string()))
                })
                .collect();
        }
        agent_ids
            .iter()
            .map(|agent_id| self.resolve_tool(agent_id))
            .collect()
    }
}

/// Normalized results of the read-only readiness probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReadinessProbes {
    authentication: CliAuthenticationStatus,
    doctor: CliDoctorVerdict,
    missing_dependency: bool,
}

impl ReadinessProbes {
    /// Nothing was asked, so nothing is known. Never a claim of readiness.
    fn unknown() -> Self {
        Self {
            authentication: CliAuthenticationStatus::Unknown,
            doctor: CliDoctorVerdict::Unknown,
            // No dependency probe exists yet; asserting one is missing would be a finding VaneHub
            // has not made.
            missing_dependency: false,
        }
    }
}

/// Narrows a probe outcome to the bounded facts a parser may see.
fn reading_of(outcome: &CliProbeOutcome) -> CliProbeReading<'_> {
    CliProbeReading {
        exit_code: outcome.exit_code,
        timed_out: outcome.timed_out,
        // The probe adapter reports a cancelled probe as an unstartable one, so there is no
        // separate flag to forward here.
        cancelled: false,
        stdout: &outcome.stdout,
        stderr: &outcome.stderr,
    }
}

fn compatibility_for(
    definition: &'static CliToolDefinition,
    active_version: Option<&NormalizedCliVersion>,
) -> CliCompatibilityStatus {
    if !definition
        .compatibility
        .platforms
        .supports_current_platform()
    {
        return CliCompatibilityStatus::UnsupportedPlatform;
    }
    match active_version.and_then(|version| definition.compatibility.is_below_floor(version)) {
        Some(true) => CliCompatibilityStatus::UnsupportedVersion,
        Some(false) => CliCompatibilityStatus::Supported,
        // No floor declared, or an opaque version. Either way it cannot be judged.
        None => CliCompatibilityStatus::Unknown,
    }
}

#[cfg(test)]
#[path = "environment_refresh_tests.rs"]
mod tests;
