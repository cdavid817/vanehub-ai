import type { AgentService } from "./agent-service";
import { mockAgents } from "./mock-agent-data";
import type { ImSessionConnector, Session } from "../types/agent";
import { webEvaluationClient } from "./web-evaluation-client";
import { webPromptHookClient } from "./web-prompt-hook-client";
import { webApiAgentClient } from "./web-api-agent-client";
import { webOnePieceProviderClient } from "./web-onepiece-provider-client";
import { webOnePieceProfileClient } from "./web-onepiece-profile-client";
import { webHybridRoutingClient } from "./web-hybrid-routing-client";
import { deleteWebApiAgentProviderConfig } from "./web-api-provider-state";
import { webCodeIndexClient } from "./web-code-index-client";
import { webCliEnvironmentClient } from "./web-cli-environment-client";
import { webCliParameterClient } from "./web-cli-parameter-client";
import { webCliConfigClient } from "./web-cli-config-client";
import { webScheduledTaskClient } from "./web-scheduled-task-client";
import { webContextQualityClient } from "./web-context-quality-client";
import { webSkillGovernanceClient } from "./web-skill-governance-client";
import { webSkillEvidenceClient } from "./web-skill-evidence-client";
import { webSkillAssessmentClient } from "./web-skill-assessment-client";
import { webSkillGenerationClient } from "./web-skill-generation-client";
import { webSkillEvolutionOrchestrationClient } from "./web-skill-evolution-orchestration-client";
import { webSystemActivityClient } from "./web-system-activity-client";
import { webSkillCuratorClient } from "../adapters/web-skill-curator-client";
import { webAgentRegistryClient } from "./web-agent-registry-client";

export { resetWebEvidenceForTest } from "./web-skill-evidence-client";
export { resetWebSkillCuratorForTest } from "../adapters/web-skill-curator-client";
import {
  findWebCliConfigProfile,
  requireCliConfigAgentId,
  setWebCliConfigStatus,
  webCliConfigStatus,
} from "./web-cli-config-state";

// Re-exported so the existing Web/mock test seams keep importing from one place while the
// implementation lives in the extracted module.
export { resetWebRetrievalForTest, searchWebCodeIndex } from "./web-code-index-state";
import { nowIso } from "./web-mock-clock";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";
import { webSessionWorkspaceEvidenceService } from "./web-session-workspace-evidence-client";
import { webLspClient } from "./web-lsp-client";
import { webBuiltinToolClient } from "./web-builtin-tool-client";
import { createWebCodeReviewClient } from "./web-code-review-client";
import { webDesktopUpdateClient } from "./web-desktop-update";
import { webSkillCatalogClient } from "./web-skill-catalog-client";
import { webSkillBindingClient } from "./web-skill-binding-client";
import { webSkillOverlayClient } from "./web-skill-overlay-client";
import { webSessionCategoryClient } from "./web-session-category-client";
import { webExpertRoleClient } from "./web-expert-role-client";
import { webAgentTerminalClient } from "./web-agent-terminal-client";
import { webUsageStatisticsClient } from "./web-usage-statistics-client";
import { webMissionControlClient } from "./web-mission-control-client";
import { webLoopClient } from "./web-loop-client";
import { listWebLoopDefinitions } from "./web-loop-state";

export {
  resetWebLoopsForTest,
  setWebLoopPhaseDelayForTest,
  simulateWebLoopRestartForTest,
} from "./web-loop-state";
import { webPersonalizationClient } from "./web-personalization-client";

export { resetWebAgentMemoriesForTest } from "./web-agent-memory-state";
import { listWebAgentMemories } from "./web-agent-memory-state";
import { webChatClient } from "./web-chat-client";

export {
  resolveWebMockToolApproval,
  WEB_MOCK_PLAN_EXIT_TRIGGER,
  WEB_MOCK_QUESTION_TRIGGER,
} from "./web-chat-client";
import { webSessionChatConfigClient } from "./web-session-chat-config-client";
import { webSessionRecoveryClient } from "./web-session-recovery-client";

export {
  resetWebRecoverySessionsForTest,
  seedWebRecoverySessionForTest,
} from "./web-session-recovery-state";

export {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
} from "./web-agent-run-state";
import { webKnownWorkspaceClient } from "./web-known-workspace-client";
import {
  getWebActiveSessionId,
  getWebWorkflowState,
  listWebSessions,
  nextWebSessionSequence,
  prependWebSession,
  setWebActiveSessionId,
  subscribeWebSessionStateEvents,
} from "./web-session-state";
import { webSessionQueryClient } from "./web-session-query-client";
import { webSessionLifecycleClient } from "./web-session-lifecycle-client";
import { webSessionSeatClient } from "./web-session-seat-client";
import {
  listWebSkillApiAgentBindings,
  listWebSkillMountPaths,
  listWebSkills,
  replaceWebSkillApiAgentBindings,
  replaceWebSkillMountPaths,
  replaceWebSkills,
} from "./web-skill-state";

