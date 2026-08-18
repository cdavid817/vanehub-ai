mod invocation;
mod sinks;

use super::agent_image::{prepare_image, AgentImage, MAX_IMAGES_PER_REQUEST};
use super::code_intelligence_tool_output::{diagnostics_outcome, hover_outcome, locations_outcome};
use super::context_projection::ContextWireShape;
use super::context_reduction::{build_structured_summary_turns, reconstruct_candidate};
use super::memory_actions::{apply_memory_actions, render_existing_manifest};
use super::memory_directory::is_within_memory_directory;
use super::memory_selection_gateway::RuntimeAgentMemorySelectionAdapter;
use super::memory_surfaced::{mark_surfaced, unsurfaced_candidates};
use super::model_context_catalog;
use super::skill_tool_catalog_adapter::resolve_skill_tool_catalog;
use super::tool_call_accumulator::ToolCallAccumulator;
use super::tools::{
    background_shell_registry, execute_edit, execute_file, execute_file_image_read, execute_glob,
    execute_grep, execute_notebook, execute_shell, is_reviewed_image_path, render_task_list,
    task_list_prompt_section, task_list_store, validate_task_list, BackgroundStartError,
    GrepRequest, KillOutcome, NotebookRequest, ToolExecutionOutcome,
    MAX_BACKGROUND_COMMANDS_PER_SESSION, OUTPUT_MODE_FILES,
};
use super::SqliteNativeToolRepository;
use crate::contexts::agent_runtime::application::{
    ask_user_question_tool_definition, code_intelligence_tool_definitions,
    delegate_utility_skill_tool_definition, plan_mode_tool_catalog, recall_tool_definition,
    search_code_tool_definition, tool_catalog, AgentChatConfiguration, AgentClockPort,
    AgentCodeIntelligenceContext, AgentCodeIntelligencePort, AgentCodeRetrievalOutcome,
    AgentCoreInstructionsPort, AgentDocumentInput, AgentDocumentPositionInput, AgentLog,
    AgentLogLevel, AgentLoggingPort, AgentMcpToolPort, AgentMemory, AgentMemoryPort,
    AgentMemorySelectionPort, AgentPermissionPort, AgentPersonalizationPort, AgentProcessEventSink,
    AgentProcessGateway, AgentRetrievalOutcome, AgentRetrievalPort, AgentRuntimeApplicationError,
    AgentSkillPort, AgentSkillReadRequest, AgentWorkspaceMutation, AgentWorkspaceMutationPort,
    ApiAgentGateway, ApiCredentialPort, ApiProviderConfig, BoundSkillPrompt, ContextAnalysisInput,
    ContextAnalysisService, ContextEngineOutcome, ContextEngineService, ContextQualityRecorder,
    ConversationHistoryPort, ExistingToolHandler, ExistingToolHandlerRegistry,
    GenerationProcessEvent, GenerationProcessFailure, GenerationProcessRequest, MemorySource,
    NativeToolAuthorizationStatus, NativeToolDispatchRequest, NativeToolDispatcher,
    NativeToolExecutionContext, NativeToolExecutionMode, NativeToolProgress,
    NativeToolProgressPhase, NativeToolProgressSink, NativeToolRegistry, NativeToolResultEnvelope,
    NativeToolResultStatus, PersonalizationSettings, ProcessStopInitiator, ReportedUsageTotals,
    SaveMemoryInput, SkillToolUseProvenance, StartedGenerationProcess, StoredToolOperation,
    StoredToolOperationStatus, ToolApprovalDecision, ToolApprovalPort, ToolDefinition,
    ToolEligibilityContext, ToolLifecycleEvent, ToolLifecyclePhase, ToolUseBlock,
    UtilityDelegationApplicationService, WorkflowLaunchOutcome, WorkflowLaunchRequest,
    ASK_USER_QUESTION_TOOL_NAME, DELEGATE_UTILITY_SKILL_TOOL_NAME, EDIT_TOOL_NAME,
    EXIT_PLAN_MODE_TOOL_NAME, FILE_TOOL_NAME, FIND_DEFINITION_TOOL_NAME, FIND_REFERENCES_TOOL_NAME,
    GET_DIAGNOSTICS_TOOL_NAME, GET_HOVER_TOOL_NAME, GLOB_TOOL_NAME, GREP_TOOL_NAME,
    IMAGE_ARTIFACT_METADATA_KEY, INTERFACE_FORMAT_OPENAI_COMPATIBLE, LIST_SKILLS_TOOL_NAME,
    LOAD_SKILL_TOOL_NAME, MAX_PLAN_CHARS, MAX_QUESTION_CHARS, MAX_QUESTION_OPTIONS,
    MAX_QUESTION_OPTION_CHARS, MCP_TOOL_NAME_PREFIX, MIN_QUESTION_OPTIONS, NOTEBOOK_TOOL_NAME,
    READ_SKILL_RESOURCE_TOOL_NAME, RECALL_TOOL_NAME, REMEMBER_TOOL_NAME, SEARCH_CODE_TOOL_NAME,
    SHELL_KILL_TOOL_NAME, SHELL_OUTPUT_TOOL_NAME, SHELL_TOOL_NAME, TODO_WRITE_TOOL_NAME,
};
use crate::contexts::agent_runtime::domain::{
    build_optimization_plan, parse_memory_actions, select_authoritative_compaction,
    verify_optimization_candidate, AutomaticCompactionState, CompactionBypassReason,
    CompactionPath, CompactionTriggerSource, ContextAssessmentInvariants, ContextAssessmentOutcome,
    ContextAssessmentPath, ContextAssessmentReason, ContextAssessmentTriggerSource, ContextBudget,
    ContextCapacity, ContextCompactionEvidence, ContextOptimizationBudget,
    ContextQualityAssessment, ContextQualityAssessmentInput, ContextQualityAssessmentRecord,
    ContextRequest, ContextSnapshot, FallbackReason, MemoryType, OptimizationActionKind,
    OptimizationOutcome, RetentionClass, UsageAnchor, UtilityDelegationLimits,
    UtilityDelegationRequest, AUTOMATIC_COMPACTION_POLICY_VERSION, CONTEXT_OPTIMIZER_VERSION,
    CONTEXT_QUALITY_HISTORY_HARD_LIMIT, CONTEXT_VERIFIER_VERSION, MEMORY_ACTIONS_INSTRUCTION,
    STRUCTURED_SUMMARY_PROMPT,
};
use crate::contexts::artifacts::application::ArtifactService;
use crate::contexts::permissions::domain::{Action, Effect, Resource};
use crate::contexts::sessions::api::{SessionsApi, UsagePurpose, UsageStatus};
use crate::contexts::skill_evolution_evidence::application::{
    NativeExecutionFact, RuntimeEvidenceProjector,
};
use crate::contexts::skill_evolution_evidence::domain::{
    EnvelopeCommon, FailureClass, ObservedSkillRevision, OperationClass, SafeCounts,
    SkillAssociationKind, SourceFidelity, TerminalOutcome,
};
use crate::contexts::tooling::skill_tools::application::{
    SkillToolBinding, SkillToolCatalogContext, SkillToolCatalogMode, SkillToolCatalogPort,
    SkillToolDispatchOutcome, SkillToolExecutionLifecyclePhase, SkillToolExecutionLifecyclePort,
    SkillToolExecutionPort, SkillToolExecutionRequest,
};
use crate::platform::filesystem::BoundedFilesystem;
use crate::platform::network::blocking_http_client;
use invocation::{
    begin_api_invocation, estimated_input_characters, finish_api_invocation,
    record_context_snapshot,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sinks::{EvidenceCountingSink, EvidenceToolCounts};
use std::collections::HashMap;
use std::io::BufRead;
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::Instant;
use tauri::Emitter;

// Re-exports that keep every `api_process_adapter::…` path resolving for callers
// outside this module. Each was already `pub(crate)` before the split.
pub(crate) use invocation::{
    begin_child_invocation, finish_child_invocation, wire_format_for, ChildInvocationIdentity,
    WireFormat,
};

// Imports and re-exports that exist only so `tests.rs`'s `use super::*;` keeps
// resolving. They are not this module's API — no production caller reads them.
#[cfg(test)]
pub(crate) use invocation::context_snapshot_diagnostic;
#[cfg(test)]
use invocation::{api_invocation_snapshot, record_accounting_diagnostic};

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
    memories: Arc<dyn AgentMemoryPort>,
    mcp: Arc<dyn AgentMcpToolPort>,
    permissions: Arc<dyn AgentPermissionPort>,
    retrieval: Arc<dyn AgentRetrievalPort>,
    code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
    workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    personalization: Arc<dyn AgentPersonalizationPort>,
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
        memories: Arc<dyn AgentMemoryPort>,
        mcp: Arc<dyn AgentMcpToolPort>,
        permissions: Arc<dyn AgentPermissionPort>,
        retrieval: Arc<dyn AgentRetrievalPort>,
        workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
        personalization: Arc<dyn AgentPersonalizationPort>,
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
            memories,
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
        memories: Arc<dyn AgentMemoryPort>,
        mcp: Arc<dyn AgentMcpToolPort>,
        permissions: Arc<dyn AgentPermissionPort>,
        retrieval: Arc<dyn AgentRetrievalPort>,
        code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
        workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
        personalization: Arc<dyn AgentPersonalizationPort>,
    ) -> Self {
        Self {
            credentials,
            config,
            history,
            logging,
            clock,
            skills,
            core_instructions,
            memories,
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
        let memories = self.memories.clone();
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
                memories,
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

#[allow(clippy::too_many_arguments)]
fn run_generation(
    mut request: GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    credentials: Arc<dyn ApiCredentialPort>,
    config: Arc<dyn ApiAgentGateway>,
    history: Arc<dyn ConversationHistoryPort>,
    logging: Arc<dyn AgentLoggingPort>,
    clock: Arc<dyn AgentClockPort>,
    skills: Arc<dyn AgentSkillPort>,
    core_instructions: Arc<dyn AgentCoreInstructionsPort>,
    memories: Arc<dyn AgentMemoryPort>,
    mcp: Arc<dyn AgentMcpToolPort>,
    permissions: Arc<dyn AgentPermissionPort>,
    retrieval: Arc<dyn AgentRetrievalPort>,
    code_intelligence: Arc<dyn AgentCodeIntelligencePort>,
    workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    personalization: Arc<dyn AgentPersonalizationPort>,
    context_quality: Option<Arc<ContextQualityRecorder>>,
    context_engine: Option<Arc<ContextEngineService>>,
    accounting: Option<SessionsApi>,
    native_tools: NativeToolRegistry,
    native_tool_operations: Option<Arc<SqliteNativeToolRepository>>,
    artifacts: Option<Arc<ArtifactService>>,
    native_tool_events: Option<tauri::AppHandle>,
    sink: Arc<dyn AgentProcessEventSink>,
    pending_approvals: PendingApprovals,
    evidence: RuntimeEvidenceProjector,
    utility_delegation: Option<UtilityDelegationApplicationService>,
    skill_tool_catalog: Option<Arc<dyn SkillToolCatalogPort>>,
    skill_tool_execution: Option<Arc<dyn SkillToolExecutionPort>>,
) {
    if request.agent.id == "onepiece" {
        if let Some(engine) = context_engine {
            let context_request = ContextRequest {
                session_id: request.session.id.clone(),
                turn_id: request.message_id.clone(),
                generation_id: request.operation_id.clone(),
                task: request.effective_prompt.clone(),
                workspace_ref: request.session.folder.clone(),
                explicit_refs: request
                    .file_references
                    .iter()
                    .map(|reference| reference.path.clone())
                    .collect(),
                model_capacity: Some(32_768),
            };
            let budget = ContextBudget {
                total: 32_768,
                reserved_system: 8_192,
                reserved_task: 4_096,
                reserved_recent_turns: 12_288,
                reserve: 2_048,
            };
            if let ContextEngineOutcome::Ready(projected) =
                engine.assemble(&context_request, &budget, cancelled.as_ref())
            {
                if !projected.provider_projection.is_empty() {
                    request
                        .effective_prompt
                        .push_str("\n\n<context-evidence>\n");
                    request
                        .effective_prompt
                        .push_str(&projected.provider_projection);
                    request.effective_prompt.push_str("\n</context-evidence>");
                }
            }
        }
    }
    let mut observed_skill_revisions = Vec::new();
    let counting_sink = EvidenceCountingSink::new(sink.clone());
    let terminal = execute_with_code_intelligence(
        &request,
        cancelled,
        credentials.as_ref(),
        config.as_ref(),
        history.as_ref(),
        &counting_sink,
        &pending_approvals,
        logging.as_ref(),
        clock.as_ref(),
        skills.as_ref(),
        core_instructions.as_ref(),
        memories.as_ref(),
        mcp.as_ref(),
        permissions.as_ref(),
        retrieval.as_ref(),
        code_intelligence.as_ref(),
        workspace_mutations.as_ref(),
        personalization.as_ref(),
        context_quality.as_deref(),
        utility_delegation.as_ref(),
        skill_tool_catalog.as_deref(),
        skill_tool_execution.as_deref(),
        &mut observed_skill_revisions,
        accounting.as_ref(),
        &native_tools,
        native_tool_operations.as_deref(),
        artifacts.as_deref(),
        native_tool_events.as_ref(),
    );
    project_native_outcomes(
        &evidence,
        &request,
        &terminal,
        observed_skill_revisions,
        counting_sink.counts(),
        clock.now(),
    );
    if let GenerationProcessEvent::Failed(failure) = &terminal {
        let _ = logging.record(AgentLog {
            level: AgentLogLevel::Error,
            category: "session.runtime.api".to_string(),
            message: failure.diagnostic.clone(),
            agent_id: Some(request.agent.id.clone()),
            session_id: Some(request.session.id.clone()),
            operation_id: Some(request.operation_id.clone()),
            run_id: None,
            trace_id: None,
            span_id: None,
            occurred_at: clock.now(),
        });
    }
    let _ = sink.handle(terminal);
}

fn project_native_outcomes(
    evidence: &RuntimeEvidenceProjector,
    request: &GenerationProcessRequest,
    terminal: &GenerationProcessEvent,
    observed_skill_revisions: Vec<ObservedSkillRevision>,
    tools: EvidenceToolCounts,
    occurred_at: String,
) {
    let (outcome, failure_class) = match terminal {
        GenerationProcessEvent::Completed(_) => (TerminalOutcome::Succeeded, None),
        GenerationProcessEvent::Failed(_) => (TerminalOutcome::Failed, Some(FailureClass::Agent)),
        _ => (TerminalOutcome::Incomplete, Some(FailureClass::Agent)),
    };
    let common = EnvelopeCommon {
        source_event_id: format!(
            "native:{}:generation",
            request.execution_context.run_id.as_str()
        ),
        occurred_at: occurred_at.clone(),
        stable_agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        message_id: Some(request.message_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        attempt_id: Some(request.operation_id.clone()),
        workspace: evidence.workspace_scope(request.session.folder.as_deref()),
        fidelity: SourceFidelity::Native,
        observed_skill_revisions,
    };
    let _ = evidence.native(NativeExecutionFact {
        common: common.clone(),
        operation_class: OperationClass::Generation,
        outcome,
        failure_class,
        safe_counts: SafeCounts {
            attempts: 1,
            failures: u32::from(outcome == TerminalOutcome::Failed),
        },
    });
    if tools.attempts > 0 {
        let mut tool_common = common;
        tool_common.source_event_id =
            format!("native:{}:tools", request.execution_context.run_id.as_str());
        let _ = evidence.native(NativeExecutionFact {
            common: tool_common,
            operation_class: OperationClass::Tool,
            outcome: if tools.failures == 0 {
                TerminalOutcome::Succeeded
            } else {
                TerminalOutcome::Failed
            },
            failure_class: (tools.failures > 0).then_some(FailureClass::Tool),
            safe_counts: SafeCounts {
                attempts: tools.attempts,
                failures: tools.failures,
            },
        });
    }
}

/// Provider-agnostic knobs from `AgentChatConfiguration` that map onto a single generation
/// request (`add-agent-chat-configuration`). Each provider's `build_request_body` reads only the
/// field(s) meaningful to its own wire format — mirrors how `WireFormat`'s other function
/// pointers already share one signature across providers with different per-provider bodies.
pub(crate) struct GenerationOptions<'a> {
    pub(crate) thinking: bool,
    pub(crate) reasoning_depth: Option<&'a str>,
    pub(crate) include_stream_usage: bool,
}

impl GenerationOptions<'_> {
    /// Used for requests that are not the user-facing turn (context compaction's own internal
    /// summarization call) — never inherits the user's turn-level settings.
    pub(crate) fn disabled() -> GenerationOptions<'static> {
        GenerationOptions {
            thinking: false,
            reasoning_depth: None,
            include_stream_usage: false,
        }
    }
}

fn generation_options_from_configuration(
    configuration: &AgentChatConfiguration,
    include_stream_usage: bool,
) -> GenerationOptions<'_> {
    GenerationOptions {
        thinking: configuration.thinking,
        reasoning_depth: configuration.reasoning_depth.as_deref(),
        include_stream_usage,
    }
}

/// Whether the session narrows the durable Agent policy to read-only planning behavior.
fn is_plan_mode(configuration: &AgentChatConfiguration) -> bool {
    configuration.execution_mode == "plan"
}

fn reviewed_stream_usage_strategy(config: &ApiProviderConfig) -> bool {
    if config.interface_format != INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        return false;
    }
    config.base_url.as_deref().is_some_and(|base_url| {
        [
            "https://api.openai.com/",
            "https://openrouter.ai/",
            "https://api.deepseek.com/",
        ]
        .iter()
        .any(|prefix| base_url.starts_with(prefix))
    })
}

