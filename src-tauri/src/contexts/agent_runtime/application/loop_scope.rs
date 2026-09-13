//! Execution-scope contracts shared by readiness, authoritative start, role launches, mediated
//! tool delivery, phase evidence and acceptance.
//!
//! Two questions are kept apart on purpose: what the user *requested* (the definition's mode and
//! literal paths) and what the selected executors can actually *enforce* (the assessment). The
//! assessment is derived from stable Agent identity, transport, adapter revision and host
//! capability witnesses, never from a provider's name or an advertised flag alone.

use super::{AgentRuntimeApplicationError, LoopVerificationCancellation};
use crate::contexts::agent_runtime::domain::{
    AgentDefinition, InteractionMode, LoopCoverage, LoopDefinition, LoopRequestedMode,
    LoopScopeConfig, LoopScopeState, LoopSideEffectChannel, LoopVerificationKind,
    LOOP_SCOPE_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::Arc;

/// Bumped whenever the mediated executor, the evidence scanner or the admission rules change
/// in a way that invalidates earlier capability witnesses.
pub(crate) const LOOP_SCOPE_RUNTIME_REVISION: &str = "loop-scope-runtime-v1";
pub(crate) const AUDIT_RECEIPT_TTL_SECONDS: i64 = 300;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopGuardRole {
    Worker,
    Verifier,
}

impl LoopGuardRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Worker => "worker",
            Self::Verifier => "verifier",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "worker" => Some(Self::Worker),
            "verifier" => Some(Self::Verifier),
            _ => None,
        }
    }
}

/// A host capability witness: what this build can prove on this machine.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopPlatformWitness {
    pub(crate) platform: String,
    pub(crate) safe_mutation: bool,
    pub(crate) detail: String,
}

/// What is known about a CLI Agent's execution chain, from the reviewed provider catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopCliWitness {
    pub(crate) provider_id: String,
    pub(crate) adapter_revision: String,
    pub(crate) executable: String,
    pub(crate) acp: bool,
    pub(crate) mapped_permission_hook: bool,
}

pub(crate) trait LoopCliCapabilityPort: Send + Sync {
    fn cli_witness(&self, agent_id: &str) -> Option<LoopCliWitness>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopSurfaceAssessment {
    pub(crate) surface: String,
    pub(crate) coverage: String,
    pub(crate) detail: String,
    pub(crate) blocking: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopExecutionAssessment {
    pub(crate) requested_mode: String,
    pub(crate) surfaces: Vec<LoopSurfaceAssessment>,
    pub(crate) blockers: Vec<String>,
    pub(crate) limitations: Vec<String>,
    pub(crate) satisfies_requested_mode: bool,
    pub(crate) acknowledgement_required: bool,
    pub(crate) witness_digest: String,
    pub(crate) assessed_at: String,
    pub(crate) simulated: bool,
}

#[cfg(test)]
impl LoopExecutionAssessment {
    pub(crate) fn coverage_for(&self, surface: &str) -> Option<LoopCoverage> {
        self.surfaces
            .iter()
            .find(|item| item.surface == surface)
            .and_then(|item| LoopCoverage::parse(&item.coverage))
    }
}

/// Identity of the real run root, bound after worktree preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopRootIdentity {
    pub(crate) canonical_path: String,
    pub(crate) device: u64,
    pub(crate) inode: u64,
    pub(crate) case_rule: String,
}

/// The immutable scope binding stored on a run once its worktree exists.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct LoopScopeBinding {
    pub(crate) schema_version: u32,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) requested_mode: String,
    pub(crate) definition_version: u64,
    pub(crate) scope_digest: String,
    pub(crate) root: LoopRootIdentity,
    pub(crate) base_commit: Option<String>,
    pub(crate) baseline_manifest_id: String,
    pub(crate) baseline_digest: String,
    pub(crate) witness_digest: String,
    pub(crate) audit_receipt_id: Option<String>,
    pub(crate) bound_at: String,
}

impl LoopScopeBinding {
    pub(crate) fn mode(&self) -> Option<LoopRequestedMode> {
        LoopRequestedMode::parse(&self.requested_mode)
    }

