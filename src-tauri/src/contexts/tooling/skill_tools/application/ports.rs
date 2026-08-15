use super::{
    SkillToolApplicationError, SkillToolFileEntry, SkillToolLogEvent, SkillToolPackageRef,
    SkillToolRevisionState,
};
use crate::contexts::tooling::skill_tools::domain::{
    BoundedJsonSchema, ContentHash, SkillToolCapability, SkillToolDiagnosticSummary, SkillToolKey,
    SkillToolLifecycle, SkillToolLimits, SkillToolRevision, SkillToolTrustDecision,
    SkillToolTrustRecord,
};
use serde_json::Value;

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

/// Result of one module invocation, as the sandbox reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolModuleOutcome {
    Completed(Value),
    LimitBreached { limit: String },
    Trapped { detail: String },
    Cancelled,
}

/// Executes a WebAssembly tool. Implemented natively in section 4; absent implementations report
/// `ModuleRuntimeUnavailable` so declarative tools keep working with the module runtime disabled.
pub(crate) trait SkillToolModuleRuntime: Send + Sync {
    fn is_available(&self) -> bool;

    fn invoke(
        &self,
        package: &SkillToolPackageRef,
        key: &SkillToolKey,
        export: &str,
        input: &Value,
        limits: &SkillToolLimits,
    ) -> Result<SkillToolModuleOutcome, SkillToolApplicationError>;
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

/// Contributes eligible Skill tools to a generation context.
pub(crate) trait SkillToolCatalogPort: Send + Sync {
    fn catalog_for(
        &self,
        workspace_path: Option<&str>,
        loaded_skill_ids: &[String],
    ) -> Result<Vec<SkillToolCatalogEntry>, SkillToolApplicationError>;
}

/// The identity a delegated host operation is evaluated under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SkillToolPrincipal {
    pub(crate) parent_agent_id: String,
    pub(crate) key: SkillToolKey,
    pub(crate) workspace_path: Option<String>,
    pub(crate) session_id: Option<String>,
    pub(crate) delegation_chain: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SkillToolDispatchOutcome {
    Completed(Value),
    Denied { reason: String },
    Cancelled,
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