#[allow(clippy::too_many_arguments)]
fn execute_with_code_intelligence(
    request: &GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    credentials: &dyn ApiCredentialPort,
    config: &dyn ApiAgentGateway,
    history: &dyn ConversationHistoryPort,
    sink: &dyn AgentProcessEventSink,
    pending_approvals: &PendingApprovals,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    skills: &dyn AgentSkillPort,
    core_instructions: &dyn AgentCoreInstructionsPort,
    memories: &dyn AgentMemoryPort,
    mcp: &dyn AgentMcpToolPort,
    permissions: &dyn AgentPermissionPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: &dyn AgentCodeIntelligencePort,
    workspace_mutations: &dyn AgentWorkspaceMutationPort,
    personalization: &dyn AgentPersonalizationPort,
    context_quality: Option<&ContextQualityRecorder>,
    utility_delegation: Option<&UtilityDelegationApplicationService>,
    skill_tool_catalog: Option<&dyn SkillToolCatalogPort>,
    skill_tool_execution: Option<&dyn SkillToolExecutionPort>,
    observed_skill_revisions: &mut Vec<ObservedSkillRevision>,
    accounting: Option<&SessionsApi>,
    native_tools: &NativeToolRegistry,
    native_tool_operations: Option<&SqliteNativeToolRepository>,
    artifacts: Option<&ArtifactService>,
    native_tool_events: Option<&tauri::AppHandle>,
) -> GenerationProcessEvent {
    let agent_id = request.agent.id.as_str();
    let provider_config = if let Some(profile) = request.endpoint_profile.as_ref() {
        ApiProviderConfig {
            source_provider_id: profile.source_provider_id.clone(),
            model_id: profile.model_id.clone(),
            interface_format: profile.interface_format.clone(),
            base_url: profile.base_url.clone(),
            auto_approve_tools: false,
        }
    } else {
        match config.provider_config(agent_id) {
            Ok(Some(config)) => config,
            Ok(None) => {
                return failed_configuration(agent_id, "No model is configured for this agent.");
            }
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let endpoint_metadata = if request.endpoint_profile.is_some() {
        None
    } else {
        match config.active_endpoint_profile_metadata(agent_id) {
            Ok(metadata) => metadata,
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let endpoint_capacity = request
        .endpoint_profile
        .as_ref()
        .and_then(|profile| {
            let window = profile.context_window_tokens?;
            (profile.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                context_window_tokens: window,
                maximum_output_tokens: Some(profile.reserved_output_tokens),
                metadata_revision: profile.context_capacity_provenance.clone(),
                source_identity: format!("endpoint-profile:{}", profile.profile_id),
            })
        })
        .or_else(|| {
            endpoint_metadata.as_ref().and_then(|metadata| {
                let window = metadata
                    .context_window_tokens
                    .and_then(|value| u64::try_from(value).ok())?;
                (metadata.context_capacity_provenance != "unknown").then(|| ContextCapacity {
                    context_window_tokens: window,
                    maximum_output_tokens: u64::try_from(metadata.reserved_output_tokens).ok(),
                    metadata_revision: metadata.context_capacity_provenance.clone(),
                    source_identity: format!("endpoint-profile:{}", metadata.profile_id),
                })
            })
        });
    let authentication_mode = if let Some(profile) = request.endpoint_profile.as_ref() {
        profile.authentication_mode.clone()
    } else {
        match config.api_endpoint_authentication_mode(agent_id) {
            Ok(mode) => mode,
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let credential_id = request.endpoint_profile.as_ref().map_or_else(
        || agent_id.to_string(),
        |profile| format!("onepiece-profile:{}", profile.profile_id),
    );
    let fetched_credential = if authentication_mode == "required" {
        match credentials.fetch(&credential_id) {
            Ok(Some(key)) => Ok(Some(key)),
            Ok(None) if request.endpoint_profile.is_some() => credentials.fetch(agent_id),
            other => other,
        }
    } else {
        Ok(None)
    };
    let api_key = match fetched_credential {
        Ok(Some(key)) => key,
        Ok(None) if authentication_mode != "required" => String::new(),
        Ok(None) => {
            return failed_configuration(agent_id, "No API key is stored for this agent.");
        }
        Err(error) => return failed_non_retryable(&error.to_string()),
    };
    let wire_format = match wire_format_for(&provider_config) {
        Ok(wire_format) => wire_format,
        Err(message) => return failed_configuration(agent_id, message),
    };
    let generation_personalization =
        resolve_personalization_settings(personalization, logging, clock, request);
    // Built here, and the prompt resolved once, before the round-trip loop below. That is what
    // makes the system prompt byte-identical across every round trip of this generation.
    let selection = RuntimeAgentMemorySelectionAdapter::new(credentials, config);
    let system = resolve_system_prompt_with_settings(
        agent_id,
        core_instructions,
        &generation_personalization,
        skills,
        memories,
        &selection,
        logging,
        clock,
        request,
        observed_skill_revisions,
    );
    let recent = match history.recent_messages(&request.session.id, HISTORY_LIMIT) {
        Ok(messages) => messages,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    // Signal for the tool-assisted memory-extraction gate (`add-personalization-settings` design.md
    // D5) — seeded from the persisted message history, not from wire-format `turns`, so it needs no
    // per-provider parsing and no index-alignment with whatever `maybe_compact` later slices off
    // `turns`. Mutable: this generation's own tool round trips (below) can still flip it from
    // `false` to `true` before the in-loop `maybe_compact` call — seeding it from `recent` alone
    // would miss a session's very first tool call if compaction also triggers within that same
    // generation.
    let mut tool_assisted_session = recent.iter().any(|message| !message.tool_use.is_empty());
    let request_timeout = if let Some(profile) = request.endpoint_profile.as_ref() {
        Duration::from_millis(profile.timeout_ms)
    } else {
        match config.api_endpoint_timeout_ms(agent_id) {
            Ok(timeout_ms) => Duration::from_millis(timeout_ms),
            Err(error) => return failed_non_retryable(&error.to_string()),
        }
    };
    let client = match blocking_http_client(request_timeout) {
        Ok(client) => client,
        Err(error) => {
            return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                error.to_string(),
            ))
        }
    };
    let plan_mode = is_plan_mode(&request.configuration);
    // Never blocks, never errors (`AgentRetrievalPort::is_configured`'s own contract) — safe to
    // call unconditionally on every generation's catalog resolution, matching how `plan_mode`
    // itself is derived just above.
    let retrieval_available = retrieval.is_configured();
    let code_search_available = request
        .session
        .folder
        .as_deref()
        .and_then(|folder| {
            retrieval
                .code_retrieval()
                .map(|code| code.is_available(folder))
        })
        .unwrap_or(false);
    let code_intelligence_context = request
        .session
        .folder
        .as_deref()
        .map(AgentCodeIntelligenceContext::from_session_workspace);
    let code_intelligence_available = code_intelligence_context
        .as_ref()
        .is_some_and(|context| code_intelligence.is_available(context));
    let mut tools = resolve_tool_catalog_with_code_intelligence(
        request,
        mcp,
        logging,
        clock,
        plan_mode,
        retrieval_available,
        code_search_available,
        code_intelligence_available,
    );
    if utility_delegation.is_some() && !plan_mode {
        tools.push(delegate_utility_skill_tool_definition());
    }
    tools.extend(
        native_tools.eligible_tool_definitions(&ToolEligibilityContext {
            agent_id: request.agent.id.clone(),
            session_id: request.session.id.clone(),
            generation_id: request.operation_id.clone(),
            canonical_workspace: request.session.folder.as_deref().map(Into::into),
            execution_mode: if plan_mode {
                NativeToolExecutionMode::Plan
            } else {
                NativeToolExecutionMode::Execute
            },
            readiness: native_tools.readiness_snapshot(),
        }),
    );
    if request
        .endpoint_profile
        .as_ref()
        .is_some_and(|profile| profile.tool_calling_capability != "supported")
    {
        tools.clear();
    }
    let mut skill_tool_keys = HashMap::new();
    let mut _skill_tool_catalog_lease = None;
    let mut _skill_tool_catalog_generation = None;
    if let Some(catalog) = skill_tool_catalog {
        let loaded_roles = observed_skill_revisions
            .iter()
            .map(|observed| SkillToolBinding {
                skill_id: observed.skill_id.clone(),
                revision: observed.revision.clone(),
            })
            .collect::<Vec<_>>();
        let context = SkillToolCatalogContext::RoleGeneration {
            workspace_path: request.session.folder.clone(),
            loaded_roles,
            mode: if plan_mode {
                SkillToolCatalogMode::Plan
            } else {
                SkillToolCatalogMode::Execute
            },
        };
        let existing_names = tools.iter().map(|tool| tool.name.clone());
        match resolve_skill_tool_catalog(
            catalog,
            &context,
            existing_names,
            &provider_config.interface_format,
        ) {
            Ok(resolved) => {
                tools.extend(resolved.definitions);
                skill_tool_keys = resolved.keys_by_name;
                _skill_tool_catalog_generation = Some(resolved.generation);
                _skill_tool_catalog_lease = Some(resolved.lease);
            }
            Err(error) => {
                let _ = logging.record(AgentLog {
                    level: AgentLogLevel::Warn,
                    category: "session.runtime.api.skill-tools".to_string(),
                    message: format!("Skill tool catalog rejected: {}", error.code()),
                    agent_id: Some(request.agent.id.clone()),
                    session_id: Some(request.session.id.clone()),
                    operation_id: Some(request.operation_id.clone()),
                    run_id: None,
                    trace_id: None,
                    span_id: None,
                    occurred_at: clock.now(),
                });
            }
        }
    }
    let mut generation_options = generation_options_from_configuration(
        &request.configuration,
        reviewed_stream_usage_strategy(&provider_config),
    );
    if request
        .endpoint_profile
        .as_ref()
        .is_some_and(|profile| profile.reasoning_field_capability != "supported")
    {
        generation_options.thinking = false;
        generation_options.reasoning_depth = None;
    }
    let mut turns = (wire_format.history_to_turns)(&recent);
    let mut request_sequence = 0u32;
    let mut context_usage_anchor: Option<UsageAnchor> = None;
    let mut automatic_compaction_state = AutomaticCompactionState::with_user_preference(
        generation_personalization.automatic_context_compaction_enabled,
    );
    if let Some(failure) = maybe_compact_accounted(
        &mut turns,
        &wire_format,
        &client,
        &api_key,
        &provider_config.model_id,
        &provider_config,
        &tools,
        &generation_options,
        system.as_deref(),
        &cancelled,
        sink,
        logging,
        clock,
        request,
        memories,
        personalization,
        tool_assisted_session,
        accounting,
        &mut request_sequence,
        context_usage_anchor.as_ref(),
        &mut automatic_compaction_state,
        context_quality,
        generation_personalization.context_quality_retention_days,
    ) {
        return failure;
    }

    let mut emitted_visible_content = false;
    // Capability comes from reviewed catalog metadata, never from trying and seeing: a provider
    // that rejects an image-bearing request fails the whole generation after the user has already
    // waited, and the failure text varies by vendor (`add-agent-image-input` D3).
    let images_supported = request.endpoint_profile.as_ref().map_or_else(
        || {
            endpoint_metadata.as_ref().map_or_else(
                || {
                    model_context_catalog::accepts_image_input(
                        provider_config.source_provider_id.as_deref(),
                        &provider_config.model_id,
                    )
                },
                |metadata| metadata.image_input_capability == "supported",
            )
        },
        |profile| profile.image_input_capability == "supported",
    );
    let mut images_in_request = 0_usize;
    let mut context_recovery_attempted = false;
    for round_trip in 0..MAX_TOOL_ROUND_TRIPS {
        if cancelled.load(Ordering::SeqCst) {
            return failed_non_retryable("Generation was cancelled.");
        }
        let sequence = request_sequence;
        request_sequence = request_sequence.saturating_add(1);
        let purpose = if round_trip == 0 {
            UsagePurpose::AssistantInitial
        } else {
            UsagePurpose::ToolContinuation
        };
        let invocation = begin_api_invocation(
            accounting,
            request,
            &provider_config,
            sequence,
            purpose,
            clock,
            logging,
        );
        let body = (wire_format.build_request_body)(
            &provider_config.model_id,
            &turns,
            &tools,
            system.as_deref(),
            &generation_options,
        );
        let projection = (wire_format.project_request_context)(&body);
        let mut context_snapshot = ContextAnalysisService::analyze(
            ContextAnalysisInput {
                provider_id: provider_config.source_provider_id.clone(),
                model_id: provider_config.model_id.clone(),
                request_fingerprint: projection.request_fingerprint,
                characters: projection.characters,
                components: projection.components,
                rounds: projection.rounds,
                token_estimate_complete: projection.token_estimate_complete,
                capacity: endpoint_capacity.clone().or_else(|| {
                    (endpoint_metadata.is_none() && request.endpoint_profile.is_none()).then(
                        || {
                            model_context_catalog::resolve_capacity(
                                provider_config.source_provider_id.as_deref(),
                                &provider_config.model_id,
                            )
                        },
                    )?
                }),
                active_character_compaction: should_compact(turns_character_count(&turns)),
                invocation_sequence: sequence,
                overflow_count: projection.overflow_count,
            },
            context_usage_anchor.as_ref(),
        );
        record_context_snapshot(logging, clock, request, sequence, &context_snapshot);
        if context_snapshot.capacity.as_ref().is_some_and(|capacity| {
            context_snapshot.tokens.is_some_and(|tokens| {
                tokens.saturating_add(context_snapshot.reserved_tokens.unwrap_or_default())
                    > capacity.context_window_tokens
            })
        }) {
            finish_api_invocation(
                accounting,
                invocation.as_ref(),
                None,
                None,
                UsageStatus::Failed,
                clock,
                logging,
            );
            return failed_non_retryable(
                "The protected request content exceeds the selected endpoint Profile context budget.",
            );
        }
        let request_builder =
            (wire_format.apply_auth)(client.post(&wire_format.endpoint), &api_key);
        let estimated_input_characters = estimated_input_characters(&body, images_in_request);
        let response = match request_builder
            .header("content-type", "application/json")
            .json(&body)
            .send()
        {
            Ok(response) => response,
            Err(error) => {
                finish_api_invocation(
                    accounting,
                    invocation.as_ref(),
                    None,
                    None,
                    UsageStatus::Failed,
                    clock,
                    logging,
                );
                return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                    error.to_string(),
                ));
            }
        };
        let status = response.status();
        if !status.is_success() {
            let body_text = response.text().unwrap_or_default();
            finish_api_invocation(
                accounting,
                invocation.as_ref(),
                None,
                None,
                UsageStatus::Failed,
                clock,
                logging,
            );
            let context_failure = matches!(status.as_u16(), 400 | 413 | 422)
                && body_text.to_lowercase().contains("context");
            if context_failure && !context_recovery_attempted && turns.len() > 1 {
                context_recovery_attempted = true;
                turns.remove(0);
                continue;
            }
            return GenerationProcessEvent::Failed((wire_format.failure_from_http_status)(
                status.as_u16(),
                &body_text,
            ));
        }

        let mut reader = std::io::BufReader::new(response);
        let mut current_data: Option<String> = None;
        let mut accumulator = ToolCallAccumulator::default();
        let mut assistant_text = String::new();
        let mut round_usage = None;
        loop {
            if cancelled.load(Ordering::SeqCst) {
                finish_api_invocation(
                    accounting,
                    invocation.as_ref(),
                    round_usage.as_ref(),
                    None,
                    UsageStatus::Cancelled,
                    clock,
                    logging,
                );
                return failed_non_retryable("Generation was cancelled.");
            }
            let mut line = String::new();
            let read = match reader.read_line(&mut line) {
                Ok(read) => read,
                Err(error) => {
                    finish_api_invocation(
                        accounting,
                        invocation.as_ref(),
                        round_usage.as_ref(),
                        None,
                        UsageStatus::Failed,
                        clock,
                        logging,
                    );
                    return GenerationProcessEvent::Failed(GenerationProcessFailure::retryable(
                        format!("Failed to read the provider API response: {error}"),
                    ));
                }
            };
            if read == 0 {
                break;
            }
            let line = line.trim_end_matches(['\r', '\n']);
            if let Some(data) = line.strip_prefix("data:") {
                current_data = Some(data.trim().to_string());
                continue;
            }
            if line.is_empty() {
                if let Some(data) = current_data.take() {
                    match (wire_format.translate_sse_data)(&data, &mut accumulator) {
                        Some(GenerationProcessEvent::Completed(usage)) => {
                            round_usage = usage;
                            break;
                        }
                        Some(GenerationProcessEvent::Failed(failure)) => {
                            finish_api_invocation(
                                accounting,
                                invocation.as_ref(),
                                round_usage.as_ref(),
                                None,
                                UsageStatus::Failed,
                                clock,
                                logging,
                            );
                            return GenerationProcessEvent::Failed(failure);
                        }
                        Some(GenerationProcessEvent::Token(text)) => {
                            let starts_new_round = assistant_text.is_empty();
                            assistant_text.push_str(&text);
                            let content_delta = if emitted_visible_content && starts_new_round {
                                format!("\n{text}")
                            } else {
                                text
                            };
                            emitted_visible_content = true;
                            if sink
                                .handle(GenerationProcessEvent::Token(content_delta))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                        }
                        Some(event) => {
                            if sink.handle(event).is_err() {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                        }
                        None => {}
                    }
                }
            }
        }

        finish_api_invocation(
            accounting,
            invocation.as_ref(),
            round_usage.as_ref(),
            estimated_input_characters.map(|input| (input, assistant_text.chars().count())),
            UsageStatus::Succeeded,
            clock,
            logging,
        );
        if round_usage.as_ref().is_some_and(|usage| {
            ContextAnalysisService::finalize_reported_snapshot(
                &mut context_snapshot,
                usage.input_tokens,
            )
        }) {
            record_context_snapshot(logging, clock, request, sequence, &context_snapshot);
        }
        context_usage_anchor = round_usage.as_ref().and_then(|usage| {
            ContextAnalysisService::finalize_anchor(
                &context_snapshot,
                provider_config.source_provider_id.as_deref(),
                &provider_config.model_id,
                sequence,
                usage.input_tokens,
            )
        });

        let tool_calls = accumulator.take_completed();
        if tool_calls.is_empty() {
            return GenerationProcessEvent::Completed(None);
        }

        let mut executed: Vec<ExecutedToolCall> = Vec::with_capacity(tool_calls.len());
        for mut tool_use in tool_calls {
            if cancelled.load(Ordering::SeqCst) {
                return failed_non_retryable("Generation was cancelled.");
            }
            let input = tool_use.input.clone().unwrap_or(Value::Null);
            if native_tools.handler(&tool_use.name).is_some() {
                let outcome = match execute_registered_native_tool(
                    &mut tool_use,
                    &input,
                    request,
                    cancelled.clone(),
                    native_tools,
                    native_tool_operations,
                    native_tool_events,
                    permissions,
                    pending_approvals,
                    sink,
                    plan_mode,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                let (outcome, image_artifact_id) = outcome;
                if cancelled.load(Ordering::SeqCst) {
                    return failed_non_retryable("Generation was cancelled.");
                }
                // The tool named an Artifact, not bytes, so nothing base64 has entered the output
                // above or the operation record. Resolving it here is a local, hash-verified read.
                let image = image_artifact_id.as_deref().and_then(|artifact_id| {
                    resolve_tool_image(artifacts, artifact_id, images_supported, images_in_request)
                });
                if let Some(image) = image.as_ref() {
                    log_image_attachment(logging, clock, request, &tool_use.id, image);
                    images_in_request += 1;
                }
                tool_use.status = if outcome.is_error {
                    "failed".to_owned()
                } else {
                    "completed".to_owned()
                };
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if sink
                    .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                    .is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, image));
                continue;
            }
            // Intercepted here for the same reason `ask_user_question` is: the image has to reach
            // `build_reply_turns`, and `execute_tool_call_impl` can only return text
            // (`add-agent-image-input`).
            if images_supported && is_image_read_request(&tool_use.name, &input) {
                let folder = request.session.folder.as_deref().unwrap_or_default();
                // Checked per call, not per round trip: the counter only moves here, so a
                // round-trip-scoped check would let every image in one batch through. Exceeding
                // the budget is an explicit error rather than a silent drop -- a question
                // answered about the image that got dropped would be confident nonsense.
                let (outcome, image) = if images_in_request >= MAX_IMAGES_PER_REQUEST {
                    (
                        ToolExecutionOutcome {
                            output: format!(
                                "This request already carries the maximum of {MAX_IMAGES_PER_REQUEST} images. Ask about the images already attached before reading another."
                            ),
                            is_error: true,
                        },
                        None,
                    )
                } else {
                    match execute_file_image_read(
                        input
                            .get("path")
                            .and_then(Value::as_str)
                            .unwrap_or_default(),
                        folder,
                    ) {
                        Ok((summary, image)) => (
                            ToolExecutionOutcome {
                                output: summary,
                                is_error: false,
                            },
                            Some(image),
                        ),
                        Err(outcome) => (outcome, None),
                    }
                };
                if let Some(image) = image.as_ref() {
                    log_image_attachment(logging, clock, request, &tool_use.id, image);
                    images_in_request += 1;
                }
                tool_use.status = if outcome.is_error {
                    "failed".to_owned()
                } else {
                    "completed".to_owned()
                };
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if sink
                    .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                    .is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, image));
                continue;
            }
            // Handled here rather than in `execute_tool_call_impl` because asking needs the event
            // sink and the blocked-call channel, exactly as the approval gate below does
            // (`add-agent-user-question` D1).
            if tool_use.name == ASK_USER_QUESTION_TOOL_NAME {
                let outcome = match ask_user_question(
                    &mut tool_use,
                    &input,
                    request.interactive,
                    &cancelled,
                    pending_approvals,
                    sink,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                tool_use.status = if outcome.is_error {
                    "failed".to_owned()
                } else {
                    "completed".to_owned()
                };
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if sink
                    .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                    .is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
                continue;
            }
            // Same placement and reason as the question above: the sink and blocked-call channel
            // live here (`add-agent-plan-exit-request` D1).
            if tool_use.name == EXIT_PLAN_MODE_TOOL_NAME {
                let outcome = match request_plan_exit(
                    &mut tool_use,
                    &input,
                    request.interactive,
                    plan_mode,
                    &cancelled,
                    pending_approvals,
                    sink,
                ) {
                    Ok(outcome) => outcome,
                    Err(failure) => return failure,
                };
                tool_use.status = if outcome.is_error {
                    "failed".to_owned()
                } else {
                    "completed".to_owned()
                };
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if sink
                    .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                    .is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
                continue;
            }
            if tool_use.name.starts_with("skill__") {
                let skill_key = skill_tool_keys.get(&tool_use.name);
                tool_use.skill_provenance = skill_key.map(skill_tool_provenance);
                tool_use.status = "running".to_string();
                if emit_skill_tool_lifecycle(sink, &tool_use, ToolLifecyclePhase::Started).is_err()
                {
                    return failed_retryable("Agent generation event handling failed.");
                }
                let lifecycle = AgentSkillToolLifecycle {
                    sink,
                    tool_use: &tool_use,
                };
                let outcome = dispatch_skill_tool(
                    skill_tool_execution,
                    skill_key,
                    &tool_use.id,
                    agent_id,
                    request.session.folder.as_deref(),
                    &request.session.id,
                    &request.operation_id,
                    plan_mode,
                    &input,
                    &cancelled,
                    &lifecycle,
                );
                if matches!(outcome.output.as_str(), "cancelled")
                    || cancelled.load(Ordering::SeqCst)
                {
                    tool_use.status = "cancelled".to_string();
                    set_skill_result_summary(&mut tool_use, "cancelled");
                    let _ =
                        emit_skill_tool_lifecycle(sink, &tool_use, ToolLifecyclePhase::Cancelled);
                    return failed_non_retryable("Generation was cancelled.");
                }
                tool_use.status = if outcome.is_error {
                    "failed".to_string()
                } else {
                    "completed".to_string()
                };
                let terminal_phase = if outcome.is_error {
                    ToolLifecyclePhase::Failed
                } else {
                    ToolLifecyclePhase::Completed
                };
                let result_label = if outcome.is_error {
                    "failed"
                } else {
                    "completed"
                };
                set_skill_result_summary(&mut tool_use, result_label);
                tool_use.output = Some(Value::String(outcome.output.clone()));
                if emit_skill_tool_lifecycle(sink, &tool_use, terminal_phase).is_err() {
                    return failed_retryable("Agent generation event handling failed.");
                }
                executed.push((tool_use, outcome.output, outcome.is_error, None));
                continue;
            }
            let (permission_action, permission_resource) =
                permission_action_and_resource(&tool_use.name, &input);
            let project_key = request.session.folder.as_deref().unwrap_or("");
            let effect = permissions.evaluate(
                agent_id,
                permission_action.clone(),
                permission_resource.clone(),
                &request.session.id,
                &request.operation_id,
                project_key,
            );
            match effect {
                Effect::Allow => {}
                Effect::Deny => {
                    let denial = "Denied by policy.".to_string();
                    tool_use.status = "failed".to_string();
                    tool_use.output = Some(Value::String(denial.clone()));
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return failed_retryable("Agent generation event handling failed.");
                    }
                    executed.push((tool_use, denial, true, None));
                    continue;
                }
                Effect::Ask => {
                    tool_use.status = "awaiting_approval".to_string();
                    if sink
                        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                        .is_err()
                    {
                        return failed_retryable("Agent generation event handling failed.");
                    }
                    if let Err(error) = permissions.create_pending_approval(
                        agent_id,
                        permission_action,
                        permission_resource,
                        &request.session.id,
                        &request.operation_id,
                        &tool_use.id,
                        project_key,
                    ) {
                        return failed_non_retryable(&error.to_string());
                    }
                    match await_approval(&tool_use.id, &cancelled, pending_approvals) {
                        ApprovalOutcome::Approved => {}
                        ApprovalOutcome::Denied => {
                            let denial = "Denied by user.".to_string();
                            tool_use.status = "failed".to_string();
                            tool_use.output = Some(Value::String(denial.clone()));
                            if sink
                                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                            executed.push((tool_use, denial, true, None));
                            continue;
                        }
                        ApprovalOutcome::Cancelled => {
                            return failed_non_retryable(
                                "Generation was cancelled while a tool call was awaiting approval.",
                            );
                        }
                        // An answer delivered to a call that asked for permission means the two
                        // resolutions were crossed; fail closed rather than treat it as consent.
                        ApprovalOutcome::Answered(_) => {
                            let denial = "Denied by user.".to_string();
                            tool_use.status = "failed".to_string();
                            tool_use.output = Some(Value::String(denial.clone()));
                            if sink
                                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                                .is_err()
                            {
                                return failed_retryable("Agent generation event handling failed.");
                            }
                            executed.push((tool_use, denial, true, None));
                            continue;
                        }
                    }
                }
            }
            tool_use.status = "running".to_string();
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return failed_retryable("Agent generation event handling failed.");
            }
            let outcome = if tool_use.name == REMEMBER_TOOL_NAME
                && !resolve_personalization_settings(personalization, logging, clock, request)
                    .memory_enabled
            {
                // Memory master switch off (`add-personalization-settings`) — reject before
                // dispatching, matching `execute_remember`'s own empty-content rejection shape, so
                // this never reaches `AgentMemoryPort::save`.
                ToolExecutionOutcome {
                    output: "Memory is disabled; nothing was remembered.".to_string(),
                    is_error: true,
                }
            } else {
                execute_tool_call_with_runtime_ports(
                    &tool_use.name,
                    &input,
                    request.session.folder.as_deref(),
                    cancelled.clone(),
                    agent_id,
                    memories,
                    mcp,
                    retrieval,
                    code_intelligence,
                    workspace_mutations,
                    plan_mode,
                    skills,
                    utility_delegation,
                    request,
                )
            };
            if cancelled.load(Ordering::SeqCst) {
                return failed_non_retryable("Generation was cancelled.");
            }
            tool_use.status = if outcome.is_error {
                "failed".to_string()
            } else {
                "completed".to_string()
            };
            tool_use.output = Some(Value::String(outcome.output.clone()));
            if sink
                .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
                .is_err()
            {
                return failed_retryable("Agent generation event handling failed.");
            }
            executed.push((tool_use, outcome.output, outcome.is_error, None));
        }

        turns.extend((wire_format.build_reply_turns)(&assistant_text, &executed));
        // `tool_calls` was non-empty to reach this point (checked above), and every branch through
        // the loop above either pushes to `executed` or returns the whole generation early — so
        // reaching here always means at least one tool call was attempted this round trip.
        if !executed.is_empty() {
            tool_assisted_session = true;
        }
        if let Some(failure) = maybe_compact_accounted(
            &mut turns,
            &wire_format,
            &client,
            &api_key,
            &provider_config.model_id,
            &provider_config,
            &tools,
            &generation_options,
            system.as_deref(),
            &cancelled,
            sink,
            logging,
            clock,
            request,
            memories,
            personalization,
            tool_assisted_session,
            accounting,
            &mut request_sequence,
            context_usage_anchor.as_ref(),
            &mut automatic_compaction_state,
            context_quality,
            generation_personalization.context_quality_retention_days,
        ) {
            return failure;
        }
    }

    failed_non_retryable("Tool-use loop exceeded the maximum number of round trips.")
}