export function seedWebImSessionForTest(connector: ImSessionConnector): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-im-session-${nextWebSessionSequence()}`,
    title: `IM ${connector}`,
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\example\\im-project",
    projectPath: "D:\\example\\im-project",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    source: { kind: "im", connector },
    pinned: false,
    archived: false,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  prependWebSession(session);
  setWebActiveSessionId(session.id);
  return session;
}


const webCodeReviewClient = createWebCodeReviewClient(webSessionWorkspaceClient);

export const webAgentClient: AgentService = {
  ...webSkillCuratorClient,
  ...webSkillEvolutionOrchestrationClient,
  ...webSystemActivityClient,
  ...webEvaluationClient,
  ...webPromptHookClient,
  ...webApiAgentClient,
  ...webOnePieceProviderClient,
  ...webOnePieceProfileClient,
  ...webHybridRoutingClient,
  ...webCodeIndexClient,
  ...webCliEnvironmentClient,
  ...webCliParameterClient,
  ...webCliConfigClient,
  ...webScheduledTaskClient,
  ...webContextQualityClient,
  ...webSkillGovernanceClient,
  ...webSkillEvidenceClient,
  ...webSkillAssessmentClient,
  ...webSkillGenerationClient,
  ...webAgentRegistryClient,
  getDesktopUpdateSnapshot: webDesktopUpdateClient.getSnapshot,
  getDesktopUpdatePreferences: webDesktopUpdateClient.getPreferences,
  saveDesktopUpdatePreferences: webDesktopUpdateClient.savePreferences,
  checkForDesktopUpdate: webDesktopUpdateClient.check,
  downloadAndInstallDesktopUpdate: webDesktopUpdateClient.install,
  restartAfterDesktopUpdate: webDesktopUpdateClient.restart,
  ...webBuiltinToolClient,
  ...webSessionWorkspaceClient,
  ...webSessionWorkspaceEvidenceService,
  ...webCodeReviewClient,
  ...webLspClient,
  ...webMissionControlClient,

  async deleteApiAgent(agentId: string) {
    if (mockAgents.find((agent) => agent.id === agentId)?.agentOrigin === "builtin") {
      throw new Error("Built-in agents cannot be deleted; reset their provider configuration instead.");
    }
    const blocking: string[] = [];
    const sessionCount = listWebSessions().filter((session) => session.agentId === agentId).length;
    if (sessionCount > 0) blocking.push(`${sessionCount} sessions`);
    const memoryCount = listWebAgentMemories().filter((memory) => memory.agentId === agentId).length;
    if (memoryCount > 0) blocking.push(`${memoryCount} memories`);
    const workerCount = listWebLoopDefinitions().filter((definition) => definition.workerAgentId === agentId).length;
    if (workerCount > 0) blocking.push(`${workerCount} Loop definitions as worker`);
    const verifierCount = listWebLoopDefinitions().filter((definition) => definition.verifierAgentId === agentId).length;
    if (verifierCount > 0) blocking.push(`${verifierCount} Loop definitions as verifier`);
    if (blocking.length > 0) {
      throw new Error(`Cannot delete this agent: it is still referenced by ${blocking.join(", ")}.`);
    }
    const index = mockAgents.findIndex((agent) => agent.id === agentId);
    if (index !== -1) mockAgents.splice(index, 1);
    deleteWebApiAgentProviderConfig(agentId);
    replaceWebSkillApiAgentBindings(listWebSkillApiAgentBindings().filter((binding) => binding.agentId !== agentId));
    replaceWebSkills(listWebSkills().map((skill) => ({
      ...skill,
      boundAgentIds: skill.boundAgentIds.filter((boundAgentId) => boundAgentId !== agentId),
    })));
    replaceWebSkillMountPaths(listWebSkillMountPaths().filter((path) => path.agentId !== agentId));
  },

  ...webPersonalizationClient,

  async applyCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const profile = findWebCliConfigProfile(supportedAgentId, input.profileId);
    if (!profile) throw new Error("Profile not found.");
    if (profile.validationState === "needs-credential") throw new Error("Credential repair is required.");
    const beforeWorkflow = JSON.stringify(getWebWorkflowState());
    const beforeActiveSession = getWebActiveSessionId();
    const status = webCliConfigStatus(supportedAgentId);
    const backfilledProfileId = supportedAgentId !== "opencode"
      && status.appliedProfileId !== null
      && status.appliedProfileId !== profile.id
      ? status.appliedProfileId
      : null;
    const timestamp = nowIso();
    setWebCliConfigStatus(supportedAgentId, {
      agentId: supportedAgentId,
      appliedProfileId: profile.id,
      driftState: "applied",
      resolvedPaths: [],
      lastAppliedAt: timestamp,
      simulated: true,
      startupSync: status.startupSync,
    });
    if (JSON.stringify(getWebWorkflowState()) !== beforeWorkflow || getWebActiveSessionId() !== beforeActiveSession) {
      throw new Error("Global configuration simulation changed runtime workflow state.");
    }
    return {
      operationId: `web-cli-config-${supportedAgentId}-${Date.now()}`,
      status: "succeeded",
      agentId: supportedAgentId,
      profileId: profile.id,
      affectedPaths: [],
      driftResolution: backfilledProfileId ? "import-current" : input.driftResolution ?? null,
      backfilledProfileId,
      warnings: ["Web mode simulated the switch; no local CLI file was changed."],
      restartRequired: true,
      simulated: true,
      restored: true,
      error: null,
    };
  },
  ...webExpertRoleClient,

  ...webSessionQueryClient,

  ...webSessionRecoveryClient,

  ...webSessionCategoryClient,
  ...webLoopClient,

  ...webSessionChatConfigClient,
  ...webKnownWorkspaceClient,

  ...webSessionLifecycleClient,
  ...webSessionSeatClient,

  ...webUsageStatisticsClient,
  ...webChatClient,
  ...webAgentTerminalClient,

  async subscribeSessionEvents(handler) {
    return subscribeWebSessionStateEvents(handler);
  },
  ...webSkillCatalogClient,
  ...webSkillBindingClient,
  ...webSkillOverlayClient,
};
