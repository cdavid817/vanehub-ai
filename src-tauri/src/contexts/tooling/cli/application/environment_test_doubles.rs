//! Deterministic port doubles for the CLI application tests.
//!
//! No SQLite, no filesystem, no network, no Tauri, no live process. Every double is in-memory and
//! records what it was asked to do, so a test can assert on the exact arguments an adapter would
//! have received -- which is how "the selected version reached the process arguments" becomes a
//! testable claim rather than a hope.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use super::environment_error::CliEnvironmentError;
use super::environment_ports::{
    CliCancellation, CliClock, CliDiagnosticsPort, CliDiscoveryPort, CliDistributionPort,
    CliEnvironmentRepository, CliExecutionSpec, CliIdFactory, CliMutationCoordinator,
    CliMutationLease, CliOperationsPort, CliOutputSink, CliPlanRequest, CliProbeBudget,
    CliProbeOutcome, CliProbePort, CliProcessOutcome, CliSourcePreflight, CliSourceRegistry,
};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::bulk::CliBulkActionPlan;
use crate::contexts::tooling::cli::domain::catalog::CliVersionCatalog;
use crate::contexts::tooling::cli::domain::definition::CliDistributionDefinition;
use crate::contexts::tooling::cli::domain::ids::{
    CliActionPlanId, CliBulkPlanId, CliSourceId, CliToolId,
};
use crate::contexts::tooling::cli::domain::installation::CliInstallation;
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::plan::{
    CliActionPlan, CliActionPlanState, CliCommandPreview,
};
use crate::contexts::tooling::cli::domain::probe::CliProbeCommand;
use crate::contexts::tooling::cli::domain::snapshot::CliEnvironmentSnapshot;
use crate::contexts::tooling::cli::domain::source::CliMutationKey;

pub(super) fn timestamp(seconds: i64) -> DateTime<Utc> {
    DateTime::from_timestamp(seconds, 0).expect("timestamp")
}

pub(super) fn tool_id(agent_id: &str) -> CliToolId {
    CliToolId::new(agent_id).expect("tool id")
}

pub(super) fn source_id(value: &str) -> CliSourceId {
    CliSourceId::new(value).expect("source id")
}

/// A clock the test advances explicitly. Nothing reads the wall clock.
pub(super) struct FakeClock {
    now: Mutex<DateTime<Utc>>,
}

impl FakeClock {
    pub(super) fn at(seconds: i64) -> Arc<Self> {
        Arc::new(Self {
            now: Mutex::new(timestamp(seconds)),
        })
    }

    pub(super) fn advance_to(&self, seconds: i64) {
        *self.now.lock().expect("clock") = timestamp(seconds);
    }
}

impl CliClock for FakeClock {
    fn now(&self) -> DateTime<Utc> {
        *self.now.lock().expect("clock")
    }
}

/// Monotonic, prefix-stable ids so assertions can name them.
pub(super) struct FakeIds {
    counter: AtomicU32,
}

impl FakeIds {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            counter: AtomicU32::new(1),
        })
    }
}

impl CliIdFactory for FakeIds {
    fn next_plan_id(&self) -> CliActionPlanId {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        CliActionPlanId::new(format!("plan-{n}")).expect("plan id")
    }

    fn next_bulk_plan_id(&self) -> CliBulkPlanId {
        let n = self.counter.fetch_add(1, Ordering::SeqCst);
        CliBulkPlanId::new(format!("bulk-{n}")).expect("bulk id")
    }
}

#[derive(Default)]
pub(super) struct FakeDiscovery {
    pub(super) installations: Mutex<BTreeMap<String, Vec<CliInstallation>>>,
    pub(super) fingerprint: Mutex<String>,
    pub(super) calls: Mutex<Vec<String>>,
    pub(super) fail_with: Mutex<Option<CliEnvironmentError>>,
}

impl FakeDiscovery {
    pub(super) fn new(fingerprint: &str) -> Arc<Self> {
        Arc::new(Self {
            installations: Mutex::new(BTreeMap::new()),
            fingerprint: Mutex::new(fingerprint.to_string()),
            calls: Mutex::new(Vec::new()),
            fail_with: Mutex::new(None),
        })
    }

