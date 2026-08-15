use super::managed_mcp_relay::InvocationScopedMcpRelayAdapter;
use crate::contexts::agent_runtime::api::{AgentRuntimeApi, AgentRuntimeApiServices};
use crate::contexts::agent_runtime::application::{
    web_native_tool_handlers, AgentClockPort, AgentCodeIntelligenceResponderPort, AgentLoggingPort,
    AgentRetrievalPort, AgentRuntimeApplicationPorts, AgentRuntimeApplicationService,
    AgentTerminalApplicationPorts, AgentTerminalApplicationService, AgentWorkspaceMutationPort,
    ApiAgentGateway, ApiCredentialPort, ApplyDelegationChangesNativeToolHandler,
    ArtifactNativeToolHandler, BrowserHandoffControlPort, BrowserNativeToolHandler,
    CodeExecutionNativeToolHandler, ContextQualityQueryService, ContextQualityRecorder,
    DelegateCliNativeToolHandler, ExpertRoleApplicationPorts, ExpertRoleApplicationService,
    LoopApplicationPorts, LoopApplicationService, LoopControlApplicationPorts,
    LoopControlApplicationService, LoopOperationObserver, LoopOrchestratorApplicationService,
    LoopOrchestratorPorts, LoopProgressApplicationService, LoopRecoveryApplicationPorts,
    LoopRecoveryApplicationService, LoopVerificationApplicationPorts,
    LoopVerificationApplicationService, LoopVerifierApplicationPorts,
    LoopVerifierApplicationService, LoopWorkerApplicationPorts, LoopWorkerApplicationService,
    ManualNativeToolService, NativeToolDispatcher, NativeToolHandler,
    NativeToolReadinessReasonCode, NativeToolRegistry, OcrNativeToolHandler,
    OnePieceToolFeatureGates, SubagentNativeToolHandler, UtilityDelegationApplicationPorts,
    UtilityDelegationApplicationService,
};
use crate::contexts::agent_runtime::infrastructure::{
    builtin_expert_roles, AgentRuntimeLoggingAdapter, AgentRuntimeOperationAdapter,
    CompositeAgentProcessGateway, CredentialAwareAgentRegistry, HttpOnePieceModelDiscoveryAdapter,
    InMemoryAgentMessageTerminalCompletions, InMemoryGenerationCoordinator,
    InMemoryLoopExecutionCoordinator, InMemoryLoopRoleGenerationCompletions,
    InMemorySeatTurnCompletions, ManualNativeToolAuthorityAdapter, ManualNativeToolControl,
    ManualNativeToolOperationAdapter, NativeAgentCoreInstructionsAdapter, NativeLoopScheduler,
    NativeSeatTurnCoordinator, NativeSubagentExecutor, NativeUtilityChildExecutor,
    OsApiCredentialAdapter, PermissionsPortAdapter, PortablePtyAgentTerminalRuntime,
    RuntimeAgentApiAdapter, RuntimeAgentAvailabilityAdapter, RuntimeAgentCliProfileAdapter,
    RuntimeAgentMcpToolAdapter, RuntimeAgentMemoryExtractionAdapter,
    RuntimeAgentPersonalizationAdapter, RuntimeAgentProcessAdapter, RuntimeAgentSkillAdapter,
    RuntimeEffectivePromptAdapter, RuntimeLoopVerificationEvidenceAdapter,
    RuntimeProcessEvidenceDependencies, RuntimeUtilityLifecycleProjector,
    SessionsAgentRuntimeAdapter, SqliteAgentMemoryRepository, SqliteAgentRuntimeRepository,
    SqliteContextQualityRepository, SqliteExpertRoleRepository, SqliteLoopRepository,
    SqliteNativeToolRepository, StructuredLoopVerificationProcess, SubagentRuntime,
    SystemAgentRuntimeClock, SystemExpertRoleClock, TauriAgentRuntimeEventAdapter,
    TerminalExecutionObservability, UnavailableNativeToolPort, UuidExpertRoleIds,
    WorkspaceLoopProjectAdapter,
};
use crate::contexts::artifacts::application::{ArtifactBlobStorePolicy, ArtifactService};
use crate::contexts::artifacts::infrastructure::{
    ArtifactBlobStore, ArtifactNativeToolAdapter, BrowserArtifactAdapter, CodeArtifactAdapter,
    SqliteArtifactCatalog,
};
use crate::contexts::browser_automation::application::{
    BrowserArtifactBridge, BrowserContextPolicy, BrowserOperationService, BrowserSessionManager,
};
use crate::contexts::browser_automation::infrastructure::{
    BrowserHandoffCommandAdapter, BrowserNativeToolAdapter, PlaywrightSidecarFactory,
};
use crate::contexts::cli_delegation::application::{
    DelegationCapabilityDependencies, DelegationMode, DelegationReadinessService,
    DelegationReadinessState, DelegationTarget,
};
use crate::contexts::cli_delegation::infrastructure::{
    ClaudeDelegationNativeToolAdapter, NativeChangeSetApplyAdapter, PassiveDelegationProbe,
};
use crate::contexts::code_execution::application::{
    CodeExecutionService, SandboxProcessBackend, SandboxWorkspaceService,
};
use crate::contexts::code_execution::infrastructure::{
    CodeExecutionNativeToolAdapter, PlatformSandboxBackend, SystemCodeRuntimeAdapter,
};
use crate::contexts::desktop::api::DesktopSettingsApi;
use crate::contexts::execution_observability::api::ExecutionTelemetryPort;
use crate::contexts::execution_observability::infrastructure::{
    CompositeExecutionTelemetry, ExecutionTelemetryLifecycle, OpenTelemetryExecutionExporter,
    OsObservabilityCredentialAdapter, RandomExecutionIdentity, SqliteExecutionTimelineRepository,
};
use crate::contexts::operations::api::{
    DiagnosticLog, DiagnosticLogPort, ExternalLogExportPort, LogSeverity, OperationLogPort,
    OperationsApi,
};
use crate::contexts::operations::infrastructure::UnifiedLoggingAdapter;
use crate::contexts::permissions::api::PermissionsApi;
use crate::contexts::sessions::api::SessionsApi;
use crate::contexts::skill_evolution_evidence::application::RuntimeEvidenceProjector;
use crate::contexts::tooling::cli::api::CliApi;
use crate::contexts::tooling::cli::infrastructure::CliExecutableLocatorAdapter;
use crate::contexts::tooling::cli_parameters::CliParametersApi;
use crate::contexts::tooling::extensions::application::PaddleOcrReadinessService;
use crate::contexts::tooling::extensions::infrastructure::{
    ManagedPaddleOcrReadinessInspector, OcrNativeToolAdapter, SqliteExtensionRepository,
};
use crate::contexts::tooling::mcp::api::McpApi;
use crate::contexts::tooling::prompt_hooks::api::PromptHookApi;
use crate::contexts::tooling::sdk::api::SdkApi;
use crate::contexts::tooling::skills::api::SkillApi;
use crate::contexts::web_research::application::{FetchedBinaryRouter, GuardedFetchService};
use crate::contexts::web_research::infrastructure::{
    ArtifactFetchedBinaryAdapter, DuckDuckGoSearchAdapter, ReqwestFetchHttpAdapter,
    ReqwestSearchHttpAdapter, SystemUrlResolver, WebResearchNativeToolAdapter,
};
use crate::contexts::workspaces::api::WorkspaceApi;
use crate::platform::database::NativeDatabase;
use crate::platform::process::command_exists;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Manager};

