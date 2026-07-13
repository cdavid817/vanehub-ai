import type {
  AgentRegistryEntry,
  InteractionMode,
  LaunchResult,
  ReadinessStatus,
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
}
