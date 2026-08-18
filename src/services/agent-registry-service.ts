import type { AgentRegistryEntry, ReadinessStatus } from "../types/agent";

export interface AgentRegistryService {
  openExternalUrl(url: string): Promise<void>;
  listAgents(capabilityTag?: string): Promise<AgentRegistryEntry[]>;
  getAgentById(agentId: string): Promise<AgentRegistryEntry | null>;
  checkBrowserReadiness(agentId: string): Promise<ReadinessStatus>;
}
