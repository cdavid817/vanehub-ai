import { invoke } from "@tauri-apps/api/core";
import type { AgentService } from "./agent-service";
import type {
  AgentRegistryEntry,
  InteractionMode,
  LaunchResult,
  ReadinessStatus,
  Session,
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

  listSessions() {
    return invoke<Session[]>("list_sessions");
  },

  listArchivedSessions() {
    return invoke<Session[]>("list_archived_sessions");
  },

  getActiveSession() {
    return invoke<Session | null>("get_active_session");
  },

  createSession(input) {
    return invoke<Session>("create_session", {
      agentId: input.agentId,
      interactionMode: input.interactionMode,
      title: input.title ?? null,
      folder: input.folder ?? null,
    });
  },

  async deleteSession(sessionId: string) {
    await invoke<void>("delete_session", { sessionId });
  },

  switchSession(sessionId: string) {
    return invoke<Session>("switch_session", { sessionId });
  },

  renameSession(sessionId: string, title: string) {
    return invoke<Session>("rename_session", { sessionId, title });
  },

  pinSession(sessionId: string) {
    return invoke<Session>("pin_session", { sessionId });
  },

  unpinSession(sessionId: string) {
    return invoke<Session>("unpin_session", { sessionId });
  },

  archiveSession(sessionId: string) {
    return invoke<Session>("archive_session", { sessionId });
  },

  unarchiveSession(sessionId: string) {
    return invoke<Session>("unarchive_session", { sessionId });
  },
};
