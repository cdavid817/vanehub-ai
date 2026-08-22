//! Behaviour the CLI use cases need from the outside world.
//!
//! Each port is narrow and named for what it does, not for the technology behind it. None exposes
//! a `rusqlite::Connection`, a `tauri::AppHandle`, a `std::process::Command`, or an untyped map --
//! the application layer must remain testable with plain doubles, and an adapter must not be able
//! to smuggle a transport concern across the boundary.
//!
//! Concrete adapters are selected in bootstrap. The application never names one.

use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use chrono::{DateTime, Utc};

use super::environment_error::CliEnvironmentError;
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
use crate::contexts::tooling::cli::domain::version::NormalizedCliVersion;

/// Cooperative cancellation. Set by the operation layer, polled by long-running adapters.
///
/// Best-effort by nature: an adapter checks it between steps and before launching a child, and
/// stops checking once an irreversible step has begun.
#[derive(Clone)]
pub(crate) struct CliCancellation(Arc<AtomicBool>);

impl CliCancellation {
    pub(crate) fn new(flag: Arc<AtomicBool>) -> Self {
        Self(flag)
    }

    pub(crate) fn is_cancelled(&self) -> bool {
        self.0.load(std::sync::atomic::Ordering::SeqCst)
    }

    /// The shared flag, so a process adapter can hand it to the platform layer rather than polling
    /// it from a second place that could drift out of sync.
    pub(crate) fn signal(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.0)
    }

    /// A token that is never signalled.
    ///
    /// Post-mutation detection uses one instead of the operation's own flag. Cancelling an upgrade
    /// stops the package manager; it must not also stop VaneHub from looking at what the package
    /// manager already did. Sharing the flag makes `changed-but-failed` unreachable after a
    /// cancellation, which is precisely the case the five outcome states exist to distinguish.
    pub(crate) fn uncancelled() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    #[cfg(test)]
    pub(crate) fn never() -> Self {
        Self::uncancelled()
    }
}

/// Time and identity, injected so use cases are deterministic under test.
pub(crate) trait CliClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

pub(crate) trait CliIdFactory: Send + Sync {
    fn next_plan_id(&self) -> CliActionPlanId;
    fn next_bulk_plan_id(&self) -> CliBulkPlanId;
}

/// Bounded enumeration limits handed to discovery, so "scan the disk" is not representable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CliProbeBudget {
    pub(crate) max_candidates: usize,
    pub(crate) per_probe_timeout_seconds: u64,
    pub(crate) output_budget_bytes: usize,
}

impl Default for CliProbeBudget {
    fn default() -> Self {
        Self {
            max_candidates: 32,
            per_probe_timeout_seconds: 10,
            output_budget_bytes: CliProbeCommand::VERSION_BUDGET_BYTES,
        }
    }
}

/// Finds executables. Enumerates PATH in real order, then a bounded set of known locations.
pub(crate) trait CliDiscoveryPort: Send + Sync {
    fn discover(
        &self,
        agent_id: &CliToolId,
        executable_names: &[&str],
        budget: CliProbeBudget,
        cancellation: &CliCancellation,
    ) -> Result<Vec<CliInstallation>, CliEnvironmentError>;

    /// The non-secret fingerprint of the current environment. Plans bind to it, and a change
    /// invalidates them.
    fn environment_fingerprint(&self) -> Result<String, CliEnvironmentError>;
}

/// A bounded read-only probe result. Output is already truncated and redacted by the adapter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliProbeOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
    pub(crate) truncated: bool,
}

impl CliProbeOutcome {
    pub(crate) fn succeeded(&self) -> bool {
        !self.timed_out && self.exit_code == Some(0)
    }
}

pub(crate) trait CliProbePort: Send + Sync {
    fn run_probe(
        &self,
        executable_path: &str,
        command: CliProbeCommand,
        cancellation: &CliCancellation,
    ) -> Result<CliProbeOutcome, CliEnvironmentError>;
}

/// What a source reports about itself before it is asked to act.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliSourcePreflight {
    pub(crate) available: bool,
    /// Version of the package manager itself, where it reports one.
    pub(crate) source_version: Option<String>,
    /// Whether an exact target version can actually be honoured here. WinGet answers this
    /// dynamically; a `false` downgrades the plan to latest-only rather than silently installing
    /// latest under an exact label.
    pub(crate) supports_exact_version: bool,
    pub(crate) supports_repair: bool,
    pub(crate) requires_elevation: bool,
}

