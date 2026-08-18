import type {
  AgentService,
  RecoveryDecision,
  SessionRecoveryAcknowledgement,
  SessionRecoveryReport,
  SessionRecoverySummary,
} from "./agent-service";
import { mockAgents } from "./mock-agent-data";
import { i18n } from "../i18n";
import {
  createWebPendingApproval,
  getWebDefaultPolicyTemplate,
  isAgentAutoApproved,
  webPrincipalTemplates,
} from "./web-permissions-mock-state";
import type {
  AgentMemory,
  AgentMemoryType,
  UpdateSessionSeatsInput,
  ExportSessionInput,
  InteractionMode,
  Session,
  SessionSeat,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  SessionDetails,
  ImSessionConnector,
} from "../types/agent";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { readWebAppSettings } from "./web-settings-client";
import { defaultSessionTitleFromPath } from "../lib/session-path";
import { snapshotSeat } from "./seat-presentation";
import type { ChatConfig, ChatMessage } from "../types/chat";
import type { AgentRun } from "../types/agent-run";
import type { AgentRunnerDescriptor, AgentRunnerSelection } from "../types/agent-runner";
import { webEvaluationClient } from "./web-evaluation-client";
import { webPromptHookClient } from "./web-prompt-hook-client";
import { webApiAgentClient } from "./web-api-agent-client";
import { webOnePieceProviderClient } from "./web-onepiece-provider-client";
import { webOnePieceProfileClient } from "./web-onepiece-profile-client";
import { webHybridRoutingClient } from "./web-hybrid-routing-client";
import { deleteWebApiAgentProviderConfig } from "./web-api-provider-state";
import { webCodeIndexClient } from "./web-code-index-client";
import { discoverWebSessionCodeIndex } from "./web-code-index-state";
import { webCliToolClient } from "./web-cli-tool-client";
import { webCliParameterClient } from "./web-cli-parameter-client";
import { webCliConfigClient } from "./web-cli-config-client";
import { webScheduledTaskClient } from "./web-scheduled-task-client";
import { webContextQualityClient } from "./web-context-quality-client";
import { webSkillGovernanceClient } from "./web-skill-governance-client";
import { webSkillEvidenceClient } from "./web-skill-evidence-client";
import { webAgentRegistryClient } from "./web-agent-registry-client";
import { normalizeWebPath } from "./web-skill-location";
import { readWebMockStorage, writeWebMockStorage } from "./web-mock-storage";

export { resetWebEvidenceForTest } from "./web-skill-evidence-client";
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
import { createWebMockOperation } from "./web-operation-client";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";
import { webLspClient } from "./web-lsp-client";
import {
  defaultChatConfigForSession,
  normalizeChatConfigForSession,
  withEffectiveExecutionPolicy,
} from "./chat-configuration";
import { createWebMcpToolSimulationPlan } from "./web-mcp-tool-simulation";
import { webBuiltinToolClient } from "./web-builtin-tool-client";
import { createWebCodeReviewClient } from "./web-code-review-client";
import { webDesktopUpdateClient } from "./web-desktop-update";
import { webSkillCatalogClient } from "./web-skill-catalog-client";
import { webSkillBindingClient } from "./web-skill-binding-client";
import { webSkillOverlayClient } from "./web-skill-overlay-client";
import { webSessionCategoryClient } from "./web-session-category-client";
import { webExpertRoleClient, listWebExpertRoles } from "./web-expert-role-client";
import { webAgentTerminalClient } from "./web-agent-terminal-client";
import { webUsageStatisticsClient } from "./web-usage-statistics-client";
import { webMissionControlClient } from "./web-mission-control-client";
import { webLoopClient } from "./web-loop-client";
import { isWebLoopRoleSession, listWebLoopDefinitions } from "./web-loop-state";

export {
  resetWebLoopsForTest,
  setWebLoopPhaseDelayForTest,
  simulateWebLoopRestartForTest,
} from "./web-loop-state";
import { webChatClient } from "./web-chat-client";

export {
  resolveWebMockToolApproval,
  WEB_MOCK_PLAN_EXIT_TRIGGER,
  WEB_MOCK_QUESTION_TRIGGER,
} from "./web-chat-client";
import { WEB_MOCK_PLAN_EXIT_TRIGGER, WEB_MOCK_QUESTION_TRIGGER } from "./web-chat-client";
import {
  createWebMessageId,
  cancelWebActiveStream,
  deleteWebActiveStream,
  deleteWebChatSubscribers,
  deleteWebSessionMessages,
  emitWebChatEvent,
  getWebSessionMessages,
  hasWebActiveStream,
  publishWebChatEvent,
  setWebActiveStream,
  setWebSessionMessages,
} from "./web-chat-state";
import { prependWebAgentRun, setWebAgentRunEvents } from "./web-agent-run-state";

export {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
} from "./web-agent-run-state";
import {
  inspectMockProject,
  joinSiblingPath,
  normalizeRemoteWorkspace,
  resolveProjectPath,
  upsertKnownProject,
  upsertKnownRemoteWorkspace,
  validateWorktreeName,
  webKnownWorkspaceClient,
} from "./web-known-workspace-client";
import {
  createWebSeatId,
  emitWebSessionEvent,
  findWebSession,
  getWebActiveSessionId,
  getWebWorkflowState,
  listWebSessions,
  nextWebSessionSequence,
  prependWebSession,
  replaceWebSessions,
  setWebActiveSessionId,
  setWebWorkflowState,
  sortWebSessions,
  subscribeWebSessionStateEvents,
  updateWebSession,
} from "./web-session-state";
import type { Skill } from "../types/skill";
import {
  listWebSkillApiAgentBindings,
  listWebSkillMountPaths,
  listWebSkills,
  replaceWebSkillApiAgentBindings,
  replaceWebSkillMountPaths,
  replaceWebSkills,
} from "./web-skill-state";

function tr(key: string, values?: Record<string, string | number>) {
  return i18n.t(key, values);
}

/** Mirrors the desktop runtime's character-count compaction trigger (see `add-agent-context-compaction`), scaled down for deterministic mock sessions. */
const mockCompactionTriggerCharacters = 2_000;

const recoveryReportsBySession = new Map<string, SessionRecoveryReport[]>();
const chatConfigStorageKey = "vanehub.session-chat-config.v1";
let memoryChatConfigs: Record<string, ChatConfig> = {};

// Chat configs carry model and policy selections, never a credential, so browser storage stays
// inside the "Honest Web/mock behavior" prohibition on persisting plaintext secrets.
function readChatConfigs(): Record<string, ChatConfig> {
  return readWebMockStorage(chatConfigStorageKey, memoryChatConfigs);
}