pub(crate) struct AgentRuntimeDependencies {
    pub(crate) database: NativeDatabase,
    pub(crate) app: AppHandle,
    pub(crate) operations: OperationsApi,
    pub(crate) cli: CliApi,
    pub(crate) cli_parameters: CliParametersApi,
    pub(crate) prompts: PromptHookApi,
    pub(crate) skills: SkillApi,
    pub(crate) mcp: McpApi,
    pub(crate) sessions: SessionsApi,
    pub(crate) workspaces: WorkspaceApi,
    pub(crate) permissions: PermissionsApi,
    pub(crate) shared_registry: SharedAgentRegistry,
    /// Consumed by `RuntimeAgentApiAdapter`'s `recall` tool (Task 13). A concrete
    /// `Arc<retrieval::DeferredAgentRetrieval>`, coerced here — `assemble_retrieval` itself needs
    /// this function's own output (`AgentRuntimeApi`), so the real `RetrievalApi` cannot exist
    /// yet; `runtime.rs`'s `setup` binds it right after `assemble_retrieval` returns.
    pub(crate) retrieval: Arc<dyn AgentRetrievalPort>,
    pub(crate) code_intelligence: Arc<dyn AgentCodeIntelligenceResponderPort>,
    pub(crate) workspace_mutations: Arc<dyn AgentWorkspaceMutationPort>,
    pub(crate) desktop_settings: DesktopSettingsApi,
    pub(crate) evidence: RuntimeEvidenceProjector,
}

