import type {
  AgentMemory,
  AgentRegistryEntry,
  ApiAgentProviderConfig,
  AgentTerminalEvent,
  AgentTerminalSession,
  AgentTerminalSize,
  AssignSessionCategoryInput,
  AutomaticArchivalSettings,
  CliParameterProfile,
  CreateSessionCategoryInput,
  ManagedCliAgentId,
  ExportSessionInput,
  SaveCliParameterProfileInput,
  CliPackageOperationInput,
  CliToolStatus,
  CreateSessionInput,
  DiscoverOnePieceProviderModelsInput,
  InteractionMode,
  KnownRemoteWorkspace,
  RenameSessionCategoryInput,
  KnownProject,
  LaunchResult,
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
  SaveOnePieceProviderProfileInput,
  ValidateOnePieceProviderCredentialInput,
  UpdateApiAgentInput,
  UpdateSessionSeatsInput,
  CreateScheduledTaskInput,
  SetScheduledTaskEnabledInput,
  Session,
  SessionCategory,
  SessionDetails,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  ScheduledTask,
  ScheduledTaskRun,
  WorkflowState,
} from "../types/agent";
import type { ChatConfig, ChatMessage, ChatStreamEvent, MessageFeedback, SaveMessageFeedbackInput, SendMessageInput, SessionUsageSummary, UsageStatistics, UsageStatisticsRange } from "../types/chat";

export interface EvidenceQueryInput {
  workspace?: string;
  skillId?: string;
  limit?: number;
  cursor?: string;
}

export interface EvidenceSignalSummary {
  signalId: string;
  sourceKind: string;
  category: string;
  polarity: string;
  severity: string;
  attribution: string;
  attributionRationale: string;
  sourceFidelity: string;
  sourceAgentId?: string;
  extractorId: string;
  extractorVersion: number;
  safeSummary: string;
  occurredAt: string;
  sanitizerVersion: number;
  associationTruncatedCount: number;
  sourceLinkTruncatedCount: number;
}

export interface EvidenceSeedSummary {
  seedId: string;
  category: string;
  readiness: string;
  readinessReason: string;
  safeSummary: string;
  signalCount: number;
  truncatedSignalCount: number;
  independentRunCount: number;
  hasRecovery: boolean;
  firstOccurredAt: string;
  lastOccurredAt: string;
  createdAt: string;
}

export interface EvidencePipelineSummary {
  collectionEnabled: boolean;
  status: "healthy" | "degraded" | "disabled";
  queueDepth: number;
  failureCount: number;
}

export interface EvidenceOverview {
  signalCount: number;
  seedCount: number;
  firstOccurredAt?: string;
  lastOccurredAt?: string;
  distributions: Record<string, Record<string, number>>;
  signals: EvidenceSignalSummary[];
  seeds: EvidenceSeedSummary[];
  nextCursor?: string;
  pipeline: EvidencePipelineSummary;
  retentionDays: number;
  signalQuota: number;
  seedQuota: number;
  byteQuota: number;
  droppedCount: number;
  expiredCount: number;
}

export interface EvidenceSeedLineage {
  seed: EvidenceSeedSummary;
  signals: EvidenceSignalSummary[];
}

export interface PurgeEvidenceInput {
  operationId: string;
  workspace?: string;
  skillId: string;
  confirmed: boolean;
}