function writeChatConfigs(configs: Record<string, ChatConfig>) {
  memoryChatConfigs = configs;
  writeWebMockStorage(chatConfigStorageKey, configs);
}

function mockRecoveryReport(
  session: Session,
  recoveryRevision: number,
  decision: RecoveryDecision,
): SessionRecoveryReport {
  return {
    reportId: `web-recovery-${session.id}-${recoveryRevision}`,
    sessionId: session.id,
    recoveryRevision,
    trigger: decision === "acknowledged" ? "user_acknowledgement" : "startup",
    observedLifecycle: session.lifecycleState,
    observedExecutionRunId: session.activeExecutionRunId,
    decision,
    reasonCodes: decision === "acknowledged"
      ? ["acknowledged_by_user"]
      : ["unfinished_tool_activity"],
    evidenceRefs: [{
      kind: "session",
      sessionId: session.id,
      stateRevision: session.stateRevision,
      historyRevision: session.historyRevision,
    }],
    createdAt: nowIso(),
  };
}

function webRecoverySummary(sessionId: string): SessionRecoverySummary {
  const session = findWebSession(sessionId);
  return {
    session,
    latestReport: recoveryReportsBySession.get(sessionId)?.[0] ?? null,
  };
}

export function seedWebRecoverySessionForTest(
  status: "action_required" | "quarantined" = "action_required",
): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-recovery-session-${nextWebSessionSequence()}`,
    title: "Recovered Web session",
    agentId: "onepiece",
    interactionMode: "api",
    lifecycleState: "failed",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: "D:\\example\\recovery-project",
    projectPath: "D:\\example\\recovery-project",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  prependWebSession(session);
  setWebActiveSessionId(session.id);
  const recoveryRevision = 1;
  const recovered = updateWebSession(session.id, {
    lifecycleState: "failed",
    recoveryStatus: status,
    recoveryRevision,
    stateRevision: session.stateRevision + 1,
    activeExecutionRunId: null,
  });
  recoveryReportsBySession.set(recovered.id, [
    mockRecoveryReport(
      recovered,
      recoveryRevision,
      status === "quarantined" ? "quarantined" : "action_required",
    ),
  ]);
  emitWebSessionEvent({
    kind: status === "quarantined" ? "recovery-quarantined" : "recovery-action-required",
    sessionId: recovered.id,
    recoveryRevision,
  });
  return recovered;
}

export function resetWebRecoverySessionsForTest() {
  const recoverySessionIds = new Set(recoveryReportsBySession.keys());
  replaceWebSessions(listWebSessions().filter((session) => !recoverySessionIds.has(session.id)));
  recoverySessionIds.forEach((sessionId) => {
    deleteWebSessionMessages(sessionId);
    deleteWebActiveStream(sessionId);
  });
  recoveryReportsBySession.clear();
  const activeId = getWebActiveSessionId();
  if (activeId && recoverySessionIds.has(activeId)) setWebActiveSessionId(null);
}

export function seedWebImSessionForTest(connector: ImSessionConnector): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-im-session-${nextWebSessionSequence()}`,
    title: `IM ${connector}`,
    agentId: "codex-cli",
    interactionMode: "cli",
    lifecycleState: "idle",
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


/** Mock cross-session memories (`add-agent-cross-session-memory`, extended to CLI-wrapped agents
 * by `add-cli-memory-support`) — a single host-level pool shared by every agent kind, matching
 * the real backend's shared-pool model. Starts empty; real memories only ever come from a
 * `remember` tool call or extraction, both simulated in `sendMessage`. */
let webAgentMemories: AgentMemory[] = [];
let nextAgentMemoryId = 1;

/** Mirrors the native store's deterministic derivation: a writer that supplies no name gets one
 * from the content, and only ASCII survives slugging, so non-Latin content falls back to the
 * sequence number rather than producing an empty stem. */
function deriveMemoryName(content: string, sequence: number): string {
  const slug = content
    .split(/[^a-zA-Z0-9]+/u)
    .filter(Boolean)
    .slice(0, 6)
    .join("-")
    .toLowerCase();
  return slug || `memory-${sequence}`;
}

/** Mirrors the native bounds so the mock truncates where the desktop runtime would. */
const WEB_MEMORY_INDEX_LINE_CAP = 200;

/** Mirrors the native selection bound. */
const WEB_MEMORY_SELECTION_CAP = 5;

/**
 * Simulates one generation's memory injection: an index over the whole pool, plus the handful of
 * bodies a selection would have read in full.
 *
 * Deterministic rather than model-driven — the Web runtime must reproduce the desktop's observable
 * shape without issuing a provider request, so it stands in for the selector with recency, which
 * is the same ordering the index already uses.
 */
function simulateMemoryIndexInjection(): { indexed: number; selected: AgentMemory[] } {
  const indexed = Math.min(webAgentMemories.length, WEB_MEMORY_INDEX_LINE_CAP);
  return {
    indexed,
    selected: webAgentMemories.slice(0, Math.min(indexed, WEB_MEMORY_SELECTION_CAP)),
  };
}

function disambiguateMemoryName(base: string): string {
  if (!webAgentMemories.some((memory) => memory.name === base)) {
    return base;
  }
  let suffix = 2;
  while (webAgentMemories.some((memory) => memory.name === `${base}-${suffix}`)) {
    suffix += 1;
  }
  return `${base}-${suffix}`;
}

function createAgentMemory(
  agentId: string,
  folder: string | null,
  content: string,
  source: AgentMemory["source"],
  metadata: { name?: string; description?: string; memoryType?: AgentMemoryType | null } = {},
): AgentMemory {
  // An explicit name addresses an existing memory, so it is used as-is and replaces. A derived one
  // must not: two agents recording the same fact are two memories in the shared pool, so the
  // native store's collision suffix is mirrored here rather than silently merging them.
  const name = metadata.name ?? disambiguateMemoryName(deriveMemoryName(content, nextAgentMemoryId));
  const memory: AgentMemory = {
    // The native store's identity is the file path, so the mock mirrors that shape rather than
    // inventing an opaque id the management view would have to special-case.
    id: `${name}.md`,
    agentId,
    folder,
    name,
    description: metadata.description ?? content.split("\n")[0] ?? content,
    memoryType: metadata.memoryType ?? null,
    content,
    source,
    createdAt: nowIso(),
  };
  nextAgentMemoryId += 1;
  // Saving under an existing name replaces that memory, matching the native store's update path.
  webAgentMemories = [memory, ...webAgentMemories.filter((existing) => existing.name !== name)];
  return memory;
}


function searchText(value: string | null | undefined, query: string) {
  return (value ?? "").toLocaleLowerCase().includes(query.toLocaleLowerCase());
}