fn skill_tool_outcome(outcome: SkillToolDispatchOutcome) -> ToolExecutionOutcome {
    match outcome {
        SkillToolDispatchOutcome::Completed(value) => ToolExecutionOutcome {
            output: value.to_string(),
            is_error: false,
        },
        SkillToolDispatchOutcome::Denied { reason } => ToolExecutionOutcome {
            output: format!("Skill tool denied: {reason}"),
            is_error: true,
        },
        SkillToolDispatchOutcome::Failed { code } => ToolExecutionOutcome {
            output: format!("Skill tool failed: {code}"),
            is_error: true,
        },
        SkillToolDispatchOutcome::Cancelled => ToolExecutionOutcome {
            output: "cancelled".to_string(),
            is_error: true,
        },
    }
}

fn skill_tool_provenance(
    key: &crate::contexts::tooling::skill_tools::domain::SkillToolKey,
) -> SkillToolUseProvenance {
    SkillToolUseProvenance {
        skill_id: key.owner.as_str().to_string(),
        tool_id: key.tool.as_str().to_string(),
        revision: key.revision.as_str().to_string(),
        source_scope: key.source.scope.as_str().to_string(),
        workspace_path: key.source.workspace_path.clone(),
        redacted_result_summary: None,
    }
}

fn set_skill_result_summary(tool_use: &mut ToolUseBlock, label: &str) {
    if let Some(provenance) = tool_use.skill_provenance.as_mut() {
        provenance.redacted_result_summary = Some(label.to_string());
    }
}

fn emit_skill_tool_lifecycle(
    sink: &dyn AgentProcessEventSink,
    tool_use: &ToolUseBlock,
    phase: ToolLifecyclePhase,
) -> Result<(), AgentRuntimeApplicationError> {
    sink.handle(GenerationProcessEvent::ToolLifecycle(ToolLifecycleEvent {
        call_id: tool_use.id.clone(),
        phase,
        provider_timestamp: None,
        fidelity: crate::contexts::execution_observability::api::ExecutionFidelity::Native,
        parent_run_id: None,
        parent_trace_id: None,
        parent_span_id: None,
        delegation_id: None,
        attempt: None,
        tool_use: tool_use.clone(),
    }))
}

struct AgentSkillToolLifecycle<'a> {
    sink: &'a dyn AgentProcessEventSink,
    tool_use: &'a ToolUseBlock,
}

impl SkillToolExecutionLifecyclePort for AgentSkillToolLifecycle<'_> {
    fn transition(&self, phase: SkillToolExecutionLifecyclePhase) {
        let phase = match phase {
            SkillToolExecutionLifecyclePhase::AwaitingApproval => {
                ToolLifecyclePhase::AwaitingApproval
            }
        };
        let mut tool_use = self.tool_use.clone();
        tool_use.status = "awaiting_approval".to_string();
        let _ = emit_skill_tool_lifecycle(self.sink, &tool_use, phase);
    }
}

#[allow(clippy::too_many_arguments)]
fn dispatch_skill_tool(
    execution: Option<&dyn SkillToolExecutionPort>,
    key: Option<&crate::contexts::tooling::skill_tools::domain::SkillToolKey>,
    call_id: &str,
    parent_agent_id: &str,
    workspace_path: Option<&str>,
    session_id: &str,
    generation_id: &str,
    plan_mode: bool,
    input: &Value,
    cancelled: &AtomicBool,
    lifecycle: &dyn SkillToolExecutionLifecyclePort,
) -> ToolExecutionOutcome {
    let (Some(execution), Some(key)) = (execution, key) else {
        return ToolExecutionOutcome {
            output: "Skill tool is unknown or stale for this generation.".to_string(),
            is_error: true,
        };
    };
    execution
        .execute(SkillToolExecutionRequest {
            call_id,
            key,
            parent_agent_id,
            workspace_path,
            session_id,
            generation_id,
            mode: if plan_mode {
                SkillToolCatalogMode::Plan
            } else {
                SkillToolCatalogMode::Execute
            },
            input,
            cancelled,
            lifecycle,
        })
        .map(skill_tool_outcome)
        .unwrap_or_else(|error| ToolExecutionOutcome {
            output: format!("Skill tool failed: {}", error.code()),
            is_error: true,
        })
}

/// Sums the length of every string value reachable within `turns`, recursively — a
/// wire-format-agnostic proxy for how large a turns list is. Both wire formats nest large
/// content (tool results, tool-call arguments) inside arrays/objects rather than as a flat
/// `content` string, so a shallow field-only count would miss exactly the payloads (e.g.
/// file-read tool output) that motivate compaction in the first place.
fn turns_character_count(turns: &[Value]) -> usize {
    turns.iter().map(value_character_count).sum()
}

fn value_character_count(value: &Value) -> usize {
    match value {
        Value::String(text) => text.chars().count(),
        Value::Array(items) => items.iter().map(value_character_count).sum(),
        Value::Object(map) => map.values().map(value_character_count).sum(),
        _ => 0,
    }
}

fn should_compact(character_count: usize) -> bool {
    character_count > COMPACTION_TRIGGER_CHARACTERS
}

fn compaction_notice_block(
    message_id: &str,
    turns_before: usize,
    evidence: &ContextCompactionEvidence,
) -> Value {
    let token_value = |value: Option<u64>| {
        value.map_or_else(|| "Unavailable".to_string(), |value| value.to_string())
    };
    json!({
        "id": format!("compaction-{message_id}-{turns_before}"),
        "kind": "card",
        "v": 1,
        "title": "Conversation compacted",
        "bodyMarkdown": "Earlier context was compacted. This evidence contains measurements only and excludes conversation content.",
        "tone": "info",
        "fields": [
            { "label": "Before characters", "value": evidence.before_characters.to_string() },
            { "label": "After characters", "value": evidence.after_characters.to_string() },
            { "label": "Characters saved", "value": evidence.saved_characters.to_string() },
            { "label": "Before tokens", "value": token_value(evidence.before_tokens) },
            { "label": "After tokens", "value": token_value(evidence.after_tokens) },
            { "label": "Tokens saved", "value": token_value(evidence.saved_tokens) },
            { "label": "Measurement quality", "value": format!("{} → {}", evidence.before_quality, evidence.after_quality) },
            { "label": "Trigger source", "value": evidence.trigger_source },
            { "label": "Compaction path", "value": evidence.compaction_path },
            { "label": "Policy version", "value": evidence.policy_version },
        ],
        "meta": {
            "evidenceKind": "context-compaction",
            "attemptId": evidence.attempt_id,
            "beforeCharacters": evidence.before_characters,
            "afterCharacters": evidence.after_characters,
            "savedCharacters": evidence.saved_characters,
            "beforeTokens": evidence.before_tokens,
            "afterTokens": evidence.after_tokens,
            "savedTokens": evidence.saved_tokens,
            "beforeQuality": evidence.before_quality,
            "afterQuality": evidence.after_quality,
            "triggerSource": evidence.trigger_source,
            "compactionPath": evidence.compaction_path,
            "policyVersion": evidence.policy_version,
        },
    })
}

#[allow(clippy::too_many_arguments)]
fn resolve_tool_catalog_with_code_intelligence(
    request: &GenerationProcessRequest,
    mcp: &dyn AgentMcpToolPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    plan_mode: bool,
    retrieval_available: bool,
    code_search_available: bool,
    code_intelligence_available: bool,
) -> Vec<ToolDefinition> {
    if plan_mode {
        let mut tools = plan_mode_tool_catalog();
        if retrieval_available {
            tools.push(recall_tool_definition());
        }
        if code_search_available {
            tools.push(search_code_tool_definition());
        }
        if code_intelligence_available {
            tools.extend(code_intelligence_tool_definitions());
        }
        // Plan mode is where clarification matters most -- the whole point of the mode is to
        // settle what the work is before doing it (`add-agent-user-question`).
        if request.interactive {
            tools.push(ask_user_question_tool_definition());
        }
        return tools;
    }
    let mut tools = tool_catalog();
    let project_path = request.session.folder.as_deref().unwrap_or_default();
    match mcp.catalog_entries(project_path) {
        Ok(mcp_tools) => tools.extend(mcp_tools),
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.mcp".to_string(),
                message: format!(
                    "Failed to resolve MCP-sourced tools; continuing with the fixed tool catalog only: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
        }
    }
    if retrieval_available {
        tools.push(recall_tool_definition());
    }
    if code_search_available {
        tools.push(search_code_tool_definition());
    }
    if code_intelligence_available {
        tools.extend(code_intelligence_tool_definitions());
    }
    if request.interactive {
        tools.push(ask_user_question_tool_definition());
    }
    tools
}

