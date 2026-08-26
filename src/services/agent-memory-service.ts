import type { AgentMemory } from "../types/agent";

/** Superseded by `PersonalizationService`. Task 9.6 removes these once groups 10-11 rebuild the
 * Settings memory section, their only caller; do not add another. Memories here are one host-level
 * pool shared by every agent (`add-cli-memory-support`): no `agentId` scoping on read or bulk
 * reset, and `AgentMemory.agentId` stays as provenance only. */
export interface AgentMemoryService {
  /** @deprecated Use `queryPersonalizationMemories`: paged, and carries no bodies. */
  listAllMemories(): Promise<AgentMemory[]>;
  /** @deprecated Use `deletePersonalizationMemory`, which takes the caller's expected revision. */
  deleteAgentMemory(memoryId: string): Promise<void>;
  /** @deprecated Use `previewPersonalizationReset` then `executePersonalizationReset`. */
  resetAllMemories(): Promise<void>;
}