function sessionSearchMatches(session: Session, query: string): SessionSearchResult | null {
  const matches: SessionSearchResult["matches"] = [];
  if (searchText(session.title, query)) {
    matches.push({ kind: "title", excerpt: session.title });
  }
  const remoteWorkspace = session.remoteWorkspace;
  const projectMatch = [
    session.folder,
    session.projectPath,
    session.worktreePath,
    session.worktreeName,
    session.worktreeBranch,
    remoteWorkspace?.host,
    remoteWorkspace?.user,
    remoteWorkspace?.path,
    remoteWorkspace?.displayName,
    remoteWorkspace?.uri,
  ].find((value) => searchText(value, query));
  if (projectMatch) {
    matches.push({ kind: "project", excerpt: projectMatch });
  }
  const messageMatch = getWebSessionMessages(session.id).find((message) => searchText(message.content, query));
  if (messageMatch) {
    matches.push({
      kind: "message",
      excerpt: messageMatch.content.slice(0, 160),
      messageId: messageMatch.id,
    });
  }
  return matches.length > 0 ? { session: { ...session }, matches } : null;
}

function serializeWebSessionExport(input: ExportSessionInput): SessionExportResult {
  const session = findWebSession(input.sessionId);
  const payload = {
    version: 1,
    exportedAt: nowIso(),
    session,
    messages: getWebSessionMessages(session.id),
  };
  const content =
    input.format === "json"
      ? JSON.stringify(payload, null, 2)
      : [`# ${session.title}`, "", `- ID: \`${session.id}\``, `- Agent: \`${session.agentId}\``, "", "## Messages", ""]
          .concat(
            payload.messages.flatMap((message) => [
              `### ${message.role.toUpperCase()} - \`${message.status}\``,
              "",
              message.content,
              "",
            ]),
          )
          .join("\n");
  return {
    status: input.destinationDirectory === null ? "cancelled" : "exported",
    path: input.destinationDirectory ? `${input.destinationDirectory}\\${session.id}.${input.format === "json" ? "json" : "md"}` : null,
    content,
  };
}

/** `add-cli-memory-support`: the shared memory pool is no longer isolated per agent id, so tests
 * that seed memories can leak into later tests within the same file unless explicitly cleared. */
export function resetWebAgentMemoriesForTest() {
  webAgentMemories = [];
}

const webCodeReviewClient = createWebCodeReviewClient(webSessionWorkspaceClient);

function webRunnerDescriptors(sessionId: string, agentId: string): AgentRunnerDescriptor[] {
  const session = findWebSession(sessionId);
  if (session.agentId !== agentId) throw new Error("runner_invalid_selection");
  const descriptors: AgentRunnerDescriptor[] = [{
    selection: { kind: "local" },
    label: "Local",
    hostLabel: "This device",
    available: true,
    unavailableReason: null,
    simulated: true,
    capabilities: { interactiveInput: true, pty: false, cancellation: true, inspection: true, recovery: "none" },
  }];
  const connectionId = session.remoteSshConnectionId;
  const revision = session.remoteSshConnectionRevision;
  if (connectionId && revision && session.remoteWorkspace) {
    const connection = findWebSshConnection(connectionId);
    const credentialConfigured = connection?.authMode === "password" ? connection.hasPassword : Boolean(connection?.keyPath);
    const available = Boolean(connection
      && connection.revision === revision
      && connection.host === session.remoteWorkspace.host
      && connection.port === (session.remoteWorkspace.port ?? 22)
      && connection.user === session.remoteWorkspace.user
      && connection.hostTrust
      && credentialConfigured);
    descriptors.push({
      selection: { kind: "ssh", targetId: connectionId, targetRevision: revision },
      label: session.remoteWorkspace.displayName,
      hostLabel: session.remoteWorkspace.host,
      available,
      unavailableReason: available ? null : "ssh_authority_unavailable",
      simulated: true,
      capabilities: { interactiveInput: true, pty: true, cancellation: true, inspection: true, recovery: "inspect_only" },
    });
  }
  descriptors.push(
    {
      selection: { kind: "docker" }, label: "Docker / Sandbox", hostLabel: null,
      available: false, unavailableReason: "runner_not_implemented", simulated: true,
      capabilities: { interactiveInput: false, pty: false, cancellation: false, inspection: false, recovery: "none" },
    },
    {
      selection: { kind: "cloud" }, label: "Cloud", hostLabel: null,
      available: false, unavailableReason: "runner_not_implemented", simulated: true,
      capabilities: { interactiveInput: false, pty: false, cancellation: false, inspection: false, recovery: "none" },
    },
  );
  return descriptors;
}

function selectWebRunner(sessionId: string, agentId: string, requested?: AgentRunnerSelection): AgentRunnerDescriptor {
  const selection = requested ?? { kind: "local" };
  const descriptor = webRunnerDescriptors(sessionId, agentId).find((candidate) =>
    candidate.selection.kind === selection.kind
    && (candidate.selection.targetId ?? null) === (selection.targetId ?? null)
    && (candidate.selection.targetRevision ?? null) === (selection.targetRevision ?? null));
  if (!descriptor) throw new Error("runner_invalid_selection");
  if (!descriptor.available) throw new Error("runner_unsupported_capability");
  return descriptor;
}

function webRunRunner(descriptor: AgentRunnerDescriptor): NonNullable<AgentRun["runner"]> {
  const targetId = descriptor.selection.targetId ?? "local";
  const targetRevision = descriptor.selection.targetRevision ?? null;
  return {
    kind: descriptor.selection.kind as "local" | "ssh",
    targetId,
    targetRevision,
    label: descriptor.label,
    hostLabel: descriptor.hostLabel,
    recovery: descriptor.capabilities.recovery,
    capabilityWitness: `web-simulated:${descriptor.selection.kind}`,
    authorityWitness: `web-simulated:${targetId}:${targetRevision ?? "none"}`,
    recoveryReference: null,
  };
}