/// Resolves the agent's bound, enabled Skills (`add-agent-skill-support`) and stored memories
/// scoped to `(agent_id, request.session.folder)` (`add-agent-cross-session-memory`) into one
/// system-prompt string, or `None` if both are empty. Neither source can fail the generation on
/// lookup error — each logs its own warning and falls back to contributing nothing, matching
/// context compaction's own established best-effort-enhancement philosophy (design.md Decision 3
/// in `add-agent-skill-support`).
/// Fetches host-level personalization settings once, degrading to
/// `PersonalizationSettings::safe_fallback()` and a logged warning on lookup failure — shared by
/// every call site that needs a personalization flag (`add-personalization-settings`), matching
/// this function's neighbors' own established lookup-failure philosophy.
fn resolve_personalization_settings(
    personalization: &dyn AgentPersonalizationPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> PersonalizationSettings {
    match personalization.settings() {
        Ok(settings) => settings,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.personalization".to_string(),
                message: format!(
                    "Failed to resolve personalization settings; continuing with safe defaults: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            PersonalizationSettings::safe_fallback()
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resolve_system_prompt_with_settings(
    agent_id: &str,
    core_instructions: &dyn AgentCoreInstructionsPort,
    personalization_settings: &PersonalizationSettings,
    skills: &dyn AgentSkillPort,
    memories: &dyn AgentMemoryPort,
    selection: &dyn AgentMemorySelectionPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    observed_skill_revisions: &mut Vec<ObservedSkillRevision>,
) -> Option<String> {
    let custom_instructions_section = format_custom_instructions_section(personalization_settings);
    let core_section = match core_instructions.instructions_for(agent_id) {
        Ok(Some(core)) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Debug,
                category: "session.runtime.api.prompt".to_string(),
                message: format!("Applied core instructions version {}.", core.version),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            Some(core.markdown)
        }
        Ok(None) => None,
        Err(_) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.prompt".to_string(),
                message:
                    "Failed to resolve core instructions; continuing with optional prompt sections."
                        .to_string(),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    let skill_section = match skills
        .bound_skill_prompts(agent_id, request.session.folder.as_deref())
    {
        Ok(prompts) if prompts.is_empty() => None,
        Ok(prompts) => {
            let observed_at = clock.now();
            observed_skill_revisions.extend(prompts.iter().map(|prompt| ObservedSkillRevision {
                skill_id: prompt.id.clone(),
                revision: prompt.revision.clone(),
                association_kind: SkillAssociationKind::Injected,
                observed_at: observed_at.clone(),
            }));
            format_system_prompt(&prompts, logging, clock, request)
        }
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skills".to_string(),
                message: format!(
                    "Failed to resolve bound Skills; continuing without them in the system prompt: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            None
        }
    };
    let (memory_section, memory_bodies_section) = if !personalization_settings.memory_enabled {
        // Memory master switch off (`add-personalization-settings` D4) — skip the lookup
        // entirely rather than fetching and discarding, matching design.md D8's "no wasted work
        // when a feature is off" intent. No selection call is made either.
        (None, None)
    } else {
        match memories.list_all() {
            Ok(memories) => (
                format_memory_section(&memories),
                select_memory_bodies(&memories, selection, logging, clock, request),
            ),
            Err(error) => {
                let _ = logging.record(AgentLog {
                    level: AgentLogLevel::Warn,
                    category: "session.runtime.api.memory".to_string(),
                    message: format!(
                        "Failed to resolve stored memories; continuing without them in the system prompt: {error}"
                    ),
                    agent_id: Some(request.agent.id.clone()),
                    session_id: Some(request.session.id.clone()),
                    operation_id: Some(request.operation_id.clone()),
                    run_id: None,
                    trace_id: None,
                    span_id: None,
                    occurred_at: clock.now(),
                });
                (None, None)
            }
        }
    };
    // Changes on every `todo_write` (`add-agent-task-list` D2), so it is the most volatile section
    // of all and sits last.
    let task_list_section = task_list_prompt_section(&request.session.id);
    // Stable content first, volatile last. A prefix cache is a prefix, so the sections that change
    // most often sit at the tail where they invalidate the least. The memory index reflects the
    // pool and the bodies reflect one generation's judgment about it, so the bodies follow the
    // index; the task list changes more often than either and follows both.
    let sections: Vec<String> = [
        core_section,
        custom_instructions_section,
        skill_section,
        memory_section,
        memory_bodies_section,
        task_list_section,
    ]
    .into_iter()
    .flatten()
    .collect();
    if sections.is_empty() {
        None
    } else {
        Some(sections.join("\n\n"))
    }
}

/// Runs the relevance selection for one generation and formats the chosen bodies.
///
/// Runs once here, at generation start, rather than per provider round trip. That is forced by
/// two things at once: memory content must never enter the turns list compaction manipulates, so
/// bodies have to live in the system prompt; and a system prompt that changed every round trip
/// would invalidate the provider prefix cache on every round trip inside a tool loop.
///
/// Any failure degrades to index-only injection. Selection is an enhancement — its loss costs
/// relevance, never the generation, and the index alone still tells the model what exists.
fn select_memory_bodies(
    memories: &[AgentMemory],
    selection: &dyn AgentMemorySelectionPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> Option<String> {
    if memories.is_empty() {
        return None;
    }
    // Excluded before the call, not after: filtering afterwards would spend the bounded selection
    // budget on memories this session has already been shown and the caller is about to discard.
    let candidates = unsurfaced_candidates(&request.session.id, memories);
    if candidates.is_empty() {
        return None;
    }
    let selected_names = match selection.select(&request.effective_prompt, &candidates) {
        Ok(names) => names,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Memory relevance selection failed; continuing with the index alone: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return None;
        }
    };
    // Follows the selector's own order so its ranking survives into the prompt.
    let selected = selected_names
        .iter()
        .filter_map(|name| {
            candidates
                .iter()
                .find(|memory| &memory.name == name)
                .cloned()
        })
        .collect::<Vec<_>>();
    mark_surfaced(&request.session.id, &selected);
    crate::contexts::agent_runtime::application::format_memory_bodies(
        &selected,
        std::time::SystemTime::now(),
    )
}

fn format_system_prompt(
    prompts: &[BoundSkillPrompt],
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
) -> Option<String> {
    let mut used = 0usize;
    let mut sections = Vec::new();
    for prompt in prompts {
        let section = format!("## {}\n{}", prompt.name, prompt.body);
        let length = section.chars().count();
        let reason = if length > SKILL_PER_ITEM_CHARACTER_BUDGET {
            Some("per-Skill 8,000-character budget")
        } else if used.saturating_add(length) > SKILL_AGGREGATE_CHARACTER_BUDGET {
            Some("aggregate 16,000-character budget")
        } else {
            None
        };
        if let Some(reason) = reason {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.skills".to_string(),
                message: format!(
                    "Skipped Skill {} because it exceeds the {reason}",
                    prompt.id
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            continue;
        }
        used += length;
        sections.push(section);
    }
    (!sections.is_empty()).then(|| sections.join("\n\n"))
}

/// Thin delegate to `application::format_memory_index` (the formatting rule lives there so the
/// CLI-wrapped agents' send path can share it without `application` depending on
/// `infrastructure` — mirrors `format_custom_instructions_section`'s existing delegation shape).
///
/// Binds OnePiece's bounds here rather than at the call site: this surface is the system prompt,
/// and the CLI surface's far tighter bounds must never be reachable from it by accident.
fn format_memory_section(memories: &[AgentMemory]) -> Option<String> {
    crate::contexts::agent_runtime::application::format_memory_index(
        memories,
        crate::contexts::agent_runtime::application::ONEPIECE_MEMORY_INDEX_BOUNDS,
    )
}

/// Formats enabled, non-empty custom instructions into one `## Custom Instructions` section,
/// response style before about-you within it (`add-personalization-settings` design.md D3 — style
/// is a cross-cutting constraint on every response, about-you is background fact, so style gets
/// the higher-priority earlier position). Returns `None` when disabled or both fields are empty,
/// omitting either sub-heading individually when only one field is populated.
/// Thin delegate to `PersonalizationSettings::custom_instructions_block` (moved to `application`
/// in `add-cli-custom-instructions-injection` so the CLI-wrapped agents' send path can share the
/// identical formatting rule without `application` depending on `infrastructure`). Kept as a free
/// function here, rather than updating every call site to the method form, so this file's existing
/// `format_custom_instructions_section_*` tests need no changes.
fn format_custom_instructions_section(settings: &PersonalizationSettings) -> Option<String> {
    settings.custom_instructions_block()
}

#[derive(Debug)]
enum AutomaticCompactionOutcome {
    NotEligible,
    Bypassed,
    Compacted(CompactionPath),
    Failed,
    TerminalFailure(Box<GenerationProcessEvent>),
}

#[allow(clippy::too_many_arguments)]
fn maybe_compact_accounted(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    sink: &dyn AgentProcessEventSink,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    memories: &dyn AgentMemoryPort,
    personalization: &dyn AgentPersonalizationPort,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
    usage_anchor: Option<&UsageAnchor>,
    state: &mut AutomaticCompactionState,
    context_quality: Option<&ContextQualityRecorder>,
    context_quality_retention_days: i64,
) -> Option<GenerationProcessEvent> {
    let outcome = run_automatic_compaction(
        turns,
        wire_format,
        client,
        api_key,
        model,
        provider_config,
        tools,
        generation_options,
        system,
        cancelled,
        sink,
        logging,
        clock,
        request,
        memories,
        personalization,
        tool_assisted,
        accounting,
        request_sequence,
        usage_anchor,
        state,
        context_quality,
        context_quality_retention_days,
    );
    match outcome {
        AutomaticCompactionOutcome::TerminalFailure(failure) => Some(*failure),
        _ => None,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_automatic_compaction(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    sink: &dyn AgentProcessEventSink,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    memories: &dyn AgentMemoryPort,
    personalization: &dyn AgentPersonalizationPort,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
    usage_anchor: Option<&UsageAnchor>,
    state: &mut AutomaticCompactionState,
    context_quality: Option<&ContextQualityRecorder>,
    context_quality_retention_days: i64,
) -> AutomaticCompactionOutcome {
    let turn_characters = turns_character_count(turns) as u64;
    let character_decision = should_compact(turn_characters as usize);
    let body = (wire_format.build_request_body)(
        &provider_config.model_id,
        turns,
        tools,
        system,
        generation_options,
    );
    let snapshot = prepared_context_snapshot(
        wire_format,
        &body,
        provider_config,
        *request_sequence,
        character_decision,
        usage_anchor,
    );
    let decision = select_authoritative_compaction(
        &snapshot.compaction_decision,
        snapshot.active_character_compaction,
    );
    let decision_sequence = *request_sequence;
    if !decision.should_compact {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "not-eligible",
            None,
            state,
            turn_characters,
        );
        return AutomaticCompactionOutcome::NotEligible;
    }
    if turns.len() <= COMPACTION_KEEP_RECENT_TURNS {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "insufficient-context",
            None,
            state,
            turn_characters,
        );
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Bypassed,
            None,
            Some(ContextAssessmentReason::InsufficientReclaimableContext),
            None,
        );
        return AutomaticCompactionOutcome::Bypassed;
    }
    if let Some(reason) = state.bypass_reason(request.automatic_compaction, turn_characters) {
        record_compaction_control(
            logging,
            clock,
            request,
            decision_sequence,
            &snapshot,
            decision.source,
            "bypassed",
            Some(reason),
            state,
            turn_characters,
        );
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Bypassed,
            None,
            Some(reason.into()),
            None,
        );
        return AutomaticCompactionOutcome::Bypassed;
    }
    let original_turns = turns.clone();
    let (outcome, fallback_reason) = match optimize_compaction_accounted(
        &original_turns,
        &snapshot,
        wire_format,
        client,
        api_key,
        provider_config,
        tools,
        generation_options,
        system,
        cancelled,
        accounting,
        request,
        request_sequence,
        clock,
        logging,
    ) {
        Ok(candidate) => {
            *turns = candidate;
            (
                AutomaticCompactionOutcome::Compacted(CompactionPath::Optimizer),
                None,
            )
        }
        Err(reason) => {
            record_optimizer_fallback(logging, clock, request, *request_sequence, reason);
            let fallback_outcome = compatibility_compact_accounted(
                turns,
                wire_format,
                client,
                api_key,
                model,
                provider_config,
                system,
                cancelled,
                logging,
                clock,
                request,
                memories,
                personalization,
                tool_assisted,
                accounting,
                request_sequence,
            );
            (fallback_outcome, Some(reason))
        }
    };
    if let AutomaticCompactionOutcome::Compacted(path) = &outcome {
        let turns_before = original_turns.len();
        let post_body = (wire_format.build_request_body)(
            &provider_config.model_id,
            turns,
            tools,
            system,
            generation_options,
        );
        let post_snapshot = prepared_context_snapshot(
            wire_format,
            &post_body,
            provider_config,
            decision_sequence,
            should_compact(turns_character_count(turns)),
            usage_anchor,
        );
        let assessment = record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &post_snapshot,
            decision.source,
            if *path == CompactionPath::Optimizer {
                ContextAssessmentOutcome::Compacted
            } else {
                ContextAssessmentOutcome::Fallback
            },
            Some((*path).into()),
            fallback_reason.map(Into::into),
            (*path == CompactionPath::Optimizer).then_some(ContextAssessmentInvariants::passed()),
        );
        let evidence = ContextCompactionEvidence::project(
            &snapshot,
            &post_snapshot,
            decision.source,
            *path,
            assessment.attempt_id.clone(),
        );
        if sink
            .handle(GenerationProcessEvent::RichBlock(compaction_notice_block(
                &request.message_id,
                turns_before,
                &evidence,
            )))
            .is_err()
        {
            return AutomaticCompactionOutcome::TerminalFailure(Box::new(failed_retryable(
                "Agent generation event handling failed.",
            )));
        }
    }
    if matches!(outcome, AutomaticCompactionOutcome::Failed) {
        record_context_quality_assessment(
            context_quality,
            context_quality_retention_days,
            clock,
            request,
            decision_sequence,
            &snapshot,
            &snapshot,
            decision.source,
            ContextAssessmentOutcome::Failed,
            Some(ContextAssessmentPath::Compatibility),
            Some(
                fallback_reason
                    .map(Into::into)
                    .unwrap_or(ContextAssessmentReason::ProviderFailure),
            ),
            None,
        );
    }
    match &outcome {
        AutomaticCompactionOutcome::Compacted(_) => {
            state.record_success(turns_character_count(turns) as u64);
        }
        AutomaticCompactionOutcome::Failed => state.record_failure(),
        _ => {}
    }
    record_compaction_control(
        logging,
        clock,
        request,
        *request_sequence,
        &snapshot,
        decision.source,
        match &outcome {
            AutomaticCompactionOutcome::Compacted(_) => "compacted",
            AutomaticCompactionOutcome::Failed => "failed",
            AutomaticCompactionOutcome::TerminalFailure(_) => "terminal-failure",
            AutomaticCompactionOutcome::NotEligible => "not-eligible",
            AutomaticCompactionOutcome::Bypassed => "bypassed",
        },
        None,
        state,
        turn_characters,
    );
    outcome
}

#[allow(clippy::too_many_arguments)]
fn record_context_quality_assessment(
    recorder: Option<&ContextQualityRecorder>,
    retention_days: i64,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    decision_sequence: u32,
    before: &ContextSnapshot,
    after: &ContextSnapshot,
    trigger_source: CompactionTriggerSource,
    outcome: ContextAssessmentOutcome,
    path: Option<ContextAssessmentPath>,
    reason: Option<ContextAssessmentReason>,
    invariants: Option<ContextAssessmentInvariants>,
) -> ContextQualityAssessment {
    let recorded_at = clock.now();
    let assessment = ContextQualityAssessment::new(ContextQualityAssessmentInput {
        generation_correlation: request.execution_context.run_id.as_str(),
        decision_sequence: u64::from(decision_sequence),
        outcome,
        path,
        reason,
        trigger_source: Some(ContextAssessmentTriggerSource::from(trigger_source)),
        before_characters: before.characters,
        after_characters: after.characters,
        before_tokens: before.tokens,
        after_tokens: after.tokens,
        measurement_quality: before.quality.into(),
        invariants,
        context_policy_version: AUTOMATIC_COMPACTION_POLICY_VERSION,
        optimizer_version: CONTEXT_OPTIMIZER_VERSION,
        verifier_version: CONTEXT_VERIFIER_VERSION,
    });
    if let Some(recorder) = recorder {
        recorder.record_with_retention_days(
            &ContextQualityAssessmentRecord {
                session_correlation: None,
                recorded_at,
                assessment: assessment.clone(),
            },
            retention_days,
            CONTEXT_QUALITY_HISTORY_HARD_LIMIT,
        );
    }
    assessment
}

#[allow(clippy::too_many_arguments)]
fn optimize_compaction_accounted(
    original_turns: &[Value],
    original: &ContextSnapshot,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    provider_config: &ApiProviderConfig,
    tools: &[ToolDefinition],
    generation_options: &GenerationOptions,
    system: Option<&str>,
    cancelled: &AtomicBool,
    accounting: Option<&SessionsApi>,
    request: &GenerationProcessRequest,
    request_sequence: &mut u32,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Result<Vec<Value>, FallbackReason> {
    let original_body = (wire_format.build_request_body)(
        &provider_config.model_id,
        original_turns,
        tools,
        system,
        generation_options,
    );
    let target_characters = OPTIMIZER_TARGET_CHARACTERS.min(original.characters.saturating_sub(1));
    let target_tokens = original
        .tokens
        .map(|tokens| tokens.saturating_mul(target_characters) / original.characters.max(1));
    let plan = build_optimization_plan(
        original,
        ContextOptimizationBudget {
            original_characters: original.characters,
            original_tokens: original.tokens,
            target_characters,
            target_tokens,
        },
    )
    .map_err(|_| FallbackReason::InvalidPlan)?;
    if plan.outcome == OptimizationOutcome::InsufficientReclaimableContext {
        return Err(FallbackReason::InsufficientReclaimableContext);
    }
    if plan
        .actions
        .iter()
        .any(|action| action.kind == OptimizationActionKind::ReplaceReinjectable)
    {
        return Err(FallbackReason::ReinjectionUnavailable);
    }
    let shape = context_wire_shape(provider_config);
    let summary = if let Some(boundary) = plan.summary_boundary.as_ref() {
        let selected = build_structured_summary_turns(&original_body, shape, boundary)
            .map_err(|_| FallbackReason::ReconstructionFailed)?;
        let sequence = *request_sequence;
        *request_sequence = request_sequence.saturating_add(1);
        summarize_turns_accounted(
            wire_format,
            client,
            api_key,
            provider_config,
            None,
            &selected,
            STRUCTURED_SUMMARY_PROMPT,
            cancelled,
            accounting,
            request,
            UsagePurpose::ContextCompaction,
            sequence,
            clock,
            logging,
        )
        .map_err(|_| FallbackReason::SummaryFailed)?
        .ok_or(FallbackReason::SummaryFailed)?
    } else {
        String::new()
    };
    let candidate_body = reconstruct_candidate(
        &original_body,
        shape,
        &plan,
        (!summary.is_empty()).then_some(summary.as_str()),
        &[],
    )
    .map_err(|_| FallbackReason::ReconstructionFailed)?;
    let candidate = optimizer_snapshot(
        wire_format,
        &candidate_body,
        provider_config,
        *request_sequence,
    );
    let verification = verify_optimization_candidate(original, &candidate, &plan, &[]);
    if !verification.accepted {
        return Err(FallbackReason::VerificationFailed);
    }
    record_optimizer_success(
        logging,
        clock,
        request,
        *request_sequence,
        &plan,
        original,
        &candidate,
    );
    candidate_turns(&candidate_body, shape, system).ok_or(FallbackReason::ReconstructionFailed)
}

fn record_optimizer_success(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    plan: &crate::contexts::agent_runtime::domain::ContextOptimizationPlan,
    original: &ContextSnapshot,
    candidate: &ContextSnapshot,
) {
    let action_count = |kind| {
        plan.actions
            .iter()
            .filter(|action| action.kind == kind)
            .count()
    };
    let class_count = |class| {
        original
            .components
            .iter()
            .filter(|component| component.retention_class == class)
            .count()
    };
    let fingerprints = plan
        .actions
        .iter()
        .flat_map(|action| action.source_fingerprints.iter())
        .take(8)
        .map(|value| value.as_str())
        .collect::<Vec<_>>()
        .join(",");
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Info,
        category: "session.runtime.api.context-optimizer".to_string(),
        message: format!(
            "optimizer={} verifier={} sequence={} result=accepted actions={} discard={} reinject={} microcompact={} summarize={} classes=protected:{},verbatim:{},summarizable:{},microcompactable:{},reinjectable:{},discardable:{} before_quality={:?} before_characters={} before_tokens={:?} after_quality={:?} after_characters={} after_tokens={:?} saved_characters={} fingerprints={} original_hash={} candidate_hash={} protocol_complete=true coverage_complete=true",
            CONTEXT_OPTIMIZER_VERSION,
            CONTEXT_VERIFIER_VERSION,
            request_sequence,
            plan.actions.len(),
            action_count(OptimizationActionKind::DiscardTransient),
            action_count(OptimizationActionKind::ReplaceReinjectable),
            action_count(OptimizationActionKind::MicrocompactToolResult),
            action_count(OptimizationActionKind::SummarizeRound),
            class_count(RetentionClass::Protected),
            class_count(RetentionClass::Verbatim),
            class_count(RetentionClass::Summarizable),
            class_count(RetentionClass::Microcompactable),
            class_count(RetentionClass::Reinjectable),
            class_count(RetentionClass::Discardable),
            original.quality,
            original.characters,
            original.tokens,
            candidate.quality,
            candidate.characters,
            candidate.tokens,
            original.characters.saturating_sub(candidate.characters),
            fingerprints,
            original.request_fingerprint,
            candidate.request_fingerprint,
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn record_optimizer_fallback(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    reason: FallbackReason,
) {
    let stage = match reason {
        FallbackReason::InvalidPlan | FallbackReason::InsufficientReclaimableContext => "planning",
        FallbackReason::ReductionFailed => "reduction",
        FallbackReason::ReinjectionUnavailable => "reinjection",
        FallbackReason::SummaryFailed => "summary",
        FallbackReason::ReconstructionFailed => "reconstruction",
        FallbackReason::VerificationFailed => "verification",
    };
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Warn,
        category: "session.runtime.api.context-optimizer".to_string(),
        message: format!(
            "optimizer={} verifier={} sequence={} result=fallback stage={} reason={:?}",
            CONTEXT_OPTIMIZER_VERSION, CONTEXT_VERIFIER_VERSION, request_sequence, stage, reason,
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

#[allow(clippy::too_many_arguments)]
fn record_compaction_control(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    request_sequence: u32,
    snapshot: &ContextSnapshot,
    source: CompactionTriggerSource,
    result: &'static str,
    bypass_reason: Option<CompactionBypassReason>,
    state: &AutomaticCompactionState,
    turn_characters: u64,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Debug,
        category: "agent.context.compaction.control".to_string(),
        message: format!(
            "policy={} sequence={} result={} trigger_source={:?} quality={:?} tokens={:?} token_threshold={:?} request_characters={} turn_characters={} legacy_character_compact={} token_compact={:?} bypass_reason={:?} cooldown_growth={} consecutive_failures={} circuit_open={}",
            AUTOMATIC_COMPACTION_POLICY_VERSION,
            request_sequence,
            result,
            source,
            snapshot.quality,
            snapshot.tokens,
            snapshot.compaction_decision.threshold_tokens,
            snapshot.characters,
            turn_characters,
            snapshot.active_character_compaction,
            snapshot.compaction_decision.should_compact,
            bypass_reason,
            state.growth_since_success(turn_characters),
            state.consecutive_failures(),
            state.circuit_open(),
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: Some(request.execution_context.run_id.as_str().to_string()),
        trace_id: Some(request.execution_context.trace_id.as_str().to_string()),
        span_id: Some(request.execution_context.span_id.as_str().to_string()),
        occurred_at: clock.now(),
    });
}

fn optimizer_snapshot(
    wire_format: &WireFormat,
    body: &Value,
    provider_config: &ApiProviderConfig,
    invocation_sequence: u32,
) -> ContextSnapshot {
    prepared_context_snapshot(
        wire_format,
        body,
        provider_config,
        invocation_sequence,
        true,
        None,
    )
}

fn prepared_context_snapshot(
    wire_format: &WireFormat,
    body: &Value,
    provider_config: &ApiProviderConfig,
    invocation_sequence: u32,
    character_decision: bool,
    usage_anchor: Option<&UsageAnchor>,
) -> ContextSnapshot {
    let projection = (wire_format.project_request_context)(body);
    ContextAnalysisService::analyze(
        ContextAnalysisInput {
            provider_id: provider_config.source_provider_id.clone(),
            model_id: provider_config.model_id.clone(),
            request_fingerprint: projection.request_fingerprint,
            characters: projection.characters,
            components: projection.components,
            rounds: projection.rounds,
            token_estimate_complete: projection.token_estimate_complete,
            capacity: model_context_catalog::resolve_capacity(
                provider_config.source_provider_id.as_deref(),
                &provider_config.model_id,
            ),
            active_character_compaction: character_decision,
            invocation_sequence,
            overflow_count: projection.overflow_count,
        },
        usage_anchor,
    )
}

fn context_wire_shape(provider_config: &ApiProviderConfig) -> ContextWireShape {
    if provider_config.interface_format == INTERFACE_FORMAT_OPENAI_COMPATIBLE {
        ContextWireShape::OpenAiCompatible
    } else {
        ContextWireShape::Anthropic
    }
}

fn candidate_turns(
    body: &Value,
    shape: ContextWireShape,
    system: Option<&str>,
) -> Option<Vec<Value>> {
    let mut messages = body.get("messages")?.as_array()?.clone();
    if shape == ContextWireShape::OpenAiCompatible
        && system.is_some()
        && messages
            .first()
            .and_then(|message| message.get("role"))
            .and_then(Value::as_str)
            == Some("system")
    {
        messages.remove(0);
    }
    Some(messages)
}

/// Preserves the pre-optimizer summary-only compaction path as an untouched compatibility
/// fallback. Optimizer-first orchestration calls this only with the original turns.
#[allow(clippy::too_many_arguments)]
fn compatibility_compact_accounted(
    turns: &mut Vec<Value>,
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    provider_config: &ApiProviderConfig,
    system: Option<&str>,
    cancelled: &AtomicBool,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    memories: &dyn AgentMemoryPort,
    personalization: &dyn AgentPersonalizationPort,
    tool_assisted: bool,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
) -> AutomaticCompactionOutcome {
    if turns.len() <= COMPACTION_KEEP_RECENT_TURNS {
        return AutomaticCompactionOutcome::Failed;
    }
    let split_at = turns.len() - COMPACTION_KEEP_RECENT_TURNS;
    let compaction_sequence = *request_sequence;
    *request_sequence = request_sequence.saturating_add(1);
    let summary = match summarize_turns_accounted(
        wire_format,
        client,
        api_key,
        provider_config,
        system,
        &turns[..split_at],
        SUMMARIZATION_INSTRUCTION,
        cancelled,
        accounting,
        request,
        UsagePurpose::ContextCompaction,
        compaction_sequence,
        clock,
        logging,
    ) {
        Ok(Some(summary)) => summary,
        Ok(None) | Err(_) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.compaction".to_string(),
                message:
                    "Context compaction summarization call failed; continuing without compaction."
                        .to_string(),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return AutomaticCompactionOutcome::Failed;
        }
    };
    // Piggybacks on compaction's own trigger as a cost/latency control (design.md) — runs only
    // when the session has already gotten substantial enough to compact, using the identical
    // pre-mutation slice. Gated by the memory master switch and, only for sessions where tools
    // were used, the tool-assisted-chats sub-switch (`add-personalization-settings` D4/D5) — the
    // explicit `remember` tool is unaffected by either gate, this only skips the passive,
    // automatic extraction call itself.
    let personalization_settings =
        resolve_personalization_settings(personalization, logging, clock, request);
    let extraction_allowed = personalization_settings.memory_enabled
        && (!tool_assisted || personalization_settings.memory_tool_assisted_chats_enabled);
    if extraction_allowed {
        extract_memories_accounted(
            wire_format,
            client,
            api_key,
            model,
            provider_config,
            system,
            &turns[..split_at],
            cancelled,
            request.agent.id.as_str(),
            request.session.folder.as_deref(),
            memories,
            logging,
            clock,
            request,
            accounting,
            request_sequence,
        );
    }
    let mut compacted = vec![json!({ "role": "user", "content": summary })];
    compacted.extend(turns.split_off(split_at));
    *turns = compacted;
    AutomaticCompactionOutcome::Compacted(CompactionPath::Compatibility)
}

/// Calls the model once to reduce `turns_to_summarize` to short text per `instruction`, reusing
/// the same wire-format request/response machinery as regular generation — except the full text
/// response is accumulated instead of streamed to the sink, and no tools are declared. `Ok(None)`
/// means the call completed but produced nothing (empty `turns_to_summarize`, or a blank/
/// whitespace-only response — a valid outcome, e.g. "nothing worth remembering" for extraction).
/// `Err` means the call itself didn't complete: network error, non-2xx status, cancellation, or a
/// provider-reported failure. Shared by compaction's own summarization and automatic memory
/// extraction (`extract_memories`) — both are best-effort steps that must never fail the
/// generation they're attached to, but the `Ok`/`Err` split lets `extract_memories` log only the
/// latter, matching `add-agent-cross-session-memory`'s spec.
#[allow(clippy::too_many_arguments)]
pub(crate) fn summarize_turns(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_summarize: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    max_output_tokens: Option<u32>,
) -> Result<Option<String>, String> {
    summarize_turns_with_usage(
        wire_format,
        client,
        api_key,
        model,
        system,
        turns_to_summarize,
        instruction,
        cancelled,
        max_output_tokens,
    )
    .map(|(summary, _usage)| summary)
}

#[allow(clippy::too_many_arguments)]
fn summarize_turns_with_usage(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns_to_summarize: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    max_output_tokens: Option<u32>,
) -> Result<(Option<String>, Option<ReportedUsageTotals>), String> {
    if turns_to_summarize.is_empty() {
        return Ok((None, None));
    }
    let mut prompt_turns = turns_to_summarize.to_vec();
    prompt_turns.push(json!({ "role": "user", "content": instruction }));
    // Never inherits the user's turn-level `thinking`/`reasoning_depth` — this is an internal,
    // mechanical summarization call, not the user-facing turn (`add-agent-chat-configuration`).
    let body = (wire_format.build_request_body)(
        model,
        &prompt_turns,
        &[],
        system,
        &GenerationOptions::disabled(),
    );
    // Applied after the provider builder rather than inside it: the builders serve the main
    // generation path too, where a summarization-shaped cap would be wrong. Anthropic's builder
    // sets its own default here and this deliberately overrides it for callers that opt in;
    // callers passing `None` — compaction and extraction — are left byte-identical.
    let mut body = body;
    if let Some(limit) = max_output_tokens {
        if let Some(object) = body.as_object_mut() {
            object.insert("max_tokens".to_string(), json!(limit));
        }
    }
    let (text, _tool_calls, usage) =
        stream_completion(wire_format, client, api_key, &body, cancelled)?;
    let trimmed = text.trim();
    Ok(((!trimmed.is_empty()).then(|| trimmed.to_string()), usage))
}

/// Sends one request and drains its SSE stream into assistant text, any completed tool calls, and
/// the provider's reported usage.
///
/// Shared by the internal summarization calls (which declare no tools and discard the tool-call
/// half) and by the subagent child loop (which needs it). Extracted rather than copied because a
/// second SSE reader is a second place for the `data:`/blank-line framing, cancellation, and
/// terminal-event handling to drift.
fn stream_completion(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    body: &Value,
    cancelled: &AtomicBool,
) -> Result<(String, Vec<ToolUseBlock>, Option<ReportedUsageTotals>), String> {
    let request_builder = (wire_format.apply_auth)(client.post(&wire_format.endpoint), api_key);
    let response = request_builder
        .header("content-type", "application/json")
        .json(body)
        .send()
        .map_err(|error| error.to_string())?;
    if !response.status().is_success() {
        return Err(format!("received HTTP {}", response.status()));
    }

    let mut reader = std::io::BufReader::new(response);
    let mut current_data: Option<String> = None;
    let mut accumulator = ToolCallAccumulator::default();
    let mut text = String::new();
    let mut usage = None;
    loop {
        if cancelled.load(Ordering::SeqCst) {
            return Err("cancelled".to_string());
        }
        let mut line = String::new();
        let read = reader
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        let line = line.trim_end_matches(['\r', '\n']);
        if let Some(data) = line.strip_prefix("data:") {
            current_data = Some(data.trim().to_string());
            continue;
        }
        if line.is_empty() {
            if let Some(data) = current_data.take() {
                match (wire_format.translate_sse_data)(&data, &mut accumulator) {
                    Some(GenerationProcessEvent::Completed(reported)) => {
                        usage = reported;
                        break;
                    }
                    Some(GenerationProcessEvent::Failed(failure)) => return Err(failure.diagnostic),
                    Some(GenerationProcessEvent::Token(text_delta)) => text.push_str(&text_delta),
                    _ => {}
                }
            }
        }
    }

    Ok((text, accumulator.take_completed(), usage))
}

/// Builds the reply turns that carry a child's executed tool results back into its next request.
///
/// Exists so `WireFormat`'s function pointers stay private and the subagent module never has to
/// know about `ExecutedToolCall`'s image slot, which a read-only child can never fill
/// (`add-onepiece-subagents`).
pub(crate) fn child_reply_turns(
    wire_format: &WireFormat,
    assistant_text: &str,
    executed: &[(ToolUseBlock, String, bool)],
) -> Vec<Value> {
    let executed: Vec<ExecutedToolCall> = executed
        .iter()
        .map(|(call, output, is_error)| (call.clone(), output.clone(), *is_error, None))
        .collect();
    (wire_format.build_reply_turns)(assistant_text, &executed)
}

/// One turn of a subagent child loop: sends `turns` with the child's restricted tool catalog and
/// returns what the model said plus any tool calls it wants executed
/// (`add-onepiece-subagents`).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_turn(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    model: &str,
    system: Option<&str>,
    turns: &[Value],
    tools: &[ToolDefinition],
    cancelled: &AtomicBool,
) -> Result<(String, Vec<ToolUseBlock>, Option<ReportedUsageTotals>), String> {
    // Never inherits the parent turn's thinking/reasoning settings: a child is an internal,
    // bounded investigation, not the user-facing turn.
    let body = (wire_format.build_request_body)(
        model,
        turns,
        tools,
        system,
        &GenerationOptions::disabled(),
    );
    stream_completion(wire_format, client, api_key, &body, cancelled)
}

#[allow(clippy::too_many_arguments)]
fn summarize_turns_accounted(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    config: &ApiProviderConfig,
    system: Option<&str>,
    turns: &[Value],
    instruction: &str,
    cancelled: &AtomicBool,
    accounting: Option<&SessionsApi>,
    request: &GenerationProcessRequest,
    purpose: UsagePurpose,
    request_sequence: u32,
    clock: &dyn AgentClockPort,
    logging: &dyn AgentLoggingPort,
) -> Result<Option<String>, String> {
    let invocation = begin_api_invocation(
        accounting,
        request,
        config,
        request_sequence,
        purpose,
        clock,
        logging,
    );
    let estimated_input = turns_character_count(turns).saturating_add(instruction.chars().count());
    let result = summarize_turns_with_usage(
        wire_format,
        client,
        api_key,
        &config.model_id,
        system,
        turns,
        instruction,
        cancelled,
        // Compaction summaries and extraction are unbounded here exactly as before; capping a
        // compaction summary would truncate the context it exists to preserve.
        None,
    );
    match &result {
        Ok((summary, usage)) => finish_api_invocation(
            accounting,
            invocation.as_ref(),
            usage.as_ref(),
            Some((
                estimated_input,
                summary.as_ref().map_or(0, |value| value.chars().count()),
            )),
            UsageStatus::Succeeded,
            clock,
            logging,
        ),
        Err(_) => finish_api_invocation(
            accounting,
            invocation.as_ref(),
            None,
            None,
            if cancelled.load(Ordering::SeqCst) {
                UsageStatus::Cancelled
            } else {
                UsageStatus::Failed
            },
            clock,
            logging,
        ),
    }
    result.map(|(summary, _usage)| summary)
}

/// Parses `summarize_turns`'s response as zero or more memories, one per non-empty line, and
/// saves each as `MemorySource::Automatic`. "Nothing worth remembering" (`Ok(None)`) saves
/// nothing and logs nothing — a normal, expected outcome, not a failure. An actual call failure
/// (`Err`) is logged and otherwise ignored, exactly like compaction's own summarization failure,
/// so it's visible to an operator without affecting the generation or its compaction.
#[allow(clippy::too_many_arguments)]
fn extract_memories_accounted(
    wire_format: &WireFormat,
    client: &reqwest::blocking::Client,
    api_key: &str,
    _model: &str,
    provider_config: &ApiProviderConfig,
    system: Option<&str>,
    turns_to_extract_from: &[Value],
    cancelled: &AtomicBool,
    agent_id: &str,
    folder: Option<&str>,
    memories: &dyn AgentMemoryPort,
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    accounting: Option<&SessionsApi>,
    request_sequence: &mut u32,
) {
    let memory_sequence = *request_sequence;
    *request_sequence = request_sequence.saturating_add(1);
    let response = match summarize_turns_accounted(
        wire_format,
        client,
        api_key,
        provider_config,
        system,
        turns_to_extract_from,
        &memory_extraction_instruction(memories),
        cancelled,
        accounting,
        request,
        UsagePurpose::MemoryExtraction,
        memory_sequence,
        clock,
        logging,
    ) {
        Ok(Some(response)) => response,
        Ok(None) => return,
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Automatic memory extraction call failed; continuing without it: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
            return;
        }
    };
    // A malformed response is logged and dropped, never propagated: extraction is best-effort work
    // hanging off a compaction, and the generation that triggered it must be unaffected.
    match parse_memory_actions(&response) {
        Ok(parsed) => {
            apply_memory_actions(memories, agent_id, folder, MemorySource::Automatic, &parsed);
        }
        Err(error) => {
            let _ = logging.record(AgentLog {
                level: AgentLogLevel::Warn,
                category: "session.runtime.api.memory".to_string(),
                message: format!(
                    "Automatic memory extraction returned an unusable response: {error}"
                ),
                agent_id: Some(request.agent.id.clone()),
                session_id: Some(request.session.id.clone()),
                operation_id: Some(request.operation_id.clone()),
                run_id: None,
                trace_id: None,
                span_id: None,
                occurred_at: clock.now(),
            });
        }
    }
}

/// Extraction instruction plus the existing pool's manifest. Built per call because the pool
/// changes between compactions, and without it the model cannot name a memory to update.
fn memory_extraction_instruction(memories: &dyn AgentMemoryPort) -> String {
    let existing = render_existing_manifest(memories);
    if existing.trim().is_empty() {
        MEMORY_ACTIONS_INSTRUCTION.to_string()
    } else {
        format!("{MEMORY_ACTIONS_INSTRUCTION}\n\nExisting memories:\n{existing}")
    }
}

/// Maps every built-in tool to the established permission action whose policy behavior matches
/// that tool. A name outside the built-in catalog maps to a synthetic action no template declares
/// a rule for, so hallucinated calls still fail closed to `Ask`.
fn permission_action_and_resource(tool_name: &str, input: &Value) -> (Action, Resource) {
    match tool_name {
        // Background start is deliberately not a weaker classification than a foreground call:
        // same command, same workspace, same effects -- only the wait differs
        // (`add-background-shell-execution`).
        SHELL_TOOL_NAME => (Action::shell_exec(), Resource::workspace()),
        // Reading a background command's output observes already-approved work, so it is
        // classified like the other read-only tools. Terminating one only *reduces* the effects
        // of work the user already approved, and can act on nothing else -- a handle resolves
        // solely within its own session -- so gating it behind another prompt would make stopping
        // a runaway process harder than starting it was.
        SHELL_OUTPUT_TOOL_NAME | SHELL_KILL_TOOL_NAME => {
            (Action::file_read(), Resource::new(tool_name))
        }
        // Writes only VaneHub-internal session state, with no workspace, process, or network
        // effect -- the same no-approval classification the fixed Skill tools use.
        TODO_WRITE_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        // The user's answer is itself the authorization; a separate approval prompt in front of a
        // question would ask permission to ask permission.
        ASK_USER_QUESTION_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        // Same reasoning: the decision the tool blocks on *is* the authorization, and it authorizes
        // a session mode rather than an action on a resource, so it must not classify as one
        // (`add-agent-plan-exit-request` D2).
        EXIT_PLAN_MODE_TOOL_NAME => (Action::file_read(), Resource::new(tool_name)),
        FILE_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let reading = input.get("operation").and_then(Value::as_str) == Some("read");
            // A generic file tool aimed at the memory directory is a memory operation, not a
            // workspace one (`migrate-agent-memory-to-file-store`): it maps onto the same
            // action/resource pair as `remember` and `recall`, so correcting or retracting a memory
            // is auto-approved exactly as saving one already was. Paths outside keep whatever
            // approval they required before.
            if is_within_memory_directory(path) {
                let action = if reading {
                    Action::file_read()
                } else {
                    Action::memory_write()
                };
                return (action, Resource::memory());
            }
            let resource = Resource::file_path(path);
            if reading {
                (Action::file_read(), resource)
            } else {
                (Action::file_write(), resource)
            }
        }
        GREP_TOOL_NAME | GLOB_TOOL_NAME | SEARCH_CODE_TOOL_NAME => {
            (Action::file_read(), Resource::workspace())
        }
        EDIT_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            if is_within_memory_directory(path) {
                return (Action::memory_write(), Resource::memory());
            }
            (Action::file_write(), Resource::file_path(path))
        }
        // Classified per operation, like the file tool: reading a notebook is a read, and the three
        // that rewrite it are writes against the same path.
        NOTEBOOK_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            let resource = Resource::file_path(path);
            match input.get("operation").and_then(Value::as_str) {
                Some("read") => (Action::file_read(), resource),
                _ => (Action::file_write(), resource),
            }
        }
        FIND_DEFINITION_TOOL_NAME
        | FIND_REFERENCES_TOOL_NAME
        | GET_HOVER_TOOL_NAME
        | GET_DIAGNOSTICS_TOOL_NAME => {
            let path = input.get("path").and_then(Value::as_str).unwrap_or("");
            (Action::file_read(), Resource::file_path(path))
        }
        REMEMBER_TOOL_NAME => (Action::memory_write(), Resource::memory()),
        RECALL_TOOL_NAME => (Action::file_read(), Resource::memory()),
        LIST_SKILLS_TOOL_NAME | LOAD_SKILL_TOOL_NAME | READ_SKILL_RESOURCE_TOOL_NAME => {
            (Action::file_read(), Resource::new(tool_name))
        }
        name if name.starts_with(MCP_TOOL_NAME_PREFIX) => (Action::mcp_tool(), Resource::new(name)),
        name => (Action::new(format!("unknown:{name}")), Resource::new(name)),
    }
}