    pub(super) fn set(&self, agent_id: &str, installations: Vec<CliInstallation>) {
        self.installations
            .lock()
            .expect("installations")
            .insert(agent_id.to_string(), installations);
    }

    pub(super) fn set_fingerprint(&self, fingerprint: &str) {
        *self.fingerprint.lock().expect("fingerprint") = fingerprint.to_string();
    }

    pub(super) fn discovered_agents(&self) -> Vec<String> {
        self.calls.lock().expect("calls").clone()
    }
}

impl CliDiscoveryPort for FakeDiscovery {
    fn discover(
        &self,
        agent_id: &CliToolId,
        _executable_names: &[&str],
        _budget: CliProbeBudget,
        _cancellation: &CliCancellation,
    ) -> Result<Vec<CliInstallation>, CliEnvironmentError> {
        self.calls
            .lock()
            .expect("calls")
            .push(agent_id.as_str().to_string());
        if let Some(error) = self.fail_with.lock().expect("fail").clone() {
            return Err(error);
        }
        Ok(self
            .installations
            .lock()
            .expect("installations")
            .get(agent_id.as_str())
            .cloned()
            .unwrap_or_default())
    }

    fn environment_fingerprint(&self) -> Result<String, CliEnvironmentError> {
        Ok(self.fingerprint.lock().expect("fingerprint").clone())
    }
}

#[derive(Default)]
pub(super) struct FakeProbes {
    /// Keyed by executable path.
    pub(super) outcomes: Mutex<BTreeMap<String, CliProbeOutcome>>,
    pub(super) calls: Mutex<Vec<(String, Vec<String>)>>,
}

impl FakeProbes {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(super) fn set_version(&self, path: &str, version: &str) {
        self.outcomes.lock().expect("outcomes").insert(
            path.to_string(),
            CliProbeOutcome {
                exit_code: Some(0),
                timed_out: false,
                stdout: version.to_string(),
                stderr: String::new(),
                truncated: false,
            },
        );
    }

    pub(super) fn set_failure(&self, path: &str, timed_out: bool) {
        self.outcomes.lock().expect("outcomes").insert(
            path.to_string(),
            CliProbeOutcome {
                exit_code: if timed_out { None } else { Some(1) },
                timed_out,
                stdout: String::new(),
                stderr: "probe failed".to_string(),
                truncated: false,
            },
        );
    }

    pub(super) fn invocations(&self) -> Vec<(String, Vec<String>)> {
        self.calls.lock().expect("calls").clone()
    }

    /// Response for one specific command against one path.
    ///
    /// Needed because a refresh now runs several probes against the same executable -- `--version`
    /// and then `login status` -- and keying only by path would hand the auth parser the version
    /// output.
    pub(super) fn set_command_output(
        &self,
        path: &str,
        args: &[&str],
        exit_code: Option<i32>,
        stdout: &str,
    ) {
        self.outcomes.lock().expect("outcomes").insert(
            command_key(path, args),
            CliProbeOutcome {
                exit_code,
                timed_out: false,
                stdout: stdout.to_string(),
                stderr: String::new(),
                truncated: false,
            },
        );
    }
}

/// Keys a configured response to an exact command, distinct from the bare-path fallback.
fn command_key(path: &str, args: &[&str]) -> String {
    format!("{path}\u{1}{}", args.join(" "))
}

impl CliProbePort for FakeProbes {
    fn run_probe(
        &self,
        executable_path: &str,
        command: CliProbeCommand,
        _cancellation: &CliCancellation,
    ) -> Result<CliProbeOutcome, CliEnvironmentError> {
        self.calls.lock().expect("calls").push((
            executable_path.to_string(),
            command.args.iter().map(|arg| (*arg).to_string()).collect(),
        ));
        let outcomes = self.outcomes.lock().expect("outcomes");
        // An exact command match wins; the bare path is the fallback the version probe uses.
        Ok(outcomes
            .get(&command_key(executable_path, command.args))
            .or_else(|| outcomes.get(executable_path))
            .cloned()
            .unwrap_or(CliProbeOutcome {
                exit_code: Some(1),
                timed_out: false,
                stdout: String::new(),
                stderr: "not configured".to_string(),
                truncated: false,
            }))
    }
}

