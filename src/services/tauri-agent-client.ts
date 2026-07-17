import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import type { AgentService, SessionStateEvent } from "./agent-service";
import type {
  AgentRegistryEntry,
  CliParameterProfile,
  CliPackageOperationInput,
  CliToolStatus,
  InteractionMode,
  KnownProject,
  LaunchResult,
  ProjectInspection,
  ReadinessStatus,
  Session,
  SessionDetails,
  SaveCliParameterProfileInput,
  ManagedCliAgentId,
  WorkflowState,
} from "../types/agent";
import type { ChatConfig, ChatMessage, ChatStreamEvent } from "../types/chat";
import type { OperationTask } from "../types/operation";
import type {
  Skill,
  SkillAgentMountPath,
  SkillDriftReport,
  SkillImportInput,
  SkillListResult,
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillPreview,
  SkillScopeInput,
  SkillSyncResult,
  SkillUpdateInput,
} from "../types/skill";
import { tauriSessionWorkspaceClient } from "./tauri-session-workspace-client";
import { normalizeTauriUsageStatistics } from "./tauri-usage-statistics";

export const tauriAgentClient: AgentService = {
  listAgents(capabilityTag) {
    return invoke<AgentRegistryEntry[]>("list_agents", { capabilityTag: capabilityTag ?? null });
  },

  listCliTools() {
    return invoke<CliToolStatus[]>("list_cli_tools");
  },

  refreshCliDetections(agentId) {
    return invoke<OperationTask>("refresh_cli_detections", { agentId: agentId ?? null });
  },

  installCliVersion(input: CliPackageOperationInput) {
    return invoke<OperationTask>("install_cli_version", {
      agentId: input.agentId,
      targetVersion: input.targetVersion,
      confirmedActivePath: input.confirmedActivePath ?? null,
    });
  },

  listCliParameterProfiles() {
    return invoke<CliParameterProfile[]>("list_cli_parameter_profiles");
  },

  saveCliParameterProfile(input: SaveCliParameterProfileInput) {
    return invoke<CliParameterProfile>("save_cli_parameter_profile", { input });
  },

  resetCliParameterProfile(agentId: ManagedCliAgentId) {
    return invoke<CliParameterProfile>("reset_cli_parameter_profile", { agentId });
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

  getSessionChatConfig(sessionId) {
    return invoke<ChatConfig>("get_session_chat_config", { sessionId });
  },

  saveSessionChatConfig(sessionId, config) {
    return invoke<ChatConfig>("save_session_chat_config", { sessionId, config });
  },

  listKnownProjects() {
    return invoke<KnownProject[]>("list_known_projects");
  },

  inspectProject(path: string) {
    return invoke<ProjectInspection>("inspect_project", { path });
  },

  async selectProjectDirectory() {
    const selected = await open({ directory: true, multiple: false });
    return typeof selected === "string" ? selected : null;
  },

  createSession(input) {
    return invoke<OperationTask>("create_session", {
      input,
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

  sendMessage(input) {
    return invoke<ChatMessage>("send_message", {
      sessionId: input.sessionId,
      content: input.content,
      config: input.config,
    });
  },

  listMessages(input) {
    return invoke<ChatMessage[]>("list_messages", {
      sessionId: input.sessionId,
      limit: input.limit ?? null,
      beforeId: input.beforeId ?? null,
    });
  },

  async getUsageStatistics(input) {
    const statistics = await invoke<unknown>("get_usage_statistics", {
      range: input.range,
    });
    return normalizeTauriUsageStatistics(statistics, input.range);
  },

  async stopGeneration(sessionId: string) {
    await invoke<void>("stop_generation", { sessionId });
  },

  async subscribeMessageEvents(sessionId, handler) {
    const unlisten = await listen<ChatStreamEvent>("chat:event", (event) => {
      if (event.payload.sessionId === sessionId) {
        handler(event.payload);
      }
    });
    return unlisten;
  },

  ...tauriSessionWorkspaceClient,
  async subscribeSessionEvents(handler) {
    return listen<SessionStateEvent>("session:event", (event) => handler(event.payload));
  },

  listSkills(input: SkillScopeInput) {
    return invoke<SkillListResult>("list_skills", { input });
  },

  listSkillMountPaths() {
    return invoke<SkillAgentMountPath[]>("list_skill_mount_paths");
  },

  updateSkillMountPath(agentId: string, mountPath: string) {
    return invoke<SkillMountMigrationReport>("update_skill_mount_path", { agentId, mountPath });
  },

  createSkill(input: SkillMutationInput) {
    return invoke<Skill>("create_skill", { input });
  },

  updateSkill(skillId: string, input: SkillUpdateInput) {
    return invoke<Skill>("update_skill", { skillId, input });
  },

  async deleteSkill(skillId: string, input: SkillScopeInput) {
    await invoke<void>("delete_skill", { skillId, input });
  },

  restoreBuiltinSkill(skillId: string) {
    return invoke<Skill>("restore_builtin_skill", { skillId });
  },

  setSkillEnabled(skillId: string, input: SkillScopeInput, enabled: boolean) {
    return invoke<Skill>("set_skill_enabled", { skillId, input, enabled });
  },

  setSkillAgentBindings(skillId: string, input: SkillScopeInput, agentIds: string[]) {
    return invoke<Skill>("set_skill_agent_bindings", { skillId, input, agentIds });
  },

  previewSkill(skillId: string, input: SkillScopeInput) {
    return invoke<SkillPreview>("preview_skill", { skillId, input });
  },

  importSkill(input: SkillImportInput) {
    return invoke<Skill>("import_skill", { input });
  },

  detectSkillDrift(input: SkillScopeInput) {
    return invoke<SkillDriftReport>("detect_skill_drift", { input });
  },

  syncSkillDrift(input: SkillScopeInput) {
    return invoke<SkillSyncResult>("sync_skill_drift", { input });
  },

  selectWorkspaceDirectory() {
    return invoke<string | null>("select_workspace_directory");
  },
};