#[derive(Clone)]
pub(crate) struct SharedAgentRegistry {
    pub(crate) repository: Arc<SqliteAgentRuntimeRepository>,
    pub(crate) registry: Arc<CredentialAwareAgentRegistry>,
    api_credentials: Arc<OsApiCredentialAdapter>,
    logging: Arc<AgentRuntimeLoggingAdapter>,
    clock: Arc<SystemAgentRuntimeClock>,
    unified_logging: Arc<UnifiedLoggingAdapter>,
}

pub(crate) fn assemble_shared_agent_registry(
    database: NativeDatabase,
    sdk: SdkApi,
    fallback_log_directory: PathBuf,
) -> SharedAgentRegistry {
    let unified_logging = Arc::new(UnifiedLoggingAdapter::active(fallback_log_directory));
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let operation_logs: Arc<dyn OperationLogPort> = unified_logging.clone();
    let logging = Arc::new(AgentRuntimeLoggingAdapter::new(diagnostics, operation_logs));
    let clock = Arc::new(SystemAgentRuntimeClock);
    let availability = Arc::new(RuntimeAgentAvailabilityAdapter::new(sdk));
    let repository = Arc::new(SqliteAgentRuntimeRepository::new(database, availability));
    let api_credentials = Arc::new(OsApiCredentialAdapter::new());
    let registry = Arc::new(CredentialAwareAgentRegistry::new(
        repository.clone(),
        api_credentials.clone(),
        logging.clone(),
        clock.clone(),
    ));
    SharedAgentRegistry {
        repository,
        registry,
        api_credentials,
        logging,
        clock,
        unified_logging,
    }
}

pub(crate) struct AgentRuntimeAssembly {
    pub(crate) api: AgentRuntimeApi,
    pub(crate) telemetry_lifecycle: ExecutionTelemetryLifecycle,
    pub(crate) completion_events: Arc<TauriAgentRuntimeEventAdapter>,
}

type ExecutionExporterSet = (
    Vec<Arc<dyn ExecutionTelemetryPort>>,
    Option<Arc<dyn ExternalLogExportPort>>,
);

/// What a subagent child attempt needs to run: the parent's credential and provider
/// configuration, and the accounting, clock, and logging its turns are recorded against. Bundled
/// because they travel together and only exist for this one handler.
struct SubagentDependencies {
    credentials: Arc<dyn ApiCredentialPort>,
    agents: Arc<dyn ApiAgentGateway>,
    accounting: Option<SessionsApi>,
    clock: Arc<dyn AgentClockPort>,
    logging: Arc<dyn AgentLoggingPort>,
}

fn assemble_native_tool_registry(
    database: &NativeDatabase,
    app: &AppHandle,
    cli: &CliApi,
    subagents: SubagentDependencies,
) -> Result<
    (
        NativeToolRegistry,
        Option<Arc<dyn BrowserHandoffControlPort>>,
    ),
    String,
