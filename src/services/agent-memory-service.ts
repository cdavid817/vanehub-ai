import type { AgentMemory } from "../types/agent";

export interface AgentMemoryService {
  /** `add-cli-memory-support`: memories are a single host-level pool shared by every agent — no
   * `agentId` scoping on read or bulk-reset, `AgentMemory.agentId` remains as provenance only. */
  listAllMemories(): Promise<AgentMemory[]>;
  deleteAgentMemory(memoryId: string): Promise<void>;
  resetAllMemories(): Promise<void>;
}
