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
  webPendingApprovals,
  webPrincipalTemplates,
} from "./web-permissions-mock-state";
import { upsertToolUse } from "./tool-use";
import type {
  AgentMemory,
  AgentMemoryType,
  AssignSessionCategoryInput,
  AgentTerminalEvent,
  AgentTerminalSession,
  UpdateSessionSeatsInput,
  AgentTerminalSize,
  CreateSessionCategoryInput,
  CreateSessionInput,
  ExportSessionInput,
  InteractionMode,
  KnownRemoteWorkspace,
  KnownProject,
  ProjectInspection,
  RemoteWorkspace,
  RenameSessionCategoryInput,
  Session,
  SessionSeat,
  SessionCategory,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  SessionDetails,
  ImSessionConnector,
} from "../types/agent";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { readWebAppSettings } from "./web-settings-client";
import { defaultSessionTitleFromPath, normalizeDisplayPath } from "../lib/session-path";
import { snapshotSeat } from "./seat-presentation";
import type { ChatConfig, ChatMessage, ChatStreamEvent } from "../types/chat";
import type { UsageStatistics, UsageStatisticsRange } from "../types/chat";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "./web-token-usage";
import type { AgentRun, AgentRunEvent } from "../types/agent-run";
import type { AgentRunnerDescriptor, AgentRunnerSelection } from "../types/agent-runner";
import type { MissionControlActionReceipt, MissionControlOverview, MissionControlQuery, MissionControlRunDetail, MissionControlRunSummary } from "../types/mission-control";
import { webEvaluationClient } from "./web-evaluation-client";
import type {
  ContinueLoopInput,
  LoopDefinition,
  LoopEvent,
  LoopEvidence,
  LoopIteration,
  LoopRun,
  SaveLoopDefinitionInput,
  StartLoopResult,
} from "../types/loop";
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
import { daysAgoIso, nowIso } from "./web-mock-clock";
import { createWebMockOperation } from "./web-operation-client";
import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";
import { builtinExpertRoles } from "../config/builtin-expert-roles";
import { validateExpertRoleInput } from "./expert-role-runtime";
import { aggregateSessionUsageRecords, aggregateUsageRecords, type UsageRecord } from "./usage-statistics";
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

const webRetainedTerminalTranscriptBytes = 1_000_000;
/** Mirrors the desktop runtime's character-count compaction trigger (see `add-agent-context-compaction`), scaled down for deterministic mock sessions. */
const mockCompactionTriggerCharacters = 2_000;

let nextMessageId = 1;
const recoveryReportsBySession = new Map<string, SessionRecoveryReport[]>();
let sessionCategories: SessionCategory[] = [];
let nextSessionCategoryId = 1;
let loopDefinitions: LoopDefinition[] = [];
let loopRuns: LoopRun[] = [];
let nextLoopDefinitionId = 1;
let nextLoopRunId = 1;
let nextLoopEvidenceId = 1;
let webExpertRoles: ExpertRole[] = builtinExpertRoles.map((role) => structuredClone(role));
let nextExpertRoleId = 1;
const loopSubscribers = new Map<string, Set<(event: LoopEvent) => void>>();
const loopTimers = new Map<string, ReturnType<typeof setTimeout>>();
let webLoopPhaseDelayMs = 220;
const loopRoleSessionIds = new Set<string>();
let knownProjects: KnownProject[] = [];
let knownRemoteWorkspaces: KnownRemoteWorkspace[] = [];
const messagesBySession = new Map<string, ChatMessage[]>();
const subscribersBySession = new Map<string, Set<(event: ChatStreamEvent) => void>>();
const activeStreams = new Map<string, { messageId: string; timeoutIds: Array<ReturnType<typeof setTimeout>> }>();
const terminalSubscribersBySession = new Map<string, Set<(event: AgentTerminalEvent) => void>>();
const terminalsBySession = new Map<string, AgentTerminalSession>();
const terminalTranscriptsBySession = new Map<string, string>();
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
    messagesBySession.delete(sessionId);
    activeStreams.delete(sessionId);
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

const representativeUsageRecords: UsageRecord[] = [
  {
    messageId: "web-usage-reported",
    sessionId: "web-usage-session-codex",
    agentId: "codex-cli",
    accountingKind: "reported",
    inputCount: 100,
    outputCount: 40,
    cacheReadCount: 10,
    cacheCreationCount: 5,
    occurredAt: daysAgoIso(1),
  },
  {
    messageId: "web-usage-estimated",
    sessionId: "web-usage-session-claude",
    agentId: "claude-code",
    accountingKind: "estimated",
    inputCount: 1_000,
    outputCount: 400,
    cacheReadCount: 0,
    cacheCreationCount: 0,
    occurredAt: daysAgoIso(2),
  },
];

function pathSegments(path: string) {
  return path.split(/[\\/]/).filter(Boolean);
}

function displayNameForPath(path: string) {
  return pathSegments(path).at(-1) ?? path;
}

function parentPath(path: string) {
  const normalized = path.replace(/[\\/]+$/, "");
  const separatorIndex = Math.max(normalized.lastIndexOf("\\"), normalized.lastIndexOf("/"));
  return separatorIndex <= 0 ? normalized : normalized.slice(0, separatorIndex);
}

function joinSiblingPath(projectPath: string, worktreeName: string) {
  const separator = projectPath.includes("\\") ? "\\" : "/";
  return `${parentPath(projectPath)}${separator}${displayNameForPath(projectPath)}-${worktreeName}`;
}

function validateWorktreeName(name: string) {
  const trimmed = name.trim();
  if (!trimmed || trimmed.includes("/") || trimmed.includes("\\") || trimmed.includes("..") || /[\u0000-\u001f]/.test(trimmed)) {
    throw new Error("Invalid worktree name");
  }
  return trimmed;
}

function inspectMockProject(path: string): ProjectInspection {
  const trimmedPath = path.trim();
  const isGit = !/(^|[\\/])(non-git|scratch|plain)([\\/]|$)/i.test(trimmedPath);
  return {
    path: trimmedPath,
    displayName: displayNameForPath(trimmedPath),
    isGit,
    gitRoot: isGit ? trimmedPath : null,
  };
}

function upsertKnownProject(inspection: ProjectInspection) {
  const timestamp = nowIso();
  const project: KnownProject = {
    path: inspection.path,
    displayName: inspection.displayName,
    isGit: inspection.isGit,
    lastOpenedAt: timestamp,
  };
  knownProjects = [project, ...knownProjects.filter((candidate) => candidate.path !== project.path)];
  return project;
}

function resolveProjectPath(input: CreateSessionInput) {
  const path = input.projectPath?.trim() || input.folder?.trim() || "";
  return path ? normalizeDisplayPath(path) : null;
}

function displayNameForRemotePath(path: string) {
  return path.replace(/\/+$/, "").split("/").filter(Boolean).at(-1) ?? path;
}

function normalizeRemoteWorkspace(input: NonNullable<CreateSessionInput["remoteWorkspace"]>): RemoteWorkspace {
  const host = input.host.trim();
  const port = input.port ?? 22;
  const path = input.path.trim();
  const user = input.user?.trim() || null;
  if (!host || !path) {
    throw new Error("Remote workspace requires host and path");
  }
  if (host.includes("/") || host.includes("\\") || /[\u0000-\u001f]/.test(`${host}${path}${user ?? ""}`)) {
    throw new Error("Invalid remote workspace");
  }
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error("Invalid remote workspace port");
  }
  const authority = user ? `${user}@${host}` : host;
  const portSegment = port === 22 ? "" : `:${port}`;
  return {
    host,
    port,
    user,
    path,
    displayName: input.displayName?.trim() || `${host}:${displayNameForRemotePath(path)}`,
    uri: `ssh://${authority}${portSegment}${path.startsWith("/") ? "" : "/"}${path}`,
  };
}

function upsertKnownRemoteWorkspace(remoteWorkspace: RemoteWorkspace) {
  const timestamp = nowIso();
  const known: KnownRemoteWorkspace = { ...remoteWorkspace, lastOpenedAt: timestamp };
  knownRemoteWorkspaces = [
    known,
    ...knownRemoteWorkspaces.filter((candidate) => candidate.uri !== remoteWorkspace.uri),
  ];
  return known;
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
  const messageMatch = getSessionMessages(session.id).find((message) => searchText(message.content, query));
  if (messageMatch) {
    matches.push({
      kind: "message",
      excerpt: messageMatch.content.slice(0, 160),
      messageId: messageMatch.id,
    });
  }
  return matches.length > 0 ? { session: { ...session }, matches } : null;
}

