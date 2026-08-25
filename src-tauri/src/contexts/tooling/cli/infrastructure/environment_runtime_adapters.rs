//! Concrete adapters for the ports the source-aware CLI service depends on.
//!
//! Each is the single place a transport concern lives: the operations store, the unified log, the
//! process clock, id generation, mutation serialization, and the assembled set of source adapters.
//! The application layer names none of them.

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, LogSeverity, OperationKind, OperationProgress, OperationsApi,
};
use crate::contexts::tooling::cli::application::environment_error::CliEnvironmentError;
use crate::contexts::tooling::cli::application::environment_ports::{
    CliCancellation, CliClock, CliDiagnosticsPort, CliDistributionPort, CliIdFactory,
    CliMutationCoordinator, CliMutationLease, CliOperationsPort, CliSourceRegistry,
};
use crate::contexts::tooling::cli::domain::action::CliActionKind;
use crate::contexts::tooling::cli::domain::ids::{
    CliActionPlanId, CliBulkPlanId, CliSourceId, CliToolId,
};
use crate::contexts::tooling::cli::domain::phase::CliOperationPhase;
use crate::contexts::tooling::cli::domain::source::CliMutationKey;
use crate::platform::logging::redact_text;

/// How many machine changes may run at once across the whole application.
///
/// Two, matching the documented bound. Independent mutation keys may overlap; the same key never
/// does, and neither does the same tool.
const GLOBAL_MUTATION_CAPACITY: usize = 2;

pub(crate) struct SystemEnvironmentClock;

impl CliClock for SystemEnvironmentClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

/// Plan ids that are unique per process and per machine.
///
/// A monotonic suffix is appended because two plans prepared in the same millisecond must still be
/// distinguishable in a log read after the fact, which a bare UUID makes needlessly hard.
pub(crate) struct UuidCliIdFactory {
    counter: AtomicU64,
}

impl Default for UuidCliIdFactory {
    fn default() -> Self {
        Self {
            counter: AtomicU64::new(1),
        }
    }
}

impl UuidCliIdFactory {
    fn next(&self, prefix: &str) -> String {
        format!(
            "{prefix}-{}-{}",
            self.counter.fetch_add(1, Ordering::SeqCst),
            Uuid::new_v4()
        )
    }
}

impl CliIdFactory for UuidCliIdFactory {
    fn next_plan_id(&self) -> CliActionPlanId {
        // Built here from an ASCII prefix, a counter, and a UUID, so it cannot violate the id
        // invariant. Falling back to a fixed literal on a validation failure would be worse than
        // trusting it: two plans sharing an id break the single-use guarantee.
        CliActionPlanId::trusted(self.next("cli-plan"))
    }

    fn next_bulk_plan_id(&self) -> CliBulkPlanId {
        CliBulkPlanId::trusted(self.next("cli-bulk"))
    }
}

/// Bridges CLI lifecycle operations onto the shared operations store.
#[derive(Clone)]
pub(crate) struct CliEnvironmentOperationsAdapter {
    operations: OperationsApi,
}

impl CliEnvironmentOperationsAdapter {
    pub(crate) fn new(operations: OperationsApi) -> Self {
        Self { operations }
    }
}

impl CliOperationsPort for CliEnvironmentOperationsAdapter {
    fn start(
        &self,
        related_agent_id: Option<&CliToolId>,
        message: String,
    ) -> Result<String, CliEnvironmentError> {
        self.operations
            .start(
                OperationKind::Agent,
                related_agent_id.map(|id| id.as_str().to_string()),
                Some(message),
            )
            .map(|operation| operation.id)
            .map_err(operations_error)
    }

    fn report_phase(
        &self,
        operation_id: &str,
        phase: CliOperationPhase,
        cancellable: bool,
    ) -> Result<(), CliEnvironmentError> {
        self.operations
            .report_progress(
                operation_id,
                OperationProgress::phase(phase.as_str()).with_cancellable(cancellable),
            )
            .map(|_| ())
            .map_err(operations_error)
    }

    fn report_units(
        &self,
        operation_id: &str,
        completed: u32,
        total: u32,
    ) -> Result<(), CliEnvironmentError> {
        self.operations
            .report_progress(
                operation_id,
                OperationProgress::default().with_units(completed, total),
            )
            .map(|_| ())
            .map_err(operations_error)
    }

    fn append_output(&self, operation_id: &str, line: &str) -> Result<(), CliEnvironmentError> {
        // Redacted again here even though every adapter redacts before emitting. This is the
        // boundary that persists, and a defence that only holds while every caller remembers is
        // not a defence.
        self.operations
            .append_log(operation_id, redact_text(line))
            .map(|_| ())
            .map_err(operations_error)
    }

    fn complete(
        &self,
        operation_id: &str,
        result: serde_json::Value,
    ) -> Result<(), CliEnvironmentError> {
        self.operations
            .complete(operation_id, Some(result))
            .map(|_| ())
            .map_err(operations_error)
    }

    fn fail(&self, operation_id: &str, error: String) -> Result<(), CliEnvironmentError> {
        self.operations
            .fail(operation_id, redact_text(&error))
            .map(|_| ())
            .map_err(operations_error)
    }

    fn cancellation(&self, operation_id: &str) -> Result<CliCancellation, CliEnvironmentError> {
        self.operations
            .cancellation_flag(operation_id)
            .map(CliCancellation::new)
            .map_err(operations_error)
    }
}