/// What a plan asked this source to preview.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PreviewRequest {
    pub(super) agent_id: String,
    pub(super) action: String,
    pub(super) target_version: Option<String>,
    pub(super) channel: Option<String>,
    pub(super) package_reference: Option<String>,
    pub(super) exact_version_confirmed: bool,
}

/// One source adapter. Records the exact spec it was asked to execute, which is what the
/// selected-version regression is asserted against.
pub(super) struct FakeSource {
    pub(super) id: CliSourceId,
    pub(super) preflight: Mutex<CliSourcePreflight>,
    pub(super) catalog: Mutex<Option<CliVersionCatalog>>,
    pub(super) previewed: Mutex<Vec<PreviewRequest>>,
    pub(super) executed: Mutex<Vec<CliExecutionSpec>>,
    pub(super) outcome: Mutex<CliProcessOutcome>,
    pub(super) execute_error: Mutex<Option<CliEnvironmentError>>,
    pub(super) catalog_error: Mutex<Option<CliEnvironmentError>>,
}

impl FakeSource {
    pub(super) fn new(id: &str) -> Arc<Self> {
        Arc::new(Self {
            id: source_id(id),
            preflight: Mutex::new(CliSourcePreflight {
                available: true,
                source_version: Some("1.0.0".to_string()),
                supports_exact_version: true,
                supports_repair: false,
                requires_elevation: false,
            }),
            catalog: Mutex::new(None),
            previewed: Mutex::new(Vec::new()),
            executed: Mutex::new(Vec::new()),
            outcome: Mutex::new(CliProcessOutcome {
                exit_code: Some(0),
                timed_out: false,
                cancelled: false,
                truncated: false,
            }),
            execute_error: Mutex::new(None),
            catalog_error: Mutex::new(None),
        })
    }

    pub(super) fn set_catalog(&self, catalog: CliVersionCatalog) {
        *self.catalog.lock().expect("catalog") = Some(catalog);
    }

    pub(super) fn set_process_failure(&self) {
        *self.outcome.lock().expect("outcome") = CliProcessOutcome {
            exit_code: Some(1),
            timed_out: false,
            cancelled: false,
            truncated: false,
        };
    }

    pub(super) fn executions(&self) -> Vec<CliExecutionSpec> {
        self.executed.lock().expect("executed").clone()
    }

    pub(super) fn previews(&self) -> Vec<PreviewRequest> {
        self.previewed.lock().expect("previewed").clone()
    }
}

impl CliDistributionPort for FakeSource {
    fn source_id(&self) -> CliSourceId {
        self.id.clone()
    }

    fn mutation_key(&self, agent_id: &CliToolId) -> CliMutationKey {
        match self.id.as_str() {
            "npm" => CliMutationKey::npm_global(),
            "winget" => CliMutationKey::winget(),
            _ => CliMutationKey::vendor(agent_id.as_str()),
        }
    }