export const webAgentClient: AgentService = {
  ...webEvaluationClient,
  ...webPromptHookClient,
  ...webApiAgentClient,
  ...webOnePieceProviderClient,
  ...webOnePieceProfileClient,
  ...webHybridRoutingClient,
  ...webCodeIndexClient,
  ...webCliToolClient,
  ...webCliParameterClient,
  ...webCliConfigClient,
  ...webScheduledTaskClient,
  ...webContextQualityClient,
  ...webSkillGovernanceClient,
  ...webSkillEvidenceClient,
  ...webAgentRegistryClient,
  getDesktopUpdateSnapshot: webDesktopUpdateClient.getSnapshot,
  getDesktopUpdatePreferences: webDesktopUpdateClient.getPreferences,
  saveDesktopUpdatePreferences: webDesktopUpdateClient.savePreferences,
  checkForDesktopUpdate: webDesktopUpdateClient.check,
  downloadAndInstallDesktopUpdate: webDesktopUpdateClient.install,
  restartAfterDesktopUpdate: webDesktopUpdateClient.restart,
  ...webBuiltinToolClient,
  ...webSessionWorkspaceClient,
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
    const memoryCount = webAgentMemories.filter((memory) => memory.agentId === agentId).length;
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

  async listAllMemories() {
    return webAgentMemories;
  },

  async deleteAgentMemory(memoryId: string) {
    webAgentMemories = webAgentMemories.filter((memory) => memory.id !== memoryId);
  },

  async resetAllMemories() {
    webAgentMemories = [];
  },

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

  async getWorkflowState() {
    return getWebWorkflowState();
  },

  async selectAgent(agentId: string, interactionMode: InteractionMode) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${interactionMode}.`);
    }
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: agentId,
      activeInteractionMode: interactionMode,
      lifecycleState: "idle",
    });
    return getWebWorkflowState();
  },

  async launchActiveWorkflow() {
    setWebWorkflowState({
      ...getWebWorkflowState(),
      lifecycleState: getWebWorkflowState().activeAgentId ? "running" : "failed",
    });
    return {
      workflow: getWebWorkflowState(),
      message: getWebWorkflowState().activeAgentId
        ? "Web preview session marked as running."
        : "Select an agent before launching.",
    };
  },

  async getSessionDetails(): Promise<SessionDetails> {
    const adapter = getWebWorkflowState().activeInteractionMode ?? "none";
    return {
      agentId: getWebWorkflowState().activeAgentId,
      interactionMode: getWebWorkflowState().activeInteractionMode,
      lifecycleState: getWebWorkflowState().lifecycleState,
      adapter,
      details: {
        runtime: "web",
        storage: "in-memory",
      },
    };
  },

  async listSessions() {
    return sortWebSessions(listWebSessions().filter((session) => !session.archived && !isWebLoopRoleSession(session.id)));
  },

  async listArchivedSessions() {
    return sortWebSessions(listWebSessions().filter((session) => session.archived && !isWebLoopRoleSession(session.id)));
  },

  async searchSessions(input: SessionSearchInput) {
    const query = input.query.trim();
    if (!query) return [];
    return sortWebSessions(listWebSessions().filter((session) => !isWebLoopRoleSession(session.id)))
      .map((session) => sessionSearchMatches(session, query))
      .filter((result): result is SessionSearchResult => result !== null)
      .slice(0, input.limit ?? 50);
  },

  async getSession(sessionId: string) {
    return findWebSession(sessionId);
  },

  async getSessionRecoverySummary(sessionId: string) {
    return structuredClone(webRecoverySummary(sessionId));
  },

  async listSessionRecoveryReports(sessionId: string, limit = 20) {
    findWebSession(sessionId);
    const boundedLimit = Math.max(1, Math.min(100, Math.trunc(limit)));
    return structuredClone((recoveryReportsBySession.get(sessionId) ?? []).slice(0, boundedLimit));
  },

  async acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement> {
    const session = findWebSession(sessionId);
    if (session.recoveryStatus === "quarantined") {
      throw new Error(`Recovery acknowledgement is not allowed for quarantined session ${sessionId}.`);
    }
    if (session.recoveryStatus !== "action_required") {
      throw new Error(`Recovery acknowledgement is not allowed for session ${sessionId}.`);
    }
    if (session.recoveryRevision !== expectedRecoveryRevision) {
      throw new Error(
        `Recovery revision conflict for session ${sessionId}; current revision is ${session.recoveryRevision}.`,
      );
    }
    const recoveryRevision = session.recoveryRevision + 1;
    const updated = updateWebSession(sessionId, {
      recoveryStatus: "clean",
      recoveryRevision,
      stateRevision: session.stateRevision + 1,
      activeExecutionRunId: null,
    });
    const report = mockRecoveryReport(updated, recoveryRevision, "acknowledged");
    recoveryReportsBySession.set(sessionId, [
      report,
      ...(recoveryReportsBySession.get(sessionId) ?? []),
    ]);
    emitWebSessionEvent({
      kind: "recovery-acknowledged",
      sessionId,
      recoveryRevision,
    });
    return structuredClone({ session: updated, report });
  },

  async getActiveSession() {
    const sessionId = getWebActiveSessionId();
    if (!sessionId) return null;
    return listWebSessions().find((session) => session.id === sessionId) ?? null;
  },
  ...webSessionCategoryClient,
  ...webLoopClient,

  async getSessionChatConfig(sessionId) {
    const session = findWebSession(sessionId);
    const stored = readChatConfigs()[sessionId];
    const normalized = stored
      ? normalizeChatConfigForSession(session, stored)
      : defaultChatConfigForSession(session);
    const policy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    return withEffectiveExecutionPolicy(normalized, policy);
  },

  async saveSessionChatConfig(sessionId, config) {
    const session = findWebSession(sessionId);
    const normalized = normalizeChatConfigForSession(session, config);
    writeChatConfigs({ ...readChatConfigs(), [sessionId]: normalized });
    emitWebSessionEvent({ kind: "configuration-changed", sessionId });
    const policy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    return withEffectiveExecutionPolicy(normalized, policy);
  },
  ...webKnownWorkspaceClient,

  async createSession(input) {
    const agent = mockAgents.find((candidate) => candidate.id === input.agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${input.agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(input.interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${input.interactionMode}.`);
    }
    if (agent.availabilityState !== "available" && agent.availabilityState !== "unknown") {
      throw new Error(agent.unavailableReason ?? `${agent.displayName} is not available.`);
    }
    const remoteWorkspace = input.remoteWorkspace ? normalizeRemoteWorkspace(input.remoteWorkspace) : null;
    if (agent.id === "onepiece" && remoteWorkspace) {
      throw new Error("OnePiece supports local projects and local Git worktrees only.");
    }
    const sshConnection = input.remoteWorkspace?.sshConnectionId
      ? findWebSshConnection(input.remoteWorkspace.sshConnectionId)
      : null;
    if (input.remoteWorkspace?.sshConnectionId && !sshConnection) {
      throw new Error(`SSH connection not found: ${input.remoteWorkspace.sshConnectionId}`);
    }
    if (
      sshConnection &&
      remoteWorkspace &&
      (sshConnection.host !== remoteWorkspace.host ||
        sshConnection.port !== (remoteWorkspace.port ?? 22) ||
        sshConnection.user !== (remoteWorkspace.user ?? ""))
    ) {
      throw new Error(
        "SSH connection endpoint does not match the remote workspace snapshot.",
      );
    }
    if (remoteWorkspace && input.worktree?.enabled) {
      throw new Error("Remote workspace cannot use Git worktree");
    }
    const projectPath = remoteWorkspace ? null : resolveProjectPath(input);
    const inspection = projectPath ? inspectMockProject(projectPath) : null;
    if (inspection) {
      upsertKnownProject(inspection);
    }
    if (remoteWorkspace) {
      upsertKnownRemoteWorkspace(remoteWorkspace);
    }
    let effectiveFolder = remoteWorkspace?.uri ?? projectPath;
    let worktreePath: string | null = null;
    let worktreeName: string | null = null;
    let worktreeBranch: string | null = null;
    if (input.worktree?.enabled) {
      if (!inspection?.isGit) {
        throw new Error("Git worktree unavailable");
      }
      worktreeName = validateWorktreeName(input.worktree.name ?? "");
      worktreePath = joinSiblingPath(inspection.path, worktreeName);
      worktreeBranch = `vanehub/${worktreeName}`;
      effectiveFolder = worktreePath;
    }
    const timestamp = nowIso();
    const titleSource = remoteWorkspace?.displayName || effectiveFolder || "";
    const normalizedSeats = (input.seats?.length ? input.seats : [{ agentId: input.agentId, roleId: null }]).map(
      (seat) => ({
        ...snapshotSeat(seat, mockAgents, listWebExpertRoles()),
        seatId: createWebSeatId(),
        joinedAt: timestamp,
        leftAt: null,
      }),
    );
    const session: Session = {
      id: `web-session-${nextWebSessionSequence()}`,
      title: input.title?.trim() || defaultSessionTitleFromPath(titleSource) || tr("createSession.sessionPlaceholder"),
      agentId: normalizedSeats[0]?.agentId ?? input.agentId,
      // Mirrors the native normalization: no seats means one seat built from the Agent.
      seats: normalizedSeats,
      interactionMode: input.interactionMode,
      lifecycleState: "idle",
      recoveryStatus: "clean",
      recoveryRevision: 0,
      stateRevision: 0,
      historyRevision: 0,
      activeExecutionRunId: null,
      folder: effectiveFolder,
      projectPath,
      worktreePath,
      worktreeName,
      worktreeBranch,
      remoteWorkspace,
      remoteSshConnectionId: sshConnection?.id ?? null,
      remoteSshConnectionRevision: sshConnection?.revision ?? null,
      runtimeSessionId: null,
      categoryId: null,
      pinned: false,
      archived: false,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    prependWebSession(session);
    setWebActiveSessionId(session.id);
    discoverWebSessionCodeIndex(session);
    emitWebSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    });
    return createWebMockOperation({
      id: `web-session-create-${session.id}-${Date.now()}`,
      kind: "workspace",
      relatedEntityId: remoteWorkspace?.uri ?? projectPath,
      message: `Created mock session ${session.id}`,
      terminalStatus: "succeeded",
      error: null,
      result: session as unknown as Record<string, unknown>,
    });
  },

  async deleteSession(sessionId: string) {
    findWebSession(sessionId);
    cancelWebActiveStream(sessionId);
    deleteWebSessionMessages(sessionId);
    recoveryReportsBySession.delete(sessionId);
    deleteWebChatSubscribers(sessionId);
    const configs = { ...readChatConfigs() };
    delete configs[sessionId];
    writeChatConfigs(configs);
    replaceWebSessions(listWebSessions().filter((session) => session.id !== sessionId));
    if (getWebActiveSessionId() === sessionId) {
      setWebActiveSessionId(null);
      emitWebSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
  },

  async switchSession(sessionId: string) {
    const session = findWebSession(sessionId);
    if (session.archived) {
      throw new Error(`Cannot switch to archived session: ${sessionId}`);
    }
    setWebActiveSessionId(session.id);
    emitWebSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    setWebWorkflowState({
      ...getWebWorkflowState(),
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    });
    return session;
  },

  async renameSession(sessionId: string, title: string) {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      throw new Error(tr("web.error.sessionTitleRequired"));
    }
    return updateWebSession(sessionId, { title: trimmedTitle });
  },

  async updateSessionSeats(input: UpdateSessionSeatsInput) {
    const session = findWebSession(input.sessionId);
    if (session.updatedAt !== input.expectedUpdatedAt) {
      throw new Error("validation error: Session participants changed since they were loaded.");
    }
    if (input.seats.length === 0) {
      throw new Error("validation error: A session must keep at least one active participant.");
    }
    const changedAt = nowIso();
    const historical = session.seats ?? [{
      seatId: `${session.id}:seat:0`,
      agentId: session.agentId,
      roleId: null,
      joinedAt: session.createdAt,
      leftAt: null,
    }];
    const retained = new Set<string>();
    const additions: SessionSeat[] = [];
    for (const requested of input.seats) {
      const existing = historical.find((seat) =>
        seat.leftAt == null && !retained.has(seat.seatId ?? "") &&
        ((Boolean(requested.seatId) && seat.seatId === requested.seatId &&
          seat.agentId === requested.agentId && seat.roleId === requested.roleId) ||
          (!requested.seatId && seat.agentId === requested.agentId && seat.roleId === requested.roleId)),
      );
      if (existing?.seatId) {
        retained.add(existing.seatId);
      } else {
        additions.push({
          ...requested,
          seatId: createWebSeatId(),
          joinedAt: changedAt,
          leftAt: null,
        });
      }
    }
    const seats = [
      ...historical.map((seat) =>
        seat.leftAt == null && !retained.has(seat.seatId ?? "")
          ? { ...seat, leftAt: changedAt }
          : seat,
      ),
      ...additions,
    ];
    const firstActive = seats.find((seat) => seat.leftAt == null);
    if (!firstActive) {
      throw new Error("validation error: A session must keep at least one active participant.");
    }
    return updateWebSession(input.sessionId, { seats, agentId: firstActive.agentId });
  },

  async rebindRemoteSessionSshConnection(
    sessionId: string,
    connectionId: string,
  ) {
    const session = findWebSession(sessionId);
    if (!session.remoteWorkspace) {
      throw new Error(
        "Only remote workspace sessions can bind an SSH connection.",
      );
    }
    const connection = findWebSshConnection(connectionId);
    if (!connection) {
      throw new Error(`SSH connection not found: ${connectionId}`);
    }
    if (
      connection.host !== session.remoteWorkspace.host ||
      connection.port !== (session.remoteWorkspace.port ?? 22) ||
      connection.user !== (session.remoteWorkspace.user ?? "")
    ) {
      throw new Error(
        "SSH connection endpoint does not match the remote workspace snapshot.",
      );
    }
    return updateWebSession(sessionId, {
      remoteSshConnectionId: connection.id,
      remoteSshConnectionRevision: connection.revision,
    });
  },

  async pinSession(sessionId: string) {
    return updateWebSession(sessionId, { pinned: true });
  },

  async unpinSession(sessionId: string) {
    return updateWebSession(sessionId, { pinned: false });
  },

  async archiveSession(sessionId: string) {
    const cancelled = cancelWebActiveStream(sessionId);
    const session = updateWebSession(sessionId, { archived: true, ...(cancelled ? { lifecycleState: "stopped" } : {}) });
    if (getWebActiveSessionId() === sessionId) {
      setWebActiveSessionId(null);
      emitWebSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
    return session;
  },

  async unarchiveSession(sessionId: string) {
    return updateWebSession(sessionId, { archived: false });
  },

  async exportSession(input: ExportSessionInput) {
    return serializeWebSessionExport(input);
  },

  async listAgentRunners(sessionId, agentId) {
    return structuredClone(webRunnerDescriptors(sessionId, agentId));
  },

  async sendMessage(input) {
    const session = findWebSession(input.sessionId);
    if (session.archived) throw new Error("Archived sessions cannot accept messages.");
    if (session.recoveryStatus !== "clean") {
      throw new Error(`Session recovery state ${session.recoveryStatus} blocks new messages.`);
    }
    if (session.activeExecutionRunId !== null) {
      throw new Error("A generation is already active for this session.");
    }
    const config = normalizeChatConfigForSession(session, input.config);
    const agentPolicy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    const effectiveExecutionPolicy = withEffectiveExecutionPolicy(
      config,
      agentPolicy,
    ).effectiveExecutionPolicy;
    if (hasWebActiveStream(input.sessionId)) {
      throw new Error("A generation is already active for this session.");
    }
    const selectedRunner = selectWebRunner(input.sessionId, session.agentId, input.runner);
    const timestamp = nowIso();
    const activeSeats = (session.seats ?? []).filter((seat) => seat.leftAt == null);
    const firstSpeakerSeatId = activeSeats.length > 1 ? activeSeats[0]?.seatId : undefined;
    const existingMessages = getWebSessionMessages(input.sessionId);
    const nextSequence = existingMessages.reduce(
      (maximum, message) => Math.max(maximum, message.sessionSequence),
      0,
    ) + 1;
    const executionRunId = `web-run-${input.sessionId}-${Date.now()}`;
    prependWebAgentRun({
      id: executionRunId,
      owner: { ownerType: "agent_generation", ownerId: session.agentId },
      links: [{ linkType: "session", linkId: input.sessionId }],
      parentRunId: null,
      state: "running",
      recoveryPolicy: selectedRunner.selection.kind === "ssh" ? "owner_reconciles" : "not_recoverable",
      runner: webRunRunner(selectedRunner),
      retryCount: 0,
      maxRetries: 0,
      reasonCode: null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 2,
      lastWitness: `web-simulated-runner-start:${selectedRunner.selection.kind}`,
    });
    setWebAgentRunEvents(executionRunId, []);
    const userMessage: ChatMessage = {
      id: createWebMessageId(),
      sessionId: input.sessionId,
      role: "user",
      content: input.content.trim(),
      status: "completed",
      fileReferences: input.fileReferences,
      createdAt: timestamp,
      updatedAt: timestamp,
      sessionSequence: nextSequence,
      executionRunId,
    };
    const assistantMessage: ChatMessage = {
      id: createWebMessageId(),
      sessionId: input.sessionId,
      role: "assistant",
      speakerSeatId: firstSpeakerSeatId,
      content: "",
      status: "streaming",
      createdAt: timestamp,
      updatedAt: timestamp,
      sessionSequence: nextSequence + 1,
      executionRunId,
    };
    setWebSessionMessages(input.sessionId, [...existingMessages, userMessage, assistantMessage]);
    updateWebSession(input.sessionId, {
      lifecycleState: "running",
      activeExecutionRunId: executionRunId,
      stateRevision: session.stateRevision + 1,
      historyRevision: session.historyRevision + 2,
    });

    const responseText = `Mock ${session.agentId} response: I received "${userMessage.content}". This is a streaming preview in Web mode.`;
    const tokens = responseText.match(/.{1,6}/g) ?? [responseText];
    // Memory simulation below is gated on these (`add-personalization-settings`) — unlike custom
    // instructions, memory's on/off effect is structurally observable in mock mode via the
    // `tool_use`/`rich_block` event stream, so the mock must respect the toggles rather than
    // always firing. Every mock session already simulates a tool call (shell/remember/mcp) below,
    // so it is "tool-assisted" under the real definition — the sub-toggle applies accordingly.
    const personalizationSettings = readWebAppSettings();
    const memoryEnabled = personalizationSettings.memoryEnabled;
    const toolAssistedExtractionEnabled = personalizationSettings.memoryToolAssistedChatsEnabled;
    const automaticCompactionEnabled = personalizationSettings.automaticContextCompactionEnabled;
    const timeoutIds: Array<ReturnType<typeof setTimeout>> = [];
    const startTimeoutId = setTimeout(() => {
      emitWebChatEvent({ type: "started", sessionId: input.sessionId, messageId: assistantMessage.id });
    }, 80);
    timeoutIds.push(startTimeoutId);
    const historyCharacterCount = getWebSessionMessages(input.sessionId).reduce(
      (total, message) => total + message.content.length,
      0,
    );
    if (automaticCompactionEnabled && historyCharacterCount > mockCompactionTriggerCharacters) {
      const afterCharacters = Math.min(
        historyCharacterCount,
        Math.max(userMessage.content.length, Math.ceil(historyCharacterCount * 0.4)),
      );
      const savedCharacters = Math.max(0, historyCharacterCount - afterCharacters);
      const compactionTimeoutId = setTimeout(() => {
        publishWebChatEvent({
          type: "rich_block",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          block: {
            id: `web-compaction-${assistantMessage.id}`,
            kind: "card",
            v: 1,
            title: "Conversation compacted",
            bodyMarkdown: "Earlier context was compacted. This evidence contains measurements only and excludes conversation content.",
            tone: "info",
            fields: [
              { label: "Before characters", value: String(historyCharacterCount) },
              { label: "After characters", value: String(afterCharacters) },
              { label: "Characters saved", value: String(savedCharacters) },
              { label: "Before tokens", value: "Unavailable" },
              { label: "After tokens", value: "Unavailable" },
              { label: "Tokens saved", value: "Unavailable" },
              { label: "Measurement quality", value: "characters-only → characters-only" },
              { label: "Trigger source", value: "character-fallback" },
              { label: "Compaction path", value: "compatibility" },
              { label: "Policy version", value: "onepiece-context-production-v1" },
            ],
            meta: {
              evidenceKind: "context-compaction",
              beforeCharacters: historyCharacterCount,
              afterCharacters,
              savedCharacters,
              beforeTokens: null,
              afterTokens: null,
              savedTokens: null,
              beforeQuality: "characters-only",
              afterQuality: "characters-only",
              triggerSource: "character-fallback",
              compactionPath: "compatibility",
              policyVersion: "onepiece-context-production-v1",
            },
          },
        });
      }, 150);
      timeoutIds.push(compactionTimeoutId);
      // Extraction (`add-agent-cross-session-memory`) piggybacks on the same trigger as
      // compaction in the real runtime, so the mock fires it at the identical condition.
      if (
        memoryEnabled &&
        toolAssistedExtractionEnabled &&
        mockAgents.find((candidate) => candidate.id === session.agentId)?.launch.kind === "api"
      ) {
        const extractionTimeoutId = setTimeout(() => {
          const memory = createAgentMemory(
            session.agentId,
            session.folder,
            `Extracted from a long conversation: "${userMessage.content.slice(0, 60)}"`,
            "automatic",
          );
          publishWebChatEvent({
            type: "rich_block",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            block: {
              id: `web-memory-extracted-${assistantMessage.id}`,
              kind: "card",
              v: 1,
              title: "Memory extracted",
              bodyMarkdown: `Saved for future sessions: "${memory.content}"`,
              tone: "info",
            },
          });
        }, 150);
        timeoutIds.push(extractionTimeoutId);
      }
    }
    // CLI-completion-triggered extraction (`add-cli-memory-support`): unlike the compaction-gated
    // block above, the real backend's CLI extraction fires after every completed CLI turn with no
    // length threshold (design.md D3 MVP) and is gated only by the memory master toggle, never the
    // tool-assisted sub-toggle — that sub-toggle governs only OnePiece's own compaction-triggered
    // extraction (see `personalization.memory.toolAssistedDesc`).
    if (memoryEnabled && mockAgents.find((candidate) => candidate.id === session.agentId)?.launch.kind === "cli") {
      const cliExtractionTimeoutId = setTimeout(() => {
        const memory = createAgentMemory(
          session.agentId,
          session.folder,
          `Extracted from a CLI session: "${userMessage.content.slice(0, 60)}"`,
          "automatic",
        );
        publishWebChatEvent({
          type: "rich_block",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          block: {
            id: `web-memory-extracted-${assistantMessage.id}`,
            kind: "card",
            v: 1,
            title: "Memory extracted",
            bodyMarkdown: `Saved for future sessions: "${memory.content}"`,
            tone: "info",
          },
        });
      }, 150);
      timeoutIds.push(cliExtractionTimeoutId);
    }
    // Shared pool (`add-cli-memory-support`): any memory from any agent counts, not just ones
    // produced by this session's own agent/folder.
    const hadExistingMemories = webAgentMemories.length > 0;
    if (memoryEnabled && hadExistingMemories) {
      // `add-two-tier-memory-recall`: the index is what every request carries, so the mock reports
      // how many memories it names. Neither this nor the selection below depends on an embedding
      // source being configured — memory has to work on an installation without retrieval.
      const injected = simulateMemoryIndexInjection();
      const memoryInjectionTimeoutId = setTimeout(() => {
        publishWebChatEvent({
          type: "rich_block",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          block: {
            id: `web-memory-applied-${assistantMessage.id}`,
            kind: "card",
            v: 1,
            title: "Memory applied",
            bodyMarkdown: `This response was influenced by memories saved in earlier sessions. Index carried ${injected.indexed} of them; ${injected.selected.length} read in full.`,
            tone: "info",
          },
        });
      }, 150);
      timeoutIds.push(memoryInjectionTimeoutId);
    }
    const sessionWorkspace = session.folder ? normalizeWebPath(session.folder, "Workspace path") : null;
    const boundSkillNames = listWebSkillApiAgentBindings()
      .filter((binding) => binding.agentId === session.agentId)
      .map((binding) =>
        listWebSkills().find(
          (skill) =>
            skill.id === binding.skillId &&
            skill.scope === binding.scope &&
            skill.workspacePath === binding.workspacePath,
        ),
      )
      .filter(
        (skill): skill is Skill =>
          skill != null &&
          skill.enabled &&
          (skill.scope === "global" ||
            (skill.scope === "workspace" && skill.workspacePath === sessionWorkspace)),
      )
      .map((skill) => skill.metadata.name);
    if (boundSkillNames.length > 0) {
      const skillTimeoutId = setTimeout(() => {
        publishWebChatEvent({
          type: "rich_block",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          block: {
            id: `web-skills-${assistantMessage.id}`,
            kind: "card",
            v: 1,
            title: "Skill instructions applied",
            bodyMarkdown: `This response was influenced by: ${boundSkillNames.join(", ")}.`,
            tone: "info",
          },
        });
      }, 150);
      timeoutIds.push(skillTimeoutId);
    }
    tokens.forEach((contentDelta, index) => {
      const timeoutId = setTimeout(() => {
        publishWebChatEvent({ type: "token", sessionId: input.sessionId, messageId: assistantMessage.id, contentDelta });
      }, 240 + index * 90);
      timeoutIds.push(timeoutId);
    });
    if (config.thinking) {
      const thinkingTimeoutId = setTimeout(() => {
        publishWebChatEvent({
          type: "thinking",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          contentDelta: "Mock thinking: checking session context and selected config.",
        });
      }, 180);
      timeoutIds.push(thinkingTimeoutId);
    }
    const toolUseTimeoutId = setTimeout(() => {
      publishWebChatEvent({
        type: "tool_use",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        toolUse: {
          id: `web-tool-${assistantMessage.id}`,
          name: "read_file",
          input: { path: "README.md" },
          output: "Loaded deterministic Web preview content.",
          status: "completed",
        },
      });
    }, 210);
    timeoutIds.push(toolUseTimeoutId);
    const agent = mockAgents.find((candidate) => candidate.id === session.agentId);
    if (agent?.launch.kind === "api" && effectiveExecutionPolicy !== "readonly") {
      // Trusted agents (`add-agent-tool-trust`) skip the simulated approval step for shell,
      // mirroring the real backend's `requires_approval` short-circuit exactly.
      const isTrusted = isAgentAutoApproved(session.agentId);
      const callId = `web-tool-approval-${assistantMessage.id}`;
      const approvalTimeoutId = setTimeout(() => {
        if (isTrusted) {
          publishWebChatEvent({
            type: "tool_use",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            toolUse: {
              id: callId,
              name: "shell",
              input: { command: "echo mock" },
              output: "mock\n",
              status: "completed",
            },
          });
          return;
        }
        createWebPendingApproval(callId, {
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolName: "shell",
          agentId: session.agentId,
          action: "shell.exec",
          resource: "workspace",
          riskLevel: "L2",
          createdAt: new Date().toISOString(),
        });
        publishWebChatEvent({
          type: "tool_use",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolUse: {
            id: callId,
            name: "shell",
            input: { command: "echo mock" },
            status: "awaiting_approval",
          },
        });
      }, 230);
      timeoutIds.push(approvalTimeoutId);
      // Clarification round trip (`add-agent-user-question`). Gated on an explicit marker in the
      // message rather than emitted every turn: a question blocks until answered, so simulating
      // one unconditionally would leave every other mock conversation waiting on a card.
      if (input.content.includes(WEB_MOCK_QUESTION_TRIGGER)) {
        const questionTimeoutId = setTimeout(() => {
          publishWebChatEvent({
            type: "tool_use",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            toolUse: {
              id: `web-tool-question-${assistantMessage.id}`,
              name: "ask_user_question",
              input: {
                question: "Which approach should the simulated agent take?",
                options: ["Rewrite the module", "Patch it in place"],
              },
              status: "awaiting_input",
            },
          });
        }, 240);
        timeoutIds.push(questionTimeoutId);
      }
      // Request to leave plan mode (`add-agent-plan-exit-request`).
      if (input.content.includes(WEB_MOCK_PLAN_EXIT_TRIGGER)) {
        const planExitTimeoutId = setTimeout(() => {
          publishWebChatEvent({
            type: "tool_use",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            toolUse: {
              id: `web-tool-plan-exit-${assistantMessage.id}`,
              name: "exit_plan_mode",
              input: {
                plan: "Rewrite the parser, then update its three callers.",
              },
              status: "awaiting_input",
            },
          });
        }, 245);
        timeoutIds.push(planExitTimeoutId);
      }
      // Read-only search (`add-onepiece-search-and-edit-tools`): `grep` is classified
      // `AutoApprove`, so it follows `remember`'s no-approval path rather than `shell`'s gated
      // one. Output is a fixed fake result — the Web runtime never touches a real filesystem.
      const grepTimeoutId = setTimeout(() => {
        publishWebChatEvent({
          type: "tool_use",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolUse: {
            id: `web-grep-${assistantMessage.id}`,
            name: "grep",
            input: { pattern: "export function", output_mode: "files_with_matches" },
            output: "src/App.tsx\nsrc/main.tsx",
            status: "completed",
          },
        });
      }, 233);
      timeoutIds.push(grepTimeoutId);
      // Explicit path (`add-agent-cross-session-memory`): simulates the model calling the
      // `remember` tool, mirroring the deterministic `read_file`/`shell` tool_use events above.
      // Gated on `memoryEnabled` only — the tool-assisted sub-toggle never affects explicit
      // saves (`add-personalization-settings`).
      if (memoryEnabled) {
        const rememberTimeoutId = setTimeout(() => {
          const memory = createAgentMemory(
            session.agentId,
            session.folder,
            `User said: "${userMessage.content.slice(0, 60)}"`,
            "explicit",
          );
          publishWebChatEvent({
            type: "tool_use",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            toolUse: {
              id: `web-remember-${assistantMessage.id}`,
              name: "remember",
              input: { content: memory.content },
              output: "Saved.",
              status: "completed",
            },
          });
        }, 235);
        timeoutIds.push(rememberTimeoutId);
      }
      // MCP-sourced tool call (`add-agent-mcp-tools`): simulates the model calling a tool
      // exposed by a configured MCP server. Always approval-gated, mirroring the same
      // `webPendingApprovals`/`resolvePendingApproval` flow `shell` already uses above — the
      // real backend floors every MCP-sourced tool call at `Ask` unconditionally
      // (`add-permissions-core` design.md D3), with no auto-approve path, unlike `remember`.
      const mcpCallId = `web-tool-approval-mcp-${assistantMessage.id}`;
      const mcpSimulation = createWebMcpToolSimulationPlan({
        callId: mcpCallId,
        catalog: [
          {
            name: "mcp__mock-server__search",
            description: "Search deterministic Web preview data",
            inputSchema: { type: "object", properties: { query: { type: "string" } } },
          },
        ],
        toolName: "mcp__mock-server__search",
        arguments: { query: "mock" },
        result: "mock MCP result",
      });
      const mcpApprovalTimeoutId = setTimeout(() => {
        if (!mcpSimulation.success) {
          publishWebChatEvent({
            type: "tool_use",
            sessionId: input.sessionId,
            messageId: assistantMessage.id,
            toolUse: mcpSimulation.failed,
          });
          return;
        }
        createWebPendingApproval(mcpCallId, {
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolName: mcpSimulation.awaitingApproval.name,
          input: mcpSimulation.completed.input,
          output: mcpSimulation.completed.output,
          agentId: session.agentId,
          action: "mcp.tool",
          resource: mcpSimulation.awaitingApproval.name,
          riskLevel: "L2",
          createdAt: new Date().toISOString(),
        });
        publishWebChatEvent({
          type: "tool_use",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolUse: mcpSimulation.awaitingApproval,
        });
      }, 237);
      timeoutIds.push(mcpApprovalTimeoutId);
    }
    const richCardTimeoutId = setTimeout(() => {
      publishWebChatEvent({
        type: "rich_block",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        block: {
          id: `web-rich-card-${assistantMessage.id}`,
          kind: "card",
          v: 1,
          title: "Web preview summary",
          bodyMarkdown: "Mock Rich Block rendering is active for this session.",
          tone: "info",
          fields: [
            { label: "Agent", value: session.agentId },
            { label: "Mode", value: session.interactionMode },
          ],
        },
      });
    }, 260);
    timeoutIds.push(richCardTimeoutId);
    const richChecklistTimeoutId = setTimeout(() => {
      publishWebChatEvent({
        type: "rich_block",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
        block: {
          id: `web-rich-checklist-${assistantMessage.id}`,
          kind: "checklist",
          v: 1,
          title: "Mock validation",
          items: [
            { id: "contract", text: "Stream event normalized", checked: true },
            { id: "render", text: "Rich Block attached to message", checked: true },
          ],
        },
      });
    }, 300);
    timeoutIds.push(richChecklistTimeoutId);
    const completeTimeoutId = setTimeout(() => {
      publishWebChatEvent({
        type: "completed",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
      });
    }, 320 + tokens.length * 90);
    timeoutIds.push(completeTimeoutId);
    setWebActiveStream(input.sessionId, { messageId: assistantMessage.id, timeoutIds });
    return assistantMessage;
  },

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
