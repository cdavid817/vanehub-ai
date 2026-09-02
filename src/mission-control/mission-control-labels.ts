import type { AgentRegistryEntry } from "../types/agent";

/**
 * 16.7's own "retain stable ids for queries" -- the returned label is for display only; every
 * caller keeps filtering/mutating against the raw id, never this resolved string.
 *
 * `agentId` is not reliably a real `AgentRegistryEntry.id` even when present: both backends set
 * `MissionControlRunSummary.agentId` from the run's own `ownerId` whenever `ownerType` merely
 * *contains* "agent" or "generation" (see `project()` in mission_control.rs), which can be a
 * generation/session id with no corresponding registry entry. Falling back to the raw id rather
 * than hiding it is deliberate -- a silent blank would look like data loss, not a known mismatch.
 */
export function resolveAgentDisplayName(agents: readonly AgentRegistryEntry[], agentId: string | null): string | null {
  if (!agentId) return null;
  return agents.find((agent) => agent.id === agentId)?.displayName ?? agentId;
}
