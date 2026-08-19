import type {
  ExportSessionInput,
  InteractionMode,
  LaunchResult,
  Session,
  SessionDetails,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  WorkflowState,
} from "../types/agent";
import type { AgentRunnerDescriptor } from "../types/agent-runner";

export interface SessionQueryService {
  getWorkflowState(): Promise<WorkflowState>;
  selectAgent(agentId: string, interactionMode: InteractionMode): Promise<WorkflowState>;
  launchActiveWorkflow(): Promise<LaunchResult>;
  getSessionDetails(): Promise<SessionDetails>;
  listSessions(): Promise<Session[]>;
  listArchivedSessions(): Promise<Session[]>;
  searchSessions(input: SessionSearchInput): Promise<SessionSearchResult[]>;
  getSession(sessionId: string): Promise<Session>;
  getActiveSession(): Promise<Session | null>;
  exportSession(input: ExportSessionInput): Promise<SessionExportResult>;
  listAgentRunners(sessionId: string, agentId: string): Promise<AgentRunnerDescriptor[]>;
}