function findCategory(categoryId: string) {
  const category = sessionCategories.find((candidate) => candidate.id === categoryId);
  if (!category) {
    throw new Error(`Category not found: ${categoryId}`);
  }
  return category;
}

function validateCategoryName(name: string, exceptId?: string) {
  const trimmed = name.trim();
  if (!trimmed) throw new Error("Category name cannot be empty.");
  const duplicate = sessionCategories.some((category) => category.name === trimmed && category.id !== exceptId);
  if (duplicate) throw new Error("Category name already exists.");
  return trimmed;
}

function serializeWebSessionExport(input: ExportSessionInput): SessionExportResult {
  const session = findWebSession(input.sessionId);
  const payload = {
    version: 1,
    exportedAt: nowIso(),
    session,
    messages: getSessionMessages(session.id),
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

function aggregateWebUsageStatistics(range: UsageStatisticsRange): UsageStatistics {
  return aggregateUsageRecords(representativeUsageRecords, range);
}

function createMessageId() {
  const id = `web-message-${nextMessageId}`;
  nextMessageId += 1;
  return id;
}

function getSessionMessages(sessionId: string) {
  return messagesBySession.get(sessionId) ?? [];
}

function setSessionMessages(sessionId: string, nextMessages: ChatMessage[]) {
  messagesBySession.set(sessionId, nextMessages);
}

function upsertMessage(message: ChatMessage) {
  const messages = getSessionMessages(message.sessionId);
  const index = messages.findIndex((candidate) => candidate.id === message.id);
  if (index === -1) {
    setSessionMessages(message.sessionId, [...messages, message]);
    return;
  }
  const nextMessages = [...messages];
  nextMessages[index] = message;
  setSessionMessages(message.sessionId, nextMessages);
}

function emitChatEvent(event: ChatStreamEvent) {
  const subscribers = subscribersBySession.get(event.sessionId);
  subscribers?.forEach((handler) => handler(event));
}

function finishWebGeneration(sessionId: string, lifecycleState: Session["lifecycleState"]) {
  const session = findWebSession(sessionId);
  const runId = session.activeExecutionRunId;
  if (runId) {
    const run = webAgentRuns.find((candidate) => candidate.id === runId);
    if (run && !terminalRunStates.has(run.state)) {
      updateWebAgentRun(
        run.id,
        run.version,
        lifecycleState === "idle" ? "completed" : lifecycleState === "failed" ? "failed" : "cancelled",
      );
    }
  }
  updateWebSession(sessionId, {
    lifecycleState,
    activeExecutionRunId: null,
    stateRevision: session.stateRevision + 1,
  });
}

function applyStreamEvent(event: ChatStreamEvent) {
  // The turn status belongs to the session, not to any message.
  if (event.type === "turn_status") return;
  const messages = getSessionMessages(event.sessionId);
  const message = messages.find((candidate) => candidate.id === event.messageId);
  if (!message) return;
  const timestamp = nowIso();
  if (event.type === "token") {
    upsertMessage({ ...message, content: `${message.content}${event.contentDelta}`, updatedAt: timestamp });
  } else if (event.type === "thinking") {
    upsertMessage({
      ...message,
      thinkingContent: `${message.thinkingContent ?? ""}${event.contentDelta}`,
      updatedAt: timestamp,
    });
  } else if (event.type === "tool_use") {
    upsertMessage({ ...message, toolUse: upsertToolUse(message.toolUse ?? [], event.toolUse), updatedAt: timestamp });
  } else if (event.type === "rich_block") {
    const blocks = message.richBlocks ?? [];
    const blockIndex = blocks.findIndex((block) => block.id === event.block.id);
    const richBlocks =
      blockIndex === -1
        ? [...blocks, event.block]
        : blocks.map((block, index) => (index === blockIndex ? event.block : block));
    upsertMessage({ ...message, richBlocks, updatedAt: timestamp });
  } else if (event.type === "completed") {
    upsertMessage({ ...message, status: "completed", tokenUsage: event.tokenUsage, updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "idle");
  } else if (event.type === "failed") {
    upsertMessage({ ...message, status: "failed", error: event.error, updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "failed");
  } else if (event.type === "cancelled") {
    upsertMessage({ ...message, status: "cancelled", updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
    finishWebGeneration(event.sessionId, "stopped");
  }
}

function publishChatEvent(event: ChatStreamEvent) {
  applyStreamEvent(event);
  emitChatEvent(event);
}

/**
 * Resolves a simulated tool call awaiting approval and publishes the resulting `tool_use` event
 * — the same behavior `resolveToolApproval` used to provide directly on this client, extracted
 * to a plain export so `web-permissions-client.ts` can call it (`permissions-approval`'s
 * `resolvePendingApproval` is the new frontend entry point; this is its Web/mock backing).
 */
export function resolveWebMockToolApproval(sessionId: string, callId: string, approved: boolean): boolean {
  findWebSession(sessionId);
  const pending = webPendingApprovals.get(callId);
  if (!pending || pending.sessionId !== sessionId) return false;
  webPendingApprovals.delete(callId);
  publishChatEvent({
    type: "tool_use",
    sessionId,
    messageId: pending.messageId,
    toolUse: {
      id: callId,
      name: pending.toolName,
      input: pending.input ?? { command: "echo mock" },
      output: approved ? pending.output ?? "mock\n" : "Denied by user.",
      status: approved ? "completed" : "failed",
    },
  });
  return true;
}

/**
 * Web/mock backing for `resolveAgentQuestion`. Unlike the desktop runtime there is no blocked
 * generation to resume, so "delivered" means only that a tool block in this session was still
 * showing `awaiting_input` — the mock reports the round trip as simulated rather than implying a
 * real wait ended.
 */
/**
 * Marker that makes the Web/mock runtime simulate a clarification round trip. Web/mock has no
 * model deciding when to ask, so the trigger stands in for that decision.
 */
export const WEB_MOCK_QUESTION_TRIGGER = "[ask-me]";

/**
 * Marker that makes the Web/mock runtime simulate a request to leave plan mode. Same reason as the
 * question trigger: the request blocks until decided, so emitting one every turn would leave every
 * other mock conversation waiting on a card.
 */
export const WEB_MOCK_PLAN_EXIT_TRIGGER = "[plan-done]";

function resolveSimulatedQuestion(sessionId: string, callId: string, answer: string): boolean {
  findWebSession(sessionId);
  const message = getSessionMessages(sessionId).find((entry) =>
    entry.toolUse?.some((tool) => tool.id === callId && tool.status === "awaiting_input"),
  );
  const pending = message?.toolUse?.find((tool) => tool.id === callId);
  if (!message || !pending) return false;
  publishChatEvent({
    type: "tool_use",
    sessionId,
    messageId: message.id,
    toolUse: { ...pending, output: answer, status: "completed" },
  });
  return true;
}

/**
 * Web/mock backing for `resolvePlanExit`. Same simulation as an answer: nothing is blocked, so
 * "delivered" means a matching tool block was still showing `awaiting_input`. The recorded output
 * differs by decision so the mock cannot make a decline look like an approval.
 */
function resolveSimulatedPlanExit(sessionId: string, callId: string, approved: boolean): boolean {
  findWebSession(sessionId);
  const message = getSessionMessages(sessionId).find((entry) =>
    entry.toolUse?.some((tool) => tool.id === callId && tool.status === "awaiting_input"),
  );
  const pending = message?.toolUse?.find((tool) => tool.id === callId);
  if (!message || !pending) return false;
  publishChatEvent({
    type: "tool_use",
    sessionId,
    messageId: message.id,
    toolUse: {
      ...pending,
      output: approved
        ? "The user approved your plan and this session has left plan mode."
        : "The user did not approve this plan. The session is still in plan mode.",
      status: approved ? "completed" : "failed",
    },
  });
  return true;
}

function emitTerminalEvent(event: AgentTerminalEvent, recordOutput = true) {
  if (recordOutput && event.type === "output") {
    terminalTranscriptsBySession.set(event.sessionId, appendTerminalTranscript(
      terminalTranscriptsBySession.get(event.sessionId) ?? "",
      event.content,
    ));
  }
  const subscribers = terminalSubscribersBySession.get(event.sessionId);
  subscribers?.forEach((handler) => handler(event));
}

function appendTerminalTranscript(current: string, content: string) {
  let transcript = `${current}${content}`;
  if (transcript.length <= webRetainedTerminalTranscriptBytes) {
    return transcript;
  }
  transcript = transcript.slice(transcript.length - webRetainedTerminalTranscriptBytes);
  return transcript;
}

function upsertTerminalSession(session: AgentTerminalSession) {
  terminalsBySession.set(session.sessionId, session);
}

function cancelActiveStream(sessionId: string) {
  const activeStream = activeStreams.get(sessionId);
  if (!activeStream) return false;
  activeStream.timeoutIds.forEach((timeoutId) => clearTimeout(timeoutId));
  activeStreams.delete(sessionId);
  publishChatEvent({ type: "cancelled", sessionId, messageId: activeStream.messageId });
  return true;
}

function cloneLoopValue<T>(value: T): T {
  return structuredClone(value);
}

function validateLoopDefinitionInput(input: SaveLoopDefinitionInput) {
  const name = input.name.trim();
  const projectPath = input.projectPath.trim();
  const baseBranch = input.baseBranch.trim();
  const goal = input.goal.trim();
  if (!name || !projectPath || !baseBranch || !goal) throw new Error(tr("loops.editor.error.scope"));
  if (!mockAgents.some((agent) => agent.id === input.workerAgentId)) throw new Error(tr("loops.web.error.unsupportedWorker", { agentId: input.workerAgentId }));
  if (!mockAgents.some((agent) => agent.id === input.verifierAgentId)) throw new Error(tr("loops.web.error.unsupportedVerifier", { agentId: input.verifierAgentId }));
  if (input.acceptanceCriteria.every((criterion) => !criterion.trim())) throw new Error(tr("loops.editor.error.acceptance"));
  if (input.verificationCommands.length === 0) throw new Error(tr("loops.editor.error.verificationRequired"));
  for (const command of input.verificationCommands) {
    if (!command.id.trim() || !command.program.trim() || command.timeoutSeconds < 1) throw new Error(tr("loops.web.error.invalidCommand"));
    const workingDirectory = command.workingDirectory?.trim() ?? null;
    if (workingDirectory && (/^(?:[a-zA-Z]:[\\/]|[\\/])/.test(workingDirectory) || workingDirectory.split(/[\\/]+/).includes(".."))) {
      throw new Error(tr("loops.editor.error.verificationDirectory"));
    }
  }
  const { limits } = input;
  if (
    limits.maxIterations < 1 || limits.maxIterations > 20 ||
    limits.stepTimeoutSeconds < 1 || limits.totalTimeoutSeconds < limits.stepTimeoutSeconds ||
    limits.maxConsecutiveRuntimeErrors < 1 || limits.maxConsecutiveNoProgress < 1
  ) throw new Error(tr("loops.editor.error.limits"));
  return {
    ...input,
    name,
    projectPath,
    baseBranch,
    goal,
    acceptanceCriteria: input.acceptanceCriteria.map((value) => value.trim()).filter(Boolean),
    allowedPaths: input.allowedPaths.map((value) => value.trim()).filter(Boolean),
    protectedPaths: input.protectedPaths.map((value) => value.trim()).filter(Boolean),
    verificationCommands: input.verificationCommands.map((command) => ({
      ...command,
      id: command.id.trim(),
      program: command.program.trim(),
      args: command.args.map((value) => value.trim()).filter(Boolean),
      workingDirectory: command.workingDirectory?.trim() || null,
    })),
    limits: { ...input.limits },
  };
}

function findLoopDefinition(definitionId: string) {
  const definition = loopDefinitions.find((candidate) => candidate.id === definitionId);
  if (!definition) throw new Error(tr("loops.web.error.definitionNotFound", { definitionId }));
  return definition;
}

function findLoopRun(runId: string) {
  const run = loopRuns.find((candidate) => candidate.id === runId);
  if (!run) throw new Error(tr("loops.web.error.runNotFound", { runId }));
  return run;
}

function emitLoopEvent(run: LoopRun, kind: LoopEvent["kind"] = "run-updated") {
  run.updatedAt = nowIso();
  const event: LoopEvent = { kind, run: cloneLoopValue(run) };
  loopSubscribers.get(run.id)?.forEach((handler) => handler(event));
}

function addLoopEvidence(
  run: LoopRun,
  iteration: LoopIteration | null,
  input: Omit<LoopEvidence, "id" | "runId" | "iterationId" | "createdAt">,
) {
  const evidence: LoopEvidence = {
    ...input,
    id: `web-loop-evidence-${nextLoopEvidenceId++}`,
    runId: run.id,
    iterationId: iteration?.id ?? null,
    createdAt: nowIso(),
  };
  if (iteration) iteration.evidence.push(evidence);
  emitLoopEvent(run, "evidence-added");
}

/** `add-cli-memory-support`: the shared memory pool is no longer isolated per agent id, so tests
 * that seed memories can leak into later tests within the same file unless explicitly cleared. */
export function resetWebAgentMemoriesForTest() {
  webAgentMemories = [];
}

export function resetWebLoopsForTest() {
  loopTimers.forEach((timer) => clearTimeout(timer));
  loopTimers.clear();
  loopSubscribers.clear();
  replaceWebSessions(listWebSessions().filter((session) => !loopRoleSessionIds.has(session.id)));
  loopRoleSessionIds.forEach((sessionId) => messagesBySession.delete(sessionId));
  loopRoleSessionIds.clear();
  loopDefinitions = [];
  loopRuns = [];
  nextLoopDefinitionId = 1;
  nextLoopRunId = 1;
  nextLoopEvidenceId = 1;
}

export function simulateWebLoopRestartForTest(runId: string): LoopRun {
  const run = findLoopRun(runId);
  if (!["queued", "running", "awaiting-acceptance"].includes(run.status)) {
    throw new Error(tr("loops.web.error.recoveryState"));
  }
  const timer = loopTimers.get(run.id);
  if (timer) clearTimeout(timer);
  loopTimers.delete(run.id);
  run.status = "paused";
  run.terminalReason = "recovery-required";
  run.pauseRequested = false;
  run.activeOperationId = null;
  emitLoopEvent(run);
  return cloneLoopValue(run);
}

function currentLoopIteration(run: LoopRun) {
  const iteration = run.iterations.at(-1);
  if (!iteration) throw new Error(tr("loops.web.error.iterationNotFound", { runId: run.id }));
  return iteration;
}

function createWebLoopIteration(runId: string, sequence: number, feedback: string | null): LoopIteration {
  return {
    id: `web-loop-iteration-${runId}-${sequence}`,
    runId,
    sequence,
    status: "running",
    workerSessionId: `web-loop-worker-${runId}-${sequence}`,
    verifierSessionId: null,
    workerSummary: null,
    verifierRecommendation: null,
    verifierFindings: [],
    decisionReason: null,
    diffFingerprint: null,
    checkFailureFingerprint: null,
    userFeedback: feedback,
    evidence: [],
    startedAt: nowIso(),
    completedAt: null,
  };
}

function createWebLoopRoleSession(run: LoopRun, iteration: LoopIteration, role: "worker" | "verifier") {
  const sessionId = role === "worker" ? iteration.workerSessionId : iteration.verifierSessionId;
  if (!sessionId || loopRoleSessionIds.has(sessionId)) return;
  const timestamp = nowIso();
  const agentId = role === "worker"
    ? run.definitionSnapshot.workerAgentId
    : run.definitionSnapshot.verifierAgentId;
  const session: Session = {
    id: sessionId,
    title: `${run.definitionSnapshot.name} - ${tr(`loops.inspection.role.${role}`)}`,
    agentId,
    interactionMode: "cli",
    lifecycleState: "stopped",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: run.worktreePath,
    projectPath: run.projectPath,
    worktreePath: run.worktreePath,
    worktreeName: run.worktreeName,
    worktreeBranch: run.worktreeBranch,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    source: { kind: "desktop", connector: null },
    pinned: false,
    archived: false,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
  loopRoleSessionIds.add(sessionId);
  prependWebSession(session);
}

function scheduleWebLoopPhase(run: LoopRun) {
  const existing = loopTimers.get(run.id);
  if (existing) clearTimeout(existing);
  const timeoutId = setTimeout(() => {
    loopTimers.delete(run.id);
    if (run.status !== "queued" && run.status !== "running") return;
    if (run.pauseRequested) {
      run.pauseRequested = false;
      run.status = "paused";
      projectWebOwnerRun(run.id, "paused");
      emitLoopEvent(run);
      return;
    }

    if (run.status === "queued") {
      run.status = "running";
      run.startedAt = nowIso();
      run.worktreeName = `loop-${run.definitionId}-${run.id}`;
      run.worktreeBranch = `vanehub/${run.worktreeName}`;
      run.worktreePath = `${run.projectPath}-${run.worktreeName}`;
      run.phase = "acting";
      projectWebOwnerRun(run.id, "running");
      const iteration = createWebLoopIteration(run.id, 1, null);
      run.iterations.push(iteration);
      createWebLoopRoleSession(run, iteration, "worker");
      addLoopEvidence(run, null, {
        kind: "worktree",
        status: "passed",
        summary: tr("loops.web.evidence.worktreePrepared"),
        operationId: run.activeOperationId,
        commandId: null,
        exitCode: 0,
        durationMs: 180,
        details: { simulated: true, path: run.worktreePath },
      });
      scheduleWebLoopPhase(run);
      return;
    }

    const iteration = currentLoopIteration(run);
    if (run.phase === "acting") {
      iteration.workerSummary = tr("loops.web.evidence.workerCompleted");
      iteration.diffFingerprint = `mock-diff-${run.id}-${iteration.sequence}`;
      addLoopEvidence(run, iteration, {
        kind: "worker",
        status: "passed",
        summary: iteration.workerSummary,
        operationId: `web-loop-worker-operation-${run.id}-${iteration.sequence}`,
        commandId: null,
        exitCode: 0,
        durationMs: 420,
        details: { simulated: true, changedFiles: 3, additions: 48, deletions: 12 },
      });
      run.phase = "verifying";
      projectWebOwnerRun(run.id, "verifying");
      emitLoopEvent(run, "iteration-updated");
      scheduleWebLoopPhase(run);
      return;
    }

    if (run.phase === "verifying") {
      run.definitionSnapshot.verificationCommands.forEach((command) => {
        const failed = command.program.toLowerCase() === "false";
        addLoopEvidence(run, iteration, {
          kind: "verification",
          status: failed ? "failed" : "passed",
          summary: `${command.program} ${command.args.join(" ")}`.trim(),
          operationId: `web-loop-check-${run.id}-${iteration.sequence}-${command.id}`,
          commandId: command.id,
          exitCode: failed ? 1 : 0,
          durationMs: 240,
          details: { simulated: true, required: command.required },
        });
      });
      const requiredCheckFailed = iteration.evidence.some(
        (evidence) => evidence.kind === "verification" && evidence.status === "failed" && evidence.details?.required === true,
      );
      iteration.verifierSessionId = `web-loop-verifier-${run.id}-${iteration.sequence}`;
      createWebLoopRoleSession(run, iteration, "verifier");
      iteration.verifierRecommendation = requiredCheckFailed ? "revise" : "pass";
      iteration.verifierFindings = requiredCheckFailed
        ? [tr("loops.web.evidence.requiredCheckFailed")]
        : [tr("loops.web.evidence.checksPassed"), tr("loops.web.evidence.protectedPathsUnchanged")];
      addLoopEvidence(run, iteration, {
        kind: "verifier",
        status: requiredCheckFailed ? "blocked" : "passed",
        summary: requiredCheckFailed
          ? tr("loops.web.evidence.verifierRevise")
          : tr("loops.web.evidence.verifierAccept"),
        operationId: `web-loop-verifier-operation-${run.id}-${iteration.sequence}`,
        commandId: null,
        exitCode: null,
        durationMs: 320,
        details: { simulated: true, recommendation: iteration.verifierRecommendation },
      });
      run.phase = "deciding";
      emitLoopEvent(run, "iteration-updated");
      scheduleWebLoopPhase(run);
      return;
    }

    if (run.phase === "deciding") {
      const requiredCheckFailed = iteration.evidence.some(
        (evidence) => evidence.kind === "verification" && evidence.status === "failed" && evidence.details?.required === true,
      );
      iteration.status = requiredCheckFailed ? "failed" : "awaiting-acceptance";
      iteration.decisionReason = requiredCheckFailed
        ? tr("loops.web.evidence.decisionCheckFailed")
        : tr("loops.web.evidence.decisionReady");
      iteration.completedAt = nowIso();
      run.status = requiredCheckFailed ? "failed" : "awaiting-acceptance";
      projectWebOwnerRun(run.id, requiredCheckFailed ? "failed" : "verifying");
      run.phase = "finalizing";
      run.terminalReason = requiredCheckFailed ? "verification-failed" : null;
      run.completedAt = requiredCheckFailed ? nowIso() : null;
      addLoopEvidence(run, iteration, {
        kind: "decision",
        status: requiredCheckFailed ? "failed" : "passed",
        summary: iteration.decisionReason,
        operationId: null,
        commandId: null,
        exitCode: null,
        durationMs: null,
        details: { simulated: true, decision: run.status },
      });
    }
  }, webLoopPhaseDelayMs);
  loopTimers.set(run.id, timeoutId);
}

export function setWebLoopPhaseDelayForTest(delayMs: number): void {
  webLoopPhaseDelayMs = Math.max(1, Math.min(delayMs, 10_000));
}

const webCodeReviewClient = createWebCodeReviewClient(webSessionWorkspaceClient);
const WEB_RUN_TIME = "2026-08-16T00:00:00.000Z";
let webAgentRuns: AgentRun[] = [{
  id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
  owner: { ownerType: "web_demo", ownerId: "web-session-open" },
  links: [{ linkType: "session", linkId: "web-session-open" }],
  parentRunId: null,
  state: "paused",
  recoveryPolicy: "not_recoverable",
  runner: {
    kind: "local", targetId: "local", targetRevision: null, label: "Local", hostLabel: "This device",
    recovery: "none", capabilityWitness: "web-demo-local", authorityWitness: "web-demo-local", recoveryReference: null,
  },
  retryCount: 1,
  maxRetries: 2,
  reasonCode: "web_demo_paused",
  createdAt: WEB_RUN_TIME,
  updatedAt: WEB_RUN_TIME,
  version: 4,
  lastWitness: "web-demo-pause",
}, ...([
  ["waiting_approval", "approval_required"], ["waiting_user", "user_question"],
  ["retrying", "provider_backoff"], ["stuck", "runner_disconnected"],
  ["failed", "runner_interrupted"], ["completed", null], ["running", null],
] as const).map(([state, reasonCode], index): AgentRun => ({
  id: `018f0f17-4d6a-7e20-b41d-66c5271a29${index}`,
  owner: { ownerType: index === 5 ? "evaluation" : "agent", ownerId: `web-owner-${index}` },
  links: [{ linkType: "session", linkId: `web-session-${index}` }, ...(index === 4 ? [{ linkType: "review", linkId: "web-review-1" }] : [])],
  parentRunId: null, state, recoveryPolicy: "owner_reconciles", retryCount: state === "retrying" ? 1 : 0,
  maxRetries: 2, reasonCode, createdAt: `2026-08-16T00:0${index + 1}:00.000Z`,
  updatedAt: `2026-08-16T00:0${index + 1}:30.000Z`, version: 2, lastWitness: `web-${state}`,
  runner: index === 3 || index === 4 || index === 6 ? {
    kind: "ssh", targetId: "web-demo-ssh", targetRevision: 1, label: "Build host", hostLabel: "build.example.test",
    recovery: "inspect_only", capabilityWitness: "web-demo-ssh", authorityWitness: "web-demo-ssh-v1", recoveryReference: null,
  } : undefined,
}))];
const defaultWebAgentRuns = structuredClone(webAgentRuns);
const webAgentRunEvents = new Map<string, AgentRunEvent[]>([[webAgentRuns[0].id, [{
  sequence: 4,
  state: "paused",
  trigger: "pause",
  timestamp: WEB_RUN_TIME,
  reasonCode: "web_demo_paused",
  witness: "web-demo-pause",
}]]]);

export function seedWebMissionControlRunsForTest(count: 100 | 1_000): void {
  const states: AgentRun["state"][] = [
    "running", "waiting_approval", "waiting_user", "retrying", "blocked", "failed", "completed",
  ];
  webAgentRuns = Array.from({ length: count }, (_, index): AgentRun => {
    const state = states[index % states.length];
    const timestamp = new Date(Date.parse(WEB_RUN_TIME) + index * 1_000).toISOString();
    return {
      id: `018f0f17-4d6a-7e20-b41d-${String(index).padStart(12, "0")}`,
      owner: { ownerType: "agent", ownerId: `performance-agent-${index % 10}` },
      links: [{ linkType: "session", linkId: `performance-session-${index}` }],
      parentRunId: null,
      state,
      recoveryPolicy: "owner_reconciles",
      retryCount: state === "retrying" ? 1 : 0,
      maxRetries: 2,
      reasonCode: ["blocked", "failed"].includes(state) ? `performance_${state}` : null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 1,
      lastWitness: `performance-fixture:${index}`,
    };
  });
  webAgentRunEvents.clear();
}

export function resetWebMissionControlRunsForTest(): void {
  webAgentRuns = structuredClone(defaultWebAgentRuns);
  webAgentRunEvents.clear();
  webAgentRunEvents.set(webAgentRuns[0].id, [{
    sequence: 4,
    state: "paused",
    trigger: "pause",
    timestamp: WEB_RUN_TIME,
    reasonCode: "web_demo_paused",
    witness: "web-demo-pause",
  }]);
}

function updateWebAgentRun(runId: string, version: number, state: AgentRun["state"]): AgentRun {
  const current = webAgentRuns.find((run) => run.id === runId);
  if (!current) throw new Error(`run not found: ${runId}`);
  if (["completed", "failed", "cancelled"].includes(current.state)) return current;
  if (current.version !== version) throw new Error("run version conflict");
  const nextVersion = version + 1;
  const updatedAt = `2026-08-16T00:00:${String(nextVersion).padStart(2, "0")}.000Z`;
  const updated = { ...current, state, reasonCode: null, version: nextVersion, updatedAt };
  webAgentRuns = webAgentRuns.map((run) => run.id === runId ? updated : run);
  const events = webAgentRunEvents.get(runId) ?? [];
  events.push({
    sequence: nextVersion,
    state,
    trigger: state === "cancelled" ? "cancel_user" : "resume",
    timestamp: updatedAt,
    reasonCode: null,
    witness: `web-${state}:${runId}:${version}`,
  });
  webAgentRunEvents.set(runId, events);
  return updated;
}

function projectWebOwnerRun(ownerId: string, state: AgentRun["state"]): void {
  const run = webAgentRuns.find((item) => item.owner.ownerId === ownerId);
  if (run && run.state !== state && !["completed", "failed", "cancelled"].includes(run.state)) {
    updateWebAgentRun(run.id, run.version, state);
  }
}

const terminalRunStates = new Set<AgentRun["state"]>(["completed", "failed", "cancelled"]);
const activeRunStates = new Set<AgentRun["state"]>(["created", "preparing", "running", "waiting_approval", "waiting_user", "paused", "retrying", "blocked", "stuck", "verifying"]);

function webMissionSummary(run: AgentRun): MissionControlRunSummary {
  const session = run.links.find((link) => link.linkType === "session");
  const review = run.links.find((link) => link.linkType === "review");
  const attention = run.state === "waiting_approval" ? "approval" : run.state === "waiting_user" ? "user"
    : ["blocked", "stuck"].includes(run.state) ? "stuck" : run.state === "failed" ? "failed" : review ? "review" : null;
  const actions: MissionControlRunSummary["actions"] = ["open"];
  if (!terminalRunStates.has(run.state)) actions.push("cancel");
  if (["paused", "blocked", "stuck"].includes(run.state)) actions.push("resume");
  if (["failed", "stuck"].includes(run.state) && run.retryCount < run.maxRetries) actions.push("retry");
  if (run.state === "waiting_approval") actions.push("approval");
  if (review) actions.push("review");
  if (["completed", "failed"].includes(run.state)) actions.push("verify");
  return {
    runId: run.id, version: run.version, ownerType: run.owner.ownerType, ownerId: run.owner.ownerId,
    agentId: run.owner.ownerType === "agent" ? run.owner.ownerId : null, title: `Run ${run.owner.ownerId}`,
    state: run.state, createdAt: run.createdAt, updatedAt: run.updatedAt,
    endedAt: terminalRunStates.has(run.state) ? run.updatedAt : null, projectId: null, workspace: null,
    phase: run.state, attention, reasonCode: run.reasonCode,
    verification: run.state === "verifying" ? "running" : run.state === "completed" ? "passed" : run.state === "failed" ? "failed" : "unavailable",
    tokens: null, cost: null, actions,
    navigation: review ? { kind: "review", id: review.linkId, sessionId: session?.linkId } : session ? { kind: "session", id: session.linkId } : null,
    runner: run.runner ?? null,
  };
}

function webMissionOverview(query: MissionControlQuery): MissionControlOverview {
  const limit = Math.max(1, Math.min(query.limit ?? 20, 50));
  const offset = query.cursor ? Number(query.cursor) : 0;
  if (!Number.isSafeInteger(offset) || offset < 0) throw new Error("invalid mission control cursor");
  let runs = webAgentRuns.map(webMissionSummary).filter((run) =>
    (!query.states?.length || query.states.includes(run.state))
    && (!query.agentId || run.agentId === query.agentId)
    && (!query.projectId || run.projectId === query.projectId)
    && (!query.runner || run.runner?.kind === query.runner));
  const priority = (run: MissionControlRunSummary) => run.attention ? 0 : 1;
  runs = runs.sort((left, right) => query.sort === "oldest"
    ? left.createdAt.localeCompare(right.createdAt)
    : query.sort === "attention" ? priority(left) - priority(right) || right.createdAt.localeCompare(left.createdAt)
      : right.createdAt.localeCompare(left.createdAt));
  const page = (items: MissionControlRunSummary[]) => ({ items: items.slice(offset, offset + limit), nextCursor: offset + limit < items.length ? String(offset + limit) : null });
  const count = (state: AgentRun["state"]) => webAgentRuns.filter((run) => run.state === state).length;
  return {
    counts: { running: count("running"), waitingApproval: count("waiting_approval"), waitingUser: count("waiting_user"), retrying: count("retrying"), blocked: count("blocked") + count("stuck"), failed: count("failed"), completedRecently: count("completed") },
    attention: page(runs.filter((run) => run.attention)), active: page(runs.filter((run) => activeRunStates.has(run.state))),
    recent: page(runs.filter((run) => terminalRunStates.has(run.state))),
  };
}

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
  async getAgentRun(runId) {
    const run = webAgentRuns.find((item) => item.id === runId);
    if (!run) throw new Error(`run not found: ${runId}`);
    return run;
  },
  async listAgentRuns(offset = 0, limit = 50, filter) {
    const bounded = Math.max(1, Math.min(limit, 100));
    const filtered = webAgentRuns.filter((run) =>
      (!filter?.ownerType || run.owner.ownerType === filter.ownerType)
      && (!filter?.ownerId || run.owner.ownerId === filter.ownerId)
      && (!filter?.parentRunId || run.parentRunId === filter.parentRunId)
      && (!filter?.state || run.state === filter.state));
    return { items: filtered.slice(offset, offset + bounded), offset, limit: bounded };
  },
  async listAgentRunEvents(runId, offset = 0, limit = 50) {
    const bounded = Math.max(1, Math.min(limit, 100));
    return (webAgentRunEvents.get(runId) ?? []).slice(offset, offset + bounded);
  },
  async cancelAgentRun(runId, version) {
    const cancelled = updateWebAgentRun(runId, version, "cancelled");
    for (const child of webAgentRuns.filter((run) => run.parentRunId === runId)) {
      if (!["completed", "failed", "cancelled"].includes(child.state)) {
        updateWebAgentRun(child.id, child.version, "cancelled");
      }
    }
    return cancelled;
  },
  async resumeAgentRun(runId, version) {
    const run = webAgentRuns.find((item) => item.id === runId);
    if (!run || !["paused", "blocked", "stuck"].includes(run.state)) throw new Error("run cannot be resumed");
    return updateWebAgentRun(runId, version, "running");
  },
  async getMissionControlOverview(query = {}) { return structuredClone(webMissionOverview(query)); },
  async getMissionControlRun(runId): Promise<MissionControlRunDetail> {
    const run = webAgentRuns.find((item) => item.id === runId);
    if (!run) throw new Error(`run not found: ${runId}`);
    const linked = new Set(run.links.map((link) => link.linkType));
    const facets: MissionControlRunDetail["facets"] = ["overview", "plan", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"].map((facet) => ({
      facet: facet as MissionControlRunDetail["facets"][number]["facet"],
      state: facet === "overview" || facet === "timeline" || facet === "logs" || linked.has(facet) ? "available" : "unavailable",
    }));
    return structuredClone({ run: webMissionSummary(run), facets });
  },
  async performMissionControlAction(input): Promise<MissionControlActionReceipt> {
    const current = webAgentRuns.find((run) => run.id === input.runId);
    if (!current) throw new Error(`run not found: ${input.runId}`);
    if (current.version !== input.version) throw new Error("run version conflict");
    if (input.action === "cancel") return { run: webMissionSummary(await this.cancelAgentRun(input.runId, input.version)), operationId: null };
    if (input.action === "resume") return { run: webMissionSummary(await this.resumeAgentRun(input.runId, input.version)), operationId: null };
    if (input.action === "retry") {
      if (!["failed", "stuck"].includes(current.state) || current.retryCount >= current.maxRetries) throw new Error("run cannot be retried");
      const retried = { ...current, id: `${current.id}-retry-${current.retryCount + 1}`, state: "retrying" as const, retryCount: current.retryCount + 1, version: 1, updatedAt: "2026-08-16T00:10:00.000Z", parentRunId: current.id, lastWitness: `web-retry:${current.id}` };
      webAgentRuns = [retried, ...webAgentRuns];
      return { run: webMissionSummary(retried), operationId: `web-retry-operation-${retried.id}` };
    }
    if (input.action === "verify") return { run: webMissionSummary(current), operationId: `web-verification-${current.id}` };
    throw new Error("mission control action is unsupported");
  },

  async deleteApiAgent(agentId: string) {
    if (mockAgents.find((agent) => agent.id === agentId)?.agentOrigin === "builtin") {
      throw new Error("Built-in agents cannot be deleted; reset their provider configuration instead.");
    }
    const blocking: string[] = [];
    const sessionCount = listWebSessions().filter((session) => session.agentId === agentId).length;
    if (sessionCount > 0) blocking.push(`${sessionCount} sessions`);
    const memoryCount = webAgentMemories.filter((memory) => memory.agentId === agentId).length;
    if (memoryCount > 0) blocking.push(`${memoryCount} memories`);
    const workerCount = loopDefinitions.filter((definition) => definition.workerAgentId === agentId).length;
    if (workerCount > 0) blocking.push(`${workerCount} Loop definitions as worker`);
    const verifierCount = loopDefinitions.filter((definition) => definition.verifierAgentId === agentId).length;
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

  async listExpertRoles(): Promise<ExpertRole[]> {
    return structuredClone(webExpertRoles);
  },

  async saveExpertRole(input: SaveExpertRoleInput): Promise<ExpertRole> {
    const errors = validateExpertRoleInput(input);
    if (errors.length > 0) throw new Error(errors.join("; "));
    const timestamp = nowIso();
    const existing = input.id ? webExpertRoles.find((role) => role.id === input.id) : undefined;
    if (input.id && !existing) throw new Error(`Expert role not found: ${input.id}`);
    // Built-in roles are read-only; the UI copies them into a user role instead of editing.
    if (existing?.origin === "builtin") throw new Error("Built-in expert roles cannot be edited.");
    const role: ExpertRole = {
      id: existing?.id ?? `web-expert-role-${nextExpertRoleId++}`,
      displayName: input.displayName.trim(),
      avatar: input.avatar,
      color: input.color,
      responsibility: input.responsibility.trim(),
      instruction: input.instruction,
      skillIds: [...input.skillIds],
      reviewPolicy: { ...input.reviewPolicy },
      preferredProviders: [...input.preferredProviders],
      origin: "user",
      createdAt: existing?.createdAt ?? timestamp,
      updatedAt: timestamp,
    };
    webExpertRoles = existing
      ? webExpertRoles.map((candidate) => (candidate.id === role.id ? role : candidate))
      : [...webExpertRoles, role];
    return structuredClone(role);
  },

  async deleteExpertRole(roleId: string): Promise<void> {
    const role = webExpertRoles.find((candidate) => candidate.id === roleId);
    if (!role) throw new Error(`Expert role not found: ${roleId}`);
    if (role.origin === "builtin") throw new Error("Built-in expert roles cannot be deleted.");
    webExpertRoles = webExpertRoles.filter((candidate) => candidate.id !== roleId);
  },

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
    return sortWebSessions(listWebSessions().filter((session) => !session.archived && !loopRoleSessionIds.has(session.id)));
  },

  async listArchivedSessions() {
    return sortWebSessions(listWebSessions().filter((session) => session.archived && !loopRoleSessionIds.has(session.id)));
  },

  async searchSessions(input: SessionSearchInput) {
    const query = input.query.trim();
    if (!query) return [];
    return sortWebSessions(listWebSessions().filter((session) => !loopRoleSessionIds.has(session.id)))
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

  async listSessionCategories() {
    return [...sessionCategories].sort((left, right) => left.sortOrder - right.sortOrder || left.name.localeCompare(right.name));
  },

  async createSessionCategory(input: CreateSessionCategoryInput) {
    const timestamp = nowIso();
    const category: SessionCategory = {
      id: `web-category-${nextSessionCategoryId++}`,
      name: validateCategoryName(input.name),
      sortOrder: sessionCategories.length,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    sessionCategories = [...sessionCategories, category];
    return category;
  },

  async renameSessionCategory(input: RenameSessionCategoryInput) {
    const category = findCategory(input.categoryId);
    const timestamp = nowIso();
    const updated = { ...category, name: validateCategoryName(input.name, input.categoryId), updatedAt: timestamp };
    sessionCategories = sessionCategories.map((candidate) => (candidate.id === input.categoryId ? updated : candidate));
    return updated;
  },

  async deleteSessionCategory(categoryId: string) {
    findCategory(categoryId);
    sessionCategories = sessionCategories.filter((category) => category.id !== categoryId);
    replaceWebSessions(listWebSessions().map((session) => (session.categoryId === categoryId ? { ...session, categoryId: null, updatedAt: nowIso() } : session)));
  },

  async assignSessionCategory(input: AssignSessionCategoryInput) {
    if (input.categoryId) findCategory(input.categoryId);
    return updateWebSession(input.sessionId, { categoryId: input.categoryId });
  },

  async listLoopDefinitions() {
    return cloneLoopValue([...loopDefinitions].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt)));
  },

  async createLoopDefinition(input: SaveLoopDefinitionInput) {
    const validated = validateLoopDefinitionInput(input);
    const timestamp = nowIso();
    const definition: LoopDefinition = {
      ...validated,
      id: `web-loop-${nextLoopDefinitionId++}`,
      version: 1,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    loopDefinitions = [definition, ...loopDefinitions];
    return cloneLoopValue(definition);
  },

  async updateLoopDefinition(definitionId: string, input: SaveLoopDefinitionInput) {
    const current = findLoopDefinition(definitionId);
    if (input.expectedVersion != null && input.expectedVersion !== current.version) throw new Error(tr("loops.web.error.versionConflict"));
    const validated = validateLoopDefinitionInput(input);
    const updated: LoopDefinition = {
      ...validated,
      id: current.id,
      version: current.version + 1,
      createdAt: current.createdAt,
      updatedAt: nowIso(),
    };
    loopDefinitions = loopDefinitions.map((candidate) => candidate.id === definitionId ? updated : candidate);
    return cloneLoopValue(updated);
  },

  async deleteLoopDefinition(definitionId: string) {
    findLoopDefinition(definitionId);
    if (loopRuns.some((run) => run.definitionId === definitionId && ["queued", "running", "paused", "awaiting-acceptance"].includes(run.status))) {
      throw new Error(tr("loops.web.error.activeRunDelete"));
    }
    loopDefinitions = loopDefinitions.filter((candidate) => candidate.id !== definitionId);
  },

  async listLoopRuns(definitionId?: string) {
    const runs = definitionId ? loopRuns.filter((run) => run.definitionId === definitionId) : loopRuns;
    return cloneLoopValue([...runs].sort((left, right) => right.createdAt.localeCompare(left.createdAt)));
  },

  async getLoopRun(runId: string) {
    return cloneLoopValue(findLoopRun(runId));
  },

  async startLoop(definitionId: string): Promise<StartLoopResult> {
    const definition = findLoopDefinition(definitionId);
    if (!definition.enabled) throw new Error(tr("loops.web.error.definitionDisabled"));
    if (loopRuns.some((run) => run.definitionId === definitionId && ["queued", "running", "paused", "awaiting-acceptance"].includes(run.status))) {
      throw new Error(tr("loops.web.error.activeRunExists"));
    }
    const timestamp = nowIso();
    const runId = `web-loop-run-${nextLoopRunId++}`;
    const operationId = `web-loop-prepare-${runId}`;
    const run: LoopRun = {
      id: runId,
      definitionId,
      definitionSnapshot: cloneLoopValue(definition),
      status: "queued",
      phase: "preparing",
      terminalReason: null,
      currentIteration: 1,
      consecutiveRuntimeErrors: 0,
      consecutiveNoProgress: 0,
      pauseRequested: false,
      projectPath: definition.projectPath,
      worktreePath: null,
      worktreeName: null,
      worktreeBranch: null,
      activeOperationId: operationId,
      iterations: [],
      simulated: true,
      createdAt: timestamp,
      startedAt: null,
      updatedAt: timestamp,
      completedAt: null,
    };
    loopRuns = [run, ...loopRuns];
    const canonicalId = `018f0f17-4d6a-7e20-b41d-66c5271a${String(nextLoopRunId).padStart(4, "0")}`;
    webAgentRuns = [{
      id: canonicalId,
      owner: { ownerType: "loop_run", ownerId: runId },
      links: [{ linkType: "loop_definition", linkId: definitionId }],
      parentRunId: null,
      state: "preparing",
      recoveryPolicy: "owner_reconciles",
      retryCount: 0,
      maxRetries: 3,
      reasonCode: null,
      createdAt: timestamp,
      updatedAt: timestamp,
      version: 2,
      lastWitness: `web-loop-prepare:${runId}`,
    }, ...webAgentRuns];
    webAgentRunEvents.set(canonicalId, []);
    emitLoopEvent(run);
    scheduleWebLoopPhase(run);
    return { run: cloneLoopValue(run), operationId };
  },

  async pauseLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "queued" && run.status !== "running") throw new Error(tr("loops.web.error.pauseState"));
    run.pauseRequested = true;
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async resumeLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "paused") throw new Error(tr("loops.web.error.resumeState"));
    run.status = run.iterations.length === 0 ? "queued" : "running";
    projectWebOwnerRun(run.id, "running");
    run.terminalReason = null;
    run.pauseRequested = false;
    emitLoopEvent(run);
    scheduleWebLoopPhase(run);
    return cloneLoopValue(run);
  },

  async cancelLoop(runId: string) {
    const run = findLoopRun(runId);
    if (["succeeded", "failed", "cancelled"].includes(run.status)) return cloneLoopValue(run);
    const timer = loopTimers.get(run.id);
    if (timer) clearTimeout(timer);
    loopTimers.delete(run.id);
    run.status = "cancelled";
    projectWebOwnerRun(run.id, "cancelled");
    run.terminalReason = "user-stopped";
    run.completedAt = nowIso();
    run.pauseRequested = false;
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async acceptLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "awaiting-acceptance") throw new Error(tr("loops.web.error.acceptanceState"));
    run.status = "succeeded";
    projectWebOwnerRun(run.id, "completed");
    run.terminalReason = "goal-met";
    run.completedAt = nowIso();
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async continueLoop(input: ContinueLoopInput) {
    const run = findLoopRun(input.runId);
    const feedback = input.feedback.trim();
    if (run.status !== "awaiting-acceptance") throw new Error(tr("loops.web.error.acceptanceState"));
    if (!feedback) throw new Error(tr("loops.web.error.feedbackRequired"));
    if (run.currentIteration >= run.definitionSnapshot.limits.maxIterations) throw new Error(tr("loops.web.error.maxIterations"));
    run.currentIteration += 1;
    const iteration = createWebLoopIteration(run.id, run.currentIteration, feedback);
    run.iterations.push(iteration);
    createWebLoopRoleSession(run, iteration, "worker");
    run.status = "running";
    projectWebOwnerRun(run.id, "running");
    run.phase = "acting";
    run.terminalReason = null;
    emitLoopEvent(run, "iteration-updated");
    scheduleWebLoopPhase(run);
    return cloneLoopValue(run);
  },

  async rejectLoop(runId: string) {
    const run = findLoopRun(runId);
    if (run.status !== "awaiting-acceptance") throw new Error(tr("loops.web.error.acceptanceState"));
    run.status = "cancelled";
    projectWebOwnerRun(run.id, "cancelled");
    run.terminalReason = "user-rejected";
    run.completedAt = nowIso();
    emitLoopEvent(run);
    return cloneLoopValue(run);
  },

  async subscribeLoopEvents(runId: string, handler: (event: LoopEvent) => void) {
    const subscribers = loopSubscribers.get(runId) ?? new Set<(event: LoopEvent) => void>();
    subscribers.add(handler);
    loopSubscribers.set(runId, subscribers);
    return () => {
      subscribers.delete(handler);
      if (subscribers.size === 0) loopSubscribers.delete(runId);
    };
  },

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

  async listKnownProjects() {
    return knownProjects.map((project) => ({ ...project }));
  },

  async listKnownRemoteWorkspaces() {
    return knownRemoteWorkspaces.map((workspace) => ({ ...workspace }));
  },

  async inspectProject(path: string) {
    if (!path.trim()) {
      throw new Error(tr("web.error.projectPathRequired"));
    }
    return inspectMockProject(path);
  },

  async selectProjectDirectory() {
    return "D:\\\\example-workspace";
  },

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
        ...snapshotSeat(seat, mockAgents, webExpertRoles),
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
    cancelActiveStream(sessionId);
    messagesBySession.delete(sessionId);
    recoveryReportsBySession.delete(sessionId);
    subscribersBySession.delete(sessionId);
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
    const cancelled = cancelActiveStream(sessionId);
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
    if (activeStreams.has(input.sessionId)) {
      throw new Error("A generation is already active for this session.");
    }
    const selectedRunner = selectWebRunner(input.sessionId, session.agentId, input.runner);
    const timestamp = nowIso();
    const activeSeats = (session.seats ?? []).filter((seat) => seat.leftAt == null);
    const firstSpeakerSeatId = activeSeats.length > 1 ? activeSeats[0]?.seatId : undefined;
    const existingMessages = getSessionMessages(input.sessionId);
    const nextSequence = existingMessages.reduce(
      (maximum, message) => Math.max(maximum, message.sessionSequence),
      0,
    ) + 1;
    const executionRunId = `web-run-${input.sessionId}-${Date.now()}`;
    webAgentRuns = [{
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
    }, ...webAgentRuns];
    webAgentRunEvents.set(executionRunId, []);
    const userMessage: ChatMessage = {
      id: createMessageId(),
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
      id: createMessageId(),
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
    setSessionMessages(input.sessionId, [...existingMessages, userMessage, assistantMessage]);
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
      emitChatEvent({ type: "started", sessionId: input.sessionId, messageId: assistantMessage.id });
    }, 80);
    timeoutIds.push(startTimeoutId);
    const historyCharacterCount = getSessionMessages(input.sessionId).reduce(
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
        publishChatEvent({
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
          publishChatEvent({
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
        publishChatEvent({
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
        publishChatEvent({
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
        publishChatEvent({
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
        publishChatEvent({ type: "token", sessionId: input.sessionId, messageId: assistantMessage.id, contentDelta });
      }, 240 + index * 90);
      timeoutIds.push(timeoutId);
    });
    if (config.thinking) {
      const thinkingTimeoutId = setTimeout(() => {
        publishChatEvent({
          type: "thinking",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          contentDelta: "Mock thinking: checking session context and selected config.",
        });
      }, 180);
      timeoutIds.push(thinkingTimeoutId);
    }
    const toolUseTimeoutId = setTimeout(() => {
      publishChatEvent({
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
          publishChatEvent({
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
        publishChatEvent({
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
          publishChatEvent({
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
          publishChatEvent({
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
        publishChatEvent({
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
          publishChatEvent({
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
          publishChatEvent({
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
        publishChatEvent({
          type: "tool_use",
          sessionId: input.sessionId,
          messageId: assistantMessage.id,
          toolUse: mcpSimulation.awaitingApproval,
        });
      }, 237);
      timeoutIds.push(mcpApprovalTimeoutId);
    }
    const richCardTimeoutId = setTimeout(() => {
      publishChatEvent({
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
      publishChatEvent({
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
      publishChatEvent({
        type: "completed",
        sessionId: input.sessionId,
        messageId: assistantMessage.id,
      });
    }, 320 + tokens.length * 90);
    timeoutIds.push(completeTimeoutId);
    activeStreams.set(input.sessionId, { messageId: assistantMessage.id, timeoutIds });
    return assistantMessage;
  },

  async listMessages(input) {
    findWebSession(input.sessionId);
    const limit = input.limit ?? 50;
    const messages = getSessionMessages(input.sessionId);
    const endIndex = input.beforeId
      ? messages.findIndex((message) => message.id === input.beforeId)
      : messages.length;
    const boundedEndIndex = endIndex === -1 ? messages.length : endIndex;
    return messages.slice(Math.max(0, boundedEndIndex - limit), boundedEndIndex);
  },

  async saveMessageFeedback(input) {
    const message = Array.from(messagesBySession.values())
      .flat()
      .find((candidate) => candidate.id === input.messageId);
    if (!message || message.role !== "assistant" || message.status !== "completed") {
      throw new Error("message-not-eligible");
    }
    const currentRevision = message.feedback?.revision ?? 0;
    if (currentRevision !== input.expectedRevision) {
      throw new Error(`feedback-conflict:${currentRevision}`);
    }
    if (input.state === "corrected" && !input.correctionNote?.trim()) {
      throw new Error("invalid-feedback");
    }
    if (input.state === null) {
      message.feedback = { state: null, revision: currentRevision + 1 };
      return message.feedback;
    }
    message.feedback = {
      state: input.state,
      revision: currentRevision + 1,
      ...(input.correctionNote?.trim()
        ? { correctionNote: input.correctionNote.trim().slice(0, 1_000) }
        : {}),
    };
    return message.feedback;
  },

  async getUsageStatistics(input) {
    return aggregateWebUsageStatistics(input.range);
  },

  async getSessionUsageSummary(sessionId: string) {
    findWebSession(sessionId);
    const generated = aggregateSessionUsageRecords(representativeUsageRecords, sessionId);
    return generated;
  },

  async getTokenUsageSummary(input) {
    if (!input.sessionId) return queryWebTokenUsageSummary(input);
    const session = findWebSession(input.sessionId);
    return queryWebTokenUsageSummary({
      ...input,
      sessionId: session.agentId === "onepiece" ? "web-token-onepiece" : "web-token-cli",
    });
  },

  async getTokenUsageDetails(input) {
    if (!input.sessionId) return queryWebTokenUsageDetails(input);
    const session = findWebSession(input.sessionId);
    return queryWebTokenUsageDetails({
      ...input,
      sessionId: session.agentId === "onepiece" ? "web-token-onepiece" : "web-token-cli",
    });
  },

  /**
   * The Web runtime simulates the round trip: nothing is actually blocked on the answer, so this
   * reports delivery only when a matching tool block is still showing `awaiting_input` and marks
   * it completed with the answer, rather than claiming a real generation resumed.
   */
  async resolveAgentQuestion(sessionId: string, callId: string, answer: string) {
    return resolveSimulatedQuestion(sessionId, callId, answer);
  },

  async resolvePlanExit(sessionId: string, callId: string, approved: boolean) {
    return resolveSimulatedPlanExit(sessionId, callId, approved);
  },

  async stopGeneration(sessionId: string) {
    findWebSession(sessionId);
    if (!cancelActiveStream(sessionId)) return;
    updateWebSession(sessionId, { lifecycleState: "stopped" });
  },

  async openAgentTerminal(sessionId: string, size: AgentTerminalSize) {
    const session = findWebSession(sessionId);
    const existing = terminalsBySession.get(sessionId);
    if (existing?.state === "running") {
      const transcript = terminalTranscriptsBySession.get(sessionId) ?? "";
      if (transcript) {
        setTimeout(() => {
          emitTerminalEvent(
            {
              type: "output",
              terminalId: existing.terminalId,
              sessionId,
              content: transcript,
            },
            false,
          );
        }, 0);
      }
      return existing;
    }
    const runtimeSessionId = session.runtimeSessionId ?? `web-runtime-${session.id}`;
    const terminal: AgentTerminalSession = {
      terminalId: `web-terminal-${session.id}`,
      sessionId: session.id,
      agentId: session.agentId,
      state: "running",
      capability: "simulated",
      size,
      runtimeSessionId,
      retained: true,
    };
    upsertTerminalSession(terminal);
    updateWebSession(sessionId, { lifecycleState: "running", runtimeSessionId });
    setTimeout(() => {
      emitTerminalEvent({
        type: "runtime_session_id",
        terminalId: terminal.terminalId,
        sessionId,
        runtimeSessionId,
      });
    }, 30);
    return terminal;
  },

  async sendAgentTerminalInput(terminalId: string, content: string) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) {
      throw new Error("Agent terminal is not connected.");
    }
    emitTerminalEvent({
      type: "output",
      terminalId,
      sessionId: terminal.sessionId,
      content,
    });
  },

  async resizeAgentTerminal(terminalId: string, size: AgentTerminalSize) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) {
      throw new Error("Agent terminal is not connected.");
    }
    upsertTerminalSession({ ...terminal, size });
  },

  async stopAgentTerminal(terminalId: string) {
    const terminal = [...terminalsBySession.values()].find((candidate) => candidate.terminalId === terminalId);
    if (!terminal) return false;
    terminalsBySession.delete(terminal.sessionId);
    terminalTranscriptsBySession.delete(terminal.sessionId);
    updateWebSession(terminal.sessionId, { lifecycleState: "stopped" });
    emitTerminalEvent({
      type: "state",
      terminalId,
      sessionId: terminal.sessionId,
      state: "stopped",
      error: null,
    });
    return true;
  },

  async subscribeAgentTerminalEvents(sessionId, handler) {
    const subscribers = terminalSubscribersBySession.get(sessionId) ?? new Set<(event: AgentTerminalEvent) => void>();
    subscribers.add(handler);
    terminalSubscribersBySession.set(sessionId, subscribers);
    return () => {
      const currentSubscribers = terminalSubscribersBySession.get(sessionId);
      currentSubscribers?.delete(handler);
      if (currentSubscribers?.size === 0) {
        terminalSubscribersBySession.delete(sessionId);
      }
    };
  },

  async subscribeMessageEvents(sessionId, handler) {
    const subscribers = subscribersBySession.get(sessionId) ?? new Set<(event: ChatStreamEvent) => void>();
    subscribers.add(handler);
    subscribersBySession.set(sessionId, subscribers);
    return () => {
      const currentSubscribers = subscribersBySession.get(sessionId);
      currentSubscribers?.delete(handler);
      if (currentSubscribers?.size === 0) {
        subscribersBySession.delete(sessionId);
      }
    };
  },

  async subscribeSessionEvents(handler) {
    return subscribeWebSessionStateEvents(handler);
  },
  ...webSkillCatalogClient,
  ...webSkillBindingClient,
  ...webSkillOverlayClient,

  async selectWorkspaceDirectory() {
    return "D:\\\\example-workspace";
  },
};