/// A proposed action, handed to a source adapter so it can produce the command preview.
///
/// The target version is the one the *user selected*, carried through unchanged. Nothing downstream
/// substitutes a latest version for it -- that substitution is the defect this change removes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct CliPlanRequest<'a> {
    pub(crate) agent_id: &'a CliToolId,
    pub(crate) action: CliActionKind,
    pub(crate) target_version: Option<&'a NormalizedCliVersion>,
    pub(crate) channel: Option<&'a str>,
    /// Resolved from the backend registry, never from the frontend.
    pub(crate) package_reference: Option<&'a str>,
    /// Whether this source's preflight confirmed it can honour an exact version here.
    pub(crate) exact_version_confirmed: bool,
}

/// The command a source will run, resolved from a plan. Structured, never a shell string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliExecutionSpec {
    pub(crate) program: String,
    pub(crate) args: Vec<String>,
    pub(crate) requires_network: bool,
    pub(crate) requires_elevation: bool,
}

/// How a source's process ended. Output is bounded and redacted before it gets here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CliProcessOutcome {
    pub(crate) exit_code: Option<i32>,
    pub(crate) timed_out: bool,
    pub(crate) cancelled: bool,
    pub(crate) truncated: bool,
}

impl CliProcessOutcome {
    pub(crate) fn succeeded(&self) -> bool {
        !self.timed_out && !self.cancelled && self.exit_code == Some(0)
    }
}

/// Where an adapter streams its bounded, redacted output.
pub(crate) trait CliOutputSink: Send + Sync {
    fn emit(&self, line: &str);
}

/// Where an adapter announces the stage it has reached.
///
/// Only the adapter knows when a download ends and an install begins, and that boundary is exactly
/// where cancellation stops being safe. Reporting both phases from the caller would either show
/// `mutating` during a download or offer cancel during an install; neither is true.
///
/// Reporting a phase must never fail an operation, so this returns nothing.
pub(crate) trait CliPhaseSink: Send + Sync {
    fn enter(&self, phase: CliOperationPhase, cancellable: bool);
}

/// One distribution source: its catalog, its preflight, and its execution.
///
/// `list_versions` returns a catalog stamped with this adapter's own source id, which is what makes
/// borrowing another source's catalog impossible rather than merely discouraged.
pub(crate) trait CliDistributionPort: Send + Sync {
    fn source_id(&self) -> CliSourceId;

    fn mutation_key(&self, agent_id: &CliToolId) -> CliMutationKey;

    fn preflight(
        &self,
        definition: &CliDistributionDefinition,
        cancellation: &CliCancellation,
    ) -> Result<CliSourcePreflight, CliEnvironmentError>;

    fn list_versions(
        &self,
        agent_id: &CliToolId,
        definition: &CliDistributionDefinition,
        channel: Option<&str>,
        cancellation: &CliCancellation,
    ) -> Result<CliVersionCatalog, CliEnvironmentError>;

    /// The argv this source would run for a proposed action.
    ///
    /// Called during planning, before a plan exists, so the preview the user reviews is produced
    /// by the same adapter that will execute it. The plan then carries that preview, and
    /// `build_execution` derives the spec from it -- the reviewed command and the executed one are
    /// the same object rather than two independent constructions that can disagree.
    fn build_command_preview(
        &self,
        request: &CliPlanRequest<'_>,
        definition: &CliDistributionDefinition,
    ) -> Result<CliCommandPreview, CliEnvironmentError>;

    fn build_execution(
        &self,
        plan: &CliActionPlan,
        definition: &CliDistributionDefinition,
    ) -> Result<CliExecutionSpec, CliEnvironmentError>;

    fn execute(
        &self,
        spec: CliExecutionSpec,
        cancellation: &CliCancellation,
        output: &dyn CliOutputSink,
        phases: &dyn CliPhaseSink,
    ) -> Result<CliProcessOutcome, CliEnvironmentError>;
}

/// The assembled set of source adapters. Built in bootstrap; the application only looks up.
pub(crate) trait CliSourceRegistry: Send + Sync {
    fn adapter(&self, source_id: &CliSourceId) -> Option<Arc<dyn CliDistributionPort>>;
}

/// Persistence. Every write that must be all-or-nothing is one method, not a sequence the caller
/// composes.
pub(crate) trait CliEnvironmentRepository: Send + Sync {
    fn list_snapshots(&self) -> Result<Vec<CliEnvironmentSnapshot>, CliEnvironmentError>;