export interface PurgeEvidenceOutcome {
  operationId: string;
  deletedSignals: number;
  deletedSeeds: number;
  deletedFeedback: number;
}
import type {
  TokenUsageDetailsPage,
  TokenUsageDetailsQuery,
  TokenUsageSummary,
  TokenUsageSummaryQuery,
} from "../types/token-usage";
import type { OperationTask } from "../types/operation";
import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  LoopRun,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";
import type {
  CreateShellInput,
  DirectoryListing,
  DocumentListing,
  FileContent,
  FileSearchListing,
  GitDiffResult,
  GitDiffSource,
  GitStatusResult,
  ResizeShellInput,
  SessionLogExportResult,
  SessionLogPage,
  SessionLogQuery,
  ShellEvent,
  ShellSession,
} from "../types/session-workspace";
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
  SkillOverlayDetail,
  SkillOverlayFileInput,
  SkillOverlayGuidanceInput,
  SkillOverlayHistoryInput,
  SkillOverlayHistoryPage,
  SkillOverlayImportInput,
  SkillOverlayImportReview,
  SkillOverlayMutationOutcome,
  SkillOverlayMutationStateInput,
  SkillOverlayPatchInput,
  SkillOverlayPreview,
  SkillOverlayPreviewInput,
  SkillOverlayPromotionInput,
  SkillOverlaySummary,
  SkillOverlayTargetInput,
} from "../types/skill-overlay";
import type {
  SkillOverlayReconciliationInput,
  SkillOverlayReconciliationPreview,
} from "../types/skill-overlay-reconciliation";
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
import type { FolderOpenerAvailability, FolderOpenerId, FolderOpenerPreferences, OpenSessionFolderResult, SaveFolderOpenerPreferencesInput } from "../types/folder-opener";
import type {
  ApplyCliConfigProfileInput,
  CliConfigApplyResult,
  CliConfigDiscoveryResult,
  CliConfigPreset,
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
import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";
import type {
  CodeEmbeddingConfirmation,
  CodeIndexAuditEntry,
  CodeIndexAutomaticMode,
  CodeIndexConfigurationInput,
  CodeIndexStatus,
  CodeIndexWorkspace,
} from "../types/code-index";
import type {
  LspConfiguration,
  LspLanguageId,
  LspServerDiscovery,
  LspServerStatus,
  LspServerTestResult,
  LspWorkspaceTrust,
  LspWorkspaceTrustUpdate,
} from "../types/lsp";
import type {
  ContextQualityHistoryPage,
  ContextQualityHistoryQuery,
  ContextQualitySummary,
  ContextQualitySummaryQuery,
} from "../types/context-quality";
import type { ContextEvidenceManifest, ContextEvidenceManifestPage, ContextEvidenceManifestQuery } from "../types/context-engine";
import type { BuiltinToolService } from "./builtin-tool-service";
import type { AddReviewCommentInput, CodeReview, ReviewAction, ReviewComment, ReviewDecision, ReviewDiffFile, ReviewRevertReceipt, RevertReviewChangeInput } from "../types/code-review";

export interface AgentService extends BuiltinToolService {
  openCodeReview(sessionId: string): Promise<CodeReview>;
  getCodeReview(reviewId: string): Promise<CodeReview>;
  loadCodeReviewFile(sessionId: string, path: string, expectedSnapshot: string): Promise<ReviewDiffFile>;
  addCodeReviewComment(input: AddReviewCommentInput): Promise<ReviewComment>;
  resolveCodeReviewComment(reviewId: string, commentId: string): Promise<CodeReview>;
  selectCodeReviewComment(reviewId: string, commentId: string, selected: boolean): Promise<CodeReview>;
  setCodeReviewDecision(reviewId: string, decision: ReviewDecision): Promise<CodeReview>;
  revertCodeReviewChange(input: RevertReviewChangeInput): Promise<ReviewRevertReceipt>;
  sendCodeReviewFeedback(reviewId: string, acknowledgeStale: boolean): Promise<{ messageId: string }>;
  startCodeReviewAction(reviewId: string, action: ReviewAction): Promise<{ operationId: string }>;
  openExternalUrl(url: string): Promise<void>;
  listAgents(capabilityTag?: string): Promise<AgentRegistryEntry[]>;
  registerApiAgent(input: RegisterApiAgentInput): Promise<AgentRegistryEntry>;
  getApiAgentProviderConfig(agentId: string): Promise<ApiAgentProviderConfig | null>;
  getOnePieceProviderConfig(): Promise<OnePieceProviderConfig>;
  saveOnePieceProviderConfig(input: SaveOnePieceProviderConfigInput): Promise<OnePieceProviderConfig>;
  resetOnePieceProviderConfig(): Promise<OnePieceProviderConfig>;
  listOnePieceProviderProfiles(): Promise<OnePieceProviderProfiles>;
  listOnePieceProviderPresets(): Promise<OnePieceProviderPreset[]>;
  discoverOnePieceProviderModels(input: DiscoverOnePieceProviderModelsInput): Promise<OnePieceProviderModelDiscoveryResult>;
  validateOnePieceProviderCredential(input: ValidateOnePieceProviderCredentialInput): Promise<ProviderCredentialValidationResult>;
  saveOnePieceProviderProfile(input: SaveOnePieceProviderProfileInput): Promise<OnePieceProviderProfiles>;
  activateOnePieceProviderProfile(profileId: string): Promise<OnePieceProviderProfiles>;
  deleteOnePieceProviderProfile(profileId: string): Promise<OnePieceProviderProfiles>;
  updateApiAgent(agentId: string, input: UpdateApiAgentInput): Promise<AgentRegistryEntry>;
  deleteApiAgent(agentId: string): Promise<void>;
  /** `add-cli-memory-support`: memories are a single host-level pool shared by every agent — no
   * `agentId` scoping on read or bulk-reset, `AgentMemory.agentId` remains as provenance only. */
  listAllMemories(): Promise<AgentMemory[]>;
  deleteAgentMemory(memoryId: string): Promise<void>;
  resetAllMemories(): Promise<void>;
  listContextQualityHistory(input: ContextQualityHistoryQuery): Promise<ContextQualityHistoryPage>;
  getContextQualitySummary(input: ContextQualitySummaryQuery): Promise<ContextQualitySummary>;
  listContextEvidenceManifests(input: ContextEvidenceManifestQuery): Promise<ContextEvidenceManifestPage>;
  getContextEvidenceManifest(generationId: string): Promise<ContextEvidenceManifest | null>;
  // Configuration, index status and rebuild are all global: retrieval applies to every agent,
  // so status aggregates across every agent and `scope_folder`, and rebuild requeues all of them.
  getRetrievalConfiguration(): Promise<RetrievalConfiguration>;
  saveRetrievalConfiguration(profileId: string, modelId: string): Promise<void>;
  saveCodeIndexAutomaticMode(mode: CodeIndexAutomaticMode): Promise<void>;
  listEmbeddingModels(profileId: string, transientCredential?: string): Promise<EmbeddingModelOption[]>;
  getRetrievalIndexStatus(): Promise<RetrievalIndexStatus>;
  rebuildRetrievalIndex(): Promise<void>;
  listCodeIndexWorkspaces(): Promise<CodeIndexWorkspace[]>;
  getCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  registerCodeIndexWorkspace(root: string, displayName: string): Promise<CodeIndexWorkspace>;
  saveCodeIndexConfiguration(
    workspaceId: string,
    configuration: CodeIndexConfigurationInput,
  ): Promise<CodeIndexWorkspace>;
  refreshCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexStatus>;
  confirmCodeIndexEmbedding(
    workspaceId: string,
    profileId: string,
    model: string,
    generation: number,
  ): Promise<CodeEmbeddingConfirmation>;
  getCodeIndexStatus(workspaceId: string): Promise<CodeIndexStatus>;
  listCodeIndexAudit(workspaceId: string, limit?: number): Promise<CodeIndexAuditEntry[]>;
  rebuildCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  disableCodeIndexWorkspace(workspaceId: string): Promise<CodeIndexWorkspace>;
  deleteCodeIndexWorkspace(workspaceId: string): Promise<void>;
  getLspConfiguration(): Promise<LspConfiguration>;
  saveLspConfiguration(configuration: LspConfiguration): Promise<void>;
  listLspWorkspaceTrust(): Promise<LspWorkspaceTrust[]>;
  updateLspWorkspaceTrust(update: LspWorkspaceTrustUpdate): Promise<LspWorkspaceTrust>;
  discoverLspServers(): Promise<LspServerDiscovery[]>;
  testLspServer(language: LspLanguageId): Promise<LspServerTestResult>;
  getLspServerStatus(): Promise<LspServerStatus[]>;
  listCliTools(): Promise<CliToolStatus[]>;
  refreshCliDetections(agentId?: string): Promise<OperationTask>;
  installCliVersion(input: CliPackageOperationInput): Promise<OperationTask>;
  upgradeAllCliVersions(): Promise<OperationTask>;
  listCliParameterProfiles(): Promise<CliParameterProfile[]>;
  saveCliParameterProfile(input: SaveCliParameterProfileInput): Promise<CliParameterProfile>;
  resetCliParameterProfile(agentId: ManagedCliAgentId): Promise<CliParameterProfile>;
  listCliConfigPresets(agentId: string): Promise<CliConfigPreset[]>;
  listCliConfigProfiles(agentId: string): Promise<CliConfigProfile[]>;
  getCliConfigStatus(agentId: string): Promise<CliConfigStatus>;
  saveCliConfigProfile(input: SaveCliConfigProfileInput): Promise<CliConfigProfile>;
  validateCliConfigCredential(input: ValidateCliConfigCredentialInput): Promise<ProviderCredentialValidationResult>;
  duplicateCliConfigProfile(agentId: string, profileId: string): Promise<CliConfigProfile>;
  deleteCliConfigProfile(input: DeleteCliConfigProfileInput): Promise<void>;
  importCliConfigProfile(input: ImportCliConfigProfileInput): Promise<CliConfigProfile>;
  discoverCliConfigProfiles(agentId: string): Promise<CliConfigDiscoveryResult>;
  importDiscoveredCliConfigProfiles(input: ImportDiscoveredCliConfigInput): Promise<ImportDiscoveredCliConfigResult>;
  applyCliConfigProfile(input: ApplyCliConfigProfileInput): Promise<CliConfigApplyResult>;
  getAgentById(agentId: string): Promise<AgentRegistryEntry | null>;
  getWorkflowState(): Promise<WorkflowState>;
  selectAgent(agentId: string, interactionMode: InteractionMode): Promise<WorkflowState>;
  checkBrowserReadiness(agentId: string): Promise<ReadinessStatus>;
  launchActiveWorkflow(): Promise<LaunchResult>;
  getSessionDetails(): Promise<SessionDetails>;
  listSessions(): Promise<Session[]>;
  listArchivedSessions(): Promise<Session[]>;
  searchSessions(input: SessionSearchInput): Promise<SessionSearchResult[]>;
  getSession(sessionId: string): Promise<Session>;
  getSessionRecoverySummary(sessionId: string): Promise<SessionRecoverySummary>;
  listSessionRecoveryReports(sessionId: string, limit?: number): Promise<SessionRecoveryReport[]>;
  acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement>;
  getActiveSession(): Promise<Session | null>;
  listSessionCategories(): Promise<SessionCategory[]>;
  createSessionCategory(input: CreateSessionCategoryInput): Promise<SessionCategory>;
  renameSessionCategory(input: RenameSessionCategoryInput): Promise<SessionCategory>;
  deleteSessionCategory(categoryId: string): Promise<void>;
  assignSessionCategory(input: AssignSessionCategoryInput): Promise<Session>;
  getAutomaticArchivalSettings(): Promise<AutomaticArchivalSettings>;
  saveAutomaticArchivalSettings(input: AutomaticArchivalSettings): Promise<AutomaticArchivalSettings>;
  listScheduledTasks(): Promise<ScheduledTask[]>;
  listScheduledTaskRuns(taskId: string): Promise<ScheduledTaskRun[]>;
  createScheduledTask(input: CreateScheduledTaskInput): Promise<ScheduledTask>;
  setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput): Promise<ScheduledTask>;
  deleteScheduledTask(taskId: string): Promise<void>;
  listLoopDefinitions(): Promise<LoopDefinition[]>;
  createLoopDefinition(input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput): Promise<LoopDefinition>;
  deleteLoopDefinition(definitionId: string): Promise<void>;
  listLoopRuns(definitionId?: string): Promise<LoopRun[]>;
  getLoopRun(runId: string): Promise<LoopRun>;
  startLoop(definitionId: string): Promise<StartLoopResult>;
  pauseLoop(runId: string): Promise<LoopRun>;
  resumeLoop(runId: string): Promise<LoopRun>;
  cancelLoop(runId: string): Promise<LoopRun>;
  acceptLoop(runId: string): Promise<LoopRun>;
  continueLoop(input: ContinueLoopInput): Promise<LoopRun>;
  rejectLoop(runId: string): Promise<LoopRun>;
  subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void): Promise<() => void>;
  getSessionChatConfig(sessionId: string): Promise<ChatConfig>;
  saveSessionChatConfig(sessionId: string, config: ChatConfig): Promise<ChatConfig>;
  listKnownProjects(): Promise<KnownProject[]>;
  listKnownRemoteWorkspaces(): Promise<KnownRemoteWorkspace[]>;
  inspectProject(path: string): Promise<ProjectInspection>;
  selectProjectDirectory(): Promise<string | null>;
  createSession(input: CreateSessionInput): Promise<OperationTask>;
  deleteSession(sessionId: string): Promise<void>;
  switchSession(sessionId: string): Promise<Session>;
  renameSession(sessionId: string, title: string): Promise<Session>;
  updateSessionSeats(input: UpdateSessionSeatsInput): Promise<Session>;
  rebindRemoteSessionSshConnection(sessionId: string, connectionId: string): Promise<Session>;
  pinSession(sessionId: string): Promise<Session>;
  unpinSession(sessionId: string): Promise<Session>;
  archiveSession(sessionId: string): Promise<Session>;
  unarchiveSession(sessionId: string): Promise<Session>;
  exportSession(input: ExportSessionInput): Promise<SessionExportResult>;
  sendMessage(input: SendMessageInput): Promise<ChatMessage>;
  listMessages(input: { sessionId: string; limit?: number; beforeId?: string }): Promise<ChatMessage[]>;
  saveMessageFeedback(input: SaveMessageFeedbackInput): Promise<MessageFeedback>;
  querySkillEvolutionEvidence(input: EvidenceQueryInput): Promise<EvidenceOverview>;
  getSkillEvolutionSeedLineage(seedId: string, input: EvidenceQueryInput): Promise<EvidenceSeedLineage | null>;
  purgeSkillEvolutionEvidence(input: PurgeEvidenceInput): Promise<PurgeEvidenceOutcome>;
  getUsageStatistics(input: { range: UsageStatisticsRange }): Promise<UsageStatistics>;
  getSessionUsageSummary(sessionId: string): Promise<SessionUsageSummary>;
  getTokenUsageSummary(input: TokenUsageSummaryQuery): Promise<TokenUsageSummary>;
  getTokenUsageDetails(input: TokenUsageDetailsQuery): Promise<TokenUsageDetailsPage>;
  /**
   * Delivers the user's answer to a tool call waiting in `awaiting_input`. Resolves to whether a
   * live waiter received it, so the caller can distinguish a delivered answer from one aimed at a
   * question that already resolved or whose generation is gone.
   */
  resolveAgentQuestion(sessionId: string, callId: string, answer: string): Promise<boolean>;
  /**
   * Delivers the user's decision on an `exit_plan_mode` call waiting in `awaiting_input`. Resolves
   * to whether a live waiter received it — the caller must only change the session's execution
   * mode when it did, so an approval aimed at a dead generation cannot leave a session
   * write-capable on the strength of a decision the model never saw.
   */
  resolvePlanExit(sessionId: string, callId: string, approved: boolean): Promise<boolean>;
  stopGeneration(sessionId: string): Promise<void>;
  openAgentTerminal(sessionId: string, size: AgentTerminalSize): Promise<AgentTerminalSession>;
  sendAgentTerminalInput(terminalId: string, content: string): Promise<void>;
  resizeAgentTerminal(terminalId: string, size: AgentTerminalSize): Promise<void>;
  stopAgentTerminal(terminalId: string): Promise<boolean>;
  subscribeAgentTerminalEvents(
    sessionId: string,
    handler: (event: AgentTerminalEvent) => void,
  ): Promise<() => void>;
  subscribeMessageEvents(
    sessionId: string,
    handler: (event: ChatStreamEvent) => void,
  ): Promise<() => void>;
  listSessionDirectory(sessionId: string, path?: string): Promise<DirectoryListing>;
  readSessionFile(sessionId: string, path: string): Promise<FileContent>;
  listSessionDocuments(sessionId: string): Promise<DocumentListing>;
  searchSessionFiles(sessionId: string, query: string, maxResults?: number): Promise<FileSearchListing>;
  getSessionGitStatus(sessionId: string): Promise<GitStatusResult>;
  getSessionGitDiff(sessionId: string, path: string, source: GitDiffSource): Promise<GitDiffResult>;
  listSessionLogs(input: SessionLogQuery): Promise<SessionLogPage>;
  exportSessionLogs(input: SessionLogQuery): Promise<SessionLogExportResult>;
  listFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  refreshFolderOpeners(): Promise<FolderOpenerAvailability[]>;
  getFolderOpenerPreferences(): Promise<FolderOpenerPreferences>;
  saveFolderOpenerPreferences(input: SaveFolderOpenerPreferencesInput): Promise<FolderOpenerPreferences>;
  openSessionFolder(sessionId: string, openerId: FolderOpenerId): Promise<OpenSessionFolderResult>;
  subscribeFolderOpenerEvents(handler: () => void): Promise<() => void>;
  createShell(input: CreateShellInput): Promise<ShellSession>;
  writeShellInput(shellId: string, content: string): Promise<void>;
  resetShellDirectory(shellId: string): Promise<void>;
  resizeShell(input: ResizeShellInput): Promise<void>;
  killShell(shellId: string): Promise<void>;
  subscribeShellEvents(shellId: string, handler: (event: ShellEvent) => void): Promise<() => void>;
  subscribeSessionEvents(handler: (event: SessionStateEvent) => void): Promise<() => void>;
  listExpertRoles(): Promise<ExpertRole[]>;
  saveExpertRole(input: SaveExpertRoleInput): Promise<ExpertRole>;
  deleteExpertRole(roleId: string): Promise<void>;
  listSkills(input: SkillScopeInput): Promise<SkillListResult>;
  getSkillOverview(input: SkillScopeInput): Promise<SkillOverview>;
  listSkillMountPaths(): Promise<SkillAgentMountPath[]>;
  updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport>;
  createSkill(input: SkillMutationInput): Promise<Skill>;
  updateSkill(skillId: string, input: SkillUpdateInput): Promise<Skill>;
  deleteSkill(skillId: string, input: SkillScopeInput): Promise<void>;
  restoreBuiltinSkill(skillId: string): Promise<Skill>;
  setSkillEnabled(skillId: string, input: SkillScopeInput, enabled: boolean): Promise<Skill>;
  setSkillAgentBindings(skillId: string, input: SkillScopeInput, agentIds: string[]): Promise<Skill>;
  bindSkillToCliAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<Skill>;
  unbindSkillFromCliAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<Skill>;
  bindSkillToApiAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<void>;
  unbindSkillFromApiAgent(skillId: string, input: SkillScopeInput, agentId: string): Promise<void>;
  listSkillApiAgentBindings(skillId: string, input: SkillScopeInput): Promise<string[]>;
  previewSkill(skillId: string, input: SkillScopeInput): Promise<SkillPreview>;
  loadSkill(input: SkillLoadInput): Promise<SkillLoadOutcome>;
  readSkillResource(input: SkillResourceReadInput): Promise<SkillResourceReadOutcome>;
  importSkill(input: SkillImportInput): Promise<Skill>;
  detectSkillDrift(input: SkillScopeInput): Promise<SkillDriftReport>;
  syncSkillDrift(input: SkillScopeInput): Promise<SkillSyncResult>;
  getSkillOverlaySummary(input: SkillOverlayTargetInput): Promise<SkillOverlaySummary>;
  getSkillOverlayDetail(input: SkillOverlayTargetInput): Promise<SkillOverlayDetail>;
  previewSkillOverlay(input: SkillOverlayPreviewInput): Promise<SkillOverlayPreview>;
  getSkillOverlayHistory(input: SkillOverlayHistoryInput): Promise<SkillOverlayHistoryPage>;
  createSkillOverlayPatch(input: SkillOverlayPatchInput): Promise<SkillOverlayMutationOutcome>;
  createSkillOverlayGuidance(input: SkillOverlayGuidanceInput): Promise<SkillOverlayMutationOutcome>;
  addSkillOverlayFile(input: SkillOverlayFileInput): Promise<SkillOverlayMutationOutcome>;
  replaceSkillOverlayFile(input: SkillOverlayFileInput): Promise<SkillOverlayMutationOutcome>;
  importSkillOverlay(input: SkillOverlayImportInput): Promise<SkillOverlayImportReview>;
  promoteSkillOverlay(input: SkillOverlayPromotionInput): Promise<SkillOverlayMutationOutcome>;
  disableSkillOverlayMutation(
    input: SkillOverlayMutationStateInput,
  ): Promise<SkillOverlayMutationOutcome>;
  revertSkillOverlayMutation(
    input: SkillOverlayMutationStateInput,
  ): Promise<SkillOverlayMutationOutcome>;
  previewSkillOverlayReconciliation(
    input: SkillOverlayReconciliationInput,
  ): Promise<SkillOverlayReconciliationPreview>;
  reconcileSkillOverlay(
    input: SkillOverlayReconciliationInput,
  ): Promise<SkillOverlayMutationOutcome>;
  listPromptHooks(): Promise<PromptHookListResult>;
  createPromptHook(input: PromptHookMutationInput): Promise<PromptHook>;
  updatePromptHook(hookId: string, input: PromptHookUpdateInput): Promise<PromptHook>;
  deletePromptHook(hookId: string): Promise<void>;
  setPromptHookEnabled(hookId: string, enabled: boolean): Promise<PromptHook>;
  setPromptHookCliBindings(hookId: string, agentIds: string[]): Promise<PromptHook>;
  previewPromptHook(input: PromptHookPreviewInput): Promise<PromptHookPreview>;
  previewPromptAssembly(input: PromptAssemblyPreviewInput): Promise<PromptHookPreview>;
  listPromptHookTraces(limit?: number): Promise<PromptHookTraceSummary[]>;
  listPromptHookVariables(): Promise<PromptHookVariableDefinition[]>;
  savePromptHookDraft(input: SavePromptHookDraftInput): Promise<PromptHookDraft>;
  publishPromptHook(input: PublishPromptHookInput): Promise<PromptHookVersion>;
  getPromptHookVersionHistory(hookId: string): Promise<PromptHookVersionHistory>;
  rollbackPromptHook(input: RollbackPromptHookInput): Promise<PromptHookVersion>;
  selectWorkspaceDirectory(): Promise<string | null>;
}