    fn preflight(
        &self,
        _definition: &CliDistributionDefinition,
        _cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError> {
        Ok(self.preflight.lock().expect("preflight").clone())
    }

    fn list_versions(
        &self,
        _agent_id: &CliToolId,
        _definition: &CliDistributionDefinition,
        _channel: Option<&str>,
        _cancellation: &CliCancellation,
    ) -> Result<CliVersionCatalog, CliEnvironmentError> {
        if let Some(error) = self.catalog_error.lock().expect("catalog error").clone() {
            return Err(error);
        }
        self.catalog
            .lock()
            .expect("catalog")
            .clone()
            .ok_or_else(|| {
                CliEnvironmentError::CatalogUnavailable {
                source_id: self.id.as_str().to_string(),
                reason: crate::contexts::tooling::cli::domain::catalog::
                    CliCatalogUnavailableReason::SourceUnavailable,
            }
            })
    }

    fn build_command_preview(
        &self,
        request: &CliPlanRequest<'_>,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliCommandPreview, CliEnvironmentError> {
        self.previewed
            .lock()
            .expect("previewed")
            .push(PreviewRequest {
                agent_id: request.agent_id.as_str().to_string(),
                exact_version_confirmed: request.exact_version_confirmed,
                action: request.action.as_str().to_string(),
                target_version: request
                    .target_version
                    .map(|version| version.as_str().to_string()),
                channel: request.channel.map(str::to_string),
                package_reference: request.package_reference.map(str::to_string),
            });

        // Mirrors the npm adapter: the target lands in the package spec verbatim, so a test can
        // assert that the version the user selected reached the arguments.
        let package = request.package_reference.unwrap_or("fixture-package");
        let arg = match request.target_version {
            Some(version) => format!("{package}@{}", version.as_str()),
            None => format!("{package}@latest"),
        };
        Ok(CliCommandPreview::new(
            self.id.as_str(),
            vec!["install".to_string(), "--global".to_string(), arg],
        ))
    }

    fn build_execution(
        &self,
        plan: &CliActionPlan,
        _definition: &CliDistributionDefinition,
    ) -> Result<CliExecutionSpec, CliEnvironmentError> {
        // Mirrors what a real adapter does: the spec comes from the plan, never from a UI field.
        Ok(CliExecutionSpec {
            program: plan.command_preview.program.clone(),
            args: plan.command_preview.args.clone(),
            requires_network: plan.requires_network,
            requires_elevation: plan.requires_elevation,
        })
    }

    fn execute(
        &self,
        spec: CliExecutionSpec,
        cancellation: &CliCancellation,
        output: &dyn CliOutputSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError> {
        self.executed.lock().expect("executed").push(spec);
        output.emit("fixture output");
        if let Some(error) = self.execute_error.lock().expect("execute error").clone() {
            return Err(error);
        }
        if cancellation.is_cancelled() {
            return Ok(CliProcessOutcome {
                exit_code: None,
                timed_out: false,
                cancelled: true,
                truncated: false,
            });
        }
        Ok(self.outcome.lock().expect("outcome").clone())
    }
}

#[derive(Default)]
pub(super) struct FakeSourceRegistry {
    sources: Mutex<BTreeMap<String, Arc<dyn CliDistributionPort>>>,
}

impl FakeSourceRegistry {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(super) fn register(&self, source: Arc<dyn CliDistributionPort>) {
        self.sources
            .lock()
            .expect("sources")
            .insert(source.source_id().as_str().to_string(), source);
    }
}

impl CliSourceRegistry for FakeSourceRegistry {
    fn adapter(&self, source_id: &CliSourceId) -> Option<Arc<dyn CliDistributionPort>> {
        self.sources
            .lock()
            .expect("sources")
            .get(source_id.as_str())
            .map(Arc::clone)
    }
}

#[derive(Default)]
pub(super) struct FakeRepository {
    pub(super) snapshots: Mutex<BTreeMap<String, CliEnvironmentSnapshot>>,
    pub(super) catalogs: Mutex<BTreeMap<String, CliVersionCatalog>>,
    pub(super) plans: Mutex<BTreeMap<String, CliActionPlan>>,
    pub(super) bulk_plans: Mutex<BTreeMap<String, CliBulkActionPlan>>,
    pub(super) save_error: Mutex<Option<CliEnvironmentError>>,
    pub(super) snapshot_writes: Mutex<Vec<String>>,
}

impl FakeRepository {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    fn catalog_key(agent_id: &CliToolId, source_id: &CliSourceId, channel: Option<&str>) -> String {
        format!(
            "{}|{}|{}",
            agent_id.as_str(),
            source_id.as_str(),
            channel.unwrap_or_default()
        )
    }

    pub(super) fn snapshot(&self, agent_id: &str) -> Option<CliEnvironmentSnapshot> {
        self.snapshots
            .lock()
            .expect("snapshots")
            .get(agent_id)
            .cloned()
    }

    pub(super) fn written_agents(&self) -> Vec<String> {
        self.snapshot_writes.lock().expect("writes").clone()
    }

    pub(super) fn plan(&self, plan_id: &str) -> Option<CliActionPlan> {
        self.plans.lock().expect("plans").get(plan_id).cloned()
    }
}

impl CliEnvironmentRepository for FakeRepository {
    fn list_snapshots(&self) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError> {
        Ok(self
            .snapshots
            .lock()
            .expect("snapshots")
            .values()
            .cloned()
            .collect())
    }

    fn load_snapshot(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Option<CliEnvironmentSnapshot>, CliEnvironmentError> {
        Ok(self
            .snapshots
            .lock()
            .expect("snapshots")
            .get(agent_id.as_str())
            .cloned())
    }

    fn save_snapshot_atomic(
        &self,
        snapshot: &CliEnvironmentSnapshot,
    ) -> Result<(), CliEnvironmentError> {
        if let Some(error) = self.save_error.lock().expect("save error").clone() {
            return Err(error);
        }
        self.snapshot_writes
            .lock()
            .expect("writes")
            .push(snapshot.agent_id.as_str().to_string());
        self.snapshots
            .lock()
            .expect("snapshots")
            .insert(snapshot.agent_id.as_str().to_string(), snapshot.clone());
        Ok(())
    }

    fn load_catalog(
        &self,
        agent_id: &CliToolId,
        source_id: &CliSourceId,
        channel: Option<&str>,
    ) -> Result<Option<CliVersionCatalog>, CliEnvironmentError> {
        Ok(self
            .catalogs
            .lock()
            .expect("catalogs")
            .get(&Self::catalog_key(agent_id, source_id, channel))
            .cloned())
    }

    fn save_catalog(&self, catalog: &CliVersionCatalog) -> Result<(), CliEnvironmentError> {
        self.catalogs.lock().expect("catalogs").insert(
            Self::catalog_key(
                &catalog.agent_id,
                &catalog.source_id,
                catalog.channel.as_deref(),
            ),
            catalog.clone(),
        );
        Ok(())
    }

    fn create_action_plan(&self, plan: &CliActionPlan) -> Result<(), CliEnvironmentError> {
        self.plans
            .lock()
            .expect("plans")
            .insert(plan.id.as_str().to_string(), plan.clone());
        Ok(())
    }

    fn load_action_plan(
        &self,
        plan_id: &CliActionPlanId,
    ) -> Result<Option<CliActionPlan>, CliEnvironmentError> {
        Ok(self
            .plans
            .lock()
            .expect("plans")
            .get(plan_id.as_str())
            .cloned())
    }

    fn list_draft_plans(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Vec<CliActionPlan>, CliEnvironmentError> {
        Ok(self
            .plans
            .lock()
            .expect("plans")
            .values()
            .filter(|plan| &plan.agent_id == agent_id && plan.state == CliActionPlanState::Draft)
            .cloned()
            .collect())
    }

    fn begin_action_plan_execution(
        &self,
        plan_id: &CliActionPlanId,
        expected_revision: u32,
        current_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<CliActionPlan, CliEnvironmentError> {
        let mut plans = self.plans.lock().expect("plans");
        let plan = plans
            .get_mut(plan_id.as_str())
            .ok_or(CliEnvironmentError::PlanNotFound)?;
        plan.admit_execution(expected_revision, current_fingerprint, now)?;
        // The transition is part of the same critical section, exactly as SQLite must do it.
        plan.state = CliActionPlanState::Executing;
        Ok(plan.clone())
    }

    fn finish_action_plan(
        &self,
        plan_id: &CliActionPlanId,
        state: CliActionPlanState,
        _now: DateTime<Utc>,
    ) -> Result<(), CliEnvironmentError> {
        if let Some(plan) = self.plans.lock().expect("plans").get_mut(plan_id.as_str()) {
            plan.state = state;
        }
        Ok(())
    }

    fn create_bulk_plan_atomic(
        &self,
        bulk: &CliBulkActionPlan,
        item_plans: &[CliActionPlan],
    ) -> Result<(), CliEnvironmentError> {
        if let Some(error) = self.save_error.lock().expect("save error").clone() {
            // All-or-nothing: no item plan is left behind when the batch fails.
            return Err(error);
        }
        let mut plans = self.plans.lock().expect("plans");
        for plan in item_plans {
            plans.insert(plan.id.as_str().to_string(), plan.clone());
        }
        self.bulk_plans
            .lock()
            .expect("bulk plans")
            .insert(bulk.id.as_str().to_string(), bulk.clone());
        Ok(())
    }

    fn load_bulk_plan(
        &self,
        plan_id: &CliBulkPlanId,
    ) -> Result<Option<CliBulkActionPlan>, CliEnvironmentError> {
        Ok(self
            .bulk_plans
            .lock()
            .expect("bulk plans")
            .get(plan_id.as_str())
            .cloned())
    }

    fn expire_stale_plans(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<usize, CliEnvironmentError> {
        let mut expired = 0;
        for plan in self.plans.lock().expect("plans").values_mut() {
            if expired >= limit {
                break;
            }
            if plan.state == CliActionPlanState::Draft && plan.is_expired(now) {
                plan.state = CliActionPlanState::Expired;
                expired += 1;
            }
        }
        Ok(expired)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RecordedOperation {
    pub(super) id: String,
    pub(super) agent_id: Option<String>,
    pub(super) phases: Vec<String>,
    pub(super) units: Vec<(u32, u32)>,
    pub(super) output: Vec<String>,
    pub(super) terminal: Option<String>,
    pub(super) result: Option<serde_json::Value>,
    pub(super) error: Option<String>,
    pub(super) cancellable: Vec<bool>,
}

#[derive(Default)]
pub(super) struct FakeOperations {
    pub(super) operations: Mutex<Vec<RecordedOperation>>,
    pub(super) cancel_after: Mutex<Option<String>>,
    counter: AtomicU32,
}

impl FakeOperations {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(super) fn all(&self) -> Vec<RecordedOperation> {
        self.operations.lock().expect("operations").clone()
    }

    pub(super) fn find(&self, id: &str) -> Option<RecordedOperation> {
        self.all().into_iter().find(|operation| operation.id == id)
    }

    /// Marks an operation as cancelled, so the next `cancellation()` a use case takes is already
    /// signalled.
    pub(super) fn cancel(&self, id: &str) {
        *self.cancel_after.lock().expect("cancel") = Some(id.to_string());
    }

    fn with<R>(&self, id: &str, mutate: impl FnOnce(&mut RecordedOperation) -> R) -> R {
        let mut operations = self.operations.lock().expect("operations");
        let operation = operations
            .iter_mut()
            .find(|operation| operation.id == id)
            .expect("operation exists");
        mutate(operation)
    }
}

impl CliOperationsPort for FakeOperations {
    fn start(
        &self,
        related_agent_id: Option<&CliToolId>,
        _message: String,
    ) -> Result<String, CliEnvironmentError> {
        let id = format!("op-{}", self.counter.fetch_add(1, Ordering::SeqCst));
        self.operations
            .lock()
            .expect("operations")
            .push(RecordedOperation {
                id: id.clone(),
                agent_id: related_agent_id.map(|agent| agent.as_str().to_string()),
                phases: Vec::new(),
                units: Vec::new(),
                output: Vec::new(),
                terminal: None,
                result: None,
                error: None,
                cancellable: Vec::new(),
            });
        Ok(id)
    }

    fn report_phase(
        &self,
        operation_id: &str,
        phase: CliOperationPhase,
        cancellable: bool,
    ) -> Result<(), CliEnvironmentError> {
        self.with(operation_id, |operation| {
            operation.phases.push(phase.as_str().to_string());
            operation.cancellable.push(cancellable);
        });
        Ok(())
    }

    fn report_units(
        &self,
        operation_id: &str,
        completed: u32,
        total: u32,
    ) -> Result<(), CliEnvironmentError> {
        self.with(operation_id, |operation| {
            operation.units.push((completed, total));
        });
        Ok(())
    }

    fn append_output(&self, operation_id: &str, line: &str) -> Result<(), CliEnvironmentError> {
        self.with(operation_id, |operation| {
            operation.output.push(line.to_string());
        });
        Ok(())
    }

    fn complete(
        &self,
        operation_id: &str,
        result: serde_json::Value,
    ) -> Result<(), CliEnvironmentError> {
        self.with(operation_id, |operation| {
            operation.terminal = Some("succeeded".to_string());
            operation.result = Some(result);
        });
        Ok(())
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), CliEnvironmentError> {
        self.with(operation_id, |operation| {
            operation.terminal = Some("failed".to_string());
            operation.error = Some(error);
        });
        Ok(())
    }

    fn cancellation(&self, operation_id: &str) -> Result<CliCancellation, CliEnvironmentError> {
        let cancelled = self
            .cancel_after
            .lock()
            .expect("cancel")
            .as_deref()
            .is_some_and(|id| id == operation_id);
        Ok(CliCancellation::new(Arc::new(
            std::sync::atomic::AtomicBool::new(cancelled),
        )))
    }
}

struct FakeLease {
    agent_id: CliToolId,
    key: CliMutationKey,
    held: Arc<Mutex<Vec<(String, String)>>>,
    released: Arc<std::sync::atomic::AtomicBool>,
}

impl CliMutationLease for FakeLease {
    fn agent_id(&self) -> &CliToolId {
        &self.agent_id
    }

    fn mutation_key(&self) -> &CliMutationKey {
        &self.key
    }

    fn release(&self) {
        // Idempotent: a double release must not free someone else's reservation.
        if self.released.swap(true, Ordering::SeqCst) {
            return;
        }
        let mut held = self.held.lock().expect("held");
        if let Some(index) = held
            .iter()
            .position(|(agent, key)| agent == self.agent_id.as_str() && key == self.key.as_str())
        {
            held.remove(index);
        }
    }
}

impl Drop for FakeLease {
    fn drop(&mut self) {
        self.release();
    }
}

pub(super) struct FakeCoordinator {
    pub(super) held: Arc<Mutex<Vec<(String, String)>>>,
    pub(super) capacity: usize,
    pub(super) detection_safe: bool,
}

impl FakeCoordinator {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self {
            held: Arc::new(Mutex::new(Vec::new())),
            capacity: 2,
            detection_safe: false,
        })
    }

    pub(super) fn currently_held(&self) -> Vec<(String, String)> {
        self.held.lock().expect("held").clone()
    }
}

impl CliMutationCoordinator for FakeCoordinator {
    fn try_reserve(
        &self,
        agent_id: &CliToolId,
        key: &CliMutationKey,
    ) -> Result<Option<Arc<dyn CliMutationLease>>, CliEnvironmentError> {
        let mut held = self.held.lock().expect("held");
        // One mutation per tool, one per package-manager resource, and a global ceiling.
        if held.iter().any(|(agent, _)| agent == agent_id.as_str())
            || held.iter().any(|(_, existing)| existing == key.as_str())
            || held.len() >= self.capacity
        {
            return Ok(None);
        }
        held.push((agent_id.as_str().to_string(), key.as_str().to_string()));
        Ok(Some(Arc::new(FakeLease {
            agent_id: agent_id.clone(),
            key: key.clone(),
            held: Arc::clone(&self.held),
            released: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })))
    }

    fn global_capacity(&self) -> usize {
        self.capacity
    }

    fn may_detect_now(&self, key: &CliMutationKey) -> bool {
        let held = self.held.lock().expect("held");
        !held.iter().any(|(_, existing)| existing == key.as_str()) || self.detection_safe
    }
}

#[derive(Default)]
pub(super) struct FakeDiagnostics {
    pub(super) entries: Mutex<Vec<String>>,
}

impl FakeDiagnostics {
    pub(super) fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    pub(super) fn messages(&self) -> Vec<String> {
        self.entries.lock().expect("entries").clone()
    }
}

impl CliDiagnosticsPort for FakeDiagnostics {
    fn record(
        &self,
        operation_id: &str,
        agent_id: Option<&CliToolId>,
        action: Option<CliActionKind>,
        message: &str,
    ) {
        self.entries.lock().expect("entries").push(format!(
            "{operation_id}|{}|{}|{message}",
            agent_id.map(CliToolId::as_str).unwrap_or("-"),
            action.map(CliActionKind::as_str).unwrap_or("-")
        ));
    }
}