    fn load_snapshot(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Option<CliEnvironmentSnapshot>, CliEnvironmentError>;

    fn save_snapshot_atomic(
        &self,
        snapshot: &CliEnvironmentSnapshot,
    ) -> Result<(), CliEnvironmentError>;

    fn load_catalog(
        &self,
        agent_id: &CliToolId,
        source_id: &CliSourceId,
        channel: Option<&str>,
    ) -> Result<Option<CliVersionCatalog>, CliEnvironmentError>;

    fn save_catalog(&self, catalog: &CliVersionCatalog) -> Result<(), CliEnvironmentError>;

    fn create_action_plan(&self, plan: &CliActionPlan) -> Result<(), CliEnvironmentError>;

    fn load_action_plan(
        &self,
        plan_id: &CliActionPlanId,
    ) -> Result<Option<CliActionPlan>, CliEnvironmentError>;

    /// Draft plans for one tool. Used by bulk preparation to pick up the plan it just created;
    /// deliberately narrow rather than a general query surface.
    fn list_draft_plans(
        &self,
        agent_id: &CliToolId,
    ) -> Result<Vec<CliActionPlan>, CliEnvironmentError>;

    /// Atomically validates revision, state, and expiry and moves `draft -> executing`.
    ///
    /// This is the single-use gate. Two concurrent callers cannot both be admitted, which is why
    /// it lives in the repository rather than as a read-then-write in the service.
    fn begin_action_plan_execution(
        &self,
        plan_id: &CliActionPlanId,
        expected_revision: u32,
        current_fingerprint: &str,
        now: DateTime<Utc>,
    ) -> Result<CliActionPlan, CliEnvironmentError>;

    fn finish_action_plan(
        &self,
        plan_id: &CliActionPlanId,
        state: CliActionPlanState,
        now: DateTime<Utc>,
    ) -> Result<(), CliEnvironmentError>;

    /// A bulk plan and every item plan it points at are inserted together or not at all.
    fn create_bulk_plan_atomic(
        &self,
        bulk: &CliBulkActionPlan,
        item_plans: &[CliActionPlan],
    ) -> Result<(), CliEnvironmentError>;

    fn load_bulk_plan(
        &self,
        plan_id: &CliBulkPlanId,
    ) -> Result<Option<CliBulkActionPlan>, CliEnvironmentError>;

    /// Marks expired draft plans, bounded per call so maintenance cannot stall startup.
    fn expire_stale_plans(
        &self,
        now: DateTime<Utc>,
        limit: usize,
    ) -> Result<usize, CliEnvironmentError>;
}

/// Observable operations. The CLI use cases never touch the operation store directly.
pub(crate) trait CliOperationsPort: Send + Sync {
    fn start(
        &self,
        related_agent_id: Option<&CliToolId>,
        message: String,
    ) -> Result<String, CliEnvironmentError>;

    fn report_phase(
        &self,
        operation_id: &str,
        phase: CliOperationPhase,
        cancellable: bool,
    ) -> Result<(), CliEnvironmentError>;

    fn report_units(
        &self,
        operation_id: &str,
        completed: u32,
        total: u32,
    ) -> Result<(), CliEnvironmentError>;

    fn append_output(&self, operation_id: &str, line: &str) -> Result<(), CliEnvironmentError>;

    fn complete(
        &self,
        operation_id: &str,
        result: serde_json::Value,
    ) -> Result<(), CliEnvironmentError>;

    fn fail(&self, operation_id: &str, error: String) -> Result<(), CliEnvironmentError>;

    fn cancellation(&self, operation_id: &str) -> Result<CliCancellation, CliEnvironmentError>;
}

/// A held reservation. Dropping it must release exactly once, so a cancelled or panicking use case
/// cannot leave a tool permanently locked.
pub(crate) trait CliMutationLease: Send + Sync {
    fn agent_id(&self) -> &CliToolId;
    fn mutation_key(&self) -> &CliMutationKey;
    /// Idempotent. Called explicitly on the success path and again by `Drop` on any other path.
    fn release(&self);
}

/// Serializes machine changes.
///
/// Two rules the frontend must never be asked to implement: one mutation per tool, and one per
/// package-manager resource. A disabled button is not a lock.
pub(crate) trait CliMutationCoordinator: Send + Sync {
    /// `None` when the reservation cannot be taken right now. The caller decides whether that is a
    /// queue or a rejection; the coordinator does not block.
    fn try_reserve(
        &self,
        agent_id: &CliToolId,
        key: &CliMutationKey,
    ) -> Result<Option<Arc<dyn CliMutationLease>>, CliEnvironmentError>;

