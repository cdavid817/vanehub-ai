import type {
  AgentRegistryEntry,
  CliPackageOperationInput,
  CliToolStatus,
  InteractionMode,
  LaunchResult,
  ReadinessStatus,
  Session,
  SessionDetails,
  WorkflowState,
} from "../types/agent";
import type { ChatMessage, ChatStreamEvent, SendMessageInput } from "../types/chat";
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
  getAgentById(agentId: string): Promise<AgentRegistryEntry | null>;
  getWorkflowState(): Promise<WorkflowState>;
  selectAgent(agentId: string, interactionMode: InteractionMode): Promise<WorkflowState>;
  checkBrowserReadiness(agentId: string): Promise<ReadinessStatus>;
  launchActiveWorkflow(): Promise<LaunchResult>;
  getSessionDetails(): Promise<SessionDetails>;
  listSessions(): Promise<Session[]>;
  listArchivedSessions(): Promise<Session[]>;
  getActiveSession(): Promise<Session | null>;
  createSession(input: {
    agentId: string;
    interactionMode: InteractionMode;
    title?: string;
    folder?: string | null;
  }): Promise<Session>;
  deleteSession(sessionId: string): Promise<void>;
  switchSession(sessionId: string): Promise<Session>;
  renameSession(sessionId: string, title: string): Promise<Session>;
  pinSession(sessionId: string): Promise<Session>;
  unpinSession(sessionId: string): Promise<Session>;
  archiveSession(sessionId: string): Promise<Session>;
  unarchiveSession(sessionId: string): Promise<Session>;
  sendMessage(input: SendMessageInput): Promise<ChatMessage>;
  listMessages(input: { sessionId: string; limit?: number; beforeId?: string }): Promise<ChatMessage[]>;
  stopGeneration(sessionId: string): Promise<void>;
  subscribeMessageEvents(
    sessionId: string,
    handler: (event: ChatStreamEvent) => void,
  ): Promise<() => void>;
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
