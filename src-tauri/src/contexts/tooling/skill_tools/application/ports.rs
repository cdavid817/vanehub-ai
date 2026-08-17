use super::{
    EffectiveSkillToolPackage, SkillToolApplicationError, SkillToolDiscoveryOutcome,
    SkillToolFileEntry, SkillToolLogEvent, SkillToolOwnerKind, SkillToolPackageRef,
    SkillToolRevisionState,
};
use crate::contexts::tooling::skill_tools::domain::{
    BoundedJsonSchema, ContentHash, ModuleImplementation, SkillToolCapability,
    SkillToolDiagnosticSummary, SkillToolKey, SkillToolLifecycle, SkillToolLimits,
    SkillToolRevision, SkillToolTrustDecision, SkillToolTrustRecord,
};
use serde_json::Value;
use std::any::Any;
use std::collections::HashSet;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

/// Reads tool content out of one resolved Skill package.
///
/// Every method takes the package it must stay inside rather than a free path, so containment is
/// a property of the contract and not of each caller remembering to check.
pub(crate) trait SkillToolPackageSource: Send + Sync {
    /// `Ok(None)` when the package ships no manifest — the common case, and not an error.
    fn read_manifest(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Option<Vec<u8>>, SkillToolApplicationError>;

    /// Every file under the reserved tool directory, so undeclared content can be reported.
    fn list_tool_files(
        &self,
        package: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolFileEntry>, SkillToolApplicationError>;

    fn read_implementation(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
    ) -> Result<Vec<u8>, SkillToolApplicationError>;
}

/// Verifies that content still matches the hash a manifest bound it to.
///
/// Separate from the source because verification runs again at trust time and call time, when the
/// bytes may have been replaced since discovery read them.
pub(crate) trait SkillToolIntegrityPort: Send + Sync {
    fn verify_implementation(
        &self,
        package: &SkillToolPackageRef,
        relative_path: &str,
        expected: &ContentHash,
    ) -> Result<(), SkillToolApplicationError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolSchemaViolation {
    pub(crate) pointer: String,
    pub(crate) code: String,
}

/// Validates a concrete payload against a schema already proven to be in the bounded subset.
pub(crate) trait SkillToolSchemaValidationPort: Send + Sync {
    fn validate_instance(
        &self,
        schema: &BoundedJsonSchema,
        instance: &Value,
    ) -> Result<(), Vec<SkillToolSchemaViolation>>;
}

/// Revision-bound trust, enablement, validation, quarantine, and diagnostics.
///
/// Reads and writes are keyed by `SkillToolRevision` throughout: state for a revision that no
/// longer exists is retained as evidence, never rebound to whatever replaced it.
pub(crate) trait SkillToolStateRepository: Send + Sync {
    fn revision_states(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<Vec<SkillToolRevisionState>, SkillToolApplicationError>;

    fn revision_state(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolRevisionState>, SkillToolApplicationError>;

    /// Inserts a newly discovered revision without disturbing an existing row's governance state.
    fn record_discovered(
        &self,
        state: &SkillToolRevisionState,
    ) -> Result<(), SkillToolApplicationError>;

    fn save_lifecycle(
        &self,
        revision: &SkillToolRevision,
        lifecycle: &SkillToolLifecycle,
        validation_code: Option<&str>,
        diagnostics: &SkillToolDiagnosticSummary,
        updated_at: &str,
    ) -> Result<(), SkillToolApplicationError>;

    fn trust_record(
        &self,
        revision: &SkillToolRevision,
    ) -> Result<Option<SkillToolTrustRecord>, SkillToolApplicationError>;

    /// Writes the trust decision and the derived enablement eligibility as one unit; a decision
    /// that lands without its lifecycle row would leave a tool trusted by no observable state.
    fn save_trust(
        &self,
        record: &SkillToolTrustRecord,
        decision: SkillToolTrustDecision,
    ) -> Result<(), SkillToolApplicationError>;
}

pub(crate) trait SkillToolEffectiveDiscoveryPort: Send + Sync {
    fn discover_effective(
        &self,
        owner: &SkillToolPackageRef,
    ) -> Result<
        (
            EffectiveSkillToolPackage,
            SkillToolDiscoveryOutcome,
            SkillToolOwnerKind,
        ),
        SkillToolApplicationError,
    >;
}

/// Result of one module invocation, as the sandbox reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolModuleOutcome {
    Completed(Value),
    LimitBreached { limit: String },
    Trapped { detail: String },
    Cancelled,
}

pub(crate) trait SkillToolModuleHostCallPort: Send + Sync {
    fn call(&self, request: &Value) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError>;
}

/// Executes a WebAssembly tool. Implemented natively in section 4; absent implementations report
/// `ModuleRuntimeUnavailable` so declarative tools keep working with the module runtime disabled.
pub(crate) trait SkillToolModuleRuntime: Send + Sync {
    fn is_available(&self) -> bool;

    #[allow(clippy::too_many_arguments)]
    fn invoke(
        &self,
        package: &SkillToolPackageRef,
        key: &SkillToolKey,
        module: &ModuleImplementation,
        export: &str,
        input: &Value,
        limits: &SkillToolLimits,
        cancelled: &AtomicBool,
        host_calls: Arc<dyn SkillToolModuleHostCallPort>,
    ) -> Result<SkillToolModuleOutcome, SkillToolApplicationError>;
}

pub(crate) trait SkillToolCompiledArtifactPort: Send + Sync {
    fn retain_revisions(&self, revisions: &HashSet<SkillToolRevision>);
}

/// One tool as the agent catalog needs to see it: a provider-visible name, a description, and an
/// input schema — never an implementation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolCatalogEntry {
    pub(crate) canonical_name: String,
    pub(crate) description: String,
    pub(crate) input_schema: Value,
    pub(crate) key: SkillToolKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolBinding {
    pub(crate) skill_id: String,
    pub(crate) revision: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolCatalogMode {
    Plan,
    Execute,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolCatalogContext {
    RoleGeneration {
        workspace_path: Option<String>,
        loaded_roles: Vec<SkillToolBinding>,
        mode: SkillToolCatalogMode,
    },
    UtilityDelegation {
        workspace_path: Option<String>,
        utility: SkillToolBinding,
        mode: SkillToolCatalogMode,
    },
    ExternalCli {
        workspace_path: Option<String>,
    },
}

impl SkillToolCatalogContext {
    pub(crate) fn workspace_path(&self) -> Option<&str> {
        match self {
            Self::RoleGeneration { workspace_path, .. }
            | Self::UtilityDelegation { workspace_path, .. }
            | Self::ExternalCli { workspace_path } => workspace_path.as_deref(),
        }
    }

    pub(crate) fn support_state(&self) -> &'static str {
        match self {
            Self::ExternalCli { .. } => "unsupported-external-cli-bridge",
            _ => "supported-native-api",
        }
    }
}

/// Contributes eligible Skill tools to a generation context.
pub(crate) trait SkillToolCatalogPort: Send + Sync {
    fn catalog_for(
        &self,
        context: &SkillToolCatalogContext,
    ) -> Result<SkillToolCatalogSnapshot, SkillToolApplicationError>;
}

pub(crate) struct SkillToolCatalogSnapshot {
    pub(crate) generation: u64,
    pub(crate) entries: Vec<SkillToolCatalogEntry>,
    pub(crate) lease: Arc<dyn Any + Send + Sync>,
}

pub(crate) struct SkillToolExecutionRequest<'a> {
    pub(crate) call_id: &'a str,
    pub(crate) key: &'a SkillToolKey,
    pub(crate) parent_agent_id: &'a str,
    pub(crate) workspace_path: Option<&'a str>,
    pub(crate) session_id: &'a str,
    pub(crate) generation_id: &'a str,
    pub(crate) mode: SkillToolCatalogMode,
    pub(crate) input: &'a Value,
    pub(crate) cancelled: &'a AtomicBool,
    pub(crate) lifecycle: &'a dyn SkillToolExecutionLifecyclePort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolExecutionLifecyclePhase {
    AwaitingApproval,
}

pub(crate) trait SkillToolExecutionLifecyclePort: Send + Sync {
    fn transition(&self, phase: SkillToolExecutionLifecyclePhase);
}

/// Executes only an immutable key retained by the generation's pinned catalog snapshot.
pub(crate) trait SkillToolExecutionPort: Send + Sync {
    fn execute(
        &self,
        request: SkillToolExecutionRequest<'_>,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError>;
}

/// The identity a delegated host operation is evaluated under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolPrincipal {
    pub(crate) parent_agent_id: String,
    pub(crate) key: SkillToolKey,
    pub(crate) workspace_path: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) generation_id: String,
    pub(crate) delegation_chain: Vec<String>,
}

impl SkillToolPrincipal {
    pub(crate) fn new(
        parent_agent_id: &str,
        key: SkillToolKey,
        workspace_path: Option<&str>,
        session_id: &str,
        generation_id: &str,
        delegation_chain: Vec<String>,
    ) -> Result<Self, SkillToolApplicationError> {
        let parent_agent_id = required_bounded(parent_agent_id, 128)?;
        let session_id = required_bounded(session_id, 128)?;
        let generation_id = required_bounded(generation_id, 128)?;
        let workspace_path = workspace_path
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if key.source.workspace_path.as_deref() != workspace_path.as_deref()
            && key.source.workspace_path.is_some()
        {
            return Err(SkillToolApplicationError::HostDenied(
                "principal-workspace".to_string(),
            ));
        }
        if delegation_chain.len() > 4
            || delegation_chain
                .iter()
                .any(|item| item.is_empty() || item.chars().count() > 256)
        {
            return Err(SkillToolApplicationError::HostDenied(
                "principal-delegation".to_string(),
            ));
        }
        Ok(Self {
            parent_agent_id,
            key,
            workspace_path,
            session_id: Some(session_id),
            generation_id,
            delegation_chain,
        })
    }
}

fn required_bounded(value: &str, maximum: usize) -> Result<String, SkillToolApplicationError> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > maximum {
        Err(SkillToolApplicationError::HostDenied(
            "principal-context".to_string(),
        ))
    } else {
        Ok(value.to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolDispatchOutcome {
    Completed(Value),
    Denied { reason: String },
    Failed { code: String },
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SkillToolPermissionDecision {
    Allow,
    Ask,
    Deny,
}

/// Evaluates one concrete delegated operation. A manifest declaration is only admission to this
/// boundary; implementations must resolve policy independently for every call.
pub(crate) trait SkillToolPermissionPort: Send + Sync {
    fn evaluate(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
    ) -> SkillToolPermissionDecision;
}

pub(crate) trait SkillToolApprovalPort: Send + Sync {
    fn create_pending(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
        call_id: &str,
    ) -> Result<String, SkillToolApplicationError>;
}

/// Routes a delegated operation through the existing tool execution and permission boundaries.
///
/// Returning `Denied` rather than an error keeps a policy refusal distinguishable from a runtime
/// failure, which matters because only the second may count toward quarantine.
pub(crate) trait SkillToolHostDispatchPort: Send + Sync {
    fn dispatch(
        &self,
        principal: &SkillToolPrincipal,
        capability: &SkillToolCapability,
        arguments: &Value,
    ) -> Result<SkillToolDispatchOutcome, SkillToolApplicationError>;
}

pub(crate) trait SkillToolInvocationBudgetPort: Send + Sync {
    fn reserve_host_call(&self) -> Result<(), SkillToolApplicationError>;
    fn consume_output(&self, bytes: u64) -> Result<(), SkillToolApplicationError>;
}

/// Records a successful invocation against the owning Skill's usage counters.
pub(crate) trait SkillToolUsagePort: Send + Sync {
    fn record_invocation(
        &self,
        key: &SkillToolKey,
        occurred_at: &str,
    ) -> Result<(), SkillToolApplicationError>;
}

pub(crate) trait SkillToolLoggingPort: Send + Sync {
    fn record(&self, event: &SkillToolLogEvent) -> Result<(), SkillToolApplicationError>;
}

pub(crate) trait SkillToolClockPort: Send + Sync {
    fn now(&self) -> String;
}

#[cfg(test)]
#[path = "principal_tests.rs"]
mod principal_tests;
