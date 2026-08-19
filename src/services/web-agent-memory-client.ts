import type { AgentMemoryService } from "./agent-memory-service";
import {
  clearWebAgentMemories,
  deleteWebAgentMemory,
  listWebAgentMemories,
} from "./web-agent-memory-state";

export const webAgentMemoryClient: AgentMemoryService = {
  async listAllMemories() {
    return listWebAgentMemories();
  },

  async deleteAgentMemory(memoryId: string) {
    deleteWebAgentMemory(memoryId);
  },

  async resetAllMemories() {
    clearWebAgentMemories();
  },
};