    /// How many mutations may run at once across the whole application.
    fn global_capacity(&self) -> usize;

    /// Whether a read-only detection may run against `key` right now.
    ///
    /// True when nothing holds the key, or when the source declares reads safe during its own
    /// writes. False means the detection waits rather than racing a package manager mid-write and
    /// reporting a half-installed tree as the machine's state.
    fn may_detect_now(&self, key: &CliMutationKey) -> bool;
}

/// Semantic diagnostics. The application emits through this; it never reaches a log file itself.
pub(crate) trait CliDiagnosticsPort: Send + Sync {
    fn record(
        &self,
        operation_id: &str,
        agent_id: Option<&CliToolId>,
        action: Option<CliActionKind>,
        message: &str,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_probe_outcome_only_succeeds_on_a_clean_zero_exit() {
        let clean = CliProbeOutcome {
            exit_code: Some(0),
            timed_out: false,
            stdout: "1.2.3".to_string(),
            stderr: String::new(),
            truncated: false,
        };
        assert!(clean.succeeded());

        // A timeout that still reported exit 0 is not a success: the value is untrustworthy.
        assert!(!CliProbeOutcome {
            timed_out: true,
            ..clean.clone()
        }
        .succeeded());
        assert!(!CliProbeOutcome {
            exit_code: Some(1),
            ..clean.clone()
        }
        .succeeded());
        // No exit code at all means the process never reported one.
        assert!(!CliProbeOutcome {
            exit_code: None,
            ..clean
        }
        .succeeded());
    }

    #[test]
    fn a_cancelled_process_is_never_a_success_even_at_exit_zero() {
        let outcome = CliProcessOutcome {
            exit_code: Some(0),
            timed_out: false,
            cancelled: false,
            truncated: false,
        };
        assert!(outcome.succeeded());

        // Cancelled mid-write can still exit 0. Treating that as success is how a partial change
        // gets reported as a completed one.
        assert!(!CliProcessOutcome {
            cancelled: true,
            ..outcome.clone()
        }
        .succeeded());
        assert!(!CliProcessOutcome {
            timed_out: true,
            ..outcome
        }
        .succeeded());
    }

    #[test]
    fn the_default_probe_budget_is_bounded_in_every_dimension() {
        let budget = CliProbeBudget::default();
        assert!(budget.max_candidates > 0);
        assert!(budget.per_probe_timeout_seconds > 0);
        assert_eq!(
            budget.output_budget_bytes,
            CliProbeCommand::VERSION_BUDGET_BYTES
        );
    }

    #[test]
    fn cancellation_reflects_the_shared_flag() {
        let flag = Arc::new(AtomicBool::new(false));
        let cancellation = CliCancellation::new(Arc::clone(&flag));
        assert!(!cancellation.is_cancelled());

        flag.store(true, std::sync::atomic::Ordering::SeqCst);
        assert!(cancellation.is_cancelled());
        // A clone observes the same flag, so an adapter handed a copy still sees the request.
        assert!(cancellation.clone().is_cancelled());
        assert!(!CliCancellation::never().is_cancelled());
    }

    #[test]
    fn a_preflight_that_cannot_confirm_exact_versions_says_so() {
        // WinGet answers this dynamically. `false` must downgrade the plan rather than let an
        // exact request run as a latest install under an exact label.
        let preflight = CliSourcePreflight {
            available: true,
            source_version: Some("1.6.0".to_string()),
            supports_exact_version: false,
            supports_repair: false,
            requires_elevation: false,
        };
        assert!(preflight.available);
        assert!(!preflight.supports_exact_version);
        assert!(!preflight.supports_repair);
    }

    #[test]
    fn an_execution_spec_carries_argv_not_a_command_line() {
        let spec = CliExecutionSpec {
            program: "npm".to_string(),
            args: vec![
                "install".to_string(),
                "--global".to_string(),
                "@openai/codex@1.2.3".to_string(),
            ],
            requires_network: true,
            requires_elevation: false,
        };
        assert_eq!(spec.args.len(), 3);
        // Each argument is one value; nothing here needs splitting or quoting.
        assert!(spec.args.iter().all(|arg| !arg.contains(' ')));
        assert!(spec.requires_network);
    }
}