    pub(crate) fn scope(&self) -> Result<LoopScopeConfig, AgentRuntimeApplicationError> {
        if self.schema_version != LOOP_SCOPE_SCHEMA_VERSION {
            return Err(AgentRuntimeApplicationError::Loop(
                "Loop scope binding uses an unsupported schema version.".to_string(),
            ));
        }
        LoopScopeConfig::parse(&self.allowed_paths, &self.protected_paths)
            .map_err(|error| AgentRuntimeApplicationError::Loop(error.to_string()))
    }
}

/// The result of binding a prepared worktree: root identity plus the sealed baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopRootBinding {
    pub(crate) root: LoopRootIdentity,
    pub(crate) baseline_manifest_id: String,
    pub(crate) baseline_digest: String,
    pub(crate) baseline_entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopManifestRef {
    pub(crate) manifest_id: String,
    pub(crate) digest: String,
    pub(crate) entries: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopManifestChangeView {
    pub(crate) path: String,
    pub(crate) kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopNativeCheckView {
    pub(crate) status: String,
    pub(crate) findings: Vec<String>,
    pub(crate) inspected_files: usize,
    pub(crate) binary_excluded: usize,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopSealReceipt {
    pub(crate) manifest_id: String,
    pub(crate) digest: String,
    pub(crate) copied_objects: usize,
    pub(crate) copied_bytes: u64,
}

/// A stable, redacted failure from the platform boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopScopeFailure {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl LoopScopeFailure {
    pub(crate) fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<LoopScopeFailure> for AgentRuntimeApplicationError {
    fn from(failure: LoopScopeFailure) -> Self {
        AgentRuntimeApplicationError::Loop(format!("{}: {}", failure.code, failure.message))
    }
}

/// The real-filesystem side of scope enforcement. Implemented by infrastructure; application code
/// never touches paths or handles itself.
pub(crate) trait LoopScopePlatformPort: Send + Sync {
    fn witness(&self) -> LoopPlatformWitness;
    fn bind_root(
        &self,
        run_id: &str,
        worktree_path: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopRootBinding, LoopScopeFailure>;
    fn verify_root(
        &self,
        worktree_path: &str,
        root: &LoopRootIdentity,
    ) -> Result<(), LoopScopeFailure>;
    fn capture_manifest(
        &self,
        run_id: &str,
        worktree_path: &str,
        manifest_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopManifestRef, LoopScopeFailure>;
    fn diff_manifests(
        &self,
        run_id: &str,
        baseline_manifest_id: &str,
        current_manifest_id: &str,
    ) -> Result<Vec<LoopManifestChangeView>, LoopScopeFailure>;
    fn native_check(
        &self,
        run_id: &str,
        worktree_path: &str,
        root: &LoopRootIdentity,
        baseline_manifest_id: &str,
        current_manifest_id: &str,
    ) -> Result<LoopNativeCheckView, LoopScopeFailure>;
    /// Copies the actual contents named by a fresh stable manifest into immutable objects and
    /// returns its identity. The caller compares the digest with verified evidence.
    fn seal_contents(
        &self,
        run_id: &str,
        worktree_path: &str,
        manifest_id: &str,
        cancellation: &LoopVerificationCancellation,
    ) -> Result<LoopSealReceipt, LoopScopeFailure>;
    fn guard(
        &self,
        binding: &LoopScopeBinding,
        role: LoopGuardRole,
    ) -> Result<Arc<dyn LoopScopeGuard>, LoopScopeFailure>;
}

/// The per-session enforcement handle handed to native tools and ACP proxies. Everything it
/// admits was resolved against the bound root; nothing it rejects produced an effect.
pub(crate) trait LoopScopeGuard: Send + Sync {
    /// Whether file writes are pre-admitted and delivered through the safe boundary. False only
    /// on hosts without the boundary, where audit mode relies on artifact validation alone.
    fn mediated(&self) -> bool;
    fn admit_channel(&self, channel: LoopSideEffectChannel) -> Result<(), String>;
    /// Classification only, for refusing an out-of-bound request before it is offered for
    /// approval. Delivery re-admits the real resource.
    fn admit_write(&self, requested: &str) -> Result<(), String>;
    fn read(&self, requested: &str) -> Result<Vec<u8>, String>;
    fn write(&self, requested: &str, content: &[u8]) -> Result<(), String>;
}

/// The trusted-ownership lookup: given a session id the backend derives the run, the frozen
/// binding and the role. A model- or frontend-supplied run id never reaches this path.
pub(crate) trait LoopScopeAuthorityPort: Send + Sync {
    fn guard_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<Arc<dyn LoopScopeGuard>>, AgentRuntimeApplicationError>;
}

/// Inputs the assessor needs about one role Agent.
pub(crate) struct LoopRoleAgentFacts {
    pub(crate) agent_id: String,
    pub(crate) interaction_mode: InteractionMode,
    pub(crate) trusted_api: bool,
    pub(crate) cli: Option<LoopCliWitness>,
}

impl LoopRoleAgentFacts {
    pub(crate) fn from_definition(
        agent: &AgentDefinition,
        trusted_api: bool,
        cli: Option<LoopCliWitness>,
    ) -> Self {
        Self {
            agent_id: agent.id().as_str().to_string(),
            interaction_mode: if agent.supports(InteractionMode::Cli) {
                InteractionMode::Cli
            } else {
                InteractionMode::Api
            },
            trusted_api,
            cli,
        }
    }
}

pub(crate) struct LoopAssessmentInput<'a> {
    pub(crate) definition: &'a LoopDefinition,
    pub(crate) worker: Option<LoopRoleAgentFacts>,
    pub(crate) verifier: Option<LoopRoleAgentFacts>,
    pub(crate) platform: LoopPlatformWitness,
    pub(crate) assessed_at: String,
}

/// Derives coverage per role and surface. Pure: every input is a witness the caller obtained
/// from a repository, the reviewed provider catalog or the host.
pub(crate) fn assess_execution(input: LoopAssessmentInput<'_>) -> LoopExecutionAssessment {
    let mode = input
        .definition
        .requested_mode()
        .unwrap_or(LoopRequestedMode::PreventiveRequired);
    let legacy = input.definition.scope_state() == LoopScopeState::LegacyUnverified;
    let mut surfaces = Vec::new();
    let mut witness = Sha256::new();
    witness.update(LOOP_SCOPE_RUNTIME_REVISION.as_bytes());
    witness.update(b"\0");
    witness.update(input.platform.platform.as_bytes());
    witness.update(if input.platform.safe_mutation {
        "\0safe\0"
    } else {
        "\0unsafe\0"
    });

    let (worker_coverage, worker_detail) = match &input.worker {
        None => (
            LoopCoverage::Unknown,
            "Worker Agent is unavailable.".to_string(),
        ),
        Some(facts) => worker_coverage(facts, &input.platform),
    };
    if let Some(facts) = &input.worker {
        hash_role(&mut witness, "worker", facts);
    }
    surfaces.push(surface("worker", worker_coverage, worker_detail, mode));

    let (verifier_coverage, verifier_detail) = match &input.verifier {
        None => (
            LoopCoverage::Unknown,
            "Verifier Agent is unavailable.".to_string(),
        ),
        Some(facts) => verifier_coverage(facts),
    };
    if let Some(facts) = &input.verifier {
        hash_role(&mut witness, "verifier", facts);
    }
    surfaces.push(surface(
        "verifier",
        verifier_coverage,
        verifier_detail,
        mode,
    ));

    for command in &input.definition.values().verification_commands {
        let (coverage, detail) = match command.kind() {
            LoopVerificationKind::NativeCheck => (
                LoopCoverage::CompleteEnforcement,
                format!(
                    "In-process check {} inspects sealed snapshots without launching a process.",
                    command.program()
                ),
            ),
            LoopVerificationKind::Process => (
                LoopCoverage::ArtifactValidationOnly,
                format!(
                    "Process check {} runs {} with no containment of its descendants; only the final workspace is validated.",
                    command.id(),
                    command.program()
                ),
            ),
        };
        witness.update(command.id().as_bytes());
        witness.update(command.kind().as_str().as_bytes());
        witness.update(command.program().as_bytes());
        surfaces.push(surface(
            &format!("verification:{}", command.id()),
            coverage,
            detail,
            mode,
        ));
    }
    // Root binding, manifests, native checks and sealing all go through the handle-relative
    // boundary; without it there is no artifact validation either, so audit mode is not an
    // option on such a host rather than a promise that fails at the first bind.
    let (artifact_coverage, artifact_detail) = if input.platform.safe_mutation {
        (
            LoopCoverage::CompleteEnforcement,
            "Complete worktree manifests are captured at every phase boundary.".to_string(),
        )
    } else {
        (
            LoopCoverage::Unsupported,
            format!(
                "{} has no handle-relative root binding, manifest capture or sealed acceptance; no Loop mode can run here.",
                input.platform.platform
            ),
        )
    };
    surfaces.push(surface(
        "artifact-validation",
        artifact_coverage,
        artifact_detail,
        mode,
    ));

    let mut blockers = Vec::new();
    if legacy {
        blockers.push(
            "scope-legacy-unverified: this definition predates scope enforcement; save it with explicit allowed paths and a requested mode."
                .to_string(),
        );
    } else if let Err(error) = input.definition.scope() {
        blockers.push(format!("{}: {error}", error.code()));
    }
    let verifier_read_only = verifier_coverage == LoopCoverage::CompleteEnforcement;
    if !verifier_read_only {
        blockers.push(
            "verifier-not-read-only: the selected Verifier cannot be proven read-only before effects."
                .to_string(),
        );
    }
    let mut limitations = Vec::new();
    for item in &surfaces {
        let coverage = LoopCoverage::parse(&item.coverage).unwrap_or(LoopCoverage::Unknown);
        if coverage != LoopCoverage::CompleteEnforcement {
            limitations.push(format!(
                "{}: {} — {}",
                item.surface, item.coverage, item.detail
            ));
        }
        if item.blocking && item.surface != "verifier" {
            blockers.push(format!(
                "{}: {} does not satisfy {}",
                item.surface,
                item.coverage,
                mode.as_str()
            ));
        }
    }
    if mode == LoopRequestedMode::ArtifactAudited {
        limitations.push(
            "artifact-audited assumes a cooperative CLI and a trusted host control plane; uncontained same-identity processes could tamper with host storage and transient or outside-root writes are not observed."
                .to_string(),
        );
    }
    let satisfies = blockers.is_empty();
    LoopExecutionAssessment {
        requested_mode: mode.as_str().to_string(),
        surfaces,
        blockers,
        limitations,
        satisfies_requested_mode: satisfies,
        acknowledgement_required: mode == LoopRequestedMode::ArtifactAudited,
        witness_digest: format!("sha256:{}", hex(&witness.finalize())),
        assessed_at: input.assessed_at,
        simulated: false,
    }
}

fn worker_coverage(
    facts: &LoopRoleAgentFacts,
    platform: &LoopPlatformWitness,
) -> (LoopCoverage, String) {
    match facts.interaction_mode {
        InteractionMode::Api => {
            if !facts.trusted_api {
                return (
                    LoopCoverage::Unknown,
                    "API Worker requires tool-use trust before it can be assessed.".to_string(),
                );
            }
            if platform.safe_mutation {
                (
                    LoopCoverage::CompleteEnforcement,
                    "Native Worker session offers only host-mediated file tools; shell, MCP, Skill tools, delegation, notebooks and language servers are disabled and every write is admitted through the handle-relative boundary."
                        .to_string(),
                )
            } else {
                (
                    LoopCoverage::ArtifactValidationOnly,
                    format!(
                        "Native Worker tools are restricted to file operations, but {} has no verified handle-relative delivery; only final artifacts are validated.",
                        platform.platform
                    ),
                )
            }
        }
        InteractionMode::Cli => match &facts.cli {
            Some(witness) if witness.acp => (
                LoopCoverage::MediatedToolsOnly,
                format!(
                    "{} ({}) proxies fs/terminal requests through the host, but its internal tools and subprocesses mutate the worktree without per-action mediation.",
                    witness.provider_id, witness.adapter_revision
                ),
            ),
            Some(witness) if witness.mapped_permission_hook => (
                LoopCoverage::MediatedToolsOnly,
                format!(
                    "{} ({}) reports mapped tool hooks to the host, but unmapped internal tools and subprocesses remain uncovered.",
                    witness.provider_id, witness.adapter_revision
                ),
            ),
            Some(witness) => (
                LoopCoverage::ArtifactValidationOnly,
                format!(
                    "{} ({}) executes with no host mediation of its mutations; only final artifacts are validated.",
                    witness.provider_id, witness.adapter_revision
                ),
            ),
            None => (
                LoopCoverage::Unknown,
                format!(
                    "No reviewed execution witness exists for CLI Agent {}.",
                    facts.agent_id
                ),
            ),
        },
        _ => (
            LoopCoverage::Unsupported,
            format!(
                "Interaction mode {} has no Loop execution boundary.",
                facts.interaction_mode.as_str()
            ),
        ),
    }
}

fn verifier_coverage(facts: &LoopRoleAgentFacts) -> (LoopCoverage, String) {
    match facts.interaction_mode {
        InteractionMode::Api if facts.trusted_api => (
            LoopCoverage::CompleteEnforcement,
            "Native Verifier session runs the read-only catalog; every mutation channel is refused before effects."
                .to_string(),
        ),
        InteractionMode::Api => (
            LoopCoverage::Unknown,
            "API Verifier requires tool-use trust before it can be assessed.".to_string(),
        ),
        InteractionMode::Cli => (
            LoopCoverage::Unsupported,
            "A CLI Verifier cannot be proven read-only: its internal tools and subprocesses can mutate the worktree regardless of the prompt."
                .to_string(),
        ),
        _ => (
            LoopCoverage::Unsupported,
            format!(
                "Interaction mode {} has no Loop execution boundary.",
                facts.interaction_mode.as_str()
            ),
        ),
    }
}

fn surface(
    name: &str,
    coverage: LoopCoverage,
    detail: String,
    mode: LoopRequestedMode,
) -> LoopSurfaceAssessment {
    LoopSurfaceAssessment {
        surface: name.to_string(),
        coverage: coverage.as_str().to_string(),
        detail,
        blocking: !coverage.satisfies(mode),
    }
}

fn hash_role(hasher: &mut Sha256, role: &str, facts: &LoopRoleAgentFacts) {
    hasher.update(role.as_bytes());
    hasher.update(b"\0");
    hasher.update(facts.agent_id.as_bytes());
    hasher.update(b"\0");
    hasher.update(facts.interaction_mode.as_str().as_bytes());
    hasher.update(if facts.trusted_api {
        "\0trusted\0"
    } else {
        "\0untrusted\0"
    });
    if let Some(cli) = &facts.cli {
        hasher.update(cli.provider_id.as_bytes());
        hasher.update(b"\0");
        hasher.update(cli.adapter_revision.as_bytes());
        hasher.update(b"\0");
        hasher.update(cli.executable.as_bytes());
        hasher.update(if cli.acp { "\0acp\0" } else { "\0no-acp\0" });
    }
}

pub(crate) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

pub(crate) fn assessment_digest(assessment: &LoopExecutionAssessment) -> String {
    let mut hasher = Sha256::new();
    hasher.update(assessment.requested_mode.as_bytes());
    hasher.update(assessment.witness_digest.as_bytes());
    for item in &assessment.surfaces {
        hasher.update(item.surface.as_bytes());
        hasher.update(b"=");
        hasher.update(item.coverage.as_bytes());
        hasher.update(b"\n");
    }
    format!("sha256:{}", hex(&hasher.finalize()))
}

/// Which control transition an admission challenge was issued for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LoopControlAction {
    Start,
    Resume,
    Continue,
}

impl LoopControlAction {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Start => "start",
            Self::Resume => "resume",
            Self::Continue => "continue",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "start" => Some(Self::Start),
            "resume" => Some(Self::Resume),
            "continue" => Some(Self::Continue),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrepareLoopAdmissionRequest {
    pub(crate) action: LoopControlAction,
    pub(crate) definition_id: Option<String>,
    pub(crate) run_id: Option<String>,
    pub(crate) expected_revision: u64,
    pub(crate) client_context: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopAdmissionView {
    pub(crate) action: LoopControlAction,
    pub(crate) target_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) requested_mode: LoopRequestedMode,
    pub(crate) scope_digest: String,
    pub(crate) allowed_paths: Vec<String>,
    pub(crate) protected_paths: Vec<String>,
    pub(crate) assessment: LoopExecutionAssessment,
    pub(crate) acknowledgement_required: bool,
    /// Present only when an acknowledgement can be made: audit mode with every hard constraint
    /// satisfied. Strict mode never issues one.
    pub(crate) challenge_id: Option<String>,
    pub(crate) expires_at: Option<String>,
}

/// The stored challenge/receipt row. `acknowledged_at` set means a receipt exists; `consumed_at`
/// set means it authorized exactly one committed transition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopAuditReceipt {
    pub(crate) id: String,
    pub(crate) challenge_id: String,
    pub(crate) action: LoopControlAction,
    pub(crate) target_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) scope_digest: String,
    pub(crate) assessment_digest: String,
    pub(crate) client_context: String,
    pub(crate) app_epoch: String,
    pub(crate) created_at: String,
    pub(crate) expires_at: String,
    pub(crate) acknowledged_at: Option<String>,
    pub(crate) consumed_at: Option<String>,
}

/// The parameters every control request must carry. Legacy callers that omit them cannot
/// obtain audit execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopControlEnvelope {
    pub(crate) expected_revision: Option<u64>,
    pub(crate) idempotency_key: Option<String>,
    pub(crate) audit_acknowledgement_id: Option<String>,
}

impl LoopControlEnvelope {
    pub(crate) fn legacy() -> Self {
        Self {
            expected_revision: None,
            idempotency_key: None,
            audit_acknowledgement_id: None,
        }
    }
}

/// What a receipt must match at consumption time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LoopAuditConsumption {
    pub(crate) receipt_id: String,
    pub(crate) action: LoopControlAction,
    pub(crate) target_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) scope_digest: String,
    pub(crate) assessment_digest: String,
    pub(crate) app_epoch: String,
    pub(crate) now: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contexts::agent_runtime::domain::{
        LoopDefinitionInput, LoopLimits, LoopVerificationCommand, NATIVE_CHECK_PATCH_WHITESPACE,
    };

    fn definition(
        mode: Option<LoopRequestedMode>,
        native: bool,
        version: Option<u32>,
    ) -> LoopDefinition {
        let command = if native {
            LoopVerificationCommand::new_with_kind(
                "whitespace".to_string(),
                LoopVerificationKind::NativeCheck,
                NATIVE_CHECK_PATCH_WHITESPACE.to_string(),
                Vec::new(),
                None,
                30,
                true,
            )
        } else {
            LoopVerificationCommand::new(
                "tests".to_string(),
                "npm".to_string(),
                vec!["test".to_string()],
                None,
                30,
                true,
            )
        }
        .expect("command");
        LoopDefinition::new(LoopDefinitionInput {
            id: "loop-1".to_string(),
            name: "Loop".to_string(),
            enabled: true,
            project_path: "/repo".to_string(),
            base_branch: "main".to_string(),
            goal: "goal".to_string(),
            acceptance_criteria: vec!["done".to_string()],
            allowed_paths: vec!["src".to_string()],
            protected_paths: vec!["src/generated".to_string()],
            worker_agent_id: "worker".to_string(),
            verifier_agent_id: "verifier".to_string(),
            verification_commands: vec![command],
            limits: LoopLimits::new(3, 60, 600, 2, 2).expect("limits"),
            version: 1,
            created_at: "t".to_string(),
            updated_at: "t".to_string(),
            scope_schema_version: version,
            requested_mode: mode,
        })
        .expect("definition")
    }

    fn api(agent_id: &str) -> LoopRoleAgentFacts {
        LoopRoleAgentFacts {
            agent_id: agent_id.to_string(),
            interaction_mode: InteractionMode::Api,
            trusted_api: true,
            cli: None,
        }
    }

    fn cli(agent_id: &str, acp: bool) -> LoopRoleAgentFacts {
        LoopRoleAgentFacts {
            agent_id: agent_id.to_string(),
            interaction_mode: InteractionMode::Cli,
            trusted_api: false,
            cli: Some(LoopCliWitness {
                provider_id: agent_id.to_string(),
                adapter_revision: "acp-v1".to_string(),
                executable: agent_id.to_string(),
                acp,
                mapped_permission_hook: false,
            }),
        }
    }

    fn unix() -> LoopPlatformWitness {
        LoopPlatformWitness {
            platform: "linux".to_string(),
            safe_mutation: true,
            detail: "handles".to_string(),
        }
    }

    #[test]
    fn strict_mode_is_satisfied_only_by_mediated_worker_native_check_and_read_only_verifier() {
        let assessment = assess_execution(LoopAssessmentInput {
            definition: &definition(None, true, Some(1)),
            worker: Some(api("worker")),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(
            assessment.satisfies_requested_mode,
            "{:?}",
            assessment.blockers
        );
        assert!(!assessment.acknowledgement_required);
        assert_eq!(
            assessment.coverage_for("worker"),
            Some(LoopCoverage::CompleteEnforcement)
        );
        assert_eq!(
            assessment.coverage_for("verifier"),
            Some(LoopCoverage::CompleteEnforcement)
        );

        let process = assess_execution(LoopAssessmentInput {
            definition: &definition(None, false, Some(1)),
            worker: Some(api("worker")),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(!process.satisfies_requested_mode);
        assert!(process
            .blockers
            .iter()
            .any(|blocker| blocker.contains("verification:tests")));

        let cli_worker = assess_execution(LoopAssessmentInput {
            definition: &definition(None, true, Some(1)),
            worker: Some(cli("qwen-code", true)),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert_eq!(
            cli_worker.coverage_for("worker"),
            Some(LoopCoverage::MediatedToolsOnly)
        );
        assert!(!cli_worker.satisfies_requested_mode);
    }

    #[test]
    fn audit_mode_admits_uncovered_workers_but_never_a_cli_verifier_or_unknown_identity() {
        let audited = assess_execution(LoopAssessmentInput {
            definition: &definition(Some(LoopRequestedMode::ArtifactAudited), false, Some(1)),
            worker: Some(cli("codex-cli", false)),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(audited.satisfies_requested_mode, "{:?}", audited.blockers);
        assert!(audited.acknowledgement_required);
        assert!(audited
            .limitations
            .iter()
            .any(|item| item.contains("cooperative CLI")));
        assert!(audited
            .limitations
            .iter()
            .any(|item| item.starts_with("worker:")));

        let cli_verifier = assess_execution(LoopAssessmentInput {
            definition: &definition(Some(LoopRequestedMode::ArtifactAudited), false, Some(1)),
            worker: Some(cli("codex-cli", false)),
            verifier: Some(cli("codex-cli", false)),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(!cli_verifier.satisfies_requested_mode);
        assert!(cli_verifier
            .blockers
            .iter()
            .any(|item| item.contains("verifier-not-read-only")));

        let unknown = assess_execution(LoopAssessmentInput {
            definition: &definition(Some(LoopRequestedMode::ArtifactAudited), false, Some(1)),
            worker: Some(LoopRoleAgentFacts {
                agent_id: "mystery".to_string(),
                interaction_mode: InteractionMode::Cli,
                trusted_api: false,
                cli: None,
            }),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(!unknown.satisfies_requested_mode);
    }

    #[test]
    fn legacy_definitions_and_unsupported_platforms_block_and_change_the_witness() {
        let legacy = assess_execution(LoopAssessmentInput {
            definition: &definition(None, true, None),
            worker: Some(api("worker")),
            verifier: Some(api("verifier")),
            platform: unix(),
            assessed_at: "t".to_string(),
        });
        assert!(legacy
            .blockers
            .iter()
            .any(|item| item.contains("scope-legacy-unverified")));

        let windows = assess_execution(LoopAssessmentInput {
            definition: &definition(None, true, Some(1)),
            worker: Some(api("worker")),
            verifier: Some(api("verifier")),
            platform: LoopPlatformWitness {
                platform: "windows".to_string(),
                safe_mutation: false,
                detail: "none".to_string(),
            },
            assessed_at: "t".to_string(),
        });
        assert_eq!(
            windows.coverage_for("worker"),
            Some(LoopCoverage::ArtifactValidationOnly)
        );
        assert!(!windows.satisfies_requested_mode);
        assert_eq!(
            windows.coverage_for("artifact-validation"),
            Some(LoopCoverage::Unsupported)
        );
        assert!(windows
            .blockers
            .iter()
            .any(|item| item.starts_with("artifact-validation: unsupported")));
        // Audit mode is refused on the same host: readiness and bind_root agree.
        let audited = assess_execution(LoopAssessmentInput {
            definition: &definition(Some(LoopRequestedMode::ArtifactAudited), true, Some(1)),
            worker: Some(api("worker")),
            verifier: Some(api("verifier")),
            platform: LoopPlatformWitness {
                platform: "windows".to_string(),
                safe_mutation: false,
                detail: "none".to_string(),
            },
            assessed_at: "t".to_string(),
        });
        assert!(!audited.satisfies_requested_mode);
        assert_ne!(windows.witness_digest, legacy.witness_digest);
        assert_ne!(assessment_digest(&windows), assessment_digest(&legacy));
    }
}
