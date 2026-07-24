import type {
  AgentRegistryEntry,
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
  InteractionMode,
  KnownRemoteWorkspace,
  RenameSessionCategoryInput,
  KnownProject,
  LaunchResult,
  ProjectInspection,
  ReadinessStatus,
  CreateScheduledTaskInput,
  SetScheduledTaskEnabledInput,
  Session,
  SessionCategory,
  SessionDetails,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  ScheduledTask,
  WorkflowState,
} from "../types/agent";
import type { ChatConfig, ChatMessage, ChatStreamEvent, SendMessageInput, SessionUsageSummary, UsageStatistics, UsageStatisticsRange } from "../types/chat";
import type { OperationTask } from "../types/operation";
import type {
  CoordinationRun,
  StartCoordinationInput,
  StartCoordinationResult,
} from "../types/coordination";
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
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillPreview,
  SkillScopeInput,
  SkillSyncResult,
  SkillUpdateInput,
} from "../types/skill";
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

export interface AgentService {
  listAgents(capabilityTag?: string): Promise<AgentRegistryEntry[]>;
  listCliTools(): Promise<CliToolStatus[]>;
  refreshCliDetections(agentId?: string): Promise<OperationTask>;
  installCliVersion(input: CliPackageOperationInput): Promise<OperationTask>;
  upgradeAllCliVersions(): Promise<OperationTask>;
  listCliParameterProfiles(): Promise<CliParameterProfile[]>;
  saveCliParameterProfile(input: SaveCliParameterProfileInput): Promise<CliParameterProfile>;
  resetCliParameterProfile(agentId: ManagedCliAgentId): Promise<CliParameterProfile>;
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
  getActiveSession(): Promise<Session | null>;
  listSessionCategories(): Promise<SessionCategory[]>;
  createSessionCategory(input: CreateSessionCategoryInput): Promise<SessionCategory>;
  renameSessionCategory(input: RenameSessionCategoryInput): Promise<SessionCategory>;
  deleteSessionCategory(categoryId: string): Promise<void>;
  assignSessionCategory(input: AssignSessionCategoryInput): Promise<Session>;
  getAutomaticArchivalSettings(): Promise<AutomaticArchivalSettings>;
  saveAutomaticArchivalSettings(input: AutomaticArchivalSettings): Promise<AutomaticArchivalSettings>;
  listScheduledTasks(): Promise<ScheduledTask[]>;
  createScheduledTask(input: CreateScheduledTaskInput): Promise<ScheduledTask>;
  setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput): Promise<ScheduledTask>;
  deleteScheduledTask(taskId: string): Promise<void>;
  startCoordination(input: StartCoordinationInput): Promise<StartCoordinationResult>;
  listCoordinationRuns(): Promise<CoordinationRun[]>;
  getCoordinationRun(runId: string): Promise<CoordinationRun>;
  cancelCoordinationRun(runId: string): Promise<CoordinationRun>;
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
  pinSession(sessionId: string): Promise<Session>;
  unpinSession(sessionId: string): Promise<Session>;
  archiveSession(sessionId: string): Promise<Session>;
  unarchiveSession(sessionId: string): Promise<Session>;
  exportSession(input: ExportSessionInput): Promise<SessionExportResult>;
  sendMessage(input: SendMessageInput): Promise<ChatMessage>;
  listMessages(input: { sessionId: string; limit?: number; beforeId?: string }): Promise<ChatMessage[]>;
  getUsageStatistics(input: { range: UsageStatisticsRange }): Promise<UsageStatistics>;
  getSessionUsageSummary(sessionId: string): Promise<SessionUsageSummary>;
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
  listSkills(input: SkillScopeInput): Promise<SkillListResult>;
  listSkillMountPaths(): Promise<SkillAgentMountPath[]>;
  updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport>;
  createSkill(input: SkillMutationInput): Promise<Skill>;
  updateSkill(skillId: string, input: SkillUpdateInput): Promise<Skill>;
  deleteSkill(skillId: string, input: SkillScopeInput): Promise<void>;
  restoreBuiltinSkill(skillId: string): Promise<Skill>;
  setSkillEnabled(skillId: string, input: SkillScopeInput, enabled: boolean): Promise<Skill>;
  setSkillAgentBindings(skillId: string, input: SkillScopeInput, agentIds: string[]): Promise<Skill>;
  previewSkill(skillId: string, input: SkillScopeInput): Promise<SkillPreview>;
  importSkill(input: SkillImportInput): Promise<Skill>;
  detectSkillDrift(input: SkillScopeInput): Promise<SkillDriftReport>;
  syncSkillDrift(input: SkillScopeInput): Promise<SkillSyncResult>;
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
  | { kind: "configuration-changed"; sessionId: string };
