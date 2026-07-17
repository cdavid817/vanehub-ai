import type {
  AgentRegistryEntry,
  CliParameterProfile,
  ManagedCliAgentId,
  SaveCliParameterProfileInput,
  CliPackageOperationInput,
  CliToolStatus,
  CreateSessionInput,
  InteractionMode,
  KnownProject,
  LaunchResult,
  ProjectInspection,
  ReadinessStatus,
  Session,
  SessionDetails,
  WorkflowState,
} from "../types/agent";
import type { ChatConfig, ChatMessage, ChatStreamEvent, SendMessageInput, UsageStatistics, UsageStatisticsRange } from "../types/chat";
import type { OperationTask } from "../types/operation";
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

export interface AgentService {
  listAgents(capabilityTag?: string): Promise<AgentRegistryEntry[]>;
  listCliTools(): Promise<CliToolStatus[]>;
  refreshCliDetections(): Promise<OperationTask>;
  installCliVersion(input: CliPackageOperationInput): Promise<OperationTask>;
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
  getActiveSession(): Promise<Session | null>;
  getSessionChatConfig(sessionId: string): Promise<ChatConfig>;
  saveSessionChatConfig(sessionId: string, config: ChatConfig): Promise<ChatConfig>;
  listKnownProjects(): Promise<KnownProject[]>;
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
  sendMessage(input: SendMessageInput): Promise<ChatMessage>;
  listMessages(input: { sessionId: string; limit?: number; beforeId?: string }): Promise<ChatMessage[]>;
  getUsageStatistics(input: { range: UsageStatisticsRange }): Promise<UsageStatistics>;
  stopGeneration(sessionId: string): Promise<void>;
  subscribeMessageEvents(
    sessionId: string,
    handler: (event: ChatStreamEvent) => void,
  ): Promise<() => void>;
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
  selectWorkspaceDirectory(): Promise<string | null>;
}

export type SessionStateEvent =
  | { kind: "active-session-changed"; sessionId: string | null }
  | { kind: "configuration-changed"; sessionId: string };
