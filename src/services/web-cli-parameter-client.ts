import type { CliParameterService } from "./cli-service";
import {
  createCliParameterProfile,
  defaultCliParameterSelections,
  normalizeCliParameterSelections,
} from "./cli-parameter-catalog";
import { readWebMockStorage, writeWebMockStorage } from "./web-mock-storage";
import type { CliParameterSelections, ManagedCliAgentId } from "../types/agent";
import { managedCliAgentIds } from "../types/agent";

const cliParameterStorageKey = "vanehub.cli-parameter-profiles.v1";

// Selections are catalog flag choices — no credential reaches browser storage, so this satisfies
// the `frontend-runtime-architecture` prohibition in "Honest Web/mock behavior".
let memoryCliParameterSelections: Partial<Record<ManagedCliAgentId, CliParameterSelections>> = {};

function readCliParameterSelections(): Partial<Record<ManagedCliAgentId, CliParameterSelections>> {
  return readWebMockStorage(cliParameterStorageKey, memoryCliParameterSelections);
}

function writeCliParameterSelections(value: Partial<Record<ManagedCliAgentId, CliParameterSelections>>) {
  memoryCliParameterSelections = value;
  writeWebMockStorage(cliParameterStorageKey, value);
}

export const webCliParameterClient: CliParameterService = {
  async listCliParameterProfiles() {
    const stored = readCliParameterSelections();
    return managedCliAgentIds.map((agentId) => createCliParameterProfile(agentId, stored[agentId]));
  },

  async saveCliParameterProfile(input) {
    const selections = normalizeCliParameterSelections(input.agentId, input.selections);
    writeCliParameterSelections({ ...readCliParameterSelections(), [input.agentId]: selections });
    return createCliParameterProfile(input.agentId, selections);
  },

  async resetCliParameterProfile(agentId) {
    const stored = { ...readCliParameterSelections() };
    delete stored[agentId];
    writeCliParameterSelections(stored);
    return createCliParameterProfile(agentId, defaultCliParameterSelections(agentId));
  },
};
