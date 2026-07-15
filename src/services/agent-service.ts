import type {
  AgentRegistryEntry,
  InteractionMode,
  LaunchResult,
  ReadinessStatus,
  Session,
  SessionDetails,
  WorkflowState,
} from "../types/agent";

export interface AgentService {
  listAgents(capabilityTag?: string): Promise<AgentRegistryEntry[]>;
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
}