/// Writes CLI diagnostics into the unified log.
pub(crate) struct CliEnvironmentDiagnosticsAdapter {
    diagnostics: Arc<dyn DiagnosticLogPort>,
}

impl CliEnvironmentDiagnosticsAdapter {
    pub(crate) fn new(diagnostics: Arc<dyn DiagnosticLogPort>) -> Self {
        Self { diagnostics }
    }
}

impl CliDiagnosticsPort for CliEnvironmentDiagnosticsAdapter {
    fn record(
        &self,
        operation_id: &str,
        agent_id: Option<&CliToolId>,
        action: Option<CliActionKind>,
        message: &str,
    ) {
        let mut context = BTreeMap::new();
        context.insert("operationId".to_string(), operation_id.to_string());
        if let Some(agent_id) = agent_id {
            context.insert("agentId".to_string(), agent_id.as_str().to_string());
        }
        if let Some(action) = action {
            context.insert("action".to_string(), action.as_str().to_string());
        }
        // Redacted at the persistence boundary rather than trusting each call site.
        let _ = self.diagnostics.write_diagnostic(DiagnosticLog {
            severity: LogSeverity::Info,
            category: "cli.environment".to_string(),
            message: redact_text(message),
            context: context.into_iter().collect(),
        });
    }
}

/// One reservation over a tool and a package-manager resource.
struct HeldMutation {
    agent_id: CliToolId,
    key: CliMutationKey,
    held: Arc<Mutex<Vec<(String, String)>>>,
    released: std::sync::atomic::AtomicBool,
}

impl CliMutationLease for HeldMutation {
    fn agent_id(&self) -> &CliToolId {
        &self.agent_id
    }

    fn mutation_key(&self) -> &CliMutationKey {
        &self.key
    }

    fn release(&self) {
        // Idempotent: the success path releases explicitly and `Drop` releases again, and a second
        // release must not free a reservation someone else has since taken.
        if self.released.swap(true, Ordering::SeqCst) {
            return;
        }
        if let Ok(mut held) = self.held.lock() {
            if let Some(index) = held.iter().position(|(agent, key)| {
                agent == self.agent_id.as_str() && key == self.key.as_str()
            }) {
                held.remove(index);
            }
        }
    }
}

impl Drop for HeldMutation {
    fn drop(&mut self) {
        self.release();
    }
}

/// Serializes machine changes across the process.
#[derive(Default)]
pub(crate) struct CliEnvironmentMutationCoordinator {
    held: Arc<Mutex<Vec<(String, String)>>>,
}

impl CliMutationCoordinator for CliEnvironmentMutationCoordinator {
    fn try_reserve(
        &self,
        agent_id: &CliToolId,
        key: &CliMutationKey,
    ) -> Result<Option<Arc<dyn CliMutationLease>>, CliEnvironmentError> {
        let mut held = self.held.lock().map_err(|_| {
            CliEnvironmentError::Storage("mutation registry is poisoned".to_string())
        })?;
        // One per tool, one per package-manager resource, and a global ceiling. A disabled button
        // is not a lock, so all three are enforced here rather than in the UI.
        if held.iter().any(|(agent, _)| agent == agent_id.as_str())
            || held.iter().any(|(_, existing)| existing == key.as_str())
            || held.len() >= GLOBAL_MUTATION_CAPACITY
        {
            return Ok(None);
        }
        held.push((agent_id.as_str().to_string(), key.as_str().to_string()));
        Ok(Some(Arc::new(HeldMutation {
            agent_id: agent_id.clone(),
            key: key.clone(),
            held: Arc::clone(&self.held),
            released: std::sync::atomic::AtomicBool::new(false),
        })))
    }

    fn global_capacity(&self) -> usize {
        GLOBAL_MUTATION_CAPACITY
    }

    fn may_detect_now(&self, key: &CliMutationKey) -> bool {
        // No source currently declares reads safe during its own writes, so a held key means the
        // detection waits. Claiming otherwise would let a probe read a half-installed tree.
        self.held
            .lock()
            .map(|held| !held.iter().any(|(_, existing)| existing == key.as_str()))
            .unwrap_or(false)
    }
}

/// The assembled source adapters, keyed by the source id each one reports for itself.
#[derive(Default)]
pub(crate) struct CliSourceAdapterRegistry {
    adapters: BTreeMap<String, Arc<dyn CliDistributionPort>>,
}

impl CliSourceAdapterRegistry {
    /// Registers an adapter under the id it reports, never under a caller-supplied one.
    ///
    /// That is what makes "a plan executes exactly the source it recorded" structural: a lookup by
    /// plan source id cannot resolve to an adapter that would run something else.
    pub(crate) fn with(mut self, adapter: Arc<dyn CliDistributionPort>) -> Self {
        self.adapters
            .insert(adapter.source_id().as_str().to_string(), adapter);
        self
    }

    #[cfg(test)]
    pub(crate) fn registered_source_ids(&self) -> Vec<&str> {
        self.adapters.keys().map(String::as_str).collect()
    }
}

impl CliSourceRegistry for CliSourceAdapterRegistry {
    fn adapter(&self, source_id: &CliSourceId) -> Option<Arc<dyn CliDistributionPort>> {
        self.adapters.get(source_id.as_str()).map(Arc::clone)
    }
}

fn operations_error(error: impl std::fmt::Display) -> CliEnvironmentError {
    CliEnvironmentError::Storage(redact_text(&error.to_string()))
}

#[cfg(test)]
#[path = "environment_runtime_adapters_tests.rs"]
mod tests;