export type SessionStateEvent =
  | { kind: "active-session-changed"; sessionId: string | null }
  | { kind: "configuration-changed"; sessionId: string }
  | RecoverySessionStateEvent;

export type RecoveryDecision =
  | "completed"
  | "failed"
  | "cancelled"
  | "interrupted_without_tool_ambiguity"
  | "action_required"
  | "quarantined"
  | "retry_later"
  | "acknowledged";

export type RecoveryTrigger = "startup" | "explicit_retry" | "user_acknowledgement";

export type RecoveryReasonCode =
  | "confirmed_completed_message"
  | "confirmed_failed_message"
  | "confirmed_cancelled_operation"
  | "interrupted_tool_free_response"
  | "missing_execution_run"
  | "missing_assistant_message"
  | "unfinished_tool_activity"
  | "opaque_provider_activity"
  | "conflicting_execution_runs"
  | "conflicting_terminal_outcomes"
  | "invalid_message_sequence"
  | "invalid_execution_correlation"
  | "live_runtime_handle"
  | "storage_temporarily_unavailable"
  | "acknowledged_by_user";

export type RecoveryEvidenceReference =
  | { kind: "session"; sessionId: string; stateRevision: number; historyRevision: number }
  | { kind: "message"; messageId: string; executionRunId: string | null; status: string }
  | { kind: "operation"; operationId: string; executionRunId: string | null; status: string }
  | { kind: "tool_activity"; toolUseId: string; executionRunId: string | null; status: string }
  | { kind: "provider_resume_metadata"; present: boolean }
  | { kind: "live_runtime_handle"; executionRunId: string | null; present: boolean };

export interface SessionRecoveryReport {
  reportId: string;
  sessionId: string;
  recoveryRevision: number;
  trigger: RecoveryTrigger;
  observedLifecycle: string;
  observedExecutionRunId: string | null;
  decision: RecoveryDecision;
  reasonCodes: RecoveryReasonCode[];
  evidenceRefs: RecoveryEvidenceReference[];
  createdAt: string;
}

export interface SessionRecoverySummary {
  session: Session;
  latestReport: SessionRecoveryReport | null;
}

export interface SessionRecoveryAcknowledgement {
  session: Session;
  report: SessionRecoveryReport;
}

export type RecoverySessionStateEvent = {
  kind:
    | "recovery-started"
    | "recovery-completed"
    | "recovery-action-required"
    | "recovery-quarantined"
    | "recovery-acknowledged";
  sessionId: string;
  recoveryRevision: number;
};
