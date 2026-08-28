import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import { openUrl } from "@tauri-apps/plugin-opener";
import type { AgentService, SessionStateEvent } from "./agent-service";
import { tauriPersonalizationClient } from "./tauri-personalization-client";
import type {
  AgentRegistryEntry,
  ApiAgentProviderConfig,
  AgentTerminalEvent,
  AgentTerminalSession,
  AgentTerminalSize,
  AssignSessionCategoryInput,
  AutomaticArchivalSettings,
  CreateSessionCategoryInput,
  CreateScheduledTaskInput,
  DiscoverOnePieceProviderModelsInput,
  EndpointProfileMetadata,
  ExportSessionInput,
  InteractionMode,
  HybridRoutePreview,
  HybridRoutePreviewInput,
  HybridRoutingRule,
  KnownRemoteWorkspace,
  KnownProject,
  LaunchResult,
  LocalModelDiscoveryResult,
  OnePieceProviderConfig,
  OnePieceProviderProfiles,
  OnePieceProviderModelDiscoveryResult,
  OnePieceProviderPreset,
  ProjectInspection,
  ReadinessStatus,
  RegisterApiAgentInput,
  EmbeddingModelOption,
  RetrievalConfiguration,
  RetrievalIndexStatus,
  SaveOnePieceProviderConfigInput,
  SaveCustomOnePieceProviderProfileInput,
  SaveOnePieceProviderProfileInput,
  ValidateOnePieceProviderCredentialInput,
  UpdateApiAgentInput,
  UpdateSessionSeatsInput,
  RenameSessionCategoryInput,
  Session,
  SessionCategory,
  SessionDetails,
  ScheduledTask,
  ScheduledTaskRun,
  SetScheduledTaskEnabledInput,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  WorkflowState,
} from "../types/agent";
import { tauriCliEnvironmentClient } from "./tauri-cli-environment-client";
import type {
  CliParameterPreview,
  CliParameterProfile,
  PreviewCliParameterProfileInput,
  ResetCliParameterProfileInput,
  SaveCliParameterProfileInput,
} from "../types/cli-parameter-profile";
import { tauriSessionRecoveryClient } from "./tauri-session-recovery-client";
import type { ChatConfig, ChatMessage, ChatStreamEvent, MessageFeedback } from "../types/chat";
import type {
  ContextQualityHistoryPage,
  ContextQualityHistoryQuery,
  ContextQualitySummary,
  ContextQualitySummaryQuery,
} from "../types/context-quality";
import type { ContextEvidenceManifest, ContextEvidenceManifestPage, ContextEvidenceManifestQuery } from "../types/context-engine";
import { normalizeContextQualityError } from "./context-quality-error";
import type { TokenUsageDetailsPage, TokenUsageSummary } from "../types/token-usage";
import type { OperationTask } from "../types/operation";
import type { AgentRun, AgentRunEvent, AgentRunPage } from "../types/agent-run";
import type { AgentRunnerDescriptor } from "../types/agent-runner";
import type { MissionControlActionReceipt, MissionControlOverview, MissionControlRunDetail } from "../types/mission-control";
import type { EvaluationArena, EvaluationAttempt, EvaluationExport, EvaluationTask } from "../types/evaluation";
import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  LoopRun,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";
import type {
  PromptAssemblyPreviewInput,
  PromptHook,
  PromptHookListResult,
  PromptHookMutationInput,
  PromptHookPreview,
  PromptHookPreviewInput,
  PromptHookTraceSummary,
  PromptHookUpdateInput,
  PromptHookDraft,
  PromptHookVariableDefinition,
  PromptHookVersion,
  PromptHookVersionHistory,
  PublishPromptHookInput,
  RollbackPromptHookInput,
  SavePromptHookDraftInput,
} from "../types/prompt-hook";
import type {
  Skill,
  SkillAgentMountPath,
  SkillDriftReport,
  SkillImportInput,
  SkillListResult,
  SkillLoadInput,
  SkillLoadOutcome,
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillOverview,
  SkillPreview,
  SkillResourceReadInput,
  SkillResourceReadOutcome,
  SkillScopeInput,
  SkillSyncResult,
  SkillUpdateInput,
} from "../types/skill";
import type {
  SkillToolEnablementInput,
  SkillToolOwnerInput,
  SkillToolQuarantineInput,
  SkillToolRevision,
  SkillToolRevisionInput,
  SkillToolTrustInput,
} from "../types/skill-tools";
import type {
  SkillOverlayDetail,
  SkillOverlayHistoryPage,
  SkillOverlayImportReview,
  SkillOverlayMutationOutcome,
  SkillOverlayPreview,
  SkillOverlaySummary,
} from "../types/skill-overlay";
import type { SkillOverlayReconciliationPreview } from "../types/skill-overlay-reconciliation";
import { normalizeSkillOverlayError } from "./skill-overlay-error";
import { tauriSessionWorkspaceClient } from "./tauri-session-workspace-client";
import { normalizeTauriSessionUsageSummary, normalizeTauriUsageStatistics } from "./tauri-usage-statistics";
import { subscribeLoopRunPolling } from "./loop-run-polling";
import type {
  ApplyCliConfigProfileInput,
  CliConfigApplyResult,
  CliConfigDiscoveryResult,
  CliConfigAgentId,
  CliConfigProfile,
  CliConfigStatus,
  DeleteCliConfigProfileInput,
  ImportCliConfigProfileInput,
  ImportDiscoveredCliConfigInput,
  ImportDiscoveredCliConfigResult,
  SaveCliConfigProfileInput,
  ValidateCliConfigCredentialInput,
} from "../types/cli-agent-config";
import type { ProviderCredentialValidationResult } from "../types/provider-credential-validation";
import { cliConfigAgentIds } from "../types/cli-agent-config";
import { getCliConfigPresets } from "../config/cli-agent-provider-presets";
import { requireHttpsExternalUrl } from "./external-url";
import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";
import type { CodeIndexAutomaticMode, CodeIndexConfigurationInput } from "../types/code-index";
import type {
  LspConfiguration,
  LspLanguageId,
  LspWorkspaceTrustUpdate,
} from "../types/lsp";
import {
  normalizeCodeEmbeddingConfirmation,
  normalizeCodeIndexAuditEntries,
  normalizeCodeIndexConfiguration,
  normalizeCodeIndexStatus,
  normalizeCodeIndexWorkspace,
  normalizeCodeIndexWorkspaces,
} from "./code-index-contract";
import {
  normalizeLspConfiguration,
  normalizeLspServerDiscoveries,
  normalizeLspServerStatuses,
  normalizeLspServerTestInput,
  normalizeLspServerTestResult,
  normalizeLspWorkspaceTrust,
  normalizeLspWorkspaceTrustList,
  normalizeLspWorkspaceTrustUpdate,
} from "./lsp-contract";
import { tauriBuiltinToolClient } from "./tauri-builtin-tool-client";
import type { AddReviewCommentInput, CodeReview, ReviewAction, ReviewComment, ReviewDecision, ReviewDiffFile, ReviewRevertReceipt, RevertReviewChangeInput } from "../types/code-review";