> {
    let data_root = database
        .db_path
        .parent()
        .ok_or_else(|| "application data directory is unavailable".to_owned())?;
    let artifacts = Arc::new(ArtifactService::new(
        Arc::new(
            ArtifactBlobStore::new(
                data_root,
                ArtifactBlobStorePolicy {
                    max_blob_bytes: 256 * 1024 * 1024,
                    max_operation_items: 64,
                    max_operation_bytes: 512 * 1024 * 1024,
                    max_total_bytes: 4 * 1024 * 1024 * 1024,
                },
            )
            .map_err(|error| format!("Artifact storage is unavailable: {error:?}"))?,
        ),
        Arc::new(SqliteArtifactCatalog::new(database.clone())),
    ));
    let install_path = data_root.join("extensions").join("paddleocr");
    let readiness = PaddleOcrReadinessService::new(
        Arc::new(SqliteExtensionRepository::new(database.clone())),
        Arc::new(ManagedPaddleOcrReadinessInspector),
    )
    .check()
    .ok()
    .is_some_and(|snapshot| snapshot.ready);
    let feature_gates = OnePieceToolFeatureGates::from_environment();
    let mut readiness_reasons = BTreeMap::new();
    let mut browser_handoff: Option<Arc<dyn BrowserHandoffControlPort>> = None;
    let mut handlers: Vec<Arc<dyn NativeToolHandler>> = vec![Arc::new(
        ArtifactNativeToolHandler::new(Arc::new(ArtifactNativeToolAdapter::new(artifacts.clone()))),
    )];
    if let Some(script_path) = available_browser_sidecar(app) {
        let factory = Arc::new(PlaywrightSidecarFactory::new(
            "node".to_owned(),
            script_path,
        ));
        let operations = Arc::new(BrowserOperationService::new(
            BrowserSessionManager::new(factory),
            BrowserContextPolicy::default(),
        ));
        let handoff_control = Arc::new(BrowserHandoffCommandAdapter::new(operations.clone()));
        let artifacts = Arc::new(BrowserArtifactBridge::new(Arc::new(
            BrowserArtifactAdapter::new(artifacts.clone()),
        )));
        let port = Arc::new(BrowserNativeToolAdapter::with_handoff_control(
            operations,
            Arc::new(SystemUrlResolver),
            Some(artifacts),
            handoff_control.clone(),
        ));
        browser_handoff = Some(handoff_control);
        handlers.push(Arc::new(BrowserNativeToolHandler::new(port)));
    } else {
        handlers.push(Arc::new(BrowserNativeToolHandler::new(Arc::new(
            UnavailableNativeToolPort,
        ))));
        readiness_reasons.insert(
            "browser".to_owned(),
            NativeToolReadinessReasonCode::MissingDependency,
        );
    }
    let web = Arc::new(WebResearchNativeToolAdapter::new(
        DuckDuckGoSearchAdapter::new(Arc::new(ReqwestSearchHttpAdapter)),
        GuardedFetchService::new(
            Arc::new(SystemUrlResolver),
            Arc::new(ReqwestFetchHttpAdapter),
        ),
        FetchedBinaryRouter::new(Arc::new(ArtifactFetchedBinaryAdapter::new(
            artifacts.clone(),
        ))),
    ));
    handlers.extend(web_native_tool_handlers(web));
    let sandbox = Arc::new(PlatformSandboxBackend);
    if feature_gates.tool_enabled("code_execution") && sandbox.capabilities().ready() {
        let artifact_port = Arc::new(CodeArtifactAdapter::new(artifacts.clone()));
        let workspaces = Arc::new(
            SandboxWorkspaceService::new(
                data_root.join("code-execution"),
                artifact_port.clone(),
                Arc::new(crate::contexts::code_execution::infrastructure::SystemSandboxFilesystem),
            )
            .map_err(|error| format!("code execution workspace is unavailable: {error:?}"))?,
        );
        let service = Arc::new(CodeExecutionService::new(
            workspaces,
            sandbox,
            Arc::new(SystemCodeRuntimeAdapter),
            artifact_port,
        ));
        handlers.push(Arc::new(CodeExecutionNativeToolHandler::new(Arc::new(
            CodeExecutionNativeToolAdapter::new(service),
        ))));
    } else {
        handlers.push(Arc::new(CodeExecutionNativeToolHandler::new(Arc::new(
            UnavailableNativeToolPort,
        ))));
        readiness_reasons.insert(
            "code_execution".to_owned(),
            NativeToolReadinessReasonCode::IsolationUnavailable,
        );
    }
    // The child reuses the parent's own credential and provider configuration through the
    // existing boundary rather than receiving copies (`add-onepiece-subagents`).
    handlers.push(Arc::new(SubagentNativeToolHandler::new(Arc::new(
        NativeSubagentExecutor::new(SubagentRuntime {
            credentials: subagents.credentials,
            config: subagents.agents,
            accounting: subagents.accounting,
            clock: subagents.clock,
            logging: subagents.logging,
            artifacts: artifacts.clone(),
            operations: Arc::new(SqliteNativeToolRepository::new(database.clone())),
            operations_root: data_root.join("subagent-worktrees"),
        }),
    ))));
    if readiness {
        let port = Arc::new(OcrNativeToolAdapter::new(
            install_path,
            data_root.join("ocr-operations"),
            Arc::new(PlatformSandboxBackend),
            artifacts.clone(),
        ));
        handlers.push(Arc::new(OcrNativeToolHandler::new(port)));
    } else {
        handlers.push(Arc::new(OcrNativeToolHandler::new(Arc::new(
            UnavailableNativeToolPort,
        ))));
        readiness_reasons.insert(
            "ocr".to_owned(),
            NativeToolReadinessReasonCode::MissingDependency,
        );
    }
    let delegation_readiness = DelegationReadinessService::new(
        Arc::new(PassiveDelegationProbe::new(Arc::new(
            CliExecutableLocatorAdapter::new(),
        ))),
        DelegationCapabilityDependencies {
            process_tree_control: true,
            analyze_isolation: true,
            edit_isolation: true,
            artifact_storage: true,
            codex_network_isolation_canary: false,
        },
    )
    .check();
    let claude_analyze_ready = feature_gates.enabled("delegation", "read")
        && delegation_mode_ready(
            &delegation_readiness,
            DelegationTarget::ClaudeCode,
            DelegationMode::Analyze,
        );
    let claude_edit_ready = feature_gates.enabled("delegation", "write")
        && delegation_mode_ready(
            &delegation_readiness,
            DelegationTarget::ClaudeCode,
            DelegationMode::Edit,
        );
    if claude_analyze_ready || claude_edit_ready {
        handlers.push(Arc::new(DelegateCliNativeToolHandler::new(Arc::new(
            ClaudeDelegationNativeToolAdapter::new(
                cli.clone(),
                data_root.join("delegation"),
                artifacts.clone(),
                Arc::new(SqliteNativeToolRepository::new(database.clone())),
                claude_analyze_ready,
                claude_edit_ready,
            ),
        ))));
    } else {
        handlers.push(Arc::new(DelegateCliNativeToolHandler::new(Arc::new(
            UnavailableNativeToolPort,
        ))));
        readiness_reasons.insert(
            "delegate_cli".to_owned(),
            NativeToolReadinessReasonCode::BackendUnavailable,
        );
    }
    if feature_gates.enabled("delegation", "apply") {
        let apply_port = NativeChangeSetApplyAdapter::new(
            artifacts.clone(),
            Arc::new(SqliteNativeToolRepository::new(database.clone())),
            data_root.join("delegation-apply-recovery"),
        )
        .map_err(|error| format!("delegation apply is unavailable: {error}"))?;
        handlers.push(Arc::new(ApplyDelegationChangesNativeToolHandler::new(
            Arc::new(apply_port),
        )));
    } else {
        handlers.push(Arc::new(ApplyDelegationChangesNativeToolHandler::new(
            Arc::new(UnavailableNativeToolPort),
        )));
        readiness_reasons.insert(
            "apply_delegation_changes".to_owned(),
            NativeToolReadinessReasonCode::BackendUnavailable,
        );
    }
    let registry = NativeToolRegistry::try_new_with_feature_gates_and_readiness(
        handlers,
        feature_gates,
        readiness_reasons,
    )
    .map_err(|error| format!("native tool registry is invalid: {error:?}"))?;
    Ok((registry, browser_handoff))
}