/// Validates a question, publishes it, and blocks until the user answers.
///
/// Validation happens before anything is published, so a malformed call neither renders a card
/// nor blocks the generation. The non-interactive refusal is repeated here rather than left to the
/// catalog because the catalog only shapes what the model is *told* -- nothing stops it requesting
/// a tool it was never offered, and in an unattended attempt that request would hang until the
/// attempt's ceiling fired (`add-agent-user-question` D4).
#[allow(clippy::result_large_err)]
fn ask_user_question(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    interactive: bool,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
) -> Result<ToolExecutionOutcome, GenerationProcessEvent> {
    if !interactive {
        return Ok(ToolExecutionOutcome {
            output: "There is no interactive user in this execution context, so a question cannot \
                     be answered here. Decide using the information you have, state the assumption \
                     you made, and continue."
                .to_string(),
            is_error: true,
        });
    }
    if let Err(message) = validate_question_input(input) {
        return Ok(ToolExecutionOutcome {
            output: message,
            is_error: true,
        });
    }

    tool_use.status = "awaiting_input".to_string();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    match await_approval(&tool_use.id, cancelled, pending_approvals) {
        ApprovalOutcome::Answered(answer) => Ok(ToolExecutionOutcome {
            output: answer,
            is_error: false,
        }),
        ApprovalOutcome::Cancelled => Err(failed_non_retryable(
            "Generation was cancelled while a question was awaiting an answer.",
        )),
        // Approve/deny arriving for a question means the two resolution paths were crossed. There
        // is no answer to return, so the call fails rather than inventing one.
        ApprovalOutcome::Approved | ApprovalOutcome::Denied => Ok(ToolExecutionOutcome {
            output: "The question was dismissed without an answer.".to_string(),
            is_error: true,
        }),
    }
}

