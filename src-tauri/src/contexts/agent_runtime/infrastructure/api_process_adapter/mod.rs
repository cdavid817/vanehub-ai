mod code_intelligence;
mod compaction;
mod endpoint;
mod execution;
mod generation;
mod interactive;
mod invocation;
mod native_tools;
mod prompt;
mod sinks;

use super::agent_image::AgentImage;
use super::SqliteNativeToolRepository;
use crate::contexts::agent_runtime::application::{
    AgentClockPort, AgentCodeIntelligencePort, AgentCoreInstructionsPort, AgentLoggingPort,
    AgentMcpToolPort, AgentPermissionPort, AgentPersonalizationSnapshotPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRetrievalPort, AgentRuntimeApplicationError, AgentSkillPort,
    AgentWorkspaceMutationPort, ApiAgentGateway, ApiCredentialPort, ContextEngineService,
    ContextQualityRecorder, ConversationHistoryPort, GenerationProcessEvent,
    GenerationProcessFailure, GenerationProcessRequest, NativeToolRegistry, ProcessStopInitiator,
    StartedGenerationProcess, ToolApprovalDecision, ToolApprovalPort, ToolUseBlock,
    UtilityDelegationApplicationService, WorkflowLaunchOutcome, WorkflowLaunchRequest,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::skill_evolution_evidence::application::RuntimeEvidenceProjector;
use crate::contexts::tooling::skill_tools::application::{
    SkillToolCatalogPort, SkillToolExecutionPort,
};
use generation::run_generation;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

// Re-exports that keep every `api_process_adapter::…` path resolving for callers
// outside this module. Each was already `pub(crate)` before the split.
pub(crate) use generation::{
    child_reply_turns, run_child_turn, summarize_turns, GenerationOptions,
};
pub(crate) use invocation::{
    begin_child_invocation, finish_child_invocation, wire_format_for, ChildInvocationIdentity,
};

// Imports and re-exports that exist only so `tests.rs`'s `use super::*;` keeps
// resolving. They are not this module's API — no production caller reads them.
#[cfg(test)]
use super::agent_image::MAX_IMAGES_PER_REQUEST;
#[cfg(test)]
use super::model_context_catalog;
#[cfg(test)]
use super::tools::{
    execute_file_image_read, task_list_store, validate_task_list, ToolExecutionOutcome,
};
#[cfg(test)]
use crate::contexts::agent_runtime::application::{
    ask_user_question_tool_definition, plan_mode_tool_catalog, recall_tool_definition,
    tool_catalog, AgentCandidateOutcome, AgentCandidateSubmission, AgentChatConfiguration,
    AgentCodeIntelligenceContext, AgentCodeRetrievalOutcome, AgentDocumentInput,
    AgentDocumentPositionInput, AgentLog, AgentLogLevel, AgentMemory, AgentMemoryAccess,
    AgentMemoryBody, AgentMemoryDelivery, AgentMemoryPort, AgentMemoryProposal, AgentMemoryRef,
    AgentMemorySelectionPort, AgentPersonalizationSnapshot, AgentRetrievalOutcome,
    AgentSkillReadRequest, ApiProviderConfig, BoundSkillPrompt, GenerationPersonalizationContext,
    MemorySource, NativeToolResultEnvelope, NativeToolResultStatus, StoredToolOperation,
    StoredToolOperationStatus, ToolDefinition, ToolLifecyclePhase, ASK_USER_QUESTION_TOOL_NAME,
    EDIT_TOOL_NAME, EXIT_PLAN_MODE_TOOL_NAME, FILE_TOOL_NAME, FIND_DEFINITION_TOOL_NAME,
    FIND_REFERENCES_TOOL_NAME, GET_DIAGNOSTICS_TOOL_NAME, GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME,
    GREP_TOOL_NAME, IMAGE_ARTIFACT_METADATA_KEY, INTERFACE_FORMAT_OPENAI_COMPATIBLE,
    LIST_SKILLS_TOOL_NAME, LOAD_SKILL_TOOL_NAME, MAX_PLAN_CHARS, MAX_QUESTION_CHARS,
    MAX_QUESTION_OPTIONS, MAX_QUESTION_OPTION_CHARS, MCP_TOOL_NAME_PREFIX, MIN_QUESTION_OPTIONS,
    NOTEBOOK_TOOL_NAME, READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME, REMEMBER_TOOL_NAME,
    SEARCH_CODE_TOOL_NAME, SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME, SHELL_TOOL_NAME,
    TODO_WRITE_TOOL_NAME,
};
#[cfg(test)]
use crate::contexts::agent_runtime::domain::{
    AutomaticCompactionState, ContextAssessmentOutcome, ContextAssessmentPath,
    ContextAssessmentReason, ContextCompactionEvidence, ContextQualityAssessmentRecord,
};
#[cfg(test)]
use crate::contexts::permissions::domain::{Action, Effect, Resource};
#[cfg(test)]
use crate::contexts::sessions::api::UsagePurpose;
#[cfg(test)]
use crate::contexts::skill_evolution_evidence::domain::{
    ObservedSkillRevision, OperationClass, SafeCounts, SkillAssociationKind,
};
#[cfg(test)]
use crate::contexts::tooling::skill_tools::application::{
    SkillToolCatalogMode, SkillToolDispatchOutcome, SkillToolExecutionLifecyclePhase,
    SkillToolExecutionLifecyclePort, SkillToolExecutionRequest,
};
#[cfg(test)]
use crate::platform::network::blocking_http_client;
#[cfg(test)]
use compaction::{
    compaction_notice_block, compatibility_compact_accounted, maybe_compact_accounted,
    run_automatic_compaction, should_compact, turns_character_count, AutomaticCompactionOutcome,
};
#[cfg(test)]
use execution::{
    dispatch_skill_tool, emit_skill_tool_lifecycle, execute_with_code_intelligence,
    set_skill_result_summary, skill_tool_provenance, AgentSkillToolLifecycle,
};
#[cfg(test)]
use generation::{
    extract_memories_accounted, generation_options_from_configuration, is_plan_mode,
    project_native_outcomes,
};
#[cfg(test)]
use interactive::{
    ask_user_question, await_approval, permission_action_and_resource, request_plan_exit,
    validate_question_input, ApprovalOutcome,
};
#[cfg(test)]
use invocation::{
    api_invocation_snapshot, estimated_input_characters, record_accounting_diagnostic,
};
#[cfg(test)]
pub(crate) use invocation::{context_snapshot_diagnostic, WireFormat};
#[cfg(test)]
use native_tools::{
    artifact_ids, execute_registered_native_tool, execute_tool_call_impl, is_image_read_request,
    operation_event, parse_optional_non_negative_integer_arg, resolve_tool_image,
};
#[cfg(test)]
use prompt::{
    format_memory_section, format_system_prompt, resolve_system_prompt_with_settings,
    resolve_tool_catalog_with_code_intelligence,
};
#[cfg(test)]
use serde_json::{json, Value};
#[cfg(test)]
use sinks::EvidenceToolCounts;
#[cfg(test)]
use std::path::Path;
#[cfg(test)]
use std::time::Instant;

const HISTORY_LIMIT: i64 = 50;
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_TOOL_ROUND_TRIPS: u32 = 25;
const SKILL_PER_ITEM_CHARACTER_BUDGET: usize = 8_000;
const SKILL_AGGREGATE_CHARACTER_BUDGET: usize = 16_000;
const APPROVAL_POLL_INTERVAL: Duration = Duration::from_millis(200);
/// A conservative proxy for "the turns list is getting large enough to risk exceeding the
/// model's real context window" — character count, not a real token count, matching the
/// codebase's existing `source: "character-count"` approximation used for usage accounting.
/// ~60,000 characters stays comfortably under typical 128K-token context windows even at a
/// pessimistic ~1 char/token ratio, leaving headroom for system/tool-definition overhead and the
/// response's own `DEFAULT_MAX_TOKENS`.
const COMPACTION_TRIGGER_CHARACTERS: usize = 60_000;
const OPTIMIZER_TARGET_CHARACTERS: u64 = 45_000;
/// How many of the most recent turns stay untouched (verbatim) when compaction triggers;
/// everything older is replaced by one synthetic summary turn.
const COMPACTION_KEEP_RECENT_TURNS: usize = 6;
const SUMMARIZATION_INSTRUCTION: &str = "Summarize the conversation above concisely for your own future reference. Preserve key facts, decisions, and any outstanding tasks. Respond with only the summary text, no preamble.";
/// Deliberately asks for one fact per line with no numbering/bullets/preamble, since the
/// response is parsed by splitting on newlines (`extract_memories`) rather than an additional
/// structured-output round trip.
const ONEPIECE_CONFIGURATION_ERROR: &str = "OnePiece is not configured. Add or activate a provider configuration with an endpoint, model, and API key in Settings → Agent Configuration.";

type PendingApprovals = Arc<Mutex<HashMap<String, mpsc::Sender<ToolApprovalDecision>>>>;
/// A tool call's block, its output text, and whether execution failed — the shape both wire
/// formats need to build a reply turn from.
type ExecutedToolCall = (ToolUseBlock, String, bool, Option<AgentImage>);

/// `AgentProcessGateway` implementation for `launch_kind = "api"` agents: no subprocess is
/// spawned, generation is a direct streaming HTTP call to the provider's Messages API.
#[derive(Clone)]
pub(crate) struct RuntimeAgentApiAdapter {
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    history: Arc<dyn ConversationHistoryPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    skills: Arc<dyn AgentSkillPort>,
    core_instructions: Arc<dyn AgentCoreInstructionsPort>,
    mcp: Arc<dyn AgentMcpToolPort>,
    permissions: Arc<dyn AgentPermissionPort>,
    retrieval: Arc<dyn AgentRetrievalPort>,
    code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
    workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    personalization: Arc<dyn AgentPersonalizationSnapshotPort>,
    context_quality: Option<Arc<ContextQualityRecorder>>,
    context_engine: Option<Arc<ContextEngineService>>,
    accounting: Option<SessionsApi>,
    native_tools: NativeToolRegistry,
    native_tool_operations: Option<Arc<SqliteNativeToolRepository>>,
    artifacts: Option<Arc<ArtifactService>>,
    native_tool_events: Option<tauri::AppHandle>,
    generations: Arc<Mutex<HashMap<String, ManagedApiGeneration>>>,
    ids: Arc<AtomicU64>,
    evidence: RuntimeEvidenceProjector,
    utility_delegation: Option<UtilityDelegationApplicationService>,
    skill_tool_catalog: Option<Arc<dyn SkillToolCatalogPort>>,
    skill_tool_execution: Option<Arc<dyn SkillToolExecutionPort>>,
}

struct ManagedApiGeneration {
    request: GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    pending_approvals: PendingApprovals,
    /// Guards against a second `monitor_generation` spawning a duplicate
    /// `run_generation` thread for the same process — the CLI adapter mirrors this
    /// with its own `monitoring` flag. Without it, double-monitoring races two
    /// threads through the same generation and both remove the map entry.
    monitoring: bool,
}

impl RuntimeAgentApiAdapter {
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn new_without_code_intelligence(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
        history: Arc<dyn ConversationHistoryPort>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        skills: Arc<dyn AgentSkillPort>,
        core_instructions: Arc<dyn AgentCoreInstructionsPort>,
        mcp: Arc<dyn AgentMcpToolPort>,
        permissions: Arc<dyn AgentPermissionPort>,
        retrieval: Arc<dyn AgentRetrievalPort>,
        workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
        personalization: Arc<dyn AgentPersonalizationSnapshotPort>,
    ) -> Self {
        let code_intelligence = Arc::new(super::RuntimeAgentCodeIntelligenceAdapter::new(
            Arc::new(super::UnavailableAgentCodeIntelligenceResponder),
        ));
        Self::new_with_code_intelligence(
            credentials,
            config,
            history,
            logging,
            clock,
            skills,
            core_instructions,
            mcp,
            permissions,
            retrieval,
            code_intelligence,
            workspace_mutations,
            personalization,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new_with_code_intelligence(
        credentials: Arc<dyn ApiCredentialPort>,
        config: Arc<dyn ApiAgentGateway>,
        history: Arc<dyn ConversationHistoryPort>,
        logging: Arc<dyn AgentLoggingPort>,
        clock: Arc<dyn AgentClockPort>,
        skills: Arc<dyn AgentSkillPort>,
        core_instructions: Arc<dyn AgentCoreInstructionsPort>,
        mcp: Arc<dyn AgentMcpToolPort>,
        permissions: Arc<dyn AgentPermissionPort>,
        retrieval: Arc<dyn AgentRetrievalPort>,
        code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
        workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
        personalization: Arc<dyn AgentPersonalizationSnapshotPort>,
    ) -> Self {
        Self {
            credentials,
            config,
            history,
            logging,
            clock,
            skills,
            core_instructions,
            mcp,
            permissions,
            retrieval,
            code_intelligence,
            workspace_mutations,
            personalization,
            context_quality: None,
            context_engine: None,
            accounting: None,
            native_tools: NativeToolRegistry::empty(),
            native_tool_operations: None,
            artifacts: None,
            native_tool_events: None,
            generations: Arc::new(Mutex::new(HashMap::new())),
            ids: Arc::new(AtomicU64::new(0)),
            evidence: RuntimeEvidenceProjector::disabled(),
            utility_delegation: None,
            skill_tool_catalog: None,
            skill_tool_execution: None,
        }
    }

    pub(crate) fn with_evidence(mut self, evidence: RuntimeEvidenceProjector) -> Self {
        self.evidence = evidence;
        self
    }

    pub(crate) fn with_utility_delegation(
        mut self,
        service: UtilityDelegationApplicationService,
    ) -> Self {
        self.utility_delegation = Some(service);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_skill_tool_catalog(
        mut self,
        catalog: Arc<dyn SkillToolCatalogPort>,
    ) -> Self {
        self.skill_tool_catalog = Some(catalog);
        self
    }

    #[allow(dead_code)]
    pub(crate) fn with_skill_tool_execution(
        mut self,
        execution: Arc<dyn SkillToolExecutionPort>,
    ) -> Self {
        self.skill_tool_execution = Some(execution);
        self
    }

    pub(crate) fn with_accounting(mut self, accounting: SessionsApi) -> Self {
        self.accounting = Some(accounting);
        self
    }

    pub(crate) fn with_context_quality_recorder(
        mut self,
        recorder: Arc<ContextQualityRecorder>,
    ) -> Self {
        self.context_quality = Some(recorder);
        self
    }

    pub(crate) fn with_context_engine(mut self, engine: Arc<ContextEngineService>) -> Self {
        self.context_engine = Some(engine);
        self
    }

    pub(crate) fn with_native_tool_registry(mut self, native_tools: NativeToolRegistry) -> Self {
        self.native_tools = native_tools;
        self
    }

    /// Supplies the Artifact store the tool loop reads an image from when a native tool names one
    /// in its result metadata (`add-onepiece-visual-tool-returns`).
    pub(crate) fn with_artifacts(mut self, artifacts: Arc<ArtifactService>) -> Self {
        self.artifacts = Some(artifacts);
        self
    }

    pub(crate) fn with_native_tool_operations(
        mut self,
        repository: Arc<SqliteNativeToolRepository>,
        app: tauri::AppHandle,
    ) -> Self {
        self.native_tool_operations = Some(repository);
        self.native_tool_events = Some(app);
        self
    }
}

impl AgentProcessGateway for RuntimeAgentApiAdapter {
    fn launch_workflow(
        &self,
        _request: WorkflowLaunchRequest,
    ) -> Result<WorkflowLaunchOutcome, AgentRuntimeApplicationError> {
        Ok(WorkflowLaunchOutcome {
            adapter: "api".to_string(),
            message: "API-based agent workflow ready.".to_string(),
        })
    }

    fn start_generation(
        &self,
        request: GenerationProcessRequest,
    ) -> Result<StartedGenerationProcess, AgentRuntimeApplicationError> {
        if request.runner.kind != crate::contexts::agent_runtime::application::RunnerKind::Local {
            return Err(AgentRuntimeApplicationError::Process(
                "runner_unsupported_capability".to_string(),
            ));
        }
        if request.agent.launch.kind != "api" {
            return Err(AgentRuntimeApplicationError::Process(format!(
                "{} launch kind '{}' is unsupported for the API runtime.",
                request.agent.display_name, request.agent.launch.kind
            )));
        }
        let process_id = format!(
            "agent-api-process-{}",
            self.ids.fetch_add(1, Ordering::Relaxed) + 1
        );
        let mut generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        generations.insert(
            process_id.clone(),
            ManagedApiGeneration {
                request,
                cancelled: Arc::new(AtomicBool::new(false)),
                pending_approvals: Arc::new(Mutex::new(HashMap::new())),
                monitoring: false,
            },
        );
        Ok(StartedGenerationProcess {
            process_id,
            runner_reference: crate::contexts::agent_runtime::application::RunnerReference::local(),
            process_reference: None,
        })
    }

    fn monitor_generation(
        &self,
        process_id: &str,
        sink: Arc<dyn AgentProcessEventSink>,
    ) -> Result<(), AgentRuntimeApplicationError> {
        let (request, cancelled, pending_approvals) = {
            let mut generations = self
                .generations
                .lock()
                .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
            let managed = generations.get_mut(process_id).ok_or_else(|| {
                AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is not active."
                ))
            })?;
            if managed.monitoring {
                return Err(AgentRuntimeApplicationError::Process(format!(
                    "Agent process {process_id} is already monitored."
                )));
            }
            managed.monitoring = true;
            (
                managed.request.clone(),
                managed.cancelled.clone(),
                managed.pending_approvals.clone(),
            )
        };
        let generations = self.generations.clone();
        let process_id = process_id.to_string();
        let credentials = self.credentials.clone();
        let config = self.config.clone();
        let history = self.history.clone();
        let logging = self.logging.clone();
        let clock = self.clock.clone();
        let skills = self.skills.clone();
        let core_instructions = self.core_instructions.clone();
        let mcp = self.mcp.clone();
        let permissions = self.permissions.clone();
        let retrieval = self.retrieval.clone();
        let code_intelligence = self.code_intelligence.clone();
        let workspace_mutations = self.workspace_mutations.clone();
        let personalization = self.personalization.clone();
        let context_quality = self.context_quality.clone();
        let context_engine = self.context_engine.clone();
        let evidence = self.evidence.clone();
        let utility_delegation = self.utility_delegation.clone();
        let skill_tool_catalog = self.skill_tool_catalog.clone();
        let skill_tool_execution = self.skill_tool_execution.clone();
        let accounting = self.accounting.clone();
        let native_tools = self.native_tools.clone();
        let native_tool_operations = self.native_tool_operations.clone();
        let artifacts = self.artifacts.clone();
        let native_tool_events = self.native_tool_events.clone();
        thread::spawn(move || {
            run_generation(
                request,
                cancelled,
                credentials,
                config,
                history,
                logging,
                clock,
                skills,
                core_instructions,
                mcp,
                permissions,
                retrieval,
                code_intelligence,
                workspace_mutations,
                personalization,
                context_quality,
                context_engine,
                accounting,
                native_tools,
                native_tool_operations,
                artifacts,
                native_tool_events,
                sink,
                pending_approvals,
                evidence,
                utility_delegation,
                skill_tool_catalog,
                skill_tool_execution,
            );
            if let Ok(mut generations) = generations.lock() {
                generations.remove(&process_id);
            }
        });
        Ok(())
    }

    fn stop_generation(
        &self,
        process_id: &str,
        _initiator: ProcessStopInitiator,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        match generations.get(process_id) {
            Some(managed) => {
                managed.cancelled.store(true, Ordering::SeqCst);
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

impl ToolApprovalPort for RuntimeAgentApiAdapter {
    fn resolve(
        &self,
        process_id: &str,
        call_id: &str,
        decision: ToolApprovalDecision,
    ) -> Result<bool, AgentRuntimeApplicationError> {
        let generations = self
            .generations
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        let Some(managed) = generations.get(process_id) else {
            return Ok(false);
        };
        let mut pending = managed
            .pending_approvals
            .lock()
            .map_err(|error| AgentRuntimeApplicationError::Process(error.to_string()))?;
        match pending.remove(call_id) {
            Some(sender) => {
                let _ = sender.send(decision);
                Ok(true)
            }
            None => Ok(false),
        }
    }
}

fn failed_non_retryable(message: &str) -> GenerationProcessEvent {
    GenerationProcessEvent::Failed(GenerationProcessFailure::non_retryable(message.to_string()))
}

fn failed_configuration(agent_id: &str, diagnostic: &str) -> GenerationProcessEvent {
    let failure = GenerationProcessFailure::non_retryable(diagnostic.to_string());
    let failure = if agent_id == "onepiece" {
        failure.with_safe_error(ONEPIECE_CONFIGURATION_ERROR)
    } else {
        failure
    };
    GenerationProcessEvent::Failed(failure)
}

fn failed_retryable(message: &str) -> GenerationProcessEvent {
    GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(message.to_string()))
}

#[cfg(test)]
mod tests;