fn delegation_mode_ready(
    readiness: &[crate::contexts::cli_delegation::application::DelegationReadiness],
    target: DelegationTarget,
    mode: DelegationMode,
) -> bool {
    readiness.iter().any(|snapshot| {
        snapshot.target == target
            && snapshot.mode == mode
            && matches!(
                snapshot.state,
                DelegationReadinessState::Ready | DelegationReadinessState::Degraded
            )
    })
}

fn available_browser_sidecar(app: &AppHandle) -> Option<PathBuf> {
    if !command_exists("node", Duration::from_secs(2)) {
        return None;
    }
    let development = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("scripts")
        .join("onepiece-playwright-sidecar.mjs");
    let mut candidates = vec![development];
    if let Ok(resources) = app.path().resource_dir() {
        candidates.push(resources.join("scripts/onepiece-playwright-sidecar.mjs"));
        candidates.push(resources.join("_up_/scripts/onepiece-playwright-sidecar.mjs"));
    }
    candidates.into_iter().find(|script| {
        script.is_file()
            && script
                .parent()
                .and_then(std::path::Path::parent)
                .is_some_and(|root| root.join("node_modules/playwright/package.json").is_file())
    })
}

pub(crate) fn assemble_agent_runtime_api(
    dependencies: AgentRuntimeDependencies,
) -> Result<AgentRuntimeAssembly, String> {
    let shared = dependencies.shared_registry;
    let unified_logging = shared.unified_logging;
    let diagnostics: Arc<dyn DiagnosticLogPort> = unified_logging.clone();
    let logging = shared.logging;
    let clock = shared.clock;
    let repository = shared.repository;
    let api_credentials = shared.api_credentials;
    let registry = shared.registry;
    let execution_ids = Arc::new(RandomExecutionIdentity);
    let timeline = Arc::new(SqliteExecutionTimelineRepository::new(
        dependencies.database.clone(),
    ));
    let (exporters, log_exporter) = execution_exporters(timeline.as_ref(), diagnostics.clone());
    if let Some(exporter) = log_exporter {
        unified_logging.attach_external_exporter(exporter);
    }
    let telemetry = Arc::new(CompositeExecutionTelemetry::with_diagnostics(
        timeline.clone(),
        exporters,
        diagnostics,
    ));
    let telemetry_lifecycle =
        ExecutionTelemetryLifecycle::new(telemetry.clone(), Duration::from_secs(3));
    let provider_registry = Arc::new(
        crate::contexts::agent_runtime::infrastructure::providers::builtin_cli_provider_registry()
            .map_err(|error| error.to_string())?,
    );
    let cli_processes = Arc::new(RuntimeAgentProcessAdapter::new(
        logging.clone(),
        clock.clone(),
        execution_ids.clone(),
        telemetry.clone(),
        Arc::new(InvocationScopedMcpRelayAdapter::new(
            dependencies.database.clone(),
        )),
        provider_registry.clone(),
        RuntimeProcessEvidenceDependencies {
            evidence: dependencies.evidence.clone(),
            skills: dependencies.skills.clone(),
        },
    ));
    let accounting = dependencies.sessions.clone();
    let manual_authority = Arc::new(ManualNativeToolAuthorityAdapter::new(
        dependencies.sessions.clone(),
        dependencies.database.clone(),
    ));
    let sessions = Arc::new(SessionsAgentRuntimeAdapter::new(dependencies.sessions));
    let agent_skills = Arc::new(RuntimeAgentSkillAdapter::new(
        dependencies.skills.clone(),
        dependencies.evidence.clone(),
    ));
    let agent_memories = Arc::new(SqliteAgentMemoryRepository::new(
        dependencies.database.clone(),
    ));
    let agent_mcp_tools = Arc::new(RuntimeAgentMcpToolAdapter::new(dependencies.mcp));
    let agent_permissions = Arc::new(PermissionsPortAdapter::new(
        dependencies.permissions.clone(),
    ));
    let agent_personalization = Arc::new(RuntimeAgentPersonalizationAdapter::new(
        dependencies.desktop_settings,
    ));
    let agent_memory_extraction = Arc::new(RuntimeAgentMemoryExtractionAdapter::new(
        api_credentials.clone(),
        repository.clone(),
    ));
    let context_quality_repository = Arc::new(SqliteContextQualityRepository::new(
        dependencies.database.clone(),
    ));
    let context_quality_query =
        ContextQualityQueryService::new(context_quality_repository.clone(), clock.clone());
    let context_quality = Arc::new(ContextQualityRecorder::new(
        context_quality_repository,
        logging.clone(),
        clock.clone(),
    ));
    let code_intelligence = Arc::new(
        crate::contexts::agent_runtime::infrastructure::RuntimeAgentCodeIntelligenceAdapter::new(
            dependencies.code_intelligence,
        ),
    );
    let utility_projector = Arc::new(RuntimeUtilityLifecycleProjector::new(
        dependencies.evidence.clone(),
        logging.clone(),
        telemetry.clone(),
    ));
    let utility_delegation =
        UtilityDelegationApplicationService::new(UtilityDelegationApplicationPorts {
            resolver: agent_skills.clone(),
            executor: Arc::new(NativeUtilityChildExecutor::new(
                api_credentials.clone(),
                repository.clone(),
            )),
            lifecycle: utility_projector.clone(),
            evidence: utility_projector,
            clock: clock.clone(),
        });
    let (native_tools, browser_handoff) = assemble_native_tool_registry(
        &dependencies.database,
        &dependencies.app,
        &dependencies.cli,
        SubagentDependencies {
            credentials: api_credentials.clone(),
            agents: repository.clone(),
            accounting: Some(accounting.clone()),
            clock: clock.clone(),
            logging: logging.clone(),
        },
    )?;
    let api_processes = Arc::new(
        RuntimeAgentApiAdapter::new_with_code_intelligence(
            api_credentials.clone(),
            repository.clone(),
            sessions.clone(),
            logging.clone(),
            clock.clone(),
            agent_skills,
            Arc::new(NativeAgentCoreInstructionsAdapter),
            agent_memories.clone(),
            agent_mcp_tools,
            agent_permissions.clone(),
            dependencies.retrieval,
            code_intelligence,
            dependencies.workspace_mutations,
            agent_personalization.clone(),
        )
        .with_context_quality_recorder(context_quality)
        .with_evidence(dependencies.evidence.clone())
        .with_utility_delegation(utility_delegation)
        .with_accounting(accounting.clone())
        .with_native_tool_registry(native_tools.clone())
        .with_native_tool_operations(
            Arc::new(SqliteNativeToolRepository::new(
                dependencies.database.clone(),
            )),
            dependencies.app.clone(),
        ),
    );
    let tool_approvals = api_processes.clone();
    let manual_native_tool_service = ManualNativeToolService::new(
        NativeToolDispatcher::new(native_tools.clone()),
        agent_permissions,
        manual_authority,
        Arc::new(ManualNativeToolOperationAdapter::new(
            SqliteNativeToolRepository::new(dependencies.database.clone()),
            dependencies.app.clone(),
        )),
    );
    let manual_native_tools =
        ManualNativeToolControl::new(manual_native_tool_service, dependencies.database.clone());
    let processes: Arc<dyn crate::contexts::agent_runtime::application::AgentProcessGateway> =
        Arc::new(CompositeAgentProcessGateway::new(
            cli_processes,
            api_processes,
        ));
    let cli_profiles = Arc::new(RuntimeAgentCliProfileAdapter::new(
        dependencies.cli_parameters,
        dependencies.cli,
        dependencies.permissions,
    ));
    let events = Arc::new(TauriAgentRuntimeEventAdapter::new(dependencies.app));
    let operations = Arc::new(AgentRuntimeOperationAdapter::new(dependencies.operations));
    let loop_observer =
        LoopOperationObserver::new(operations.clone(), logging.clone(), clock.clone());
    let terminal_runtime = Arc::new(PortablePtyAgentTerminalRuntime::new(
        events.clone(),
        sessions.clone(),
        accounting,
        logging.clone(),
        clock.clone(),
        TerminalExecutionObservability::new(
            execution_ids.clone(),
            timeline.clone(),
            telemetry.clone(),
        ),
        std::env::temp_dir().join("vanehub-agent-terminal-wrappers"),
        provider_registry,
    ));
    let loop_completions = Arc::new(InMemoryLoopRoleGenerationCompletions::default());
    let seat_completions = Arc::new(InMemorySeatTurnCompletions::default());
    // Shared with the expert role service below: the roster a seat is briefed with reads the same
    // roles the settings page writes, so a role edited mid-session takes effect on the next turn.
    let expert_role_repository = Arc::new(SqliteExpertRoleRepository::new(
        dependencies.database.clone(),
    ));
    let service = AgentRuntimeApplicationService::new(AgentRuntimeApplicationPorts {
        registry: registry.clone(),
        workflows: repository.clone(),
        sessions: sessions.clone(),
        cli_profiles: cli_profiles.clone(),
        prompts: Arc::new(RuntimeEffectivePromptAdapter::new(dependencies.prompts)),
        processes: processes.clone(),
        operations: operations.clone(),
        logging: logging.clone(),
        clock: clock.clone(),
        events: events.clone(),
        generations: Arc::new(InMemoryGenerationCoordinator::default()),
        execution_ids: execution_ids.clone(),
        execution_settings: timeline.clone(),
        telemetry: telemetry.clone(),
        loop_completions: loop_completions.clone(),
        seat_completions: seat_completions.clone(),
        expert_roles: expert_role_repository.clone(),
        history: sessions.clone(),
        message_completions: Arc::new(InMemoryAgentMessageTerminalCompletions::default()),
        api_agents: repository.clone(),
        api_credentials: api_credentials.clone(),
        onepiece_model_discovery: Arc::new(HttpOnePieceModelDiscoveryAdapter),
        tool_approvals: tool_approvals.clone(),
        memories: agent_memories,
        memory_extraction: agent_memory_extraction,
        personalization: agent_personalization,
    });
    let terminal_service = AgentTerminalApplicationService::new(AgentTerminalApplicationPorts {
        registry: registry.clone(),
        sessions: sessions.clone(),
        cli_profiles: cli_profiles.clone(),
        terminals: terminal_runtime,
        logging: logging.clone(),
        clock: clock.clone(),
        events: events.clone(),
        terminal_events: events.clone(),
    });
    let expert_roles = ExpertRoleApplicationService::new(ExpertRoleApplicationPorts {
        repository: expert_role_repository,
        clock: Arc::new(SystemExpertRoleClock),
        ids: Arc::new(UuidExpertRoleIds),
        builtins: builtin_expert_roles(),
    });
    let loop_repository = Arc::new(SqliteLoopRepository::new(dependencies.database.clone()));
    let loop_projects = Arc::new(WorkspaceLoopProjectAdapter::new(dependencies.workspaces));
    let loop_execution = Arc::new(InMemoryLoopExecutionCoordinator::default());
    let loops = LoopApplicationService::new(LoopApplicationPorts {
        loops: loop_repository.clone(),
        registry: registry.clone(),
        api_agents: repository.clone(),
        projects: loop_projects.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let loop_controls = LoopControlApplicationService::new(LoopControlApplicationPorts {
        loops: loop_repository.clone(),
        execution: loop_execution.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let loop_recovery = LoopRecoveryApplicationService::new(LoopRecoveryApplicationPorts {
        loops: loop_repository.clone(),
        leases: loop_execution.clone(),
        sessions: sessions.clone(),
        observer: loop_observer.clone(),
        clock: clock.clone(),
    });
    let generations = Arc::new(service.clone());
    let loop_worker = LoopWorkerApplicationService::new(LoopWorkerApplicationPorts {
        iterations: loop_repository.clone(),
        registry: registry.clone(),
        roles: sessions.clone(),
        git: loop_projects.clone(),
        generations: generations.clone(),
        clock: clock.clone(),
    });
    let guarded_validation = Arc::new(StructuredLoopVerificationProcess::default());
    let loop_verification =
        LoopVerificationApplicationService::new(LoopVerificationApplicationPorts {
            iterations: loop_repository.clone(),
            processes: guarded_validation.clone(),
            observer: loop_observer.clone(),
            clock: clock.clone(),
            evidence: Arc::new(RuntimeLoopVerificationEvidenceAdapter::new(
                dependencies.evidence.clone(),
                dependencies.database.clone(),
            )),
        });
    let loop_verifier = LoopVerifierApplicationService::new(LoopVerifierApplicationPorts {
        iterations: loop_repository.clone(),
        registry,
        roles: sessions,
        context: loop_projects.clone(),
        generations,
    });
    let loop_orchestrator = LoopOrchestratorApplicationService::new(LoopOrchestratorPorts {
        loops: loop_repository.clone(),
        iterations: loop_repository.clone(),
        projects: loop_projects.clone(),
        verifier_context: loop_projects,
        completions: loop_completions,
        generations: Arc::new(service.clone()),
        worker: loop_worker,
        verification: loop_verification,
        verifier: loop_verifier,
        progress: LoopProgressApplicationService::new(loop_repository),
        observer: loop_observer,
        clock,
    });
    let loop_scheduler = NativeLoopScheduler::new((*loop_execution).clone(), loop_orchestrator);
    let seat_turns = NativeSeatTurnCoordinator::new(service.clone());
    Ok(AgentRuntimeAssembly {
        api: AgentRuntimeApi::new(AgentRuntimeApiServices {
            service,
            terminal_service,
            loops,
            loop_controls,
            loop_recovery,
            loop_scheduler,
            expert_roles,
            seat_turns,
            guarded_validation,
            context_quality: context_quality_query,
            native_tools,
            browser_handoff,
            manual_native_tools,
        }),
        telemetry_lifecycle,
        completion_events: events,
    })
}

fn execution_exporters(
    timeline: &SqliteExecutionTimelineRepository,
    diagnostics: Arc<dyn DiagnosticLogPort>,
) -> ExecutionExporterSet {
    let settings = match timeline.load_settings() {
        Ok(settings) => settings,
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "settings_unavailable");
            return (Vec::new(), None);
        }
    };
    if !settings.otlp_enabled {
        return (Vec::new(), None);
    }
    let Some(endpoint) = settings.otlp_endpoint.as_deref() else {
        record_telemetry_initialization_warning(diagnostics.as_ref(), "endpoint_missing");
        return (Vec::new(), None);
    };
    let credentials = OsObservabilityCredentialAdapter::new();
    let auth_token = match credentials.load_otlp_auth() {
        Ok(token) => token,
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "credential_unavailable");
            return (Vec::new(), None);
        }
    };
    match OpenTelemetryExecutionExporter::otlp_http(
        endpoint,
        settings.sampling_ratio,
        Duration::from_secs(3),
        auth_token.as_deref().map(String::as_str),
    ) {
        Ok(exporter) => {
            let exporter = Arc::new(exporter);
            (
                vec![exporter.clone() as Arc<dyn ExecutionTelemetryPort>],
                Some(exporter as Arc<dyn ExternalLogExportPort>),
            )
        }
        Err(_) => {
            record_telemetry_initialization_warning(diagnostics.as_ref(), "exporter_unavailable");
            (Vec::new(), None)
        }
    }
}

fn record_telemetry_initialization_warning(logging: &dyn DiagnosticLogPort, reason: &'static str) {
    let _ = logging.write_diagnostic(DiagnosticLog {
        severity: LogSeverity::Warn,
        category: "execution_observability.initialization".to_string(),
        message: "Optional execution telemetry export remains disabled; local execution continues"
            .to_string(),
        context: BTreeMap::from([
            ("reason".to_string(), reason.to_string()),
            ("fallback".to_string(), "local_timeline".to_string()),
        ]),
    });
}
