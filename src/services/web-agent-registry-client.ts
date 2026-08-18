import type { AgentRegistryService } from "./agent-registry-service";
import { requireHttpsExternalUrl } from "./external-url";
import { mockAgents } from "./mock-agent-data";

export const webAgentRegistryClient: AgentRegistryService = {
  async openExternalUrl(url) {
    const target = requireHttpsExternalUrl(url);
    const opened = window.open(target, "_blank", "noopener,noreferrer");
    if (!opened) throw new Error("The browser blocked the external link.");
  },
  async listAgents(capabilityTag) {
    return capabilityTag
      ? mockAgents.filter((agent) => agent.capabilityTags.includes(capabilityTag))
      : mockAgents;
  },

  async getAgentById(agentId) {
    return mockAgents.find((agent) => agent.id === agentId) ?? null;
  },

  async checkBrowserReadiness(agentId) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    const supportsBrowser = agent?.supportedInteractionModes.includes("browser") ?? false;
    return {
      ready: supportsBrowser,
      reason: supportsBrowser ? undefined : "This agent does not support browser interaction mode.",
      requiresAuthentication: supportsBrowser,
    };
  },
};
