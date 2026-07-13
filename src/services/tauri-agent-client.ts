import { invoke } from "@tauri-apps/api/core";
import type { AgentService } from "./agent-service";
import type {
  AgentRegistryEntry,
  InteractionMode,
  LaunchResult,
  ReadinessStatus,
  SessionDetails,
  WorkflowState,
} from "../types/agent";

export const tauriAgentClient: AgentService = {
  listAgents(capabilityTag) {
    return invoke<AgentRegistryEntry[]>("list_agents", { capabilityTag: capabilityTag ?? null });
  },

  getAgentById(agentId) {
    return invoke<AgentRegistryEntry>("get_agent_by_id", { agentId });
  },

  getWorkflowState() {
    return invoke<WorkflowState>("get_workflow_state");
  },

  selectAgent(agentId: string, interactionMode: InteractionMode) {
    return invoke<WorkflowState>("select_agent", { agentId, interactionMode });
  },

  checkBrowserReadiness(agentId: string) {
    return invoke<ReadinessStatus>("check_browser_readiness", { agentId });
  },

  launchActiveWorkflow() {
    return invoke<LaunchResult>("launch_active_workflow");
  },

  getSessionDetails() {
    return invoke<SessionDetails>("get_session_details");
  },
};
