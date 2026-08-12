import type { AgentRegistryEntry } from "../types/agent";
import { createSessionCliPriority, orderByAgentPriority } from "../lib/agent-display-order";

/**
 * Which Agents the create-session dialog offers, and in what order.
 *
 * Split out of `create-session-dialog-utils` when that file outgrew the 300-line limit; these four
 * answer one question — who can hold a seat — and `session-seat-assignment` needs the same answer.
 */
export const previousSessionAgentStorageKey = "vanehub.create-session.agent.v1";

export function isSessionAgentSelectable(agent: AgentRegistryEntry): boolean {
  if (agent.availabilityState === "available" || agent.availabilityState === "unknown") {
    return true;
  }
  const sdkId = agent.managedSdkDependencyId;
  return agent.availabilityState === "unavailable"
    && agent.supportedInteractionModes.includes("cli")
    && Boolean(sdkId)
    && agent.unavailableReason === `Managed SDK dependency '${sdkId}' is not installed.`;
}

export function selectSessionAgents(agents: AgentRegistryEntry[]): AgentRegistryEntry[] {
  return orderByAgentPriority(
    agents.filter((agent) =>
      agent.supportedInteractionModes.some((mode) => mode === "cli" || mode === "api"),
    ),
    (agent) => agent.id,
    [...createSessionCliPriority, "onepiece"],
  );
}

export function defaultSessionAgent(
  agents: AgentRegistryEntry[],
  previousAgentId: string | null,
): AgentRegistryEntry | null {
  if (previousAgentId === "onepiece") {
    const onepiece = agents.find((agent) =>
      agent.id === "onepiece"
      && isSessionAgentSelectable(agent),
    );
    if (onepiece) return onepiece;
  }
  return agents.find((agent) =>
    createSessionCliPriority.includes(agent.id as (typeof createSessionCliPriority)[number])
      && isSessionAgentSelectable(agent),
  ) ?? agents.find(isSessionAgentSelectable) ?? agents[0] ?? null;
}

export function groupSessionAgents(agents: AgentRegistryEntry[]) {
  return [
    {
      id: "builtin-cli" as const,
      labelKey: "onepiece.group.builtinCli",
      agents: agents.filter((agent) =>
        agent.id !== "onepiece"
        && agent.agentOrigin === "builtin"
        && agent.supportedInteractionModes.includes("cli"),
      ),
    },
    {
      id: "native" as const,
      labelKey: "onepiece.group.native",
      agents: agents.filter((agent) => agent.id === "onepiece"),
    },
    {
      id: "custom-api" as const,
      labelKey: "onepiece.group.customApi",
      agents: agents.filter((agent) =>
        agent.agentOrigin === "user" && agent.supportedInteractionModes.includes("api"),
      ),
    },
  ].filter((group) => group.agents.length > 0);
}
