import type { AgentRegistryEntry } from "../types/agent";
import { normalizeModelFamily, type ModelFamily } from "./model-family";

/**
 * Seat assignment needs a family per Agent to recommend a cross-family reviewer. This annotates
 * rather than mutating the registry entry, so the registry contract stays as the native layer
 * defines it and the derived value is visibly derived.
 */
export interface AgentWithModelFamily extends AgentRegistryEntry {
  modelFamily: ModelFamily;
}

export function withModelFamily(agents: AgentRegistryEntry[]): AgentWithModelFamily[] {
  return agents.map((agent) => ({
    ...agent,
    modelFamily: normalizeModelFamily({ id: agent.id, provider: agent.provider }),
  }));
}