function invokeSkillOverlay<TResult>(command: string, input: unknown): Promise<TResult> {
  return invoke<TResult>(command, { input }).catch((error: unknown) =>
    Promise.reject(normalizeSkillOverlayError(error)),
  );
}

/**
 * A `Result<_, String>` command rejects with the bare string, not with an `Error`.
 *
 * Callers of the service boundary are not supposed to know that, and a caller that assumes `Error`
 * silently loses the reason code and reports a generic failure instead.
 */
function rejectWithReasonCode(pending: Promise<void>): Promise<void> {
  return pending.catch((error: unknown) =>
    Promise.reject(typeof error === "string" ? new Error(error) : error),
  );
}

function requireCliConfigAgentId(agentId: string): CliConfigAgentId {
  if (cliConfigAgentIds.some((candidate) => candidate === agentId)) return agentId as CliConfigAgentId;
  throw new Error(`Unsupported CLI configuration Agent: ${agentId}`);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function isSessionStateEvent(value: unknown): value is SessionStateEvent {
  if (!isRecord(value) || typeof value.kind !== "string") return false;
  if (value.kind === "active-session-changed") {
    return value.sessionId === null || typeof value.sessionId === "string";
  }
  if (value.kind === "configuration-changed") return typeof value.sessionId === "string";
  return [
    "recovery-started",
    "recovery-completed",
    "recovery-action-required",
    "recovery-quarantined",
    "recovery-acknowledged",
  ].includes(value.kind)
    && typeof value.sessionId === "string"
    && typeof value.recoveryRevision === "number"
    && Number.isSafeInteger(value.recoveryRevision)
    && value.recoveryRevision >= 0;
}

export const tauriAgentClient: AgentService = {
  listEvaluationTasks: () => invoke<EvaluationTask[]>("list_evaluation_tasks"),
  startEvaluation: (input) => invoke<EvaluationArena>("start_evaluation", { input }),
  listEvaluationArenas: () => invoke<EvaluationArena[]>("list_evaluation_arenas"),
  getEvaluationArena: (arenaId) => invoke<EvaluationArena>("get_evaluation_arena", { arenaId }),
  cancelEvaluation: (arenaId) => invoke<EvaluationArena>("cancel_evaluation", { arenaId }),
  getEvaluationAttempt: (attemptId) => invoke<EvaluationAttempt>("get_evaluation_attempt", { attemptId }),
  exportEvaluation: (arenaId) => invoke<EvaluationExport>("export_evaluation", { arenaId }),
  getDesktopUpdateSnapshot() { return invoke("get_desktop_update_snapshot"); },
  getDesktopUpdatePreferences() { return invoke("get_desktop_update_preferences"); },
  saveDesktopUpdatePreferences(input) { return invoke("save_desktop_update_preferences", { input }); },
  checkForDesktopUpdate() { return invoke("check_for_desktop_update"); },
  downloadAndInstallDesktopUpdate() { return invoke("download_and_install_desktop_update"); },
  async restartAfterDesktopUpdate() { await invoke("restart_after_desktop_update"); },
  ...tauriBuiltinToolClient,
  openCodeReview(sessionId) {
    return invoke<CodeReview>("open_code_review", { sessionId });
  },
  getCodeReview(reviewId) {
    return invoke<CodeReview>("get_code_review", { reviewId });
  },
  loadCodeReviewFile(sessionId, path, expectedSnapshot) {
    return invoke<ReviewDiffFile>("load_code_review_file", { sessionId, path, expectedSnapshot });
  },
  addCodeReviewComment(input: AddReviewCommentInput) {
    return invoke<ReviewComment>("add_code_review_comment", { input });
  },
  resolveCodeReviewComment(reviewId, commentId) {
    return invoke<CodeReview>("resolve_code_review_comment", { reviewId, commentId });
  },
  selectCodeReviewComment(reviewId, commentId, selected) {
    return invoke<CodeReview>("select_code_review_comment", { reviewId, commentId, selected });
  },
  setCodeReviewDecision(reviewId, decision: ReviewDecision) {
    return invoke<CodeReview>("set_code_review_decision", { reviewId, decision });
  },
  revertCodeReviewChange(input: RevertReviewChangeInput) {
    return invoke<ReviewRevertReceipt>("revert_code_review_change", { input });
  },
  sendCodeReviewFeedback(reviewId, acknowledgeStale) {
    return invoke<{ messageId: string }>("send_code_review_feedback", { reviewId, acknowledgeStale });
  },
  startCodeReviewAction(reviewId, action: ReviewAction) {
    return invoke<{ operationId: string }>("start_code_review_action", { reviewId, action });
  },
  getAgentRun: (runId) => invoke<AgentRun>("get_agent_run", { runId }),
  listAgentRuns: (offset = 0, limit = 50, filter) =>
    invoke<AgentRunPage>("list_agent_runs", { filter, offset, limit }),
  listAgentRunEvents: (runId, offset = 0, limit = 50) => invoke<AgentRunEvent[]>("list_agent_run_events", { runId, offset, limit }),
  cancelAgentRun: (runId, version) => invoke<AgentRun>("cancel_agent_run", { runId, version }),
  resumeAgentRun: (runId, version) => invoke<AgentRun>("resume_agent_run", { runId, version }),
  getMissionControlOverview: (query = {}) => invoke<MissionControlOverview>("get_mission_control_overview", { query }),
  getMissionControlRun: (runId) => invoke<MissionControlRunDetail>("get_mission_control_run", { runId }),
  performMissionControlAction: (input) => invoke<MissionControlActionReceipt>("perform_mission_control_action", { input }),
  listAgentRunners: (sessionId, agentId) => invoke<AgentRunnerDescriptor[]>("list_agent_runners", { sessionId, agentId }),
  async openExternalUrl(url) {
    await openUrl(requireHttpsExternalUrl(url));
  },

  listAgents(capabilityTag) {
    return invoke<AgentRegistryEntry[]>("list_agents", { capabilityTag: capabilityTag ?? null });
  },

  registerApiAgent(input: RegisterApiAgentInput) {
    return invoke<AgentRegistryEntry>("register_api_agent", { input });
  },

  getApiAgentProviderConfig(agentId: string) {
    return invoke<ApiAgentProviderConfig | null>("get_api_agent_provider_config", { agentId });
  },

  getOnePieceProviderConfig() {
    return invoke<OnePieceProviderConfig>("get_onepiece_provider_config");
  },

  saveOnePieceProviderConfig(input: SaveOnePieceProviderConfigInput) {
    return invoke<OnePieceProviderConfig>("save_onepiece_provider_config", { input });
  },

  resetOnePieceProviderConfig() {
    return invoke<OnePieceProviderConfig>("reset_onepiece_provider_config");
  },

  listOnePieceProviderProfiles() {
    return invoke<OnePieceProviderProfiles>("list_onepiece_provider_profiles");
  },

  listOnePieceProviderPresets() {
    return invoke<OnePieceProviderPreset[]>("list_onepiece_provider_presets");
  },

  discoverOnePieceProviderModels(input: DiscoverOnePieceProviderModelsInput) {
    return invoke<OnePieceProviderModelDiscoveryResult>("discover_onepiece_provider_models", { input });
  },

  validateOnePieceProviderCredential(input: ValidateOnePieceProviderCredentialInput) {
    return invoke<ProviderCredentialValidationResult>("validate_onepiece_provider_credential", { input });
  },

  saveOnePieceProviderProfile(input: SaveOnePieceProviderProfileInput) {
    return invoke<OnePieceProviderProfiles>("save_onepiece_provider_profile", { input });
  },

  saveCustomOnePieceProviderProfile(input: SaveCustomOnePieceProviderProfileInput) {
    return invoke<OnePieceProviderProfiles>("save_custom_onepiece_provider_profile", { input });
  },

  getEndpointProfileMetadata(profileId: string) {
    return invoke<EndpointProfileMetadata | null>("get_endpoint_profile_metadata", { profileId });
  },

  discoverLocalModelEndpoints() {
    return invoke<LocalModelDiscoveryResult>("discover_local_model_endpoints");
  },

  verifyLocalModelEndpoint(baseUrl: string, timeoutMs: number) {
    return invoke<LocalModelDiscoveryResult>("verify_local_model_endpoint", {
      input: { baseUrl, timeoutMs },
    });
  },

  listHybridRoutingRules() {
    return invoke<HybridRoutingRule[]>("list_hybrid_routing_rules");
  },

  replaceHybridRoutingRules(rules: HybridRoutingRule[]) {
    return invoke<HybridRoutingRule[]>("replace_hybrid_routing_rules", { rules });
  },

  previewHybridRoute(input: HybridRoutePreviewInput) {
    return invoke<HybridRoutePreview>("preview_hybrid_route", { input });
  },

  activateOnePieceProviderProfile(profileId: string) {
    return invoke<OnePieceProviderProfiles>("activate_onepiece_provider_profile", { profileId });
  },

  deleteOnePieceProviderProfile(profileId: string) {
    return invoke<OnePieceProviderProfiles>("delete_onepiece_provider_profile", { profileId });
  },

  updateApiAgent(agentId: string, input: UpdateApiAgentInput) {
    return invoke<AgentRegistryEntry>("update_api_agent", { agentId, input });
  },

  deleteApiAgent(agentId: string) {
    return invoke<void>("delete_api_agent", { agentId });
  },

  listContextQualityHistory(input: ContextQualityHistoryQuery) {
    return invoke<ContextQualityHistoryPage>("list_context_quality_history", { input })
      .catch((error: unknown) => Promise.reject(normalizeContextQualityError(error)));
  },

  getContextQualitySummary(input: ContextQualitySummaryQuery) {
    return invoke<ContextQualitySummary>("get_context_quality_summary", { input })
      .catch((error: unknown) => Promise.reject(normalizeContextQualityError(error)));
  },

  listContextEvidenceManifests(input: ContextEvidenceManifestQuery) {
    return invoke<ContextEvidenceManifestPage>("list_context_evidence_manifests", { input });
  },

  getContextEvidenceManifest(generationId: string) {
    return invoke<ContextEvidenceManifest | null>("get_context_evidence_manifest", { generationId });
  },

  getRetrievalConfiguration() {
    return invoke<RetrievalConfiguration>("get_retrieval_configuration");
  },

  saveRetrievalConfiguration(profileId: string, modelId: string) {
    return invoke<void>("save_retrieval_configuration", { profileId, modelId });
  },

  saveCodeIndexAutomaticMode(mode: CodeIndexAutomaticMode) {
    return invoke<void>("save_code_index_automatic_mode", { mode });
  },

  listEmbeddingModels(profileId: string, transientCredential?: string) {
    return invoke<EmbeddingModelOption[]>("list_embedding_models", {
      profileId,
      transientCredential: transientCredential ?? null,
    });
  },

  getRetrievalIndexStatus() {
    return invoke<RetrievalIndexStatus>("get_retrieval_index_status");
  },

  rebuildRetrievalIndex() {
    return invoke<void>("rebuild_retrieval_index");
  },

  async listCodeIndexWorkspaces() {
    return normalizeCodeIndexWorkspaces(await invoke<unknown>("list_code_index_workspaces"));
  },

  async getCodeIndexWorkspace(workspaceId: string) {
    return normalizeCodeIndexWorkspace(await invoke<unknown>("get_code_index_workspace", { workspaceId }));
  },

  async registerCodeIndexWorkspace(root: string, displayName: string) {
    return normalizeCodeIndexWorkspace(await invoke<unknown>("register_code_index_workspace", { root, displayName }));
  },

  async saveCodeIndexConfiguration(workspaceId: string, configuration: CodeIndexConfigurationInput) {
    const normalized = normalizeCodeIndexConfiguration(configuration);
    return normalizeCodeIndexWorkspace(await invoke<unknown>("save_code_index_configuration", {
      workspaceId,
      configuration: normalized,
    }));
  },

  async refreshCodeIndexWorkspace(workspaceId: string) {
    return normalizeCodeIndexStatus(await invoke<unknown>("refresh_code_index_workspace", { workspaceId }));
  },

  async confirmCodeIndexEmbedding(workspaceId: string, profileId: string, model: string, generation: number) {
    return normalizeCodeEmbeddingConfirmation(await invoke<unknown>("confirm_code_index_embedding", {
      workspaceId,
      profileId,
      model,
      generation,
    }));
  },

  async getCodeIndexStatus(workspaceId: string) {
    return normalizeCodeIndexStatus(await invoke<unknown>("get_code_index_status", { workspaceId }));
  },

  async listCodeIndexAudit(workspaceId: string, limit?: number) {
    return normalizeCodeIndexAuditEntries(await invoke<unknown>("list_code_index_audit", {
      workspaceId,
      limit: limit ?? null,
    }));
  },

  async rebuildCodeIndexWorkspace(workspaceId: string) {
    return normalizeCodeIndexWorkspace(await invoke<unknown>("rebuild_code_index_workspace", { workspaceId }));
  },

  async disableCodeIndexWorkspace(workspaceId: string) {
    return normalizeCodeIndexWorkspace(await invoke<unknown>("disable_code_index_workspace", { workspaceId }));
  },

  deleteCodeIndexWorkspace(workspaceId: string) {
    return invoke<void>("delete_code_index_workspace", { workspaceId });
  },

  async getLspConfiguration() {
    return normalizeLspConfiguration(await invoke<unknown>("get_lsp_configuration"));
  },

  async saveLspConfiguration(configuration: LspConfiguration) {
    const normalized = normalizeLspConfiguration(configuration);
    await invoke<void>("save_lsp_configuration", { configuration: normalized });
  },

  async listLspWorkspaceTrust() {
    return normalizeLspWorkspaceTrustList(await invoke<unknown>("list_lsp_workspace_trust"));
  },

  async updateLspWorkspaceTrust(update: LspWorkspaceTrustUpdate) {
    const normalized = normalizeLspWorkspaceTrustUpdate(update);
    return normalizeLspWorkspaceTrust(await invoke<unknown>("update_lsp_workspace_trust", {
      update: normalized,
    }));
  },

  async discoverLspServers() {
    return normalizeLspServerDiscoveries(await invoke<unknown>("discover_lsp_servers"));
  },

  async testLspServer(language: LspLanguageId) {
    const input = normalizeLspServerTestInput({ language });
    return normalizeLspServerTestResult(await invoke<unknown>("test_lsp_server", { input }));
  },

  async installLspServer(language: LspLanguageId) {
    // The same input shape the test command validates, so an unregistered id is refused by the
    // one validator rather than by two that could disagree.
    await rejectWithReasonCode(
      invoke<void>("install_lsp_server", { input: normalizeLspServerTestInput({ language }) }),
    );
  },

  async uninstallLspServer(language: LspLanguageId) {
    await rejectWithReasonCode(
      invoke<void>("uninstall_lsp_server", { input: normalizeLspServerTestInput({ language }) }),
    );
  },

  async getLspServerStatus() {
    return normalizeLspServerStatuses(await invoke<unknown>("list_lsp_server_status"));
  },

  listCliParameterProfiles() {
    return invoke<CliParameterProfile[]>("list_cli_parameter_profiles");
  },

  previewCliParameterProfile(input: PreviewCliParameterProfileInput) {
    return invoke<CliParameterPreview>("preview_cli_parameter_profile", { input });
  },

  saveCliParameterProfile(input: SaveCliParameterProfileInput) {
    return invoke<CliParameterProfile>("save_cli_parameter_profile", { input });
  },

  resetCliParameterProfile(input: ResetCliParameterProfileInput) {
    return invoke<CliParameterProfile>("reset_cli_parameter_profile", { input });
  },

  listCliConfigPresets(agentId: string) {
    return Promise.resolve(getCliConfigPresets(requireCliConfigAgentId(agentId)));
  },

  listCliConfigProfiles(agentId: string) {
    return invoke<CliConfigProfile[]>("list_cli_config_profiles", { agentId });
  },

  getCliConfigStatus(agentId: string) {
    return invoke<CliConfigStatus>("get_cli_config_status", { agentId });
  },

  saveCliConfigProfile(input: SaveCliConfigProfileInput) {
    return invoke<CliConfigProfile>("save_cli_config_profile", { input });
  },

  validateCliConfigCredential(input: ValidateCliConfigCredentialInput) {
    return invoke<ProviderCredentialValidationResult>("validate_cli_config_credential", { input });
  },

  duplicateCliConfigProfile(agentId: string, profileId: string) {
    return invoke<CliConfigProfile>("duplicate_cli_config_profile", { agentId, profileId });
  },

  deleteCliConfigProfile(input: DeleteCliConfigProfileInput) {
    return invoke<void>("delete_cli_config_profile", { input });
  },

  importCliConfigProfile(input: ImportCliConfigProfileInput) {
    return invoke<CliConfigProfile>("import_cli_config_profile", { input });
  },

  discoverCliConfigProfiles(agentId: string) {
    return invoke<CliConfigDiscoveryResult>("discover_cli_config_profiles", { agentId });
  },

  importDiscoveredCliConfigProfiles(input: ImportDiscoveredCliConfigInput) {
    return invoke<ImportDiscoveredCliConfigResult>("import_discovered_cli_config_profiles", { input });
  },

  applyCliConfigProfile(input: ApplyCliConfigProfileInput) {
    return invoke<CliConfigApplyResult>("apply_cli_config_profile", { input });
  },

  listExpertRoles() {
    return invoke<ExpertRole[]>("list_expert_roles");
  },

  saveExpertRole(input: SaveExpertRoleInput) {
    return invoke<ExpertRole>("save_expert_role", { input });
  },

  deleteExpertRole(roleId: string) {
    return invoke<void>("delete_expert_role", { roleId });
  },

  getAgentById(agentId) {
    return invoke<AgentRegistryEntry>("get_agent_by_id", { agentId });
  },

  getWorkflowState() {
    return invoke<WorkflowState>("get_workflow_state");
  },

  selectAgent(agentId: string, interactionMode: InteractionMode) {
    return invoke<WorkflowState>("select_agent", { agentId, interactionMode });
  },

  checkBrowserReadiness(agentId: string) {
    return invoke<ReadinessStatus>("check_browser_readiness", { agentId });
  },

  launchActiveWorkflow() {
    return invoke<LaunchResult>("launch_active_workflow");
  },

  getSessionDetails() {
    return invoke<SessionDetails>("get_session_details");
  },

  listSessions() {
    return invoke<Session[]>("list_sessions");
  },

  listArchivedSessions() {
    return invoke<Session[]>("list_archived_sessions");
  },

  searchSessions(input: SessionSearchInput) {
    return invoke<SessionSearchResult[]>("search_sessions", {
      query: input.query,
      limit: input.limit ?? null,
    });
  },

  getSession(sessionId: string) {
    return invoke<Session>("get_session", { sessionId });
  },

  getActiveSession() {
    return invoke<Session | null>("get_active_session");
  },

  listSessionCategories() {
    return invoke<SessionCategory[]>("list_session_categories");
  },

  createSessionCategory(input: CreateSessionCategoryInput) {
    return invoke<SessionCategory>("create_session_category", { name: input.name });
  },

  renameSessionCategory(input: RenameSessionCategoryInput) {
    return invoke<SessionCategory>("rename_session_category", {
      categoryId: input.categoryId,
      name: input.name,
    });
  },

  async deleteSessionCategory(categoryId: string) {
    await invoke<void>("delete_session_category", { categoryId });
  },

  assignSessionCategory(input: AssignSessionCategoryInput) {
    return invoke<Session>("assign_session_category", {
      sessionId: input.sessionId,
      categoryId: input.categoryId,
    });
  },

  getAutomaticArchivalSettings() {
    return invoke<AutomaticArchivalSettings>("get_automatic_archival_settings");
  },

  saveAutomaticArchivalSettings(input: AutomaticArchivalSettings) {
    return invoke<AutomaticArchivalSettings>("save_automatic_archival_settings", { input });
  },

  listScheduledTasks() {
    return invoke<ScheduledTask[]>("list_scheduled_tasks");
  },
  listScheduledTaskRuns(taskId: string) {
    return invoke<ScheduledTaskRun[]>("list_scheduled_task_runs", { taskId });
  },

  createScheduledTask(input: CreateScheduledTaskInput) {
    return invoke<ScheduledTask>("create_scheduled_task", { input });
  },

  setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput) {
    return invoke<ScheduledTask>("set_scheduled_task_enabled", { input });
  },

  async deleteScheduledTask(taskId: string) {
    await invoke<void>("delete_scheduled_task", { taskId });
  },

  listLoopDefinitions() {
    return invoke<LoopDefinition[]>("list_loop_definitions");
  },

  createLoopDefinition(input: SaveLoopDefinitionInput) {
    return invoke<LoopDefinition>("create_loop_definition", { input });
  },

  updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput) {
    return invoke<LoopDefinition>("update_loop_definition", { definitionId, input });
  },

  async deleteLoopDefinition(definitionId: string) {
    await invoke<void>("delete_loop_definition", { definitionId });
  },

  listLoopRuns(definitionId?: string) {
    return invoke<LoopRun[]>("list_loop_runs", { definitionId: definitionId ?? null });
  },

  getLoopRun(runId: string) {
    return invoke<LoopRun>("get_loop_run", { runId });
  },

  startLoop(definitionId: string) {
    return invoke<StartLoopResult>("start_loop", { definitionId });
  },

  pauseLoop(runId: string) {
    return invoke<LoopRun>("pause_loop", { runId });
  },

  resumeLoop(runId: string) {
    return invoke<LoopRun>("resume_loop", { runId });
  },

  cancelLoop(runId: string) {
    return invoke<LoopRun>("cancel_loop", { runId });
  },

  acceptLoop(runId: string) {
    return invoke<LoopRun>("accept_loop", { runId });
  },

  continueLoop(input: ContinueLoopInput) {
    return invoke<LoopRun>("continue_loop", { input });
  },

  rejectLoop(runId: string) {
    return invoke<LoopRun>("reject_loop", { runId });
  },

  async subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void) {
    return subscribeLoopRunPolling(() => invoke<LoopRun>("get_loop_run", { runId }), handler);
  },

  getSessionChatConfig(sessionId) {
    return invoke<ChatConfig>("get_session_chat_config", { sessionId });
  },

  saveSessionChatConfig(sessionId, config) {
    return invoke<ChatConfig>("save_session_chat_config", { sessionId, config });
  },

  listKnownProjects() {
    return invoke<KnownProject[]>("list_known_projects");
  },

  listKnownRemoteWorkspaces() {
    return invoke<KnownRemoteWorkspace[]>("list_known_remote_workspaces");
  },

  inspectProject(path: string) {
    return invoke<ProjectInspection>("inspect_project", { path });
  },

  async selectProjectDirectory() {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },

  createSession(input) {
    return invoke<OperationTask>("create_session", {
      input,
    });
  },

  async deleteSession(sessionId: string) {
    await invoke<void>("delete_session", { sessionId });
  },

  switchSession(sessionId: string) {
    return invoke<Session>("switch_session", { sessionId });
  },

  renameSession(sessionId: string, title: string) {
    return invoke<Session>("rename_session", { sessionId, title });
  },

  updateSessionSeats(input: UpdateSessionSeatsInput) {
    return invoke<Session>("update_session_seats", { input });
  },

  rebindRemoteSessionSshConnection(sessionId: string, connectionId: string) {
    return invoke<Session>("rebind_remote_session_ssh_connection", {
      sessionId,
      connectionId,
    });
  },

  pinSession(sessionId: string) {
    return invoke<Session>("pin_session", { sessionId });
  },

  unpinSession(sessionId: string) {
    return invoke<Session>("unpin_session", { sessionId });
  },

  archiveSession(sessionId: string) {
    return invoke<Session>("archive_session", { sessionId });
  },

  unarchiveSession(sessionId: string) {
    return invoke<Session>("unarchive_session", { sessionId });
  },

  async exportSession(input: ExportSessionInput) {
    const destinationDirectory =
      input.destinationDirectory ??
      ((await open({ directory: true, multiple: false })) as string | string[] | null);
    return invoke<SessionExportResult>("export_session", {
      sessionId: input.sessionId,
      format: input.format,
      destinationDirectory: typeof destinationDirectory === "string" ? destinationDirectory : null,
    });
  },

  sendMessage(input) {
    return invoke<ChatMessage>("send_message", {
      sessionId: input.sessionId,
      content: input.content,
      config: input.config,
      fileReferences: input.fileReferences ?? null,
      runner: input.runner ?? null,
    });
  },

  listMessages(input) {
    return invoke<ChatMessage[]>("list_messages", {
      sessionId: input.sessionId,
      limit: input.limit ?? null,
      beforeId: input.beforeId ?? null,
    });
  },

  async saveMessageFeedback(input) {
    const saved = await invoke<{
      messageId: string;
      revision: number;
      state: MessageFeedback["state"] | null;
      correctionNote: string | null;
    }>("save_message_feedback", { input });
    return {
      state: saved.state,
      revision: saved.revision,
      ...(saved.correctionNote ? { correctionNote: saved.correctionNote } : {}),
    };
  },

  querySkillEvolutionEvidence(input) {
    return invoke("query_skill_evolution_evidence", { input });
  },

  getSkillEvolutionSeedLineage(seedId, input) {
    return invoke("get_skill_evolution_seed_lineage", { seedId, input });
  },

  purgeSkillEvolutionEvidence(input) {
    return invoke("purge_skill_evolution_evidence", { input });
  },

  async getUsageStatistics(input) {
    const statistics = await invoke<unknown>("get_usage_statistics", {
      range: input.range,
    });
    return normalizeTauriUsageStatistics(statistics);
  },

  async getSessionUsageSummary(sessionId: string) {
    const summary = await invoke<unknown>("get_session_usage_summary", { sessionId });
    return normalizeTauriSessionUsageSummary(summary);
  },

  getTokenUsageSummary(input) {
    return invoke<TokenUsageSummary>("get_token_usage_summary", { input });
  },

  getTokenUsageDetails(input) {
    return invoke<TokenUsageDetailsPage>("get_token_usage_details", { input });
  },

  async resolveAgentQuestion(sessionId: string, callId: string, answer: string) {
    return invoke<boolean>("resolve_agent_question", { sessionId, callId, answer });
  },

  async resolvePlanExit(sessionId: string, callId: string, approved: boolean) {
    return invoke<boolean>("resolve_plan_exit", { sessionId, callId, approved });
  },

  async stopGeneration(sessionId: string) {
    await invoke<void>("stop_generation", { sessionId });
  },


  openAgentTerminal(sessionId: string, size: AgentTerminalSize) {
    return invoke<AgentTerminalSession>("open_agent_terminal", { sessionId, size });
  },

  async sendAgentTerminalInput(terminalId: string, content: string) {
    await invoke<void>("send_agent_terminal_input", { terminalId, content });
  },

  async resizeAgentTerminal(terminalId: string, size: AgentTerminalSize) {
    await invoke<void>("resize_agent_terminal", { terminalId, size });
  },

  stopAgentTerminal(terminalId: string) {
    return invoke<boolean>("stop_agent_terminal", { terminalId });
  },

  async subscribeAgentTerminalEvents(sessionId, handler) {
    const unlisten = await listen<AgentTerminalEvent>("agent-terminal:event", (event) => {
      if (event.payload.sessionId === sessionId) {
        handler(event.payload);
      }
    });
    return unlisten;
  },

  async subscribeMessageEvents(sessionId, handler) {
    const unlisten = await listen<ChatStreamEvent>("chat:event", (event) => {
      if (event.payload.sessionId === sessionId) {
        handler(event.payload);
      }
    });
    return unlisten;
  },

  ...tauriCliEnvironmentClient,
  ...tauriPersonalizationClient,
  ...tauriSessionRecoveryClient,
  ...tauriSessionWorkspaceClient,
  async subscribeSessionEvents(handler) {
    return listen<unknown>("session:event", (event) => {
      if (isSessionStateEvent(event.payload)) handler(event.payload);
    });
  },

  listSkills(input: SkillScopeInput) {
    return invoke<SkillListResult>("list_skills", { input });
  },

  listSkillMountPaths() {
    return invoke<SkillAgentMountPath[]>("list_skill_mount_paths");
  },

  updateSkillMountPath(agentId: string, mountPath: string) {
    return invoke<SkillMountMigrationReport>("update_skill_mount_path", { agentId, mountPath });
  },

  createSkill(input: SkillMutationInput) {
    return invoke<Skill>("create_skill", { input });
  },

  updateSkill(skillId: string, input: SkillUpdateInput) {
    return invoke<Skill>("update_skill", { skillId, input });
  },

  async deleteSkill(skillId: string, input: SkillScopeInput) {
    await invoke<void>("delete_skill", { skillId, input });
  },

  restoreBuiltinSkill(skillId: string) {
    return invoke<Skill>("restore_builtin_skill", { skillId });
  },

  setSkillEnabled(skillId: string, input: SkillScopeInput, enabled: boolean) {
    return invoke<Skill>("set_skill_enabled", { skillId, input, enabled });
  },

  setSkillAgentBindings(skillId: string, input: SkillScopeInput, agentIds: string[]) {
    return invoke<Skill>("set_skill_agent_bindings", { skillId, input, agentIds });
  },

  getSkillOverview(input: SkillScopeInput) {
    return invoke<SkillOverview>("get_skill_overview", { input });
  },

  listSkillTools(input: SkillToolOwnerInput) {
    return invoke<SkillToolRevision[]>("list_skill_tools", { input });
  },

  validateSkillToolRevision(input: SkillToolRevisionInput) {
    return invoke<SkillToolRevision>("validate_skill_tool_revision", { input });
  },

  setSkillToolTrust(input: SkillToolTrustInput) {
    return invoke<SkillToolRevision>("set_skill_tool_trust", { input });
  },

  setSkillToolEnabled(input: SkillToolEnablementInput) {
    return invoke<SkillToolRevision>("set_skill_tool_enabled", { input });
  },

  quarantineSkillTool(input: SkillToolQuarantineInput) {
    return invoke<SkillToolRevision>("quarantine_skill_tool", { input });
  },

  recoverSkillTool(input: SkillToolRevisionInput) {
    return invoke<SkillToolRevision>("recover_skill_tool", { input });
  },

  getSkillToolDiagnostics(input: SkillToolRevisionInput) {
    return invoke<SkillToolRevision>("get_skill_tool_diagnostics", { input });
  },

  bindSkillToCliAgent(skillId: string, input: SkillScopeInput, agentId: string) {
    return invoke<Skill>("bind_skill_to_cli_agent", { skillId, input, agentId });
  },

  unbindSkillFromCliAgent(skillId: string, input: SkillScopeInput, agentId: string) {
    return invoke<Skill>("unbind_skill_from_cli_agent", { skillId, input, agentId });
  },

  bindSkillToApiAgent(skillId: string, input: SkillScopeInput, agentId: string) {
    return invoke<void>("bind_skill_to_api_agent", { skillId, input, agentId });
  },

  unbindSkillFromApiAgent(skillId: string, input: SkillScopeInput, agentId: string) {
    return invoke<void>("unbind_skill_from_api_agent", { skillId, input, agentId });
  },

  listSkillApiAgentBindings(skillId: string, input: SkillScopeInput) {
    return invoke<string[]>("list_skill_api_agent_bindings", { skillId, input });
  },

  previewSkill(skillId: string, input: SkillScopeInput) {
    return invoke<SkillPreview>("preview_skill", { skillId, input });
  },

  loadSkill(input: SkillLoadInput) {
    return invoke<SkillLoadOutcome>("load_skill", { input });
  },

  readSkillResource(input: SkillResourceReadInput) {
    return invoke<SkillResourceReadOutcome>("read_skill_resource", { input });
  },

  importSkill(input: SkillImportInput) {
    return invoke<Skill>("import_skill", { input });
  },

  detectSkillDrift(input: SkillScopeInput) {
    return invoke<SkillDriftReport>("detect_skill_drift", { input });
  },

  syncSkillDrift(input: SkillScopeInput) {
    return invoke<SkillSyncResult>("sync_skill_drift", { input });
  },

  getSkillOverlaySummary(input) {
    return invokeSkillOverlay<SkillOverlaySummary>("get_skill_overlay_summary", input);
  },

  getSkillOverlayDetail(input) {
    return invokeSkillOverlay<SkillOverlayDetail>("get_skill_overlay_detail", input);
  },

  previewSkillOverlay(input) {
    return invokeSkillOverlay<SkillOverlayPreview>("preview_skill_overlay", input);
  },

  getSkillOverlayHistory(input) {
    return invokeSkillOverlay<SkillOverlayHistoryPage>("get_skill_overlay_history", input);
  },

  createSkillOverlayPatch(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("create_skill_overlay_patch", input);
  },

  createSkillOverlayGuidance(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("create_skill_overlay_guidance", input);
  },

  addSkillOverlayFile(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("add_skill_overlay_file", input);
  },

  replaceSkillOverlayFile(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("replace_skill_overlay_file", input);
  },

  importSkillOverlay(input) {
    return invokeSkillOverlay<SkillOverlayImportReview>("import_skill_overlay", input);
  },

  promoteSkillOverlay(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("promote_skill_overlay", input);
  },

  disableSkillOverlayMutation(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("disable_skill_overlay_mutation", input);
  },

  revertSkillOverlayMutation(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("revert_skill_overlay_mutation", input);
  },

  previewSkillOverlayReconciliation(input) {
    return invokeSkillOverlay<SkillOverlayReconciliationPreview>("preview_skill_overlay_reconciliation", input);
  },

  reconcileSkillOverlay(input) {
    return invokeSkillOverlay<SkillOverlayMutationOutcome>("reconcile_skill_overlay", input);
  },

  listPromptHooks() {
    return invoke<PromptHookListResult>("list_prompt_hooks");
  },

  createPromptHook(input: PromptHookMutationInput) {
    return invoke<PromptHook>("create_prompt_hook", { input });
  },

  updatePromptHook(hookId: string, input: PromptHookUpdateInput) {
    return invoke<PromptHook>("update_prompt_hook", { hookId, input });
  },

  async deletePromptHook(hookId: string) {
    await invoke<void>("delete_prompt_hook", { hookId });
  },

  setPromptHookEnabled(hookId: string, enabled: boolean) {
    return invoke<PromptHook>("set_prompt_hook_enabled", { hookId, enabled });
  },

  setPromptHookCliBindings(hookId: string, agentIds: string[]) {
    return invoke<PromptHook>("set_prompt_hook_cli_bindings", { hookId, agentIds });
  },

  previewPromptHook(input: PromptHookPreviewInput) {
    return invoke<PromptHookPreview>("preview_prompt_hook", { input });
  },

  previewPromptAssembly(input: PromptAssemblyPreviewInput) {
    return invoke<PromptHookPreview>("preview_prompt_assembly", { input });
  },

  listPromptHookTraces(limit?: number) {
    return invoke<PromptHookTraceSummary[]>("list_prompt_hook_traces", { limit: limit ?? null });
  },

  listPromptHookVariables() {
    return invoke<PromptHookVariableDefinition[]>("list_prompt_hook_variables");
  },

  savePromptHookDraft(input: SavePromptHookDraftInput) {
    return invoke<PromptHookDraft>("save_prompt_hook_draft", { input });
  },

  publishPromptHook(input: PublishPromptHookInput) {
    return invoke<PromptHookVersion>("publish_prompt_hook", { input });
  },

  getPromptHookVersionHistory(hookId: string) {
    return invoke<PromptHookVersionHistory>("get_prompt_hook_version_history", { hookId });
  },

  rollbackPromptHook(input: RollbackPromptHookInput) {
    return invoke<PromptHookVersion>("rollback_prompt_hook", { input });
  },

  selectWorkspaceDirectory() {
    return invoke<string | null>("select_workspace_directory");
  },
};