/// Blocks on the user's decision to leave plan mode. Shaped like `ask_user_question` -- publish,
/// wait, report -- but resolved as an approval rather than an answer, because an answer is a string
/// the model interprets and would leave every later generation still resolving the read-only
/// catalog (`add-agent-plan-exit-request` D1).
#[allow(clippy::result_large_err)]
fn request_plan_exit(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    interactive: bool,
    plan_mode: bool,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
) -> Result<ToolExecutionOutcome, GenerationProcessEvent> {
    // Reachable even though the catalog only offers this in plan mode: a model can name any tool,
    // and a stale turn can replay one. Outside plan mode there is nothing to leave.
    if !plan_mode {
        return Ok(ToolExecutionOutcome {
            output: "This session is not in plan mode, so there is nothing to leave. You already \
                     have your full tool set; continue with the work."
                .to_string(),
            is_error: true,
        });
    }
    if !interactive {
        return Ok(ToolExecutionOutcome {
            output:
                "There is no interactive user in this execution context, so no one can approve \
                     leaving plan mode. Finish the planning you were asked for and report it."
                    .to_string(),
            is_error: true,
        });
    }
    if let Err(message) = validate_plan_input(input) {
        return Ok(ToolExecutionOutcome {
            output: message,
            is_error: true,
        });
    }

    tool_use.status = "awaiting_input".to_string();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    match await_approval(&tool_use.id, cancelled, pending_approvals) {
        // The catalog for this generation was resolved before the call and is not re-resolved, so
        // the write tools are genuinely absent until the next turn -- say so rather than let the
        // model discover it by calling a tool it was never given (D3).
        ApprovalOutcome::Approved => Ok(ToolExecutionOutcome {
            output: "The user approved your plan and this session has left plan mode. \
                     Write-capable tools become available on your next turn, not this one, so end \
                     your turn now instead of trying to start the work here."
                .to_string(),
            is_error: false,
        }),
        ApprovalOutcome::Denied => Ok(ToolExecutionOutcome {
            output:
                "The user did not approve this plan. The session is still in plan mode. Revise \
                     the plan based on what they have told you rather than asking again unchanged."
                    .to_string(),
            is_error: true,
        }),
        ApprovalOutcome::Cancelled => Err(failed_non_retryable(
            "Generation was cancelled while a plan was awaiting approval.",
        )),
        // An answer arriving for an approval means the two resolution paths were crossed. There is
        // no decision to act on, so the call fails rather than inventing one.
        ApprovalOutcome::Answered(_) => Ok(ToolExecutionOutcome {
            output: "The plan approval was dismissed without a decision.".to_string(),
            is_error: true,
        }),
    }
}

fn validate_plan_input(input: &Value) -> Result<(), String> {
    let plan = input
        .get("plan")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if plan.is_empty() {
        return Err("plan must be a non-empty string describing what you will do.".to_string());
    }
    if plan.chars().count() > MAX_PLAN_CHARS {
        return Err(format!(
            "plan is {} characters; the maximum is {MAX_PLAN_CHARS}. Summarize it to what the user \
             needs in order to decide.",
            plan.chars().count()
        ));
    }
    Ok(())
}

fn validate_question_input(input: &Value) -> Result<(), String> {
    let question = input
        .get("question")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if question.is_empty() {
        return Err("question must be a non-empty string.".to_string());
    }
    if question.chars().count() > MAX_QUESTION_CHARS {
        return Err(format!(
            "question is {} characters; the maximum is {MAX_QUESTION_CHARS}.",
            question.chars().count()
        ));
    }
    let Some(options) = input.get("options").and_then(Value::as_array) else {
        return Err("options must be an array of strings.".to_string());
    };
    if options.len() < MIN_QUESTION_OPTIONS || options.len() > MAX_QUESTION_OPTIONS {
        return Err(format!(
            "options must contain between {MIN_QUESTION_OPTIONS} and {MAX_QUESTION_OPTIONS} entries, but {} were given.",
            options.len()
        ));
    }
    for (index, option) in options.iter().enumerate() {
        let Some(text) = option.as_str() else {
            return Err(format!("option {} must be a string.", index + 1));
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Err(format!("option {} is empty.", index + 1));
        }
        if trimmed.chars().count() > MAX_QUESTION_OPTION_CHARS {
            return Err(format!(
                "option {} is {} characters; the maximum is {MAX_QUESTION_OPTION_CHARS}.",
                index + 1,
                trimmed.chars().count()
            ));
        }
    }
    Ok(())
}

enum ApprovalOutcome {
    Approved,
    Denied,
    Cancelled,
    /// A question resolved with the user's answer. Reaching this from the approval gate would mean
    /// an answer was delivered to a call that asked for permission, so that path treats it as a
    /// denial rather than silently proceeding (`add-agent-user-question` D1).
    Answered(String),
}

fn await_approval(
    call_id: &str,
    cancelled: &AtomicBool,
    pending_approvals: &PendingApprovals,
) -> ApprovalOutcome {
    let rx = {
        let (tx, rx) = mpsc::channel();
        match pending_approvals.lock() {
            Ok(mut pending) => {
                pending.insert(call_id.to_string(), tx);
            }
            Err(_) => return ApprovalOutcome::Cancelled,
        }
        rx
    };
    loop {
        if cancelled.load(Ordering::SeqCst) {
            if let Ok(mut pending) = pending_approvals.lock() {
                pending.remove(call_id);
            }
            return ApprovalOutcome::Cancelled;
        }
        match rx.recv_timeout(APPROVAL_POLL_INTERVAL) {
            Ok(ToolApprovalDecision::Approved) => return ApprovalOutcome::Approved,
            Ok(ToolApprovalDecision::Denied) => return ApprovalOutcome::Denied,
            Ok(ToolApprovalDecision::Answered(answer)) => return ApprovalOutcome::Answered(answer),
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return ApprovalOutcome::Cancelled,
        }
    }
}

#[allow(clippy::too_many_arguments)]
/// Formats the fixed rejection message used for every plan-mode enforcement gate below — the
/// same message shape regardless of which tool/operation was denied.
fn plan_mode_denial(what: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!("{what} is disabled in plan mode."),
        is_error: true,
    }
}

/// Parses a tool-call argument that should be an absent-or-non-negative integer (`offset`,
/// `limit`, `context`, `head_limit`), accepting a JSON number that arrived as either an integer
/// or an integral float -- some OpenAI-compatible providers serialize every number as a float on
/// the wire, so `100` and `100.0` must parse identically instead of the float silently falling
/// through `Value::as_u64` (which only recognizes the integer encoding) and being reinterpreted
/// as "absent". Returns `Ok(None)` when the field is absent or JSON `null`, which callers must
/// keep distinct from `Ok(Some(0))` for an explicit zero -- `grep`'s `head_limit == Some(0)` and
/// `file`'s `limit == Some(0)` guards reject the latter as degenerate input rather than reading it
/// as "unbounded" (`None`'s meaning). A value that is present but not a non-negative integer
/// (negative, fractional, or non-numeric) is rejected with the same clear-error shape the tool
/// modules themselves already use for degenerate input, instead of silently collapsing into
/// `None` and widening the effective bound.
fn parse_optional_non_negative_integer_arg(
    input: &Value,
    field: &str,
) -> Result<Option<usize>, ToolExecutionOutcome> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => match non_negative_integer(value) {
            Some(number) => Ok(Some(number)),
            None => Err(ToolExecutionOutcome {
                output: format!("{field} must be a non-negative integer (received {value})."),
                is_error: true,
            }),
        },
    }
}

/// Reads a JSON number as a non-negative integer regardless of whether it was encoded as an
/// integer (`5`) or an integral float (`5.0`) -- `Value::as_u64` alone only recognizes the
/// former. Negative, fractional, non-finite, and non-numeric values all yield `None`.
fn non_negative_integer(value: &Value) -> Option<usize> {
    if let Some(integer) = value.as_u64() {
        return Some(integer as usize);
    }
    let float = value.as_f64()?;
    (float.is_finite() && float >= 0.0 && float.fract() == 0.0).then_some(float as u64 as usize)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ListSkillsInput {
    query: Option<String>,
    #[serde(rename = "type")]
    skill_type: Option<String>,
    delivery: Option<String>,
    availability: Option<String>,
    limit: Option<usize>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct LoadSkillInput {
    id: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReadSkillResourceInput {
    uri: String,
    revision: String,
}

fn invalid_skill_tool_input(name: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: json!({
            "status": "error",
            "error": {
                "code": "invalid-input",
                "message": format!("Invalid input for {name}.")
            }
        })
        .to_string(),
        is_error: true,
    }
}

fn valid_skill_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && !value.starts_with('-')
        && !value.ends_with('-')
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

fn valid_skill_resource_uri(value: &str) -> bool {
    if value.len() > 512 || value.contains(['\\', '%']) || value.chars().any(char::is_control) {
        return false;
    }
    let Some(path) = value.strip_prefix("skill://") else {
        return false;
    };
    let mut components = path.split('/');
    let Some(id) = components.next() else {
        return false;
    };
    let Some(directory) = components.next() else {
        return false;
    };
    let resources = components.collect::<Vec<_>>();
    valid_skill_identifier(id)
        && matches!(directory, "scripts" | "references" | "templates" | "assets")
        && !resources.is_empty()
        && resources.iter().all(|component| {
            !component.is_empty()
                && *component != "."
                && *component != ".."
                && !component.starts_with('.')
                && component.chars().count() <= 240
        })
}

fn execute_skill_read(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    skills: &dyn AgentSkillPort,
) -> ToolExecutionOutcome {
    let request = match name {
        LIST_SKILLS_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<ListSkillsInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            let valid = input
                .query
                .as_deref()
                .is_none_or(|query| query.chars().count() <= 80)
                && input.limit.is_none_or(|limit| (1..=100).contains(&limit))
                && input
                    .skill_type
                    .as_deref()
                    .is_none_or(|value| matches!(value, "role" | "utility"))
                && input
                    .delivery
                    .as_deref()
                    .is_none_or(|value| matches!(value, "eager" | "on-demand"))
                && input.availability.as_deref().is_none_or(|value| {
                    matches!(
                        value,
                        "available" | "disabled" | "invalid" | "conflicting" | "unsupported"
                    )
                });
            if !valid {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::List {
                workspace_path: workspace_folder.map(str::to_string),
                query: input.query,
                skill_type: input.skill_type,
                delivery: input.delivery,
                availability: input.availability,
                limit: input.limit,
            }
        }
        LOAD_SKILL_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<LoadSkillInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            if !valid_skill_identifier(&input.id) {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::Load {
                workspace_path: workspace_folder.map(str::to_string),
                id_or_alias: input.id,
            }
        }
        READ_SKILL_RESOURCE_TOOL_NAME => {
            let Ok(input) = serde_json::from_value::<ReadSkillResourceInput>(input.clone()) else {
                return invalid_skill_tool_input(name);
            };
            if !valid_skill_resource_uri(&input.uri)
                || input.revision.is_empty()
                || input.revision.len() > 128
                || input.revision.chars().any(char::is_control)
            {
                return invalid_skill_tool_input(name);
            }
            AgentSkillReadRequest::ReadResource {
                workspace_path: workspace_folder.map(str::to_string),
                uri: input.uri,
                revision: input.revision,
            }
        }
        _ => return invalid_skill_tool_input(name),
    };
    let outcome = skills.execute_read(request);
    ToolExecutionOutcome {
        output: outcome.output,
        is_error: outcome.is_error,
    }
}

struct NativeToolOperationRecorder {
    repository: Option<SqliteNativeToolRepository>,
    events: Option<tauri::AppHandle>,
    record: Mutex<StoredToolOperation>,
}

impl std::fmt::Debug for NativeToolOperationRecorder {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeToolOperationRecorder")
            .finish_non_exhaustive()
    }
}

impl NativeToolOperationRecorder {
    fn new(
        repository: Option<&SqliteNativeToolRepository>,
        events: Option<&tauri::AppHandle>,
        request: &GenerationProcessRequest,
        tool_use: &ToolUseBlock,
    ) -> Self {
        let now = chrono::Utc::now().to_rfc3339();
        let recorder = Self {
            repository: repository.cloned(),
            events: events.cloned(),
            record: Mutex::new(StoredToolOperation {
                contract_version: 1,
                id: tool_use.id.clone(),
                session_id: request.session.id.clone(),
                generation_id: request.operation_id.clone(),
                tool_name: tool_use.name.clone(),
                status: StoredToolOperationStatus::Queued,
                progress_sequence: 0,
                progress_message: None,
                result_artifact_ids: Vec::new(),
                error_code: None,
                created_at: now.clone(),
                updated_at: now,
            }),
        };
        recorder.persist();
        recorder
    }

    fn transition(
        &self,
        status: StoredToolOperationStatus,
        message: Option<String>,
        artifact_ids: Vec<String>,
        error_code: Option<String>,
    ) {
        if let Ok(mut record) = self.record.lock() {
            record.progress_sequence = record.progress_sequence.saturating_add(1);
            record.status = status;
            record.progress_message = message;
            record.result_artifact_ids = artifact_ids;
            record.error_code = error_code;
            record.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.persist();
    }

    fn persist(&self) {
        let Ok(record) = self.record.lock().map(|record| record.clone()) else {
            return;
        };
        if let Some(repository) = &self.repository {
            let _ = repository.save_operation(&record);
        }
        if let Some(events) = &self.events {
            let _ = events.emit("builtin-tool-operation", operation_event(&record));
        }
    }
}

impl NativeToolProgressSink for NativeToolOperationRecorder {
    fn publish(&self, progress: NativeToolProgress) {
        if let Ok(mut record) = self.record.lock() {
            record.progress_sequence = record
                .progress_sequence
                .saturating_add(1)
                .max(progress.sequence.saturating_add(2));
            record.status = if progress.phase == NativeToolProgressPhase::AwaitingHuman {
                StoredToolOperationStatus::AwaitingHuman
            } else {
                StoredToolOperationStatus::Running
            };
            record.progress_message = progress.message;
            record.updated_at = chrono::Utc::now().to_rfc3339();
        }
        self.persist();
    }
}

#[allow(clippy::too_many_arguments, clippy::result_large_err)]
fn execute_registered_native_tool(
    tool_use: &mut ToolUseBlock,
    input: &Value,
    request: &GenerationProcessRequest,
    cancelled: Arc<AtomicBool>,
    registry: &NativeToolRegistry,
    operations: Option<&SqliteNativeToolRepository>,
    events: Option<&tauri::AppHandle>,
    permissions: &dyn AgentPermissionPort,
    pending_approvals: &PendingApprovals,
    sink: &dyn AgentProcessEventSink,
    plan_mode: bool,
) -> Result<(ToolExecutionOutcome, Option<String>), GenerationProcessEvent> {
    let recorder = Arc::new(NativeToolOperationRecorder::new(
        operations, events, request, tool_use,
    ));
    let authority = ToolEligibilityContext {
        agent_id: request.agent.id.clone(),
        session_id: request.session.id.clone(),
        generation_id: request.operation_id.clone(),
        canonical_workspace: request.session.folder.as_deref().map(Into::into),
        execution_mode: if plan_mode {
            NativeToolExecutionMode::Plan
        } else {
            NativeToolExecutionMode::Execute
        },
        readiness: registry.readiness_snapshot(),
    };
    let execution = NativeToolExecutionContext {
        call_id: tool_use.id.clone(),
        session_id: authority.session_id.clone(),
        generation_id: authority.generation_id.clone(),
        agent_id: authority.agent_id.clone(),
        canonical_workspace: authority.canonical_workspace.clone(),
        deadline: Instant::now() + REQUEST_TIMEOUT,
        cancelled: cancelled.clone(),
        progress: recorder.clone(),
    };
    let dispatcher = NativeToolDispatcher::new(registry.clone());
    let prepared = match dispatcher.prepare(NativeToolDispatchRequest {
        tool_name: tool_use.name.clone(),
        input: input.clone(),
        authority,
        execution,
    }) {
        Ok(prepared) => prepared,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    let project_key = request.session.folder.as_deref().unwrap_or("");
    let mut witness = match dispatcher.authorize(&prepared, permissions, project_key) {
        Ok(witness) => witness,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    if witness.status == NativeToolAuthorizationStatus::AwaitingApproval {
        recorder.transition(
            StoredToolOperationStatus::AwaitingApproval,
            None,
            Vec::new(),
            None,
        );
        tool_use.status = "awaiting_approval".to_owned();
        if sink
            .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
            .is_err()
        {
            return Err(failed_retryable("Agent generation event handling failed."));
        }
        match await_approval(&tool_use.id, &cancelled, pending_approvals) {
            ApprovalOutcome::Approved => {
                witness.status = NativeToolAuthorizationStatus::Allowed;
            }
            // An answer delivered here means the approval and question resolution paths were
            // crossed; fail closed rather than treat it as consent.
            ApprovalOutcome::Denied | ApprovalOutcome::Answered(_) => {
                recorder.transition(
                    StoredToolOperationStatus::Failed,
                    None,
                    Vec::new(),
                    Some("permission_denied".to_owned()),
                );
                return Ok((native_dispatch_error("Denied by user.".to_owned()), None));
            }
            ApprovalOutcome::Cancelled => {
                recorder.transition(
                    StoredToolOperationStatus::Cancelled,
                    None,
                    Vec::new(),
                    Some("cancelled".to_owned()),
                );
                return Err(failed_non_retryable(
                    "Generation was cancelled while a tool call was awaiting approval.",
                ));
            }
        }
    }
    recorder.transition(StoredToolOperationStatus::Running, None, Vec::new(), None);
    tool_use.status = "running".to_owned();
    if sink
        .handle(GenerationProcessEvent::ToolUse(tool_use.clone()))
        .is_err()
    {
        return Err(failed_retryable("Agent generation event handling failed."));
    }
    let result = match dispatcher.execute_authorized(prepared, &witness) {
        Ok(result) => result,
        Err(error) => {
            recorder.transition(
                StoredToolOperationStatus::Failed,
                None,
                Vec::new(),
                Some(error.code.as_str().to_owned()),
            );
            return Ok((native_dispatch_error(error.safe_message), None));
        }
    };
    let is_error = result.status != NativeToolResultStatus::Succeeded;
    recorder.transition(
        stored_status(&result),
        None,
        artifact_ids(&result),
        result.error_code.map(|code| code.as_str().to_owned()),
    );
    let image_artifact_id = result
        .metadata
        .get(IMAGE_ARTIFACT_METADATA_KEY)
        .and_then(Value::as_str)
        .map(str::to_owned);
    let output = match (result.output, result.safe_error) {
        (Some(value), _) => serde_json::to_string(&value)
            .unwrap_or_else(|_| "The native tool result could not be encoded.".to_owned()),
        (None, Some(message)) => message,
        (None, None) => "The native tool returned no result.".to_owned(),
    };
    Ok((ToolExecutionOutcome { output, is_error }, image_artifact_id))
}

/// Resolves the Artifact a native tool named as its image and prepares it for the wire.
///
/// Returns `None` whenever the image cannot be attached -- the model does not accept images, the
/// per-request budget is spent, the Artifact cannot be read, or its bytes are not a reviewed image
/// type. Every one of those degrades to the tool's existing non-image result rather than failing
/// the call: a model choice or a budget must never turn a working tool into an error
/// (`add-onepiece-visual-tool-returns`).
fn resolve_tool_image(
    artifacts: Option<&ArtifactService>,
    artifact_id: &str,
    images_supported: bool,
    images_in_request: usize,
) -> Option<AgentImage> {
    if !images_supported || images_in_request >= MAX_IMAGES_PER_REQUEST {
        return None;
    }
    let (bytes, media_type) = artifacts?.read_bytes(artifact_id).ok()?;
    prepare_image(&bytes, Some(&media_type)).ok()
}

fn stored_status(result: &NativeToolResultEnvelope) -> StoredToolOperationStatus {
    match result.status {
        NativeToolResultStatus::Succeeded => StoredToolOperationStatus::Succeeded,
        NativeToolResultStatus::Cancelled => StoredToolOperationStatus::Cancelled,
        _ => StoredToolOperationStatus::Failed,
    }
}

fn artifact_ids(result: &NativeToolResultEnvelope) -> Vec<String> {
    fn visit(value: &Value, ids: &mut Vec<String>) {
        if ids.len() >= 64 {
            return;
        }
        match value {
            Value::String(value) if value.starts_with("artifact-") => {
                if !ids.contains(value) {
                    ids.push(value.clone());
                }
            }
            Value::Array(values) => values.iter().for_each(|value| visit(value, ids)),
            Value::Object(values) => values.values().for_each(|value| visit(value, ids)),
            _ => {}
        }
    }

    let mut ids = Vec::new();
    if let Some(output) = &result.output {
        visit(output, &mut ids);
    }
    ids
}

fn operation_event(record: &StoredToolOperation) -> Value {
    let progress = record.progress_message.as_ref().map(|message| {
        json!({
            "phase": message,
            "completedUnits": record.progress_sequence,
            "totalUnits": Value::Null,
            "messageCode": Value::Null
        })
    });
    json!({
        "kind": "snapshot",
        "operation": {
            "id": record.id,
            "agentId": "onepiece",
            "sessionId": record.session_id,
            "capability": native_tool_capability(&record.tool_name),
            "operation": record.tool_name,
            "status": match record.status {
                StoredToolOperationStatus::AwaitingApproval => "queued",
                other => other.as_str(),
            },
            "progress": progress,
            "artifactIds": record.result_artifact_ids,
            "errorCode": record.error_code,
            "simulated": false,
            "createdAt": record.created_at,
            "updatedAt": record.updated_at
        }
    })
}

fn native_tool_capability(tool_name: &str) -> &'static str {
    match tool_name {
        "browser" => "browser",
        "web_search" | "web_fetch" => "web",
        "code_execution" => "code_execution",
        "ocr" => "ocr",
        "artifact" => "artifact",
        "delegate_cli" | "apply_delegation_changes" => "delegation",
        _ => "filesystem",
    }
}

fn native_dispatch_error(message: String) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message,
        is_error: true,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_tool_call_with_runtime_ports(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    cancelled: Arc<AtomicBool>,
    agent_id: &str,
    memories: &dyn AgentMemoryPort,
    mcp: &dyn AgentMcpToolPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: &dyn AgentCodeIntelligencePort,
    workspace_mutations: &dyn AgentWorkspaceMutationPort,
    plan_mode: bool,
    skills: &dyn AgentSkillPort,
    utility_delegation: Option<&UtilityDelegationApplicationService>,
    generation: &GenerationProcessRequest,
) -> ToolExecutionOutcome {
    if name == DELEGATE_UTILITY_SKILL_TOOL_NAME {
        return execute_utility_delegation(input, cancelled, utility_delegation, generation);
    }
    execute_tool_call_impl(
        name,
        input,
        workspace_folder,
        cancelled,
        agent_id,
        memories,
        mcp,
        retrieval,
        Some(code_intelligence),
        Some(workspace_mutations),
        plan_mode,
        skills,
        Some(generation.session.id.as_str()),
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct UtilityDelegationToolInput {
    skill_id: String,
    task: String,
    duration_ms: Option<u64>,
    tool_calls: Option<u32>,
    approvals: Option<u32>,
    result_chars: Option<usize>,
}

fn execute_utility_delegation(
    input: &Value,
    cancelled: Arc<AtomicBool>,
    service: Option<&UtilityDelegationApplicationService>,
    generation: &GenerationProcessRequest,
) -> ToolExecutionOutcome {
    let Some(service) = service else {
        return ToolExecutionOutcome {
            output: json!({"status":"refused","reason":"native-runtime-unavailable"}).to_string(),
            is_error: true,
        };
    };
    let parsed: UtilityDelegationToolInput = match serde_json::from_value(input.clone()) {
        Ok(value) => value,
        Err(_) => {
            return ToolExecutionOutcome {
                output: json!({"status":"refused","reason":"invalid-input"}).to_string(),
                is_error: true,
            }
        }
    };
    let defaults = UtilityDelegationLimits::default();
    let limits = match UtilityDelegationLimits::bounded(
        parsed.duration_ms.unwrap_or(defaults.duration_ms),
        parsed.tool_calls.unwrap_or(defaults.tool_calls),
        parsed.approvals.unwrap_or(defaults.approvals),
        parsed.result_chars.unwrap_or(defaults.result_chars),
    ) {
        Ok(value) => value,
        Err(_) => {
            return ToolExecutionOutcome {
                output: json!({"status":"refused","reason":"invalid-limits"}).to_string(),
                is_error: true,
            }
        }
    };
    let request = UtilityDelegationRequest {
        agent_id: generation.agent.id.clone(),
        skill_id: parsed.skill_id,
        task: parsed.task,
        parent_run_id: generation.execution_context.run_id.as_str().to_string(),
        parent_span_id: generation.execution_context.span_id.as_str().to_string(),
        session_id: generation.session.id.clone(),
        message_id: generation.message_id.clone(),
        canonical_workspace: generation.session.folder.clone(),
        depth: 0,
        limits,
    };
    match service.execute(request, cancelled) {
        Ok(result) => ToolExecutionOutcome {
            output: json!({
                "status": result.terminal.as_str(),
                "delegationId": result.delegation_id,
                "attemptId": result.attempt_id,
                "skillId": result.skill_id,
                "revision": result.revision,
                "summary": result.summary,
                "durationMs": result.duration_ms,
                "toolCount": result.counts.tool_calls,
                "approvalCount": result.counts.approvals,
                "limitReason": result.limit_reason,
            })
            .to_string(),
            is_error: result.terminal
                != crate::contexts::agent_runtime::domain::UtilityDelegationTerminal::Succeeded,
        },
        Err(_) => ToolExecutionOutcome {
            output: json!({"status":"refused","reason":"utility-resolution-failed"}).to_string(),
            is_error: true,
        },
    }
}

#[allow(clippy::too_many_arguments)]
/// The shared refusal for a background-command tool used by a session the runtime could not
/// identify. Background state is keyed by session, so without one there is no safe scope to read
/// or terminate within -- failing closed beats guessing an owner.
fn background_unavailable(reason: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!("Background commands are unavailable for this session: {reason}."),
        is_error: true,
    }
}

/// Whether this tool call is a `file` read of a reviewed image type. Both halves matter: the
/// file tool's other operations are unaffected, and a non-image read must not detour through the
/// image path (`add-agent-image-input`).
fn is_image_read_request(tool_name: &str, input: &Value) -> bool {
    if tool_name != FILE_TOOL_NAME {
        return false;
    }
    if input.get("operation").and_then(Value::as_str) != Some("read") {
        return false;
    }
    input
        .get("path")
        .and_then(Value::as_str)
        .is_some_and(is_reviewed_image_path)
}

/// Records that an image was attached, carrying its hash, media type, dimensions, and byte count
/// only. The bytes never reach a durable log: a single screenshot base64-encodes to more than the
/// whole log-line budget, so this is a size constraint as much as a privacy one.
fn log_image_attachment(
    logging: &dyn AgentLoggingPort,
    clock: &dyn AgentClockPort,
    request: &GenerationProcessRequest,
    call_id: &str,
    image: &AgentImage,
) {
    let _ = logging.record(AgentLog {
        level: AgentLogLevel::Debug,
        category: "session.runtime.api.image".to_string(),
        message: format!(
            "Attached image to tool call {call_id}: {} {}x{} {} bytes sha256:{}",
            image.media_type().as_str(),
            image.width(),
            image.height(),
            image.byte_len(),
            image.content_hash()
        ),
        agent_id: Some(request.agent.id.clone()),
        session_id: Some(request.session.id.clone()),
        operation_id: Some(request.operation_id.clone()),
        run_id: None,
        trace_id: None,
        span_id: None,
        occurred_at: clock.now(),
    });
}

fn execute_todo_write(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let Some(todos) = input.get("todos").and_then(Value::as_array) else {
        return ToolExecutionOutcome {
            output: "todos must be an array of {content, status} objects.".to_string(),
            is_error: true,
        };
    };
    let mut submitted = Vec::with_capacity(todos.len());
    for (index, todo) in todos.iter().enumerate() {
        let Some(content) = todo.get("content").and_then(Value::as_str) else {
            return ToolExecutionOutcome {
                output: format!("Task {} is missing a string content field.", index + 1),
                is_error: true,
            };
        };
        let Some(status) = todo.get("status").and_then(Value::as_str) else {
            return ToolExecutionOutcome {
                output: format!("Task {} is missing a string status field.", index + 1),
                is_error: true,
            };
        };
        submitted.push((content.to_owned(), status.to_owned()));
    }
    // Validation happens before any store access, so a rejected write provably leaves the
    // previous list untouched rather than half-applied.
    let items = match validate_task_list(&submitted) {
        Ok(items) => items,
        Err(error) => {
            return ToolExecutionOutcome {
                output: error.message(),
                is_error: true,
            }
        }
    };
    let Some(session_id) = session_id else {
        return ToolExecutionOutcome {
            output: "The task list is unavailable because this session has no runtime identity."
                .to_string(),
            is_error: true,
        };
    };
    let stored = task_list_store().replace(session_id, items);
    ToolExecutionOutcome {
        output: if stored.is_empty() {
            "Task list cleared.".to_string()
        } else {
            format!("Task list updated.\n{}", render_task_list(&stored))
        },
        is_error: false,
    }
}

/// Shortens a command for a one-line status header. Cuts on a character boundary so a multi-byte
/// character is never split into replacement characters.
fn truncate_for_label(command: &str) -> String {
    const MAX_LABEL_CHARS: usize = 100;
    if command.chars().count() <= MAX_LABEL_CHARS {
        return command.to_owned();
    }
    let head: String = command.chars().take(MAX_LABEL_CHARS).collect();
    format!("{head}...")
}

fn required_handle_arg(input: &Value) -> Result<&str, ToolExecutionOutcome> {
    match input.get("shell_id").and_then(Value::as_str) {
        Some(handle) if !handle.trim().is_empty() => Ok(handle),
        _ => Err(ToolExecutionOutcome {
            output: "shell_id must be the handle string returned when the background command was started.".to_string(),
            is_error: true,
        }),
    }
}

fn execute_shell_in_background(
    command: &str,
    workspace_folder: &str,
    session_id: Option<&str>,
) -> ToolExecutionOutcome {
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    match background_shell_registry().start(session_id, command, workspace_folder) {
        Ok(handle) => ToolExecutionOutcome {
            output: format!(
                "Started background command {handle}. It keeps running after this tool call \
                 returns. Read its output with shell_output(shell_id: \"{handle}\") and stop it \
                 with shell_kill(shell_id: \"{handle}\")."
            ),
            is_error: false,
        },
        Err(BackgroundStartError::SessionLimitReached) => ToolExecutionOutcome {
            output: format!(
                "This session already has {MAX_BACKGROUND_COMMANDS_PER_SESSION} background \
                 commands running. Stop one with shell_kill before starting another."
            ),
            is_error: true,
        },
        Err(BackgroundStartError::Spawn) => ToolExecutionOutcome {
            output: "The background command could not be started.".to_string(),
            is_error: true,
        },
    }
}

fn execute_shell_output(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let handle = match required_handle_arg(input) {
        Ok(handle) => handle,
        Err(outcome) => return outcome,
    };
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    let registry = background_shell_registry();
    let command = registry.command_label(session_id, handle);
    let Ok(output) = registry.take_output(session_id, handle) else {
        return unknown_background_handle(handle);
    };

    // Naming the command in the header matters once several handles are in flight: a status line
    // that says only "bg_3 running" leaves the model to remember which of them is the build.
    let mut report = match command {
        Some(command) => format!(
            "[{handle}] {} — {}",
            output.status.label(),
            truncate_for_label(&command)
        ),
        None => format!("[{handle}] {}", output.status.label()),
    };
    if output.dropped_bytes > 0 {
        report.push_str(&format!(
            "\n[{} earlier bytes were dropped: the command produced output faster than it was read]",
            output.dropped_bytes
        ));
    }
    if output.remaining_bytes > 0 {
        report.push_str(&format!(
            "\n[{} more bytes are buffered; call shell_output again to continue reading]",
            output.remaining_bytes
        ));
    }
    if output.text.is_empty() {
        report.push_str("\n(no new output)");
    } else {
        report.push('\n');
        report.push_str(&output.text);
    }
    ToolExecutionOutcome {
        output: report,
        // A non-zero exit is information about the command, not a failure of this tool: reporting
        // it as a tool error would make a failing build indistinguishable from a broken handle.
        is_error: false,
    }
}

fn execute_shell_kill(input: &Value, session_id: Option<&str>) -> ToolExecutionOutcome {
    let handle = match required_handle_arg(input) {
        Ok(handle) => handle,
        Err(outcome) => return outcome,
    };
    let Some(session_id) = session_id else {
        return background_unavailable("this session has no runtime identity");
    };
    match background_shell_registry().kill(session_id, handle) {
        Ok(KillOutcome::Terminated(status)) => ToolExecutionOutcome {
            output: format!("Background command {handle} and its child processes were terminated. Status: {}.", status.label()),
            is_error: false,
        },
        Ok(KillOutcome::AlreadyFinished(status)) => ToolExecutionOutcome {
            output: format!(
                "Background command {handle} had already finished, so nothing was terminated. Status: {}.",
                status.label()
            ),
            is_error: false,
        },
        Err(_) => unknown_background_handle(handle),
    }
}

fn unknown_background_handle(handle: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: format!(
            "No background command {handle} belongs to this session. Handles do not survive a \
             desktop restart and cannot be used from another session."
        ),
        is_error: true,
    }
}

#[allow(clippy::too_many_arguments)]
fn execute_tool_call_impl(
    name: &str,
    input: &Value,
    workspace_folder: Option<&str>,
    cancelled: Arc<AtomicBool>,
    agent_id: &str,
    memories: &dyn AgentMemoryPort,
    mcp: &dyn AgentMcpToolPort,
    retrieval: &dyn AgentRetrievalPort,
    code_intelligence: Option<&dyn AgentCodeIntelligencePort>,
    workspace_mutations: Option<&dyn AgentWorkspaceMutationPort>,
    plan_mode: bool,
    skills: &dyn AgentSkillPort,
    // Owning session for background commands (`add-background-shell-execution`). Never
    // model-supplied: a handle resolves only within the session that started it, so accepting a
    // session id as a tool argument would let one session read or kill another's processes.
    session_id: Option<&str>,
) -> ToolExecutionOutcome {
    let registered_handler = ExistingToolHandlerRegistry::resolve(name);
    if registered_handler == Some(ExistingToolHandler::SkillRead) {
        return execute_skill_read(name, input, workspace_folder, skills);
    }
    // `remember` has no dependency on a workspace folder — unlike shell/file, it only ever
    // touches this app's own storage — so it's handled before the workspace-folder gate below,
    // and a folder-less session can still save agent-global memories (`add-agent-cross-session-memory`).
    // It is also the one tool plan mode never restricts — see `tool_catalog::plan_mode_tool_catalog`.
    if registered_handler == Some(ExistingToolHandler::Remember) {
        return execute_remember(input, agent_id, workspace_folder, memories, retrieval);
    }
    // `recall` is handled in the same spot for the same reason: it only ever reads this app's own
    // storage, never the workspace filesystem, so it needs neither a workspace folder nor a
    // plan-mode restriction. It also needs no `agent_id`/`workspace_folder`: memories are one
    // host-level shared pool (`agent-memory-shared-pool`), so there is no slice of it to name.
    if registered_handler == Some(ExistingToolHandler::Recall) {
        return execute_recall(input, retrieval);
    }
    // Handled beside remember/recall for the same reason: it touches only VaneHub-internal
    // session state, so it needs neither a workspace folder nor a plan-mode restriction.
    if registered_handler == Some(ExistingToolHandler::TodoWrite) {
        return execute_todo_write(input, session_id);
    }
    if registered_handler == Some(ExistingToolHandler::SearchCode) {
        let Some(folder) = workspace_folder else {
            return ToolExecutionOutcome {
                output: "Code search is unavailable because this session has no workspace folder."
                    .to_string(),
                is_error: true,
            };
        };
        let Some(code_retrieval) = retrieval.code_retrieval() else {
            return ToolExecutionOutcome {
                output: "Code search is not enabled for this workspace.".to_string(),
                is_error: true,
            };
        };
        return execute_search_code(input, folder, code_retrieval);
    }
    // Plan mode (`add-agent-chat-configuration`) excludes MCP-sourced tools and `shell` from the
    // catalog entirely, and narrows `file` to `read` — but the catalog only shapes what the model
    // is *told* it can do. This is the actual enforcement boundary: nothing stops a model from
    // requesting a tool/operation it was never offered (hallucination, or prompt injection from
    // earlier tool output), so every one of these is re-checked here regardless of the catalog.
    if plan_mode && registered_handler == Some(ExistingToolHandler::Mcp) {
        return plan_mode_denial("MCP tools");
    }
    // MCP tools are similarly folder-independent: a user-scoped MCP server has no project
    // affiliation at all, so a folder-less session can still reach it (`add-agent-mcp-tools`).
    // `mcp.call_tool` re-derives visibility itself (`workspace_folder.unwrap_or_default()` mirrors
    // the CLI relay's own `project_path.unwrap_or_default()` precedent), so no separate check here.
    if registered_handler == Some(ExistingToolHandler::Mcp) {
        let outcome = mcp.call_tool(workspace_folder.unwrap_or_default(), name, input, cancelled);
        return ToolExecutionOutcome {
            output: outcome.output,
            is_error: outcome.is_error,
        };
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::Shell) {
        return plan_mode_denial("Shell commands");
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::ShellKill) {
        return plan_mode_denial("Terminating background commands");
    }
    // Reading a background command's output needs no workspace folder: the command was started
    // with one, and retrieval only touches this process's own buffers. Handled before the
    // folder gate for the same reason `remember`/`recall` are.
    if registered_handler == Some(ExistingToolHandler::ShellOutput) {
        return execute_shell_output(input, session_id);
    }
    if registered_handler == Some(ExistingToolHandler::ShellKill) {
        return execute_shell_kill(input, session_id);
    }
    if plan_mode && registered_handler == Some(ExistingToolHandler::Edit) {
        return plan_mode_denial("Editing files");
    }
    // The plan-mode catalog offers a read-only notebook, but the catalog only shapes what the model
    // is told; this is the boundary that holds if it asks for an operation it was never offered.
    if plan_mode
        && registered_handler == Some(ExistingToolHandler::Notebook)
        && input.get("operation").and_then(Value::as_str) != Some("read")
    {
        return plan_mode_denial("Editing notebooks");
    }
    let Some(folder) = workspace_folder else {
        return ToolExecutionOutcome {
            output: "This session has no workspace folder configured.".to_string(),
            is_error: true,
        };
    };
    if registered_handler == Some(ExistingToolHandler::CodeIntelligence) {
        let Some(code_intelligence) = code_intelligence else {
            return ToolExecutionOutcome {
                output: "Code intelligence is unavailable for this session.".to_owned(),
                is_error: true,
            };
        };
        return execute_code_intelligence_tool(name, input, folder, cancelled, code_intelligence);
    }
    match registered_handler {
        Some(ExistingToolHandler::Shell) => {
            let command = input
                .get("command")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if input
                .get("run_in_background")
                .and_then(Value::as_bool)
                .unwrap_or(false)
            {
                return execute_shell_in_background(command, folder, session_id);
            }
            let timeout_ms = match parse_optional_non_negative_integer_arg(input, "timeout_ms") {
                Ok(timeout_ms) => timeout_ms.map(|value| value as u64),
                Err(outcome) => return outcome,
            };
            execute_shell(command, folder, cancelled, timeout_ms)
        }
        Some(ExistingToolHandler::File) => {
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if plan_mode && operation != "read" {
                return plan_mode_denial("Writing files");
            }
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let content = input.get("content").and_then(Value::as_str);
            let offset = match parse_optional_non_negative_integer_arg(input, "offset") {
                Ok(offset) => offset,
                Err(outcome) => return outcome,
            };
            let limit = match parse_optional_non_negative_integer_arg(input, "limit") {
                Ok(limit) => limit,
                Err(outcome) => return outcome,
            };
            let outcome = execute_file(operation, path, content, offset, limit, folder);
            if operation == "write" && !outcome.is_error {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        Some(ExistingToolHandler::Grep) => {
            let context = match parse_optional_non_negative_integer_arg(input, "context") {
                Ok(context) => context.unwrap_or(0),
                Err(outcome) => return outcome,
            };
            let head_limit = match parse_optional_non_negative_integer_arg(input, "head_limit") {
                Ok(head_limit) => head_limit,
                Err(outcome) => return outcome,
            };
            execute_grep(
                GrepRequest {
                    pattern: input
                        .get("pattern")
                        .and_then(Value::as_str)
                        .unwrap_or_default(),
                    glob: input.get("glob").and_then(Value::as_str),
                    path: input.get("path").and_then(Value::as_str),
                    output_mode: input
                        .get("output_mode")
                        .and_then(Value::as_str)
                        .unwrap_or(OUTPUT_MODE_FILES),
                    context,
                    case_insensitive: input
                        .get("case_insensitive")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    head_limit,
                },
                folder,
                cancelled,
            )
        }
        Some(ExistingToolHandler::Glob) => execute_glob(
            input
                .get("pattern")
                .and_then(Value::as_str)
                .unwrap_or_default(),
            input.get("path").and_then(Value::as_str),
            folder,
            cancelled,
        ),
        Some(ExistingToolHandler::Notebook) => {
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let operation = input
                .get("operation")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let outcome = execute_notebook(
                NotebookRequest {
                    operation,
                    path,
                    cell_id: input.get("cell_id").and_then(Value::as_str),
                    cell_index: input
                        .get("cell_index")
                        .and_then(Value::as_u64)
                        .and_then(|index| usize::try_from(index).ok()),
                    source: input.get("source").and_then(Value::as_str),
                    cell_type: input.get("cell_type").and_then(Value::as_str),
                    position: input.get("position").and_then(Value::as_str),
                },
                folder,
            );
            if !outcome.is_error && operation != "read" {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        Some(ExistingToolHandler::Edit) => {
            let path = input
                .get("path")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let outcome = execute_edit(
                path,
                input
                    .get("old_string")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                input
                    .get("new_string")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
                input
                    .get("replace_all")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                folder,
            );
            if !outcome.is_error {
                publish_workspace_mutation(folder, path, workspace_mutations);
            }
            outcome
        }
        _ => ToolExecutionOutcome {
            output: format!("Unknown tool \"{name}\"."),
            is_error: true,
        },
    }
}

fn publish_workspace_mutation(
    workspace_folder: &str,
    relative_path: &str,
    workspace_mutations: Option<&dyn AgentWorkspaceMutationPort>,
) {
    let Some(workspace_mutations) = workspace_mutations else {
        return;
    };
    let Ok(boundary) = BoundedFilesystem::new(Path::new(workspace_folder)) else {
        return;
    };
    let Ok(relative_path) = boundary.validate_relative(relative_path) else {
        return;
    };
    let Ok(canonical_workspace) = Path::new(workspace_folder).canonicalize() else {
        return;
    };
    workspace_mutations.publish(AgentWorkspaceMutation {
        canonical_workspace,
        relative_path: relative_path.to_string_lossy().replace('\\', "/"),
    });
}

fn execute_code_intelligence_tool(
    name: &str,
    input: &Value,
    folder: &str,
    cancelled: Arc<AtomicBool>,
    code_intelligence: &dyn AgentCodeIntelligencePort,
) -> ToolExecutionOutcome {
    let relative_path = input
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    if relative_path.is_empty() {
        return invalid_code_intelligence_input("path must be a non-empty relative string");
    }
    let context = AgentCodeIntelligenceContext::from_session_workspace(folder);
    if name == GET_DIAGNOSTICS_TOOL_NAME {
        return diagnostics_outcome(code_intelligence.get_diagnostics(
            &context,
            &AgentDocumentInput { relative_path },
            cancelled,
        ));
    }
    let Some(line) = one_based_u32(input, "line") else {
        return invalid_code_intelligence_input("line must be a one-based integer");
    };
    let Some(column) = one_based_u32(input, "column") else {
        return invalid_code_intelligence_input("column must be a one-based integer");
    };
    let position = AgentDocumentPositionInput {
        relative_path,
        line,
        column,
    };
    match name {
        FIND_DEFINITION_TOOL_NAME => locations_outcome(
            "definitions",
            code_intelligence.find_definition(&context, &position, cancelled),
            20,
        ),
        FIND_REFERENCES_TOOL_NAME => locations_outcome(
            "references",
            code_intelligence.find_references(&context, &position, cancelled),
            50,
        ),
        GET_HOVER_TOOL_NAME => {
            hover_outcome(code_intelligence.get_hover(&context, &position, cancelled))
        }
        _ => invalid_code_intelligence_input("unsupported code-intelligence operation"),
    }
}

fn one_based_u32(input: &Value, field: &str) -> Option<u32> {
    let value = input.get(field)?.as_u64()?;
    (value > 0).then(|| u32::try_from(value).ok()).flatten()
}

fn invalid_code_intelligence_input(message: &str) -> ToolExecutionOutcome {
    ToolExecutionOutcome {
        output: message.to_owned(),
        is_error: true,
    }
}

/// After a successful save, wakes the background indexing worker (`retrieval.
/// notify_source_changed()`) so the new memory is indexed promptly instead of waiting up to one
/// reconcile poll period. That call writes nothing, waits for nothing, and cannot fail by
/// construction (`AgentRetrievalPort::notify_source_changed` returns `()`) — it is skipped
/// entirely on the empty-content rejection path above, since there is no new memory to index and
/// waking the worker would just burn a full two-table reconcile scan for nothing.
fn execute_remember(
    input: &Value,
    agent_id: &str,
    folder: Option<&str>,
    memories: &dyn AgentMemoryPort,
    retrieval: &dyn AgentRetrievalPort,
) -> ToolExecutionOutcome {
    let content = input
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if content.is_empty() {
        return ToolExecutionOutcome {
            output: "No content was provided to remember.".to_string(),
            is_error: true,
        };
    }
    // `name` addresses the memory: saving under one that already exists replaces that file rather
    // than adding a second memory for the same fact. Both stay optional so an older prompt that
    // sends content alone still saves, with the store deriving what it needs.
    let name = input.get("name").and_then(Value::as_str);
    let description = input.get("description").and_then(Value::as_str);
    let memory_type = input
        .get("type")
        .and_then(Value::as_str)
        .and_then(MemoryType::parse);
    match memories.save(SaveMemoryInput {
        agent_id,
        folder,
        name,
        description,
        memory_type,
        content,
        source: MemorySource::Explicit,
    }) {
        Ok(()) => {
            retrieval.notify_source_changed();
            ToolExecutionOutcome {
                output: "Saved.".to_string(),
                is_error: false,
            }
        }
        Err(error) => ToolExecutionOutcome {
            output: format!("Failed to save memory: {error}"),
            is_error: true,
        },
    }
}

/// Retrieval failure **never** returns `Err` here — it returns a normal tool result telling the
/// model that recall is temporarily unavailable, so generation continues. Bubbling an optional
/// enhancement's failure up as a generation failure is unacceptable (design.md §8.1): the model
/// must never confuse "search failed" with "no such memory exists".
fn execute_recall(input: &Value, retrieval: &dyn AgentRetrievalPort) -> ToolExecutionOutcome {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if query.is_empty() {
        return ToolExecutionOutcome {
            output: "No query was provided to recall.".to_string(),
            is_error: true,
        };
    }
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20) as usize;
    match retrieval.search(query, limit) {
        Ok(outcome) => ToolExecutionOutcome {
            output: serde_json::to_string(&recall_payload(&outcome))
                .unwrap_or_else(|_| "{\"results\":[]}".to_string()),
            is_error: false,
        },
        Err(_) => ToolExecutionOutcome {
            output: "Memory search is temporarily unavailable. Continue without it.".to_string(),
            is_error: false,
        },
    }
}

/// Projects `outcome` into exactly what the model should see: `content`/`created_at`/
/// `matched_via` per hit, `degraded` only when present. `source_id`/`score` are internal — no
/// decision value to the model, and raw material for hallucination if included
/// (`AgentRetrievalHit` doesn't even carry them, so there is nothing here to accidentally leak).
fn recall_payload(outcome: &AgentRetrievalOutcome) -> Value {
    let hits: Vec<Value> = outcome
        .hits
        .iter()
        .map(|hit| {
            json!({
                "content": hit.content,
                "created_at": hit.created_at,
                "matched_via": hit.matched_via,
            })
        })
        .collect();
    match &outcome.degraded {
        Some(degraded) => json!({ "results": hits, "degraded": degraded }),
        None => json!({ "results": hits }),
    }
}

fn execute_search_code(
    input: &Value,
    workspace_folder: &str,
    retrieval: &dyn crate::contexts::agent_runtime::application::AgentCodeRetrievalPort,
) -> ToolExecutionOutcome {
    let query = input
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if query.is_empty() {
        return ToolExecutionOutcome {
            output: "No query was provided to search_code.".to_string(),
            is_error: true,
        };
    }
    let limit = input
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(5)
        .clamp(1, 20) as usize;
    match retrieval.search_code(workspace_folder, query, limit) {
        Ok(outcome) => ToolExecutionOutcome {
            output: serde_json::to_string(&code_search_payload(&outcome))
                .unwrap_or_else(|_| "{\"results\":[]}".to_string()),
            is_error: false,
        },
        Err(_) => ToolExecutionOutcome {
            output: "Code search is temporarily unavailable. Continue without it.".to_string(),
            is_error: false,
        },
    }
}

fn code_search_payload(outcome: &AgentCodeRetrievalOutcome) -> Value {
    let hits = outcome
        .hits
        .iter()
        .map(|hit| {
            json!({
                "file_path": hit.file_path,
                "start_line": hit.start_line,
                "end_line": hit.end_line,
                "language": hit.language,
                "symbol_name": hit.symbol_name,
                "symbol_kind": hit.symbol_kind,
                "snippet": hit.snippet,
                "matched_via": hit.matched_via,
            })
        })
        .collect::<Vec<_>>();
    let mut payload = json!({ "results": hits });
    if let Some(degraded) = &outcome.degraded {
        payload["degraded"] = Value::String(degraded.clone());
    }
    payload
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
