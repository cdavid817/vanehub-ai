import type {
  AgentService,
  RecoveryDecision,
  SessionRecoveryAcknowledgement,
  SessionRecoveryReport,
  SessionRecoverySummary,
  SessionStateEvent,
} from "./agent-service";
import { mockAgents, mockWorkflowState } from "./mock-agent-data";
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
  AgentRegistryEntry,
  ApiAgentProviderConfig,
  AssignSessionCategoryInput,
  AutomaticArchivalSettings,
  AgentTerminalEvent,
  AgentTerminalSession,
  UpdateApiAgentInput,
  UpdateSessionSeatsInput,
  AgentTerminalSize,
  RegisterApiAgentInput,
  SaveOnePieceProviderConfigInput,
  SaveOnePieceProviderProfileInput,
  CliParameterSelections,
  CliToolStatus,
  CreateSessionCategoryInput,
  CreateSessionInput,
  CreateScheduledTaskInput,
  ExportSessionInput,
  InteractionMode,
  KnownRemoteWorkspace,
  KnownProject,
  EmbeddingModelOption,
  OnePieceProviderConfig,
  OnePieceProviderProfiles,
  ProjectInspection,
  RemoteWorkspace,
  RetrievalConfiguration,
  RetrievalIndexStatus,
  RenameSessionCategoryInput,
  ScheduledTask,
  ScheduledTaskRun,
  SetScheduledTaskEnabledInput,
  Session,
  SessionSeat,
  SessionCategory,
  SessionExportResult,
  SessionSearchInput,
  SessionSearchResult,
  SessionDetails,
  WorkflowState,
  ManagedCliAgentId,
  ImSessionConnector,
} from "../types/agent";
import { managedCliAgentIds } from "../types/agent";
import { getOnePieceProviderPresets, resolveOnePieceProviderPreset } from "../config/onepiece-provider-presets";
import { findWebSshConnection } from "./web-ssh-connection-client";
import { readWebAppSettings } from "./web-settings-client";
import {
  getWebContextQualitySummary,
  listWebContextQualityHistory,
} from "./web-context-quality";
import { requireHttpsExternalUrl } from "./external-url";
import { defaultSessionTitleFromPath, normalizeDisplayPath } from "../lib/session-path";
import { snapshotSeat } from "./seat-presentation";
import type { ChatConfig, ChatMessage, ChatStreamEvent } from "../types/chat";
import type { UsageStatistics, UsageStatisticsRange } from "../types/chat";
import { queryWebTokenUsageDetails, queryWebTokenUsageSummary } from "./web-token-usage";
import type { OperationTask } from "../types/operation";
import type { AgentRun, AgentRunEvent } from "../types/agent-run";
import type { EvaluationArena, EvaluationTask } from "../types/evaluation";
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
import type {
  PromptAssemblyPreviewInput,
  PromptHook,
  PromptHookCategory,
  PromptHookListResult,
  PromptHookMutationInput,
  PromptHookPreview,
  PromptHookPreviewInput,
  PromptHookTraceSummary,
  PromptHookUpdateInput,
  PromptHookDraft,
  PromptHookVariableDefinition,
  PromptHookVersion,
  PromptHookVersionHistory,
  PublishPromptHookInput,
  RollbackPromptHookInput,
  SavePromptHookDraftInput,
} from "../types/prompt-hook";
import {
  deleteWebPromptHookVersionState,
  publishWebPromptHook,
  renderWebPromptHookTemplate,
  rollbackWebPromptHook,
  saveWebPromptHookDraft,
  webPromptHookHistory,
  webPromptHookVariables,
} from "./web-prompt-hook-versions";
import { createWebMockOperation } from "./web-operation-client";
import type { ExpertRole, SaveExpertRoleInput } from "../types/expert-role";
import { builtinExpertRoles } from "../config/builtin-expert-roles";
import { validateExpertRoleInput } from "./expert-role-runtime";
import type {
  Skill,
  SkillAccessRefusalReason,
  SkillAgentMountPath,
  SkillDriftReport,
  SkillImportInput,
  SkillListResult,
  SkillLoadInput,
  SkillLoadOutcome,
  SkillMetadata,
  SkillMountMigrationReport,
  SkillMutationInput,
  SkillOverview,
  SkillPreview,
  SkillResourceReadInput,
  SkillResourceReadOutcome,
  SkillScope,
  SkillScopeInput,
  SkillSource,
  SkillSyncResult,
  SkillUpdateInput,
} from "../types/skill";
import { createWebSkillOverlayRuntime } from "./web-skill-overlay-runtime";
import { overlayError, webOverlayHash } from "./web-skill-overlay-support";
import {
  createCliParameterProfile,
  defaultCliParameterSelections,
  normalizeCliParameterSelections,
} from "./cli-parameter-catalog";
import { aggregateSessionUsageRecords, aggregateUsageRecords, type UsageRecord } from "./usage-statistics";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";
import { webLspClient } from "./web-lsp-client";
import {
  defaultChatConfigForSession,
  normalizeChatConfigForSession,
  withEffectiveExecutionPolicy,
} from "./chat-configuration";
import { computeNextScheduledRun, validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";
import type {
  CliConfigAgentId,
  CliConfigPayload,
  CliConfigProfile,
  CliConfigStatus,
  SaveCliConfigProfileInput,
} from "../types/cli-agent-config";
import { cliConfigAgentIds } from "../types/cli-agent-config";
import { getCliConfigPresets } from "../config/cli-agent-provider-presets";
import { createWebMcpToolSimulationPlan } from "./web-mcp-tool-simulation";
import type {
  CodeIndexAuditEntry,
  CodeIndexAutomaticMode,
  CodeIndexConfigurationInput,
  CodeIndexPhase,
  CodeIndexStatus,
  CodeIndexWorkspace,
} from "../types/code-index";
import { codeIndexLanguages } from "../types/code-index";
import { normalizeCodeIndexConfiguration } from "./code-index-contract";
import { webBuiltinToolClient } from "./web-builtin-tool-client";
import { createWebCodeReviewClient } from "./web-code-review-client";
import { webDesktopUpdateClient } from "./web-desktop-update";

function tr(key: string, values?: Record<string, string | number>) {
  return i18n.t(key, values);
}

function slugify(value: string) {
  return value
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "");
}

function webLocalCliDetectionMessage() {
  return tr("web.error.localCliDetection");
}

function webCliPackageOperationsMessage() {
  return tr("web.error.cliPackageOperations");
}

const webRetainedTerminalTranscriptBytes = 1_000_000;
/** Mirrors the desktop runtime's character-count compaction trigger (see `add-agent-context-compaction`), scaled down for deterministic mock sessions. */
const mockCompactionTriggerCharacters = 2_000;

let workflowState: WorkflowState = { ...mockWorkflowState };
let nextSessionId = 1;
const cliParameterStorageKey = "vanehub.cli-parameter-profiles.v1";
let memoryCliParameterSelections: Partial<Record<ManagedCliAgentId, CliParameterSelections>> = {};
let webCliConfigProfiles: CliConfigProfile[] = [];
const webCliConfigStatuses = new Map<CliConfigAgentId, CliConfigStatus>();

function requireCliConfigAgentId(agentId: string): CliConfigAgentId {
  const matched = cliConfigAgentIds.find((candidate) => candidate === agentId);
  if (!matched) throw new Error(`Unsupported CLI configuration Agent: ${agentId}`);
  return matched;
}

function cloneCliConfigPayload(payload: CliConfigPayload): CliConfigPayload {
  return structuredClone(payload);
}

function cliConfigNeedsCredential(payload: CliConfigPayload): boolean {
  if (payload.kind === "claude-code") return payload.authMode !== "none";
  if (payload.kind === "codex-cli") return payload.authStrategy !== "preserve-official";
  if (payload.kind === "gemini-cli") return payload.authStrategy !== "preserve-official";
  return payload.kind !== "antigravity";
}

function validateWebCliConfigInput(input: SaveCliConfigProfileInput) {
  if (input.payload.kind !== input.agentId) throw new Error("Profile payload does not match the selected Agent.");
  if (!input.name.trim() || input.name.trim().length > 80) throw new Error("Profile name is required.");
  const baseUrl = input.payload.baseUrl;
  let parsed: URL;
  try {
    parsed = new URL(baseUrl);
  } catch {
    throw new Error("Base URL must be an absolute HTTP(S) URL.");
  }
  if (!(["http:", "https:"] as const).some((protocol) => protocol === parsed.protocol) || parsed.username || parsed.password || parsed.hash) {
    throw new Error("Base URL must be HTTP(S) without credentials or fragments.");
  }
  if (input.payload.kind === "claude-code" && !input.payload.model.trim()) throw new Error("Model is required.");
  if (input.payload.kind === "codex-cli" && (!input.payload.providerId.trim() || !input.payload.model.trim())) {
    throw new Error("Provider and model are required.");
  }
  if (input.payload.kind === "gemini-cli" && !input.payload.model.trim()) throw new Error("Model is required.");
  if (input.payload.kind === "opencode") {
    if (!input.payload.providerId.trim() || input.payload.models.length === 0) throw new Error("Provider and models are required.");
    const defaultModel = input.payload.defaultModel;
    if (!input.payload.models.some((model) => model.id === defaultModel)) {
      throw new Error("Default model must exist in the model list.");
    }
  }
  if (input.sourcePresetId && !input.sourcePresetId.startsWith(`${input.agentId}-`)) {
    throw new Error("Preset is incompatible with the selected Agent.");
  }
}

function defaultWebCliConfigStatus(agentId: CliConfigAgentId): CliConfigStatus {
  return {
    agentId,
    appliedProfileId: null,
    driftState: "detached",
    resolvedPaths: [],
    lastAppliedAt: null,
    simulated: true,
    startupSync: {
      agentId,
      state: "unavailable",
      imported: 0,
      updated: 0,
      skipped: 0,
      warnings: [],
      synchronizedAt: null,
      simulated: true,
    },
  };
}

function webCliConfigStatus(agentId: CliConfigAgentId): CliConfigStatus {
  return webCliConfigStatuses.get(agentId) ?? defaultWebCliConfigStatus(agentId);
}

function nextWebCliConfigProfileId(name: string): string {
  const base = slugify(name) || "profile";
  if (!webCliConfigProfiles.some((profile) => profile.id === base)) return base;
  let suffix = 2;
  while (webCliConfigProfiles.some((profile) => profile.id === `${base}-${suffix}`)) suffix += 1;
  return `${base}-${suffix}`;
}

function saveWebCliConfigProfile(input: SaveCliConfigProfileInput): CliConfigProfile {
  validateWebCliConfigInput(input);
  const existing = input.id
    ? webCliConfigProfiles.find((profile) => profile.id === input.id && profile.agentId === input.agentId)
    : undefined;
  if (input.id && !existing) throw new Error("Profile not found.");
  const credentialConfigured = input.removeCredential
    ? false
    : input.credential
      ? true
      : existing?.credentialConfigured ?? false;
  if (!existing && cliConfigNeedsCredential(input.payload) && !credentialConfigured) {
    throw new Error("Credential repair is required.");
  }
  const timestamp = nowIso();
  let status = webCliConfigStatus(input.agentId);
  if (
    existing
    && status.appliedProfileId === existing.id
    && JSON.stringify(existing.payload) !== JSON.stringify(input.payload)
  ) {
    status = { ...status, driftState: "drifted" };
    webCliConfigStatuses.set(input.agentId, status);
  }
  const profile: CliConfigProfile = {
    id: existing?.id ?? nextWebCliConfigProfileId(input.name),
    agentId: input.agentId,
    name: input.name.trim(),
    payloadVersion: 1,
    payload: cloneCliConfigPayload(input.payload),
    sourcePresetId: input.sourcePresetId ?? existing?.sourcePresetId ?? null,
    sourcePresetVersion: input.sourcePresetVersion ?? existing?.sourcePresetVersion ?? null,
    credentialConfigured,
    validationState: cliConfigNeedsCredential(input.payload) && !credentialConfigured ? "needs-credential" : "valid",
    appliedState:
      status.appliedProfileId === existing?.id
        ? status.driftState === "applied"
          ? "applied"
          : "drifted"
        : "saved",
    createdAt: existing?.createdAt ?? timestamp,
    updatedAt: timestamp,
  };
  webCliConfigProfiles = [...webCliConfigProfiles.filter((item) => item.id !== profile.id), profile];
  return structuredClone(profile);
}

function readCliParameterSelections(): Partial<Record<ManagedCliAgentId, CliParameterSelections>> {
  if (typeof localStorage === "undefined") return memoryCliParameterSelections;
  const raw = localStorage.getItem(cliParameterStorageKey);
  if (!raw) return memoryCliParameterSelections;
  try {
    return JSON.parse(raw) as Partial<Record<ManagedCliAgentId, CliParameterSelections>>;
  } catch {
    return memoryCliParameterSelections;
  }
}

function writeCliParameterSelections(value: Partial<Record<ManagedCliAgentId, CliParameterSelections>>) {
  memoryCliParameterSelections = value;
  if (typeof localStorage !== "undefined") localStorage.setItem(cliParameterStorageKey, JSON.stringify(value));
}
let nextMessageId = 1;
let nextSeatId = 1;
let activeSessionId: string | null = null;
let sessions: Session[] = [];
const recoveryReportsBySession = new Map<string, SessionRecoveryReport[]>();
let sessionCategories: SessionCategory[] = [];
let nextSessionCategoryId = 1;
let automaticArchivalSettings: AutomaticArchivalSettings = { enabled: true, inactiveDays: 10 };
let scheduledTasks: ScheduledTask[] = [];
let nextScheduledTaskId = 1;
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
const sessionEventSubscribers = new Set<(event: SessionStateEvent) => void>();
const chatConfigStorageKey = "vanehub.session-chat-config.v1";
let memoryChatConfigs: Record<string, ChatConfig> = {};

function readChatConfigs(): Record<string, ChatConfig> {
  if (typeof localStorage === "undefined") return memoryChatConfigs;
  const raw = localStorage.getItem(chatConfigStorageKey);
  if (!raw) return memoryChatConfigs;
  try {
    return JSON.parse(raw) as Record<string, ChatConfig>;
  } catch {
    return memoryChatConfigs;
  }
}

function writeChatConfigs(configs: Record<string, ChatConfig>) {
  memoryChatConfigs = configs;
  if (typeof localStorage !== "undefined") localStorage.setItem(chatConfigStorageKey, JSON.stringify(configs));
}

function emitSessionEvent(event: SessionStateEvent) {
  sessionEventSubscribers.forEach((handler) => handler(event));
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
  const session = findSession(sessionId);
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
    id: `web-recovery-session-${nextSessionId++}`,
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
  sessions = [session, ...sessions];
  activeSessionId = session.id;
  const recoveryRevision = 1;
  const recovered = updateSession(session.id, {
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
  emitSessionEvent({
    kind: status === "quarantined" ? "recovery-quarantined" : "recovery-action-required",
    sessionId: recovered.id,
    recoveryRevision,
  });
  return recovered;
}

export function resetWebRecoverySessionsForTest() {
  const recoverySessionIds = new Set(recoveryReportsBySession.keys());
  sessions = sessions.filter((session) => !recoverySessionIds.has(session.id));
  recoverySessionIds.forEach((sessionId) => {
    messagesBySession.delete(sessionId);
    activeStreams.delete(sessionId);
  });
  recoveryReportsBySession.clear();
  if (activeSessionId && recoverySessionIds.has(activeSessionId)) activeSessionId = null;
}

export function seedWebImSessionForTest(connector: ImSessionConnector): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-im-session-${nextSessionId++}`,
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
  sessions = [session, ...sessions];
  activeSessionId = session.id;
  return session;
}

const builtinSkillSeeds = [
  {
    id: "tdd-discipline",
    name: "TDD 开发纪律助手",
    description: "引导开发过程遵循测试先行、红绿重构和回归验证纪律。",
    category: "development",
    triggers: ["TDD", "测试先行", "红绿重构"],
  },
  {
    id: "code-review",
    name: "代码审查助手",
    description: "从缺陷、回归风险、可维护性和测试缺口角度审查代码变更。",
    category: "review",
    triggers: ["代码审查", "review"],
  },
  {
    id: "code-security-scan",
    name: "代码安全扫描",
    description: "检查常见安全风险、敏感信息泄漏、命令注入和不安全文件操作。",
    category: "security",
    triggers: ["安全扫描", "security"],
  },
  {
    id: "api-doc-generation",
    name: "API 文档自动生成",
    description: "根据接口、类型和示例生成结构化 API 文档。",
    category: "documentation",
    triggers: ["API 文档", "api docs"],
  },
  {
    id: "unit-test-generation",
    name: "单元测试自动生成",
    description: "为核心函数、边界条件和回归场景生成单元测试。",
    category: "testing",
    triggers: ["单元测试", "unit test"],
  },
  {
    id: "readme-generation",
    name: "README 文档生成",
    description: "生成或改进项目 README，包括安装、使用、开发和验证说明。",
    category: "documentation",
    triggers: ["README", "项目说明"],
  },
];

let webSkillMountPaths: SkillAgentMountPath[] = mockAgents.map((agent) => ({
  agentId: agent.id,
  mountPath:
    agent.id === "claude-code"
      ? ".claude/skills"
      : agent.id === "codex-cli"
        ? ".codex/skills"
        : agent.id === "gemini-cli"
          ? ".gemini/skills"
          : agent.id === "opencode"
            ? ".opencode/skills"
            : ".skills",
  isDefault: true,
}));

let webSkills: Skill[] = builtinSkillSeeds.map((seed) => {
  const timestamp = nowIso();
  const isUserOverride = seed.id === "readme-generation";
  const isUtility = seed.id === "code-security-scan";
  return {
    id: seed.id,
    scope: "global",
    workspacePath: null,
    source: "builtin",
    enabled: true,
    skillDir: `~/.vanehub/skills/${seed.id}`,
    skillMdPath: `~/.vanehub/skills/${seed.id}/SKILL.md`,
    contentHash: `web-${seed.id}`,
    metadata: {
      id: seed.id,
      name: seed.name,
      description: seed.description,
      category: seed.category,
      version: "1.0.0",
      triggers: seed.triggers,
      aliases: seed.id === "readme-generation" ? ["docs"] : [],
      type: isUtility ? "utility" : "role",
      delivery: seed.id === "tdd-discipline" ? "on-demand" : "eager",
      compatibilityDefaults: { skillType: false, delivery: false },
    },
    boundAgentIds: ["claude-code", "codex-cli"],
    bindings: [],
    createdAt: timestamp,
    updatedAt: timestamp,
    layer: isUserOverride ? "user" : "system",
    origin: isUserOverride ? "migrated" : "shipped",
    trust: "trusted",
    availability: isUtility ? "unsupported" : "available",
    delegationCapability: isUtility
      ? { supported: false, reason: "native-runtime-unavailable" }
      : { supported: false, reason: "not-utility" },
    immutable: !isUserOverride,
    shadowedDefinitions: isUserOverride
      ? [{ layer: "system", origin: "shipped", version: "1.0.0", availability: "available" }]
      : [],
    usage: {
      viewCount: seed.id === "tdd-discipline" ? 3 : 0,
      useCount: seed.id === "tdd-discipline" ? 1 : 0,
      lastViewedAt: seed.id === "tdd-discipline" ? timestamp : null,
      lastUsedAt: seed.id === "tdd-discipline" ? timestamp : null,
      revisionWitness: "web-usage-1",
    },
  };
});

webSkills.push({
  ...webSkills[0],
  id: "project-conventions",
  scope: "workspace",
  workspacePath: "D:/example/project",
  source: "user",
  skillDir: "D:/example/project/.vanehub/skills/project-conventions",
  skillMdPath: "D:/example/project/.vanehub/skills/project-conventions/SKILL.md",
  contentHash: "web-project-conventions",
  metadata: {
    id: "project-conventions",
    name: "Project Conventions",
    description: "Project-specific conventions.",
    category: "development",
    version: "1.0.0",
    triggers: ["project"],
    aliases: [],
    type: "role",
    delivery: "on-demand",
    compatibilityDefaults: { skillType: false, delivery: false },
  },
  boundAgentIds: [],
  layer: "project",
  origin: "created",
  immutable: false,
  shadowedDefinitions: [],
  usage: {
    viewCount: 0,
    useCount: 0,
    lastViewedAt: null,
    lastUsedAt: null,
    revisionWitness: "web-project-usage-1",
  },
});

const webSkillDocuments = new Map<string, string>(
  webSkills.map((skill) => [
    `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`,
    `Built-in instructions for ${skill.metadata.name}.`,
  ]),
);
webSkillDocuments.set(
  "global::tdd-discipline",
  `Use {skill_base_dir} for supporting material.\n${"TDD guidance. ".repeat(1_100)}`,
);

const webSkillResourceDocuments = new Map<string, string>([
  ["skill://tdd-discipline/references/testing-cycle.md", "Red, green, refactor, then run regression tests."],
  ["skill://tdd-discipline/templates/test-plan.md", "# Test plan\n\n- Expected failure\n- Minimal fix\n- Regression"],
  ["skill://project-conventions/references/conventions.md", "Use the project formatting and validation commands."],
]);
let nextWebSkillRevision = 1;

function nextWebSkillHash(skillId: string) {
  const revision = nextWebSkillRevision;
  nextWebSkillRevision += 1;
  return `web-${skillId}-${revision}`;
}

/** Mock non-mount Skill-to-API-agent bindings (`add-agent-skill-support`), separate from `webSkills`' CLI mount-path `boundAgentIds`. */
let webSkillApiAgentBindings: Array<{
  skillId: string;
  scope: SkillScope;
  workspacePath: string | null;
  agentId: string;
}> = [];

const deletedBuiltinSkillIds = new Set<string>();
const purgedEvidenceScopes = new Set<string>();

/** Mock API agents' `modelId`/`interfaceFormat`/`baseUrl` (`add-agent-lifecycle-management`) —
 * kept out of `AgentRegistryEntry` itself, mirroring the real backend, where those fields live
 * behind a separate read path (`getApiAgentProviderConfig`) rather than on the CLI/API-agnostic
 * registry view. */
const webApiAgentProviderConfigs = new Map<string, ApiAgentProviderConfig>();
let webOnePieceProviderConfig: OnePieceProviderConfig = {
  provider: "VaneHub",
  modelId: null,
  interfaceFormat: null,
  baseUrl: null,
  autoApproveTools: false,
  credentialPresent: false,
};
let webOnePieceProviderProfiles: OnePieceProviderProfiles = { profiles: [], activeProfileId: null };
let nextOnePieceProviderProfileId = 1;

function applyWebOnePieceActiveProfile(profileId: string | null) {
  const active = profileId == null
    ? null
    : webOnePieceProviderProfiles.profiles.find((profile) => profile.id === profileId) ?? null;
  webOnePieceProviderProfiles = {
    activeProfileId: active?.id ?? null,
    profiles: webOnePieceProviderProfiles.profiles.map((profile) => ({
      ...profile,
      active: profile.id === active?.id,
    })),
  };
  webOnePieceProviderConfig = active ? {
    provider: active.provider,
    modelId: active.modelId,
    interfaceFormat: active.interfaceFormat,
    baseUrl: active.baseUrl,
    autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
    credentialPresent: active.credentialPresent,
  } : {
    provider: "VaneHub",
    modelId: null,
    interfaceFormat: null,
    baseUrl: null,
    autoApproveTools: false,
    credentialPresent: false,
  };
  if (active) {
    webApiAgentProviderConfigs.set("onepiece", {
      modelId: active.modelId,
      interfaceFormat: active.interfaceFormat,
      baseUrl: active.baseUrl,
      autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
    });
  } else {
    webApiAgentProviderConfigs.delete("onepiece");
  }
  const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
  if (agent) {
    agent.provider = active?.provider ?? "VaneHub";
    agent.availabilityState = active?.credentialPresent ? "available" : active ? "needs-auth" : "unavailable";
    agent.unavailableReason = active?.credentialPresent
      ? undefined
      : active
        ? "OnePiece requires an API key."
        : "OnePiece requires provider configuration.";
  }
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

/** Mock retrieval configuration (`add-retrieval-vector-search`) — a global singleton mirroring
 * the real `retrieval_configuration` table's single row; starts unconfigured like a fresh
 * install (design doc §7.4). */
let webRetrievalConfiguration: RetrievalConfiguration = {
  sourceProfileId: null,
  embeddingModel: null,
  automaticCodeIndexMode: "disabled",
};

/** Mock retrieval index status — a single global aggregate, mirroring the real one across every
 * agent and every `scope_folder` (design doc §7.4). Seeded with plausible, self-consistent counts
 * so a settings UI has something realistic to render before ever calling `rebuildRetrievalIndex`.
 * `lastFailureCategory` is deliberately always `null`: the Web/mock runtime guarantees the same
 * contract shape and observable behavior as the real one, not algorithmic equivalence with the
 * Rust-side failure classification (design doc §7.5). */
const seededWebRetrievalIndexStatus = (): RetrievalIndexStatus => ({ indexed: 12, pending: 3, failed: 2, lastFailureCategory: null });
let webRetrievalIndexStatus: RetrievalIndexStatus = seededWebRetrievalIndexStatus();

let nextWebCodeIndexId = 1;
let nextWebCodeAuditId = 1;
const webCodeIndexes = new Map<string, CodeIndexWorkspace>();
let webCodeIndexAudit: CodeIndexAuditEntry[] = [];

function emptyCodeIndexStatus(phase: CodeIndexPhase): CodeIndexStatus {
  return {
    phase,
    totalFiles: 0,
    processedFiles: 0,
    failedFiles: 0,
    totalChunks: 0,
    processedChunks: 0,
    pendingChunks: 0,
    indexedChunks: 0,
    failedChunks: 0,
    redactionCount: 0,
    estimatedEmbeddingRequests: 0,
    lastFailureCategory: null,
    updatedAt: nowIso(),
  };
}

function cloneCodeIndex(workspace: CodeIndexWorkspace): CodeIndexWorkspace {
  return structuredClone(workspace);
}

function requireWebCodeIndex(workspaceId: string): CodeIndexWorkspace {
  const workspace = webCodeIndexes.get(workspaceId);
  if (!workspace) throw new Error("Code index workspace was not found.");
  return workspace;
}

function updateWebCodeIndexPhase(workspace: CodeIndexWorkspace, phase: CodeIndexPhase) {
  const status = workspace.status;
  if (phase === "parsing") {
    Object.assign(status, {
      totalFiles: 18, processedFiles: 6, failedFiles: 0,
      totalChunks: 24, processedChunks: 0, pendingChunks: 24,
      indexedChunks: 0, failedChunks: 0, redactionCount: 2,
      estimatedEmbeddingRequests: 1,
    });
  } else if (phase === "awaiting_embedding_confirmation") {
    Object.assign(status, {
      totalFiles: 18, processedFiles: 18, failedFiles: 0,
      totalChunks: 54, processedChunks: 0, pendingChunks: 54,
      indexedChunks: 0, failedChunks: 0, redactionCount: 4,
      estimatedEmbeddingRequests: 2,
    });
  } else if (phase === "ready") {
    status.processedChunks = status.totalChunks;
    status.pendingChunks = 0;
    status.indexedChunks = status.totalChunks;
    status.failedChunks = 0;
  }
  status.phase = phase;
  status.updatedAt = nowIso();
}

function recordWebCodeIndexAudit(workspaceId: string, event: CodeIndexAuditEntry["event"]) {
  webCodeIndexAudit = [{
    auditId: nextWebCodeAuditId,
    workspaceId,
    relativePath: null,
    event,
    reason: null,
    itemCount: 1,
    createdAt: nowIso(),
  }, ...webCodeIndexAudit].slice(0, 200);
  nextWebCodeAuditId += 1;
}

function discoverWebSessionCodeIndex(session: Session) {
  const mode = webRetrievalConfiguration.automaticCodeIndexMode;
  const root = session.worktreePath ?? session.folder ?? session.projectPath;
  if (session.agentId !== "onepiece" || session.remoteWorkspace || !root || mode === "disabled") {
    return;
  }
  const normalizedRoot = root.replaceAll("\\", "/").replace(/\/$/, "").toLocaleLowerCase();
  const existing = [...webCodeIndexes.values()].find((workspace) => (
    workspace.canonicalRoot.replaceAll("\\", "/").replace(/\/$/, "").toLocaleLowerCase()
      === normalizedRoot
  ));
  if (existing) return;
  const displayName = root.split(/[\\/]/).filter(Boolean).at(-1) ?? root;
  const workspace: CodeIndexWorkspace = {
    workspaceId: `web-code-index-${nextWebCodeIndexId}`,
    canonicalRoot: root,
    displayName,
    origin: "automatic",
    enabled: true,
    mode,
    selectedRoots: [""],
    languages: [...codeIndexLanguages],
    exclusionPatterns: [],
    maxFileBytes: 100 * 1024,
    indexVersion: "1",
    generation: 1,
    status: emptyCodeIndexStatus("scanning"),
  };
  nextWebCodeIndexId += 1;
  webCodeIndexes.set(workspace.workspaceId, workspace);
}

/** Static catalog, independent of the requested profile — listing embedding models never hits
 * the network in the Web/mock runtime (design doc §7.5), so there is nothing live to discover. */
const webEmbeddingModelOptions: EmbeddingModelOption[] = [
  { id: "text-embedding-3-small", displayName: "text-embedding-3-small" },
  { id: "text-embedding-3-large", displayName: "text-embedding-3-large" },
];

const promptHookStorageKey = "vanehub.prompt-hooks.v1";
const promptHookTraceStorageKey = "vanehub.prompt-hook-traces.v1";

const defaultPromptHookBindings: ManagedCliAgentId[] = ["claude-code", "codex-cli", "gemini-cli", "opencode"];
const promptHookCategories: PromptHookCategory[] = ["bootstrap", "callback", "dynamic", "law", "navigation", "routing", "static"];

const builtinPromptHookSeeds: PromptHook[] = [
  createBuiltinPromptHook({
    id: "bootstrap-session-context",
    name: "Session Context",
    description: "Adds session and workspace context to each CLI prompt.",
    category: "bootstrap",
    stage: "session-init",
    order: 100,
    disableable: true,
    templateBody: "Session context: {{sampleInput}}",
  }),
  createBuiltinPromptHook({
    id: "law-runtime-boundary",
    name: "Runtime Boundary",
    description: "Keeps CLI behavior inside VaneHub runtime and permission boundaries.",
    category: "law",
    stage: "session-init",
    order: 200,
    disableable: false,
    templateBody: "Respect the active VaneHub runtime, permissions, and project boundaries.",
  }),
  createBuiltinPromptHook({
    id: "static-response-format",
    name: "Response Format",
    description: "Sets a concise engineering response baseline.",
    category: "static",
    stage: "session-init",
    order: 300,
    disableable: true,
    templateBody: "Use direct, actionable engineering responses with concise verification notes.",
  }),
  createBuiltinPromptHook({
    id: "dynamic-session-config",
    name: "Session Configuration",
    description: "Summarizes active session configuration for the selected CLI.",
    category: "dynamic",
    stage: "per-turn",
    order: 400,
    disableable: true,
    templateBody: "Active CLI: {{agentId}}. User request follows after the hook context.",
  }),
  createBuiltinPromptHook({
    id: "navigation-project-hints",
    name: "Project Navigation",
    description: "Encourages grounded project inspection before code changes.",
    category: "navigation",
    stage: "per-turn",
    order: 500,
    disableable: true,
    templateBody: "Inspect relevant project files and existing patterns before making changes.",
  }),
  createBuiltinPromptHook({
    id: "routing-cli-capabilities",
    name: "CLI Capability Routing",
    description: "Keeps behavior aligned with the selected CLI agent capabilities.",
    category: "routing",
    stage: "per-turn",
    order: 600,
    disableable: true,
    templateBody: "Route work through capabilities available to {{agentId}}.",
  }),
  createBuiltinPromptHook({
    id: "callback-future-channel",
    name: "Callback Channel Placeholder",
    description: "Reserved placeholder for future callback-aware workflows.",
    category: "callback",
    stage: "per-turn",
    order: 700,
    disableable: true,
    enabled: false,
    templateBody: "Callback channel support is not active in this runtime.",
  }),
];

let memoryPromptHooks: Record<string, PromptHook> = {};
let memoryPromptTraces: PromptHookTraceSummary[] = [];

const webCliTools: CliToolStatus[] = [
  {
    agentId: "claude-code",
    displayName: "Anthropic Claude Code CLI",
    provider: "Anthropic",
    executableName: "claude",
    packageName: "@anthropic-ai/claude-code",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://claude.ai/install.sh && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status' || npm install -g @anthropic-ai/claude-code@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "codex-cli",
    displayName: "OpenAI Codex CLI",
    provider: "OpenAI",
    executableName: "codex",
    packageName: "@openai/codex",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "npm install -g @openai/codex@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "gemini-cli",
    displayName: "Google Gemini CLI",
    provider: "Google",
    executableName: "gemini",
    packageName: "@google/gemini-cli",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "npm install -g @google/gemini-cli@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "opencode",
    displayName: "OpenCode CLI",
    provider: "OpenCode",
    executableName: "opencode",
    packageName: "opencode-ai",
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://opencode.ai/install && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status' || npm install -g opencode-ai@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
  {
    agentId: "antigravity-cli",
    displayName: "Google Antigravity CLI",
    provider: "Google",
    executableName: "agy",
    // Distributed only by installer script, so there is no package to name.
    packageName: null,
    installed: null,
    currentVersion: null,
    latestVersion: null,
    availableVersions: [],
    detectedPath: null,
    installCommand: "bash -lc 'tmp=$(mktemp) && wget -qO \"$tmp\" https://antigravity.google/cli/install.sh && bash \"$tmp\"; status=$?; rm -f \"$tmp\"; exit $status'",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
    environmentType: "unknown",
    installations: [],
    activeInstallationPath: null,
    conflictState: "none",
    lifecycleEligibility: "unavailable",
  },
];

function nowIso() {
  return new Date().toISOString();
}

function daysAgoIso(days: number) {
  const value = new Date();
  value.setDate(value.getDate() - days);
  return value.toISOString();
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

function normalizeWebPath(value: string, label: string) {
  const raw = value.trim().replaceAll("\\", "/");
  if (!raw) throw new Error(`${label} is required`);
  const drive = raw.match(/^[a-zA-Z]:/)?.[0];
  const absolute = raw.startsWith("/") || Boolean(drive);
  const remainder = drive ? raw.slice(drive.length) : raw;
  const segments: string[] = [];
  for (const segment of remainder.split("/")) {
    if (!segment || segment === ".") continue;
    if (segment === "..") {
      if (segments.length > 0 && segments.at(-1) !== "..") segments.pop();
      else if (!absolute) segments.push(segment);
      continue;
    }
    segments.push(segment);
  }
  const prefix = drive ? drive.toUpperCase() : raw.startsWith("/") ? "/" : "";
  const normalized = `${prefix}${prefix && prefix !== "/" && segments.length > 0 ? "/" : ""}${segments.join("/")}`;
  if (!normalized) throw new Error(`${label} is required`);
  return normalized;
}

function normalizeWebSkillLocation(input: SkillScopeInput): SkillScopeInput {
  if (input.scope === "global") return { scope: "global", workspacePath: null };
  return { scope: "workspace", workspacePath: normalizeWebPath(input.workspacePath ?? "", "Workspace path") };
}

function validateWebSkillMetadata(metadata: SkillMetadata) {
  if (!/^(?!-)[a-z0-9-]+(?<!-)$/.test(metadata.id)) {
    throw new Error("Skill id must be kebab-case letters, digits, and hyphens");
  }
  if (![metadata.name, metadata.description, metadata.category, metadata.version].every((value) => value.trim())) {
    throw new Error("Skill metadata name, description, category, and version are required");
  }
}

function validateWebSkillMutation(input: SkillMutationInput, allowedSource: SkillSource) {
  validateWebSkillMetadata(input.metadata);
  if (input.id !== input.metadata.id) throw new Error("Skill request id must match metadata id");
  if ((input.source ?? "user") !== allowedSource) {
    throw new Error(`Skill source ${input.source ?? "user"} is invalid for this operation`);
  }
  return { ...input, ...normalizeWebSkillLocation(input), source: allowedSource };
}

function webPathsOverlap(left: string, right: string) {
  const comparable = (value: string) => (/^[a-zA-Z]:/.test(value) ? value.toLocaleLowerCase() : value);
  const leftPath = comparable(left);
  const rightPath = comparable(right);
  return leftPath === rightPath || leftPath.startsWith(`${rightPath}/`) || rightPath.startsWith(`${leftPath}/`);
}

function skillScopeMatches(skill: Skill, input: SkillScopeInput) {
  const location = normalizeWebSkillLocation(input);
  return (
    skill.scope === location.scope &&
    (location.scope === "global" || skill.workspacePath === location.workspacePath)
  );
}

function mountPathForAgent(agentId: string) {
  return webSkillMountPaths.find((path) => path.agentId === agentId)?.mountPath ?? ".skills";
}

function skillDocumentKey(skill: Pick<Skill, "id" | "scope" | "workspacePath">) {
  return `${skill.scope}:${skill.workspacePath ?? ""}:${skill.id}`;
}

function requireAgentKind(agentId: string, kind: "cli" | "api") {
  const agent = mockAgents.find((candidate) => candidate.id === agentId);
  if (!agent || agent.launch.kind !== kind) {
    throw new Error(`Unknown ${kind.toUpperCase()} Agent id: ${agentId}`);
  }
}

function validateMountPath(mountPath: string) {
  const normalized = mountPath.trim().replaceAll("\\", "/");
  const segments = normalized.split("/");
  if (
    !normalized ||
    normalized.startsWith("/") ||
    /^[a-zA-Z]:/.test(normalized) ||
    segments.some((segment) => !segment || segment === "." || segment === "..") ||
    segments[0]?.toLocaleLowerCase() === ".vanehub"
  ) {
    throw new Error(`Invalid Skill mount path: ${mountPath}`);
  }
  return normalized;
}

function hydrateSkillBindings(skill: Skill): Skill {
  const bindings = skill.boundAgentIds.map((agentId) => {
    const mountPath = mountPathForAgent(agentId);
    const root = skill.scope === "global" ? "~" : (skill.workspacePath ?? ".");
    return {
      agentId,
      mountPath,
      mountedPath: `${root}/${mountPath}/${skill.id}`,
      mounted: skill.enabled,
    };
  });
  return { ...skill, bindings };
}

function buildSkillContent(skill: Skill) {
  const triggers = skill.metadata.triggers.map((trigger) => `  - ${trigger}`).join("\n");
  const body = webSkillDocuments.get(skillDocumentKey(skill)) ?? "";
  return `---\nid: ${skill.metadata.id}\nname: ${skill.metadata.name}\ndescription: ${skill.metadata.description}\ncategory: ${skill.metadata.category}\nversion: ${skill.metadata.version}\ntriggers:\n${triggers}\n---\n\n# ${skill.metadata.name}\n\n${body.trim()}\n`;
}

function webSkillResources(skillId: string) {
  const entries = [...webSkillResourceDocuments.entries()]
    .filter(([uri]) => uri.startsWith(`skill://${skillId}/`))
    .map(([uri, content]) => ({
      uri,
      relativePath: uri.slice(`skill://${skillId}/`.length),
      sizeBytes: new TextEncoder().encode(content).byteLength,
    }));
  const inDirectory = (directory: string) => entries.filter((entry) => entry.relativePath.startsWith(`${directory}/`));
  return {
    scripts: inDirectory("scripts"),
    references: inDirectory("references"),
    templates: inDirectory("templates"),
    assets: inDirectory("assets"),
    truncated: false,
  };
}

type WebSkillRefusalOutcome = Extract<SkillLoadOutcome, { status: "refused" }>;

function webSkillRefusal(
  requested: string,
  reason: SkillAccessRefusalReason,
  canonicalId: string | null = null,
): WebSkillRefusalOutcome {
  return { status: "refused", refusal: { requested, canonicalId, reason, conflictingIds: [] } };
}

function findProgressiveWebSkill(input: SkillLoadInput): WebSkillRefusalOutcome | Skill {
  const workspacePath = input.workspacePath ? normalizeWebPath(input.workspacePath, "Workspace path") : null;
  const candidates = webSkills.filter((skill) =>
    skill.scope === "global" || (workspacePath != null && skill.workspacePath === workspacePath),
  );
  const exact = candidates.find((skill) => skill.id === input.idOrAlias);
  const aliases = exact == null
    ? candidates.filter((skill) => skill.metadata.aliases?.includes(input.idOrAlias))
    : [];
  if (aliases.length > 1) {
    return {
      status: "refused",
      refusal: {
        requested: input.idOrAlias,
        canonicalId: null,
        reason: "ambiguous-alias",
        conflictingIds: aliases.map((skill) => skill.id).sort(),
      },
    };
  }
  const skill = exact ?? aliases[0];
  if (!skill) return webSkillRefusal(input.idOrAlias, "not-found");
  if (!skill.enabled) return webSkillRefusal(input.idOrAlias, "disabled", skill.id);
  if (skill.metadata.type === "utility") {
    return webSkillRefusal(input.idOrAlias, "utility-not-loadable", skill.id);
  }
  if (skill.availability !== "available") {
    return webSkillRefusal(input.idOrAlias, skill.availability, skill.id);
  }
  return skill;
}

function skillStats(skills: Skill[]) {
  return {
    total: skills.length,
    enabled: skills.filter((skill) => skill.enabled).length,
    mounted: skills.filter((skill) => skill.enabled && skill.boundAgentIds.length > 0).length,
  };
}

function findWebSkill(skillId: string, input: SkillScopeInput) {
  const skill = webSkills.find((candidate) => candidate.id === skillId && skillScopeMatches(candidate, input));
  if (!skill) {
    throw new Error(`Skill not found: ${skillId}`);
  }
  return skill;
}

function upsertWebSkill(skill: Skill) {
  const index = webSkills.findIndex(
    (candidate) =>
      candidate.id === skill.id &&
      candidate.scope === skill.scope &&
      candidate.workspacePath === skill.workspacePath,
  );
  if (index === -1) {
    webSkills = [...webSkills, skill];
    return skill;
  }
  webSkills = webSkills.map((candidate, candidateIndex) => (candidateIndex === index ? skill : candidate));
  return skill;
}

function mutationToSkill(input: SkillMutationInput): Skill {
  const location = normalizeWebSkillLocation(input);
  const timestamp = nowIso();
  const root = location.scope === "global" ? "~/.vanehub/skills" : `${location.workspacePath}/.vanehub/skills`;
  const skill: Skill = {
    id: input.id,
    scope: location.scope,
    workspacePath: location.workspacePath ?? null,
    source: input.source ?? "user",
    enabled: input.enabled,
    skillDir: `${root}/${input.id}`,
    skillMdPath: `${root}/${input.id}/SKILL.md`,
    contentHash: nextWebSkillHash(input.id),
    metadata: {
      ...input.metadata,
      aliases: input.metadata.aliases ?? [],
      type: input.metadata.type ?? "role",
      delivery: input.metadata.delivery ?? "eager",
      compatibilityDefaults: input.metadata.compatibilityDefaults ?? {
        skillType: input.metadata.type == null,
        delivery: input.metadata.delivery == null,
      },
    },
    boundAgentIds: [...input.boundAgentIds],
    bindings: [],
    createdAt: timestamp,
    updatedAt: timestamp,
    layer: location.scope === "workspace" ? "project" : "user",
    origin: input.source === "imported" ? "imported" : "created",
    trust: input.source === "imported" ? "untrusted" : "trusted",
    availability: input.metadata.type === "utility" ? "unsupported" : "available",
    delegationCapability: input.metadata.type === "utility"
      ? { supported: false, reason: "native-runtime-unavailable" }
      : { supported: false, reason: "not-utility" },
    immutable: false,
    shadowedDefinitions: [],
    usage: {
      viewCount: 0,
      useCount: 0,
      lastViewedAt: null,
      lastUsedAt: null,
      revisionWitness: "web-usage-1",
    },
  };
  webSkillDocuments.set(skillDocumentKey(skill), input.body);
  return skill;
}

function sortSessions(items: Session[]) {
  return [...items].sort((left, right) => {
    if (left.pinned !== right.pinned) return left.pinned ? -1 : 1;
    if (left.archived !== right.archived) return left.archived ? 1 : -1;
    return right.updatedAt.localeCompare(left.updatedAt);
  });
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
  const session = findSession(input.sessionId);
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

function findSession(sessionId: string) {
  const session = sessions.find((candidate) => candidate.id === sessionId);
  if (!session) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  return session;
}

function createMessageId() {
  const id = `web-message-${nextMessageId}`;
  nextMessageId += 1;
  return id;
}

function createBuiltinPromptHook(input: {
  id: string;
  name: string;
  description: string;
  category: PromptHookCategory;
  stage: PromptHook["stage"];
  order: number;
  disableable: boolean;
  templateBody: string;
  enabled?: boolean;
}): PromptHook {
  return {
    id: input.id,
    name: input.name,
    description: input.description,
    category: input.category,
    stage: input.stage,
    order: input.order,
    version: 1,
    source: "builtin",
    enabled: input.enabled ?? true,
    disableable: input.disableable,
    cliBindings: [...defaultPromptHookBindings],
    governance: {
      safetyTier: "readonly",
      transparencyTier: input.disableable ? "opt-in-view" : "visible-by-default",
      governanceTier: input.disableable ? "human-gated" : "immutable",
    },
    templateBody: input.templateBody,
    createdAt: "2026-07-18T00:00:00.000Z",
    updatedAt: "2026-07-18T00:00:00.000Z",
  };
}

function isManagedCliAgentId(value: string): value is ManagedCliAgentId {
  return managedCliAgentIds.includes(value as ManagedCliAgentId);
}

function validatePromptHookInput(input: PromptHookMutationInput | PromptHookUpdateInput) {
  if (!/^[a-z0-9][a-z0-9-]{2,63}$/.test(input.id)) {
    throw new Error("Invalid Prompt Hook id");
  }
  if (!input.name.trim()) throw new Error("Prompt Hook name is required");
  if (!promptHookCategories.includes(input.category)) throw new Error("Unsupported Prompt Hook category");
  if (input.stage !== "session-init" && input.stage !== "per-turn") throw new Error("Unsupported Prompt Hook stage");
  if (!Number.isFinite(input.order) || input.order < 0) throw new Error("Invalid Prompt Hook order");
  if (/[\u0000-\u0008\u000b\u000c\u000e-\u001f]/.test(input.templateBody)) {
    throw new Error("Prompt Hook content contains unsupported control characters");
  }
  if (!input.cliBindings.every(isManagedCliAgentId)) throw new Error("Unsupported Prompt Hook CLI binding");
}

function readStoredPromptHooks(): Record<string, PromptHook> {
  if (typeof localStorage === "undefined") return memoryPromptHooks;
  const raw = localStorage.getItem(promptHookStorageKey);
  if (!raw) return memoryPromptHooks;
  try {
    return JSON.parse(raw) as Record<string, PromptHook>;
  } catch {
    return memoryPromptHooks;
  }
}

function writeStoredPromptHooks(value: Record<string, PromptHook>) {
  memoryPromptHooks = value;
  if (typeof localStorage !== "undefined") localStorage.setItem(promptHookStorageKey, JSON.stringify(value));
}

function readPromptHookTraces(): PromptHookTraceSummary[] {
  if (typeof localStorage === "undefined") return memoryPromptTraces;
  const raw = localStorage.getItem(promptHookTraceStorageKey);
  if (!raw) return memoryPromptTraces;
  try {
    return JSON.parse(raw) as PromptHookTraceSummary[];
  } catch {
    return memoryPromptTraces;
  }
}

function writePromptHookTraces(value: PromptHookTraceSummary[]) {
  memoryPromptTraces = value.slice(0, 50);
  if (typeof localStorage !== "undefined") localStorage.setItem(promptHookTraceStorageKey, JSON.stringify(memoryPromptTraces));
}

function listEffectivePromptHooks(): PromptHook[] {
  const stored = readStoredPromptHooks();
  const builtins = builtinPromptHookSeeds.map((hook) => stored[hook.id] ?? hook);
  const userHooks = Object.values(stored).filter((hook) => hook.source === "user");
  return [...builtins, ...userHooks].sort((left, right) => {
    if (left.stage !== right.stage) return left.stage.localeCompare(right.stage);
    if (left.category !== right.category) return left.category.localeCompare(right.category);
    return left.order - right.order || left.id.localeCompare(right.id);
  });
}

function promptHookStats(hooks: PromptHook[]): PromptHookListResult["stats"] {
  return {
    total: hooks.length,
    enabled: hooks.filter((hook) => hook.enabled).length,
    builtin: hooks.filter((hook) => hook.source === "builtin").length,
    user: hooks.filter((hook) => hook.source === "user").length,
  };
}

function renderPromptHookTemplate(template: string, input: { agentId: ManagedCliAgentId; sampleInput: string }) {
  const agentName = mockAgents.find((agent) => agent.id === input.agentId)?.displayName ?? input.agentId;
  return renderWebPromptHookTemplate(template, {
    agentId: input.agentId,
    agentName,
    currentTime: nowIso(),
    sampleInput: input.sampleInput,
    sessionId: "session-preview",
  });
}

function promptHookHash(content: string) {
  let hash = 5381;
  for (let index = 0; index < content.length; index += 1) {
    hash = (hash * 33) ^ content.charCodeAt(index);
  }
  return (hash >>> 0).toString(16).padStart(8, "0");
}

function traceForHook(hook: PromptHook, status: PromptHookTraceSummary["status"], content: string | null, agentId: ManagedCliAgentId, reason?: string): PromptHookTraceSummary {
  return {
    id: `web-prompt-trace-${Date.now()}-${hook.id}`,
    hookId: hook.id,
    category: hook.category,
    stage: hook.stage,
    status,
    version: status === "fired" ? hook.version : undefined,
    contentHash: content ? promptHookHash(content) : undefined,
    tokenEstimate: content ? Math.ceil(content.length / 4) : undefined,
    reason,
    agentId,
    createdAt: nowIso(),
  };
}

function assemblePromptHooks(input: PromptAssemblyPreviewInput): PromptHookPreview {
  const traces: PromptHookTraceSummary[] = [];
  const rendered: string[] = [];
  for (const hook of listEffectivePromptHooks()) {
    if (hook.source === "user" && hook.version <= 0) {
      traces.push(traceForHook(hook, "skipped", null, input.agentId, "unpublished"));
      continue;
    }
    if (!hook.enabled) {
      traces.push(traceForHook(hook, "disabled", null, input.agentId, "disabled"));
      continue;
    }
    if (!hook.cliBindings.includes(input.agentId)) {
      traces.push(traceForHook(hook, "skipped", null, input.agentId, "unbound-cli"));
      continue;
    }
    const content = renderPromptHookTemplate(hook.templateBody ?? "", {
      agentId: input.agentId,
      sampleInput: input.sampleInput,
    });
    rendered.push(content);
    traces.push(traceForHook(hook, "fired", content, input.agentId));
  }
  const renderedContent = [...rendered, input.sampleInput].filter(Boolean).join("\n\n");
  writePromptHookTraces([...traces, ...readPromptHookTraces()]);
  return { agentId: input.agentId, renderedContent, trace: traces };
}

function mutationToPromptHook(input: PromptHookMutationInput): PromptHook {
  validatePromptHookInput(input);
  const timestamp = nowIso();
  return {
    id: input.id,
    name: input.name.trim(),
    description: input.description.trim(),
    category: input.category,
    stage: input.stage,
    order: input.order,
    version: 1,
    source: "user",
    enabled: input.enabled,
    disableable: true,
    cliBindings: [...input.cliBindings],
    governance: input.governance,
    templateBody: input.templateBody,
    createdAt: timestamp,
    updatedAt: timestamp,
  };
}

function findPromptHook(hookId: string) {
  const hook = listEffectivePromptHooks().find((candidate) => candidate.id === hookId);
  if (!hook) throw new Error(`Prompt Hook not found: ${hookId}`);
  return hook;
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
  const session = findSession(sessionId);
  updateSession(sessionId, {
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
  findSession(sessionId);
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
  findSession(sessionId);
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
  findSession(sessionId);
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

function updateSession(sessionId: string, updates: Partial<Session>) {
  const timestamp = nowIso();
  const sessionIndex = sessions.findIndex((session) => session.id === sessionId);
  if (sessionIndex === -1) {
    throw new Error(`Session not found: ${sessionId}`);
  }
  const updatedSession: Session = { ...sessions[sessionIndex], ...updates, updatedAt: timestamp };
  sessions = sessions.map((session, index) => (index === sessionIndex ? updatedSession : session));
  if (activeSessionId === sessionId) {
    workflowState = {
      ...workflowState,
      activeAgentId: updatedSession.agentId,
      activeInteractionMode: updatedSession.interactionMode,
      lifecycleState: updatedSession.lifecycleState,
    };
  }
  return updatedSession;
}

function createWebSeatId() {
  const seatId = `web-seat-${nextSeatId}`;
  nextSeatId += 1;
  return seatId;
}

function findScheduledTask(taskId: string) {
  const task = scheduledTasks.find((candidate) => candidate.id === taskId);
  if (!task) throw new Error(`Scheduled task not found: ${taskId}`);
  return task;
}

function cloneScheduledTask(task: ScheduledTask): ScheduledTask {
  return { ...task, frequency: { ...task.frequency } };
}

function validateScheduledTaskInput(input: CreateScheduledTaskInput) {
  const name = input.name.trim();
  const content = input.content.trim();
  if (!name) throw new Error("Scheduled task name is required.");
  if (!content) throw new Error("Scheduled task content is required.");
  const agent = mockAgents.find((candidate) => candidate.id === input.agentId);
  if (!agent || (agent.id !== "onepiece" && !agent.supportedInteractionModes.includes("cli"))) {
    throw new Error(`Unsupported Agent: ${input.agentId}`);
  }
  validateScheduledTaskFrequency(input.frequency);
  return { name, content };
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
  sessions = sessions.filter((session) => !loopRoleSessionIds.has(session.id));
  loopRoleSessionIds.forEach((sessionId) => messagesBySession.delete(sessionId));
  loopRoleSessionIds.clear();
  loopDefinitions = [];
  loopRuns = [];
  nextLoopDefinitionId = 1;
  nextLoopRunId = 1;
  nextLoopEvidenceId = 1;
}

export function resetWebRetrievalForTest() {
  webRetrievalConfiguration = {
    sourceProfileId: null,
    embeddingModel: null,
    automaticCodeIndexMode: "disabled",
  };
  webRetrievalIndexStatus = seededWebRetrievalIndexStatus();
  nextWebCodeIndexId = 1;
  nextWebCodeAuditId = 1;
  webCodeIndexes.clear();
  webCodeIndexAudit = [];
}

export function resetWebEvidenceForTest() {
  purgedEvidenceScopes.clear();
}

export function searchWebCodeIndex(workspaceId: string, query: string) {
  const workspace = requireWebCodeIndex(workspaceId);
  const normalizedQuery = query.trim().toLocaleLowerCase();
  const includesSourceRoot = workspace.selectedRoots.some((root) => root === "" || root === "src");
  const sourceExcluded = workspace.exclusionPatterns.some((pattern) => pattern === "src/**" || pattern === "**/*.ts");
  if (workspace.status.phase !== "ready" || !workspace.languages.includes("typescript")
    || !includesSourceRoot || sourceExcluded || normalizedQuery !== "handle_login") return [];
  if (workspace.canonicalRoot.toLocaleLowerCase().includes("second")) return [];
  return [{
    filePath: "src/auth.ts",
    startLine: 12,
    endLine: 20,
    language: "typescript",
    symbolName: "handle_login",
    symbolKind: "function",
    snippet: "export async function handle_login(request: Request) { /* redacted */ }",
    matchedVia: workspace.mode === "local" ? "keyword" : "hybrid",
  }];
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
  sessions = [session, ...sessions];
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
const webSkillOverlayRuntime = createWebSkillOverlayRuntime((target) => {
  const workspacePath = target.scope === "project" && target.workspacePath
    ? normalizeWebPath(target.workspacePath, "Workspace path")
    : null;
  const candidates = webSkills.filter((skill) =>
    skill.id === target.skillId
      && (skill.scope === "global" || (workspacePath != null && skill.workspacePath === workspacePath)),
  );
  const skill = candidates.find((candidate) => candidate.scope === "workspace") ?? candidates[0];
  if (!skill) throw overlayError("notFound", "skill-not-found", `Skill not found: ${target.skillId}`);
  const instructions = buildSkillContent(skill);
  return {
    skillId: skill.id,
    layer: skill.layer,
    instructions,
    instructionHash: webOverlayHash(instructions),
    packageHash: skill.contentHash,
    pinned: false,
  };
});

const WEB_RUN_TIME = "2026-08-16T00:00:00.000Z";
let webAgentRuns: AgentRun[] = [{
  id: "018f0f17-4d6a-7e20-b41d-66c5271a28d0",
  owner: { ownerType: "web_demo", ownerId: "web-session-open" },
  links: [{ linkType: "session", linkId: "web-session-open" }],
  parentRunId: null,
  state: "paused",
  recoveryPolicy: "not_recoverable",
  retryCount: 1,
  maxRetries: 2,
  reasonCode: "web_demo_paused",
  createdAt: WEB_RUN_TIME,
  updatedAt: WEB_RUN_TIME,
  version: 4,
  lastWitness: "web-demo-pause",
}];
const webAgentRunEvents = new Map<string, AgentRunEvent[]>([[webAgentRuns[0].id, [{
  sequence: 4,
  state: "paused",
  trigger: "pause",
  timestamp: WEB_RUN_TIME,
  reasonCode: "web_demo_paused",
  witness: "web-demo-pause",
}]]]);

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

const webEvaluationTasks: EvaluationTask[] = [
  { id: "fix-null-auth-token", version: 1, category: "bugfix", prompt: "Fix null authentication token handling.", timeoutSeconds: 120, verifierProfiles: ["npm-test", "static-files"] },
  { id: "add-parser-test", version: 1, category: "tests", prompt: "Add deterministic parser tests.", timeoutSeconds: 120, verifierProfiles: ["npm-test"] },
  { id: "refactor-search", version: 1, category: "refactor", prompt: "Refactor search without changing ordering.", timeoutSeconds: 180, verifierProfiles: ["cargo-test"] },
];
let webEvaluationArenas: EvaluationArena[] = [];

function webEvaluationAttempt(arenaId: string, task: EvaluationTask, agentId: string, index: number) {
  const succeeded = index === 0;
  return {
    id: `${arenaId}-attempt-${index + 1}`, arenaId, canonicalRunId: `${arenaId}-run-${index + 1}`,
    taskId: task.id, taskVersion: task.version,
    agent: { agentId, providerId: agentId === "onepiece" ? "onepiece" : "managed-cli", modelId: agentId === "onepiece" ? "mock-local" : null, interactionMode: agentId === "onepiece" ? "api" : "cli", configurationFingerprint: `mock-${agentId}-v1` },
    outcome: succeeded ? "succeeded" as const : "task_failed" as const,
    checks: [{ checkId: "deterministic-tests", passed: succeeded, summary: succeeded ? "42/42" : "41/42" }],
    metrics: [
      { name: "wall_time", value: 12 + index, unit: "seconds", quality: "reported" as const, source: "runtime" },
      { name: "tool_calls", value: 5 + index, unit: "count", quality: "reported" as const, source: "runtime" },
      { name: "input_tokens", value: index === 0 ? 820 : null, unit: "tokens", quality: index === 0 ? "reported" as const : "unavailable" as const, source: "provider" },
    ],
    contextEvidenceManifestId: index === 0 ? "mock-context-evidence-v1" : null, artifactIds: [`${arenaId}-diff-${index + 1}`],
    timeline: [
      { id: "prepare", kind: "lifecycle" as const, label: "Clean fixture prepared", status: "completed" },
      { id: "tool", kind: "tool" as const, label: "Patch applied", status: "completed" },
      { id: "verify", kind: "verification" as const, label: "Deterministic verification", status: succeeded ? "passed" : "failed" },
    ],
  };
}

export const webAgentClient: AgentService = {
  async listEvaluationTasks() { return structuredClone(webEvaluationTasks); },
  async startEvaluation(input) {
    const task = webEvaluationTasks.find((item) => item.id === input.taskId && item.version === input.taskVersion);
    if (!task || input.agentIds.length === 0 || input.agentIds.length > 8) throw new Error("Invalid evaluation configuration");
    const id = `web-eval-${webEvaluationArenas.length + 1}`;
    const arena: EvaluationArena = { id, operationId: `${id}-operation`, taskId: task.id, taskVersion: task.version, rankingVersion: "deterministic-v1", attempts: input.agentIds.map((agentId, index) => webEvaluationAttempt(id, task, agentId, index)) };
    webEvaluationArenas = [arena, ...webEvaluationArenas]; return structuredClone(arena);
  },
  async listEvaluationArenas() { return structuredClone(webEvaluationArenas); },
  async getEvaluationArena(arenaId) { const arena = webEvaluationArenas.find((item) => item.id === arenaId); if (!arena) throw new Error("Evaluation not found"); return structuredClone(arena); },
  async cancelEvaluation(arenaId) { const arena = await this.getEvaluationArena(arenaId); const cancelled = { ...arena, attempts: arena.attempts.map((attempt) => ["queued", "running"].includes(attempt.outcome) ? { ...attempt, outcome: "cancelled" as const } : attempt) }; webEvaluationArenas = webEvaluationArenas.map((item) => item.id === arenaId ? cancelled : item); return structuredClone(cancelled); },
  async getEvaluationAttempt(attemptId) { for (const arena of webEvaluationArenas) { const attempt = arena.attempts.find((item) => item.id === attemptId); if (attempt) return structuredClone(attempt); } throw new Error("Evaluation attempt not found"); },
  async exportEvaluation(arenaId) { return { schemaVersion: 1, arena: await this.getEvaluationArena(arenaId) }; },
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

  async listContextQualityHistory(input) {
    return listWebContextQualityHistory(input);
  },

  async getContextQualitySummary(input) {
    return getWebContextQualitySummary(input);
  },

  async listContextEvidenceManifests(input) {
    const manifest = {
      sessionId: input.sessionId ?? "web-context-session",
      turnId: "web-context-turn",
      generationId: "web-context-generation",
      policyVersion: "context-engine-v1",
      evidenceBudget: 4096,
      occupiedTokens: 768,
      selected: [{ id: "web-definition", sourceKind: "retrieval", sourceRef: "src/example.ts", startLine: 12, endLine: 28, symbol: "example", tokenEstimate: 512, reasonCodes: ["semantic-match", "symbol-relation"] }],
      rejected: [{ id: "web-memory", reasonCode: "budget-rejected" }],
      sourceOutcomes: { retrieval: "ready" as const, lsp: "unavailable" as const },
      duplicateTokensSaved: 256,
      collectionLatencyBucket: "under-50ms",
      rankingLatencyBucket: "under-10ms",
      compactionTriggered: false,
      runtime: "web-mock" as const,
    };
    return { items: input.cursor ? [] : [manifest], nextCursor: null };
  },

  async getContextEvidenceManifest(generationId) {
    const page = await this.listContextEvidenceManifests({ cursor: null, limit: 1 });
    return page.items.find((item) => item.generationId === generationId) ?? null;
  },

  async registerApiAgent(input: RegisterApiAgentInput) {
    const displayName = input.displayName.trim();
    const provider = input.provider.trim();
    const apiKey = input.apiKey.trim();
    const modelId = input.modelId.trim();
    if (!displayName || !provider || !apiKey || !modelId) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.incomplete"));
    }
    const baseUrl = input.baseUrl?.trim() || null;
    if (input.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    const baseId = slugify(displayName) || "api-agent";
    let candidateId = baseId;
    let suffix = 2;
    while (mockAgents.some((agent) => agent.id === candidateId)) {
      candidateId = `${baseId}-${suffix}`;
      suffix += 1;
    }
    const entry: AgentRegistryEntry = {
      id: candidateId,
      displayName,
      provider,
      launch: { kind: "api" },
      supportedInteractionModes: ["api"],
      availabilityState: "available",
      capabilityTags: ["api"],
      agentOrigin: "user",
    };
    mockAgents.push(entry);
    webApiAgentProviderConfigs.set(candidateId, {
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl,
      autoApproveTools: false,
    });
    return entry;
  },

  async getApiAgentProviderConfig(agentId: string) {
    return webApiAgentProviderConfigs.get(agentId) ?? null;
  },

  async getOnePieceProviderConfig() {
    return { ...webOnePieceProviderConfig };
  },

  async saveOnePieceProviderConfig(input: SaveOnePieceProviderConfigInput) {
    const provider = input.provider.trim();
    const modelId = input.modelId.trim();
    const baseUrl = input.baseUrl?.trim() || null;
    if (!provider || !modelId) throw new Error("Provider and model are required.");
    if (input.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    const hasReplacement = Boolean(input.apiKey?.trim());
    if (input.apiKey != null && !hasReplacement) throw new Error("API key cannot be empty.");
    if (!hasReplacement && !webOnePieceProviderConfig.credentialPresent) {
      throw new Error("API key is required for the first OnePiece configuration.");
    }
    webOnePieceProviderConfig = {
      provider,
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl: input.interfaceFormat === "anthropic" ? null : baseUrl,
      autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
      credentialPresent: hasReplacement || webOnePieceProviderConfig.credentialPresent,
    };
    webApiAgentProviderConfigs.set("onepiece", {
      modelId,
      interfaceFormat: input.interfaceFormat,
      baseUrl: webOnePieceProviderConfig.baseUrl,
      autoApproveTools: webOnePieceProviderConfig.autoApproveTools,
    });
    const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
    if (agent) {
      agent.provider = provider;
      agent.availabilityState = "available";
      agent.unavailableReason = undefined;
    }
    return { ...webOnePieceProviderConfig };
  },

  async resetOnePieceProviderConfig() {
    webOnePieceProviderProfiles = { profiles: [], activeProfileId: null };
    webOnePieceProviderConfig = {
      provider: "VaneHub",
      modelId: null,
      interfaceFormat: null,
      baseUrl: null,
      autoApproveTools: false,
      credentialPresent: false,
    };
    webApiAgentProviderConfigs.delete("onepiece");
    const agent = mockAgents.find((candidate) => candidate.id === "onepiece");
    if (agent) {
      agent.provider = "VaneHub";
      agent.availabilityState = "unavailable";
      agent.unavailableReason = "OnePiece requires provider configuration.";
    }
    return { ...webOnePieceProviderConfig };
  },

  async listOnePieceProviderProfiles() {
    return structuredClone(webOnePieceProviderProfiles);
  },

  async listOnePieceProviderPresets() {
    return getOnePieceProviderPresets();
  },

  async discoverOnePieceProviderModels(input) {
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) throw new Error("OnePiece provider preset was not found.");
    const profile = input.profileId
      ? webOnePieceProviderProfiles.profiles.find((candidate) => candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("OnePiece provider profile was not found.");
    if (profile && (profile.sourceProviderId !== input.providerId || profile.sourceEndpointType !== input.endpointType)) {
      throw new Error("The OnePiece profile does not belong to the selected provider.");
    }
    if (!input.apiKey?.trim() && !profile?.credentialPresent) {
      throw new Error("API key is required to fetch models for this OnePiece provider.");
    }
    const ids = [profile?.modelId, ...preset.fallbackModels]
      .filter((value): value is string => Boolean(value));
    return {
      providerId: input.providerId,
      endpointType: input.endpointType,
      models: [...new Set(ids)].map((id) => ({
        id,
        displayName: id,
        source: id === profile?.modelId ? "profile" as const : "catalog" as const,
      })),
      source: "catalog" as const,
      warning: null,
    };
  },

  async validateOnePieceProviderCredential(input) {
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) throw new Error("OnePiece provider preset was not found.");
    const profile = input.profileId
      ? webOnePieceProviderProfiles.profiles.find((candidate) => candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("OnePiece provider profile was not found.");
    if (profile && (profile.sourceProviderId !== input.providerId || profile.sourceEndpointType !== input.endpointType)) {
      throw new Error("The OnePiece profile does not belong to the selected provider.");
    }
    if (!input.modelId.trim()) throw new Error("A model is required to verify the API key.");
    const credential = input.apiKey?.trim();
    if (!credential && !profile?.credentialPresent) throw new Error("API key is required to verify this provider.");
    const status = credential === "web-invalid"
      ? "invalid-credential" as const
      : credential === "web-rate-limited"
        ? "rate-limited" as const
        : credential === "web-unavailable"
          ? "provider-unavailable" as const
          : "valid" as const;
    return { status, latencyMs: 12, httpStatus: status === "valid" ? 200 : status === "invalid-credential" ? 401 : status === "rate-limited" ? 429 : null };
  },

  async saveOnePieceProviderProfile(input: SaveOnePieceProviderProfileInput) {
    const name = input.name.trim();
    const modelId = input.modelId.trim();
    const preset = resolveOnePieceProviderPreset(input.providerId, input.endpointType);
    if (!preset) {
      throw new Error("OnePiece provider preset was not found.");
    }
    if (!name || !modelId) {
      throw new Error("Profile name and model are required.");
    }
    const existing = input.id
      ? webOnePieceProviderProfiles.profiles.find((profile) => profile.id === input.id)
      : undefined;
    if (existing?.sourceProviderId && (existing.sourceProviderId !== input.providerId || existing.sourceEndpointType !== input.endpointType)) {
      throw new Error("The provider of an existing OnePiece Profile cannot be changed.");
    }
    const credentialPresent = Boolean(input.apiKey?.trim()) || Boolean(existing?.credentialPresent);
    if (!credentialPresent) {
      throw new Error("API key is required for a new OnePiece provider Profile.");
    }
    const id = existing?.id ?? `onepiece-profile-${nextOnePieceProviderProfileId++}`;
    const active = existing?.active ?? webOnePieceProviderProfiles.profiles.length === 0;
    const profile = {
      id,
      name,
      sourceProviderId: input.providerId,
      sourceEndpointType: input.endpointType,
      sourcePresetVersion: preset.catalogVersion,
      provider: preset.provider,
      modelId,
      interfaceFormat: preset.interfaceFormat,
      baseUrl: preset.baseUrl,
      active,
      credentialPresent,
    };
    webOnePieceProviderProfiles = {
      activeProfileId: webOnePieceProviderProfiles.activeProfileId,
      profiles: existing
        ? webOnePieceProviderProfiles.profiles.map((candidate) => candidate.id === id ? profile : candidate)
        : [...webOnePieceProviderProfiles.profiles, profile],
    };
    if (active) applyWebOnePieceActiveProfile(id);
    return structuredClone(webOnePieceProviderProfiles);
  },

  async activateOnePieceProviderProfile(profileId: string) {
    const profile = webOnePieceProviderProfiles.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    if (!profile.credentialPresent) {
      throw new Error("The selected OnePiece provider Profile has no API key.");
    }
    applyWebOnePieceActiveProfile(profileId);
    return structuredClone(webOnePieceProviderProfiles);
  },

  async deleteOnePieceProviderProfile(profileId: string) {
    const profile = webOnePieceProviderProfiles.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    webOnePieceProviderProfiles = {
      activeProfileId: profile.active ? null : webOnePieceProviderProfiles.activeProfileId,
      profiles: webOnePieceProviderProfiles.profiles.filter((candidate) => candidate.id !== profileId),
    };
    if (profile.active) applyWebOnePieceActiveProfile(null);
    return structuredClone(webOnePieceProviderProfiles);
  },

  async updateApiAgent(agentId: string, input: UpdateApiAgentInput) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    const current = webApiAgentProviderConfigs.get(agentId);
    if (!agent || !current) {
      throw new Error(i18n.t("agents.updateApiAgent.errors.notFound"));
    }
    const displayName = input.displayName.trim();
    const modelId = input.modelId.trim();
    if (!displayName || !modelId) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.incomplete"));
    }
    const baseUrl = input.baseUrl?.trim() || null;
    if (current.interfaceFormat === "openai-compatible" && !baseUrl) {
      throw new Error(i18n.t("agents.registerApiAgent.errors.baseUrlRequired"));
    }
    agent.displayName = displayName;
    webApiAgentProviderConfigs.set(agentId, { ...current, modelId, baseUrl });
    return agent;
  },

  async deleteApiAgent(agentId: string) {
    if (mockAgents.find((agent) => agent.id === agentId)?.agentOrigin === "builtin") {
      throw new Error("Built-in agents cannot be deleted; reset their provider configuration instead.");
    }
    const blocking: string[] = [];
    const sessionCount = sessions.filter((session) => session.agentId === agentId).length;
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
    webApiAgentProviderConfigs.delete(agentId);
    webSkillApiAgentBindings = webSkillApiAgentBindings.filter((binding) => binding.agentId !== agentId);
    webSkills = webSkills.map((skill) => ({
      ...skill,
      boundAgentIds: skill.boundAgentIds.filter((boundAgentId) => boundAgentId !== agentId),
    }));
    webSkillMountPaths = webSkillMountPaths.filter((path) => path.agentId !== agentId);
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

  async getRetrievalConfiguration() {
    return { ...webRetrievalConfiguration };
  },

  async saveRetrievalConfiguration(profileId: string, modelId: string) {
    webRetrievalConfiguration = {
      ...webRetrievalConfiguration,
      sourceProfileId: profileId,
      embeddingModel: modelId,
    };
  },

  async saveCodeIndexAutomaticMode(mode: CodeIndexAutomaticMode) {
    webRetrievalConfiguration = { ...webRetrievalConfiguration, automaticCodeIndexMode: mode };
  },

  async listEmbeddingModels(profileId: string, transientCredential?: string) {
    const profile = webOnePieceProviderProfiles.profiles.find((candidate) => candidate.id === profileId);
    if (!profile) throw new Error("OnePiece provider profile was not found.");
    if (profile.interfaceFormat !== "openai-compatible") {
      throw new Error("Only openai-compatible OnePiece profiles support embedding model discovery.");
    }
    if (!transientCredential?.trim() && !profile.credentialPresent) {
      throw new Error("API key is required to list embedding models for this OnePiece provider.");
    }
    return webEmbeddingModelOptions.map((option) => ({ ...option }));
  },

  async getRetrievalIndexStatus() {
    return { ...webRetrievalIndexStatus };
  },

  async rebuildRetrievalIndex() {
    const status = webRetrievalIndexStatus;
    status.pending += status.indexed + status.failed;
    status.indexed = 0;
    status.failed = 0;
    status.lastFailureCategory = null;
  },

  async listCodeIndexWorkspaces() {
    return [...webCodeIndexes.values()].map(cloneCodeIndex);
  },

  async getCodeIndexWorkspace(workspaceId: string) {
    return cloneCodeIndex(requireWebCodeIndex(workspaceId));
  },

  async registerCodeIndexWorkspace(root: string, displayName: string) {
    const canonicalRoot = root.trim();
    const name = displayName.trim();
    if (!canonicalRoot || !name) throw new Error("Workspace root and display name are required.");
    const existing = [...webCodeIndexes.values()].find((workspace) => workspace.canonicalRoot === canonicalRoot);
    if (existing) return cloneCodeIndex(existing);
    const workspace: CodeIndexWorkspace = {
      workspaceId: `web-code-index-${nextWebCodeIndexId}`,
      canonicalRoot,
      displayName: name,
      origin: "manual",
      enabled: false,
      mode: "local",
      selectedRoots: [""],
      languages: [...codeIndexLanguages],
      exclusionPatterns: [],
      maxFileBytes: 100 * 1024,
      indexVersion: "1",
      generation: 0,
      status: emptyCodeIndexStatus("disabled"),
    };
    nextWebCodeIndexId += 1;
    webCodeIndexes.set(workspace.workspaceId, workspace);
    return cloneCodeIndex(workspace);
  },

  async saveCodeIndexConfiguration(workspaceId: string, configuration: CodeIndexConfigurationInput) {
    const workspace = requireWebCodeIndex(workspaceId);
    const normalized = normalizeCodeIndexConfiguration(configuration);
    Object.assign(workspace, normalized);
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus(normalized.enabled ? "scanning" : "disabled");
    return cloneCodeIndex(workspace);
  },

  async refreshCodeIndexWorkspace(workspaceId: string) {
    const workspace = requireWebCodeIndex(workspaceId);
    if (workspace.mode === "local" && workspace.status.phase === "parsing") {
      Object.assign(workspace.status, {
        totalFiles: 18,
        processedFiles: 18,
        failedFiles: 0,
        totalChunks: 54,
        processedChunks: 0,
        pendingChunks: 54,
        indexedChunks: 0,
        failedChunks: 0,
        redactionCount: 4,
        estimatedEmbeddingRequests: 0,
      });
      updateWebCodeIndexPhase(workspace, "ready");
      return structuredClone(workspace.status);
    }
    const nextPhase: Partial<Record<CodeIndexPhase, CodeIndexPhase>> = {
      scanning: "parsing",
      parsing: "awaiting_embedding_confirmation",
      embedding: "ready",
      degraded: "scanning",
      cancelling: "disabled",
    };
    updateWebCodeIndexPhase(workspace, nextPhase[workspace.status.phase] ?? workspace.status.phase);
    return structuredClone(workspace.status);
  },

  async confirmCodeIndexEmbedding(workspaceId: string, profileId: string, model: string, generation: number) {
    const workspace = requireWebCodeIndex(workspaceId);
    if (workspace.mode !== "semantic") {
      throw new Error("Local code indexes do not use embedding confirmation.");
    }
    if (!workspace.enabled || generation !== workspace.generation
      || webRetrievalConfiguration.sourceProfileId !== profileId
      || webRetrievalConfiguration.embeddingModel !== model) {
      throw new Error("Embedding confirmation is stale or does not match the active model.");
    }
    updateWebCodeIndexPhase(workspace, "embedding");
    return { profileId, model, generation };
  },

  async getCodeIndexStatus(workspaceId: string) {
    return structuredClone(requireWebCodeIndex(workspaceId).status);
  },

  async listCodeIndexAudit(workspaceId: string, limit = 50) {
    requireWebCodeIndex(workspaceId);
    const boundedLimit = Math.max(0, Math.min(100, Math.trunc(limit)));
    return structuredClone(webCodeIndexAudit.filter((entry) => entry.workspaceId === workspaceId).slice(0, boundedLimit));
  },

  async rebuildCodeIndexWorkspace(workspaceId: string) {
    const workspace = requireWebCodeIndex(workspaceId);
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus(workspace.enabled ? "scanning" : "disabled");
    recordWebCodeIndexAudit(workspaceId, "rebuilt");
    return cloneCodeIndex(workspace);
  },

  async disableCodeIndexWorkspace(workspaceId: string) {
    const workspace = requireWebCodeIndex(workspaceId);
    workspace.enabled = false;
    workspace.generation += 1;
    workspace.status = emptyCodeIndexStatus("disabled");
    return cloneCodeIndex(workspace);
  },

  async deleteCodeIndexWorkspace(workspaceId: string) {
    requireWebCodeIndex(workspaceId);
    webCodeIndexes.delete(workspaceId);
    webCodeIndexAudit = webCodeIndexAudit.filter((entry) => entry.workspaceId !== workspaceId);
  },

  async listCliTools() {
    return webCliTools.map((tool) => ({
      ...tool,
      availableVersions: [...tool.availableVersions],
      installations: tool.installations.map((installation) => ({ ...installation })),
      lastError: webLocalCliDetectionMessage(),
    }));
  },

  async refreshCliDetections(agentId?: string): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webLocalCliDetectionMessage();
    const operationId = `web-cli-refresh-${timestamp}`;
    return createWebMockOperation({
      id: operationId,
      relatedEntityId: agentId ?? null,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentIds: agentId ? [agentId] : webCliTools.map((tool) => tool.agentId) },
    });
  },

  async installCliVersion(input): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webCliPackageOperationsMessage();
    const operationId = `web-cli-install-${input.agentId}-${timestamp}`;
    return createWebMockOperation({
      id: operationId,
      relatedEntityId: input.agentId,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentId: input.agentId, targetVersion: input.targetVersion },
    });
  },

  async upgradeAllCliVersions(): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webCliPackageOperationsMessage();
    return createWebMockOperation({
      id: `web-cli-upgrade-all-${timestamp}`,
      relatedEntityId: null,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentIds: webCliTools.map((tool) => tool.agentId) },
    });
  },

  async listCliParameterProfiles() {
    const stored = readCliParameterSelections();
    return managedCliAgentIds.map((agentId) => createCliParameterProfile(agentId, stored[agentId]));
  },

  async saveCliParameterProfile(input) {
    const selections = normalizeCliParameterSelections(input.agentId, input.selections);
    writeCliParameterSelections({ ...readCliParameterSelections(), [input.agentId]: selections });
    return createCliParameterProfile(input.agentId, selections);
  },

  async resetCliParameterProfile(agentId) {
    const stored = { ...readCliParameterSelections() };
    delete stored[agentId];
    writeCliParameterSelections(stored);
    return createCliParameterProfile(agentId, defaultCliParameterSelections(agentId));
  },

  async listCliConfigPresets(agentId) {
    return getCliConfigPresets(requireCliConfigAgentId(agentId));
  },

  async listCliConfigProfiles(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    const status = webCliConfigStatus(supportedAgentId);
    return webCliConfigProfiles
      .filter((profile) => profile.agentId === supportedAgentId)
      .map((profile) => ({
        ...structuredClone(profile),
        appliedState:
          status.appliedProfileId === profile.id
            ? status.driftState === "applied"
              ? "applied" as const
              : "drifted" as const
            : "saved" as const,
      }));
  },

  async getCliConfigStatus(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    return structuredClone(webCliConfigStatus(supportedAgentId));
  },

  async saveCliConfigProfile(input) {
    return saveWebCliConfigProfile(input);
  },

  async validateCliConfigCredential(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const profile = input.profileId
      ? webCliConfigProfiles.find((candidate) => candidate.agentId === supportedAgentId && candidate.id === input.profileId)
      : undefined;
    if (input.profileId && !profile) throw new Error("Profile not found.");
    const payload = input.payload ?? profile?.payload;
    if (!payload || payload.kind !== supportedAgentId) throw new Error("A complete provider configuration is required.");
    if (!cliConfigNeedsCredential(payload)) {
      return { status: "unsupported" as const, latencyMs: 0, httpStatus: null };
    }
    const credential = input.credential?.trim();
    if (!credential && !profile?.credentialConfigured) throw new Error("Credential repair is required.");
    const status = credential === "web-invalid"
      ? "invalid-credential" as const
      : credential === "web-rate-limited"
        ? "rate-limited" as const
        : credential === "web-unavailable"
          ? "provider-unavailable" as const
          : "valid" as const;
    return { status, latencyMs: 12, httpStatus: status === "valid" ? 200 : status === "invalid-credential" ? 401 : status === "rate-limited" ? 429 : null };
  },

  async duplicateCliConfigProfile(agentId, profileId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    const source = webCliConfigProfiles.find(
      (profile) => profile.agentId === supportedAgentId && profile.id === profileId,
    );
    if (!source) throw new Error("Profile not found.");
    let name = `${source.name} Copy`;
    let suffix = 2;
    while (webCliConfigProfiles.some((profile) => profile.agentId === supportedAgentId && profile.name === name)) {
      name = `${source.name} Copy ${suffix++}`;
    }
    return saveWebCliConfigProfile({
      agentId: supportedAgentId,
      name,
      payload: cloneCliConfigPayload(source.payload),
      sourcePresetId: source.sourcePresetId,
      sourcePresetVersion: source.sourcePresetVersion,
      credential: source.credentialConfigured ? "web-simulated-credential" : null,
    });
  },

  async deleteCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const status = webCliConfigStatus(supportedAgentId);
    const exists = webCliConfigProfiles.some(
      (profile) => profile.agentId === supportedAgentId && profile.id === input.profileId,
    );
    if (!exists) throw new Error("Profile not found.");
    if (status.appliedProfileId === input.profileId && !input.detachApplied) {
      throw new Error("Applied profile must be detached before deletion.");
    }
    if (status.appliedProfileId === input.profileId) {
      webCliConfigStatuses.set(supportedAgentId, defaultWebCliConfigStatus(supportedAgentId));
    }
    webCliConfigProfiles = webCliConfigProfiles.filter((profile) => profile.id !== input.profileId);
  },

  async importCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const preset = getCliConfigPresets(supportedAgentId)[0];
    if (!preset) throw new Error("No simulated global configuration is available.");
    return saveWebCliConfigProfile({
      agentId: supportedAgentId,
      name: input.name,
      payload: cloneCliConfigPayload(preset.payload),
      sourcePresetId: null,
      sourcePresetVersion: null,
      credential: cliConfigNeedsCredential(preset.payload) ? "web-imported-credential" : null,
    });
  },

  async discoverCliConfigProfiles(agentId) {
    const supportedAgentId = requireCliConfigAgentId(agentId);
    return {
      agentId: supportedAgentId,
      state: "unavailable" as const,
      candidates: [],
      resolvedPaths: [],
      warnings: [],
      error: null,
      simulated: true,
    };
  },

  async importDiscoveredCliConfigProfiles(input) {
    requireCliConfigAgentId(input.agentId);
    if (input.candidateKeys.length > 0) {
      throw new Error("Local configuration discovery is unavailable in Web mode.");
    }
    return { imported: [], skipped: [] };
  },

  async applyCliConfigProfile(input) {
    const supportedAgentId = requireCliConfigAgentId(input.agentId);
    const profile = webCliConfigProfiles.find(
      (candidate) => candidate.agentId === supportedAgentId && candidate.id === input.profileId,
    );
    if (!profile) throw new Error("Profile not found.");
    if (profile.validationState === "needs-credential") throw new Error("Credential repair is required.");
    const beforeWorkflow = JSON.stringify(workflowState);
    const beforeActiveSession = activeSessionId;
    const status = webCliConfigStatus(supportedAgentId);
    const backfilledProfileId = supportedAgentId !== "opencode"
      && status.appliedProfileId !== null
      && status.appliedProfileId !== profile.id
      ? status.appliedProfileId
      : null;
    const timestamp = nowIso();
    webCliConfigStatuses.set(supportedAgentId, {
      agentId: supportedAgentId,
      appliedProfileId: profile.id,
      driftState: "applied",
      resolvedPaths: [],
      lastAppliedAt: timestamp,
      simulated: true,
      startupSync: status.startupSync,
    });
    if (JSON.stringify(workflowState) !== beforeWorkflow || activeSessionId !== beforeActiveSession) {
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

  async getAgentById(agentId) {
    return mockAgents.find((agent) => agent.id === agentId) ?? null;
  },

  async getWorkflowState() {
    return workflowState;
  },

  async selectAgent(agentId: string, interactionMode: InteractionMode) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    if (!agent) {
      throw new Error(`Agent not found: ${agentId}`);
    }
    if (!agent.supportedInteractionModes.includes(interactionMode)) {
      throw new Error(`${agent.displayName} does not support ${interactionMode}.`);
    }
    workflowState = {
      ...workflowState,
      activeAgentId: agentId,
      activeInteractionMode: interactionMode,
      lifecycleState: "idle",
    };
    return workflowState;
  },

  async checkBrowserReadiness(agentId: string) {
    const agent = mockAgents.find((candidate) => candidate.id === agentId);
    const supportsBrowser = agent?.supportedInteractionModes.includes("browser") ?? false;
    return {
      ready: supportsBrowser,
      reason: supportsBrowser ? undefined : "This agent does not support browser interaction mode.",
      requiresAuthentication: supportsBrowser,
    };
  },

  async launchActiveWorkflow() {
    workflowState = {
      ...workflowState,
      lifecycleState: workflowState.activeAgentId ? "running" : "failed",
    };
    return {
      workflow: workflowState,
      message: workflowState.activeAgentId
        ? "Web preview session marked as running."
        : "Select an agent before launching.",
    };
  },

  async getSessionDetails(): Promise<SessionDetails> {
    const adapter = workflowState.activeInteractionMode ?? "none";
    return {
      agentId: workflowState.activeAgentId,
      interactionMode: workflowState.activeInteractionMode,
      lifecycleState: workflowState.lifecycleState,
      adapter,
      details: {
        runtime: "web",
        storage: "in-memory",
      },
    };
  },

  async listSessions() {
    return sortSessions(sessions.filter((session) => !session.archived && !loopRoleSessionIds.has(session.id)));
  },

  async listArchivedSessions() {
    return sortSessions(sessions.filter((session) => session.archived && !loopRoleSessionIds.has(session.id)));
  },

  async searchSessions(input: SessionSearchInput) {
    const query = input.query.trim();
    if (!query) return [];
    return sortSessions(sessions.filter((session) => !loopRoleSessionIds.has(session.id)))
      .map((session) => sessionSearchMatches(session, query))
      .filter((result): result is SessionSearchResult => result !== null)
      .slice(0, input.limit ?? 50);
  },

  async getSession(sessionId: string) {
    return findSession(sessionId);
  },

  async getSessionRecoverySummary(sessionId: string) {
    return structuredClone(webRecoverySummary(sessionId));
  },

  async listSessionRecoveryReports(sessionId: string, limit = 20) {
    findSession(sessionId);
    const boundedLimit = Math.max(1, Math.min(100, Math.trunc(limit)));
    return structuredClone((recoveryReportsBySession.get(sessionId) ?? []).slice(0, boundedLimit));
  },

  async acknowledgeSessionRecovery(
    sessionId: string,
    expectedRecoveryRevision: number,
  ): Promise<SessionRecoveryAcknowledgement> {
    const session = findSession(sessionId);
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
    const updated = updateSession(sessionId, {
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
    emitSessionEvent({
      kind: "recovery-acknowledged",
      sessionId,
      recoveryRevision,
    });
    return structuredClone({ session: updated, report });
  },

  async getActiveSession() {
    if (!activeSessionId) return null;
    return sessions.find((session) => session.id === activeSessionId) ?? null;
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
    sessions = sessions.map((session) => (session.categoryId === categoryId ? { ...session, categoryId: null, updatedAt: nowIso() } : session));
  },

  async assignSessionCategory(input: AssignSessionCategoryInput) {
    if (input.categoryId) findCategory(input.categoryId);
    return updateSession(input.sessionId, { categoryId: input.categoryId });
  },

  async getAutomaticArchivalSettings() {
    return { ...automaticArchivalSettings };
  },

  async saveAutomaticArchivalSettings(input: AutomaticArchivalSettings) {
    if (input.inactiveDays < 1 || input.inactiveDays > 3650) {
      throw new Error("Invalid automatic archival threshold.");
    }
    automaticArchivalSettings = { ...input };
    return { ...automaticArchivalSettings };
  },

  async listScheduledTasks() {
    return scheduledTasks.map(cloneScheduledTask).sort((left, right) => left.nextRunAt.localeCompare(right.nextRunAt));
  },
  async listScheduledTaskRuns(taskId: string) {
    const task = findScheduledTask(taskId);
    if (!task.latestRunAt) return [];
    return [{ id: `scheduled-run:${task.id}:${task.latestRunAt}`, taskId: task.id, sessionId: task.latestRunSessionId, status: task.latestStatus, error: task.latestError, startedAt: task.latestRunAt, completedAt: task.latestRunAt }] satisfies ScheduledTaskRun[];
  },

  async createScheduledTask(input: CreateScheduledTaskInput) {
    const { name, content } = validateScheduledTaskInput(input);
    const timestamp = nowIso();
    const task: ScheduledTask = {
      id: `web-scheduled-task-${nextScheduledTaskId++}`,
      name,
      content,
      agentId: input.agentId,
      frequency: { ...input.frequency },
      enabled: true,
      nextRunAt: computeNextScheduledRun(input.frequency),
      latestStatus: "never-run",
      latestRunAt: null,
      latestRunSessionId: null,
      latestError: null,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    scheduledTasks = [task, ...scheduledTasks];
    return cloneScheduledTask(task);
  },

  async setScheduledTaskEnabled(input: SetScheduledTaskEnabledInput) {
    const task = findScheduledTask(input.taskId);
    const timestamp = nowIso();
    const updated: ScheduledTask = {
      ...task,
      enabled: input.enabled,
      nextRunAt: input.enabled ? computeNextScheduledRun(task.frequency) : task.nextRunAt,
      updatedAt: timestamp,
    };
    scheduledTasks = scheduledTasks.map((candidate) => (candidate.id === input.taskId ? updated : candidate));
    return cloneScheduledTask(updated);
  },

  async deleteScheduledTask(taskId: string) {
    findScheduledTask(taskId);
    scheduledTasks = scheduledTasks.filter((task) => task.id !== taskId);
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
    const session = findSession(sessionId);
    const stored = readChatConfigs()[sessionId];
    const normalized = stored
      ? normalizeChatConfigForSession(session, stored)
      : defaultChatConfigForSession(session);
    const policy = webPrincipalTemplates.get(session.agentId) ?? getWebDefaultPolicyTemplate();
    return withEffectiveExecutionPolicy(normalized, policy);
  },

  async saveSessionChatConfig(sessionId, config) {
    const session = findSession(sessionId);
    const normalized = normalizeChatConfigForSession(session, config);
    writeChatConfigs({ ...readChatConfigs(), [sessionId]: normalized });
    emitSessionEvent({ kind: "configuration-changed", sessionId });
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
      id: `web-session-${nextSessionId}`,
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
    nextSessionId += 1;
    sessions = [session, ...sessions];
    activeSessionId = session.id;
    discoverWebSessionCodeIndex(session);
    emitSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    workflowState = {
      ...workflowState,
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    };
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
    findSession(sessionId);
    cancelActiveStream(sessionId);
    messagesBySession.delete(sessionId);
    recoveryReportsBySession.delete(sessionId);
    subscribersBySession.delete(sessionId);
    const configs = { ...readChatConfigs() };
    delete configs[sessionId];
    writeChatConfigs(configs);
    sessions = sessions.filter((session) => session.id !== sessionId);
    if (activeSessionId === sessionId) {
      activeSessionId = null;
      emitSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
  },

  async switchSession(sessionId: string) {
    const session = findSession(sessionId);
    if (session.archived) {
      throw new Error(`Cannot switch to archived session: ${sessionId}`);
    }
    activeSessionId = session.id;
    emitSessionEvent({ kind: "active-session-changed", sessionId: session.id });
    workflowState = {
      ...workflowState,
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    };
    return session;
  },

  async renameSession(sessionId: string, title: string) {
    const trimmedTitle = title.trim();
    if (!trimmedTitle) {
      throw new Error(tr("web.error.sessionTitleRequired"));
    }
    return updateSession(sessionId, { title: trimmedTitle });
  },

  async updateSessionSeats(input: UpdateSessionSeatsInput) {
    const session = findSession(input.sessionId);
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
    return updateSession(input.sessionId, { seats, agentId: firstActive.agentId });
  },

  async rebindRemoteSessionSshConnection(
    sessionId: string,
    connectionId: string,
  ) {
    const session = findSession(sessionId);
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
    return updateSession(sessionId, {
      remoteSshConnectionId: connection.id,
      remoteSshConnectionRevision: connection.revision,
    });
  },

  async pinSession(sessionId: string) {
    return updateSession(sessionId, { pinned: true });
  },

  async unpinSession(sessionId: string) {
    return updateSession(sessionId, { pinned: false });
  },

  async archiveSession(sessionId: string) {
    const cancelled = cancelActiveStream(sessionId);
    const session = updateSession(sessionId, { archived: true, ...(cancelled ? { lifecycleState: "stopped" } : {}) });
    if (activeSessionId === sessionId) {
      activeSessionId = null;
      emitSessionEvent({ kind: "active-session-changed", sessionId: null });
    }
    return session;
  },

  async unarchiveSession(sessionId: string) {
    return updateSession(sessionId, { archived: false });
  },

  async exportSession(input: ExportSessionInput) {
    return serializeWebSessionExport(input);
  },

  async sendMessage(input) {
    const session = findSession(input.sessionId);
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
    const timestamp = nowIso();
    const activeSeats = (session.seats ?? []).filter((seat) => seat.leftAt == null);
    const firstSpeakerSeatId = activeSeats.length > 1 ? activeSeats[0]?.seatId : undefined;
    const existingMessages = getSessionMessages(input.sessionId);
    const nextSequence = existingMessages.reduce(
      (maximum, message) => Math.max(maximum, message.sessionSequence),
      0,
    ) + 1;
    const executionRunId = `web-run-${input.sessionId}-${Date.now()}`;
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
    updateSession(input.sessionId, {
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
    const boundSkillNames = webSkillApiAgentBindings
      .filter((binding) => binding.agentId === session.agentId)
      .map((binding) =>
        webSkills.find(
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
    findSession(input.sessionId);
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

  async querySkillEvolutionEvidence(input) {
    const skillId = input.skillId ?? "review";
    const scenario = input.workspace?.startsWith("mock://") ? input.workspace.slice(7) : "healthy";
    const scopeKey = `${input.workspace ?? ""}|${skillId}`;
    const empty = scenario === "empty" || scenario === "disabled" || purgedEvidenceScopes.has(scopeKey);
    const allSignals = empty ? [] : [
      { signalId: "mock-signal-3", sourceKind: "plan_verification", category: "verification_outcome", polarity: "positive", severity: "low", attribution: "verified", attributionRationale: "exact_native_observation", sourceFidelity: "native", sourceAgentId: "onepiece", extractorId: "verification_outcome", extractorVersion: 1, safeSummary: "test verification passed; checks=8", occurredAt: "2026-08-13T10:03:00Z", sanitizerVersion: 1, associationTruncatedCount: 0, sourceLinkTruncatedCount: 0 },
      { signalId: "mock-signal-2", sourceKind: "managed_cli", category: "execution_failure", polarity: "negative", severity: "high", attribution: "correlated", attributionRationale: "binding_and_mount_snapshot", sourceFidelity: "proxied", sourceAgentId: "codex-cli", extractorId: "execution_failure", extractorVersion: 1, safeSummary: "process failed with sandbox classification", occurredAt: "2026-08-13T10:02:00Z", sanitizerVersion: 1, associationTruncatedCount: 1, sourceLinkTruncatedCount: 0 },
      { signalId: "mock-signal-1", sourceKind: "native_execution", category: "execution_failure", polarity: "negative", severity: "high", attribution: "verified", attributionRationale: "exact_native_observation", sourceFidelity: "native", sourceAgentId: "onepiece", extractorId: "execution_failure", extractorVersion: 1, safeSummary: `${skillId} tool failed with sandbox classification`, occurredAt: "2026-08-13T10:01:00Z", sanitizerVersion: 1, associationTruncatedCount: 0, sourceLinkTruncatedCount: 0 },
    ];
    const offset = input.cursor ? Number.parseInt(input.cursor.replace("mock-", ""), 10) || 0 : 0;
    const limit = Math.min(Math.max(input.limit ?? 50, 1), 100);
    const signals = allSignals.slice(offset, offset + limit);
    const distributions: Record<string, Record<string, number>> = empty ? {} : {
      category: { execution_failure: 2, verification_outcome: 1 },
      attribution_strength: { correlated: 1, verified: 2 },
      extractor_id: { execution_failure: 2, verification_outcome: 1 },
      source_agent_id: { "codex-cli": 1, onepiece: 2 },
    };
    return {
      signalCount: allSignals.length,
      seedCount: empty ? 0 : 1,
      firstOccurredAt: allSignals.at(-1)?.occurredAt,
      lastOccurredAt: allSignals[0]?.occurredAt,
      distributions,
      signals,
      seeds: empty ? [] : [{ seedId: "mock-seed-1", category: "execution_failure", readiness: "ready", readinessReason: "verified_recovery_delta", safeSummary: "execution_failure pattern; signals=3 runs=2 recovery=true", signalCount: 3, truncatedSignalCount: 0, independentRunCount: 2, hasRecovery: true, firstOccurredAt: "2026-08-13T10:01:00Z", lastOccurredAt: "2026-08-13T10:03:00Z", createdAt: "2026-08-13T10:04:00Z" }],
      nextCursor: offset + signals.length < allSignals.length ? `mock-${offset + signals.length}` : undefined,
      pipeline: { collectionEnabled: scenario !== "disabled", status: scenario === "disabled" ? "disabled" as const : scenario === "degraded" || scenario === "quota" ? "degraded" as const : "healthy" as const, queueDepth: scenario === "degraded" ? 12 : 0, failureCount: scenario === "degraded" ? 2 : 0 },
      retentionDays: 90,
      signalQuota: 10_000,
      seedQuota: 2_000,
      byteQuota: 64 * 1024 * 1024,
      droppedCount: scenario === "quota" ? 7 : 0,
      expiredCount: 0,
    };
  },

  async getSkillEvolutionSeedLineage(seedId, input) {
    const overview = await this.querySkillEvolutionEvidence(input);
    const seed = overview.seeds.find((candidate) => candidate.seedId === seedId);
    return seed ? { seed, signals: overview.signals } : null;
  },

  async purgeSkillEvolutionEvidence(input) {
    if (!input.confirmed) throw new Error("Evidence purge requires confirmation");
    const current = await this.querySkillEvolutionEvidence({ workspace: input.workspace, skillId: input.skillId });
    purgedEvidenceScopes.add(`${input.workspace ?? ""}|${input.skillId}`);
    return {
      operationId: input.operationId,
      deletedSignals: current.signalCount,
      deletedSeeds: current.seedCount,
      deletedFeedback: 0,
    };
  },

  async getUsageStatistics(input) {
    return aggregateWebUsageStatistics(input.range);
  },

  async getSessionUsageSummary(sessionId: string) {
    findSession(sessionId);
    const generated = aggregateSessionUsageRecords(representativeUsageRecords, sessionId);
    return generated;
  },

  async getTokenUsageSummary(input) {
    if (!input.sessionId) return queryWebTokenUsageSummary(input);
    const session = findSession(input.sessionId);
    return queryWebTokenUsageSummary({
      ...input,
      sessionId: session.agentId === "onepiece" ? "web-token-onepiece" : "web-token-cli",
    });
  },

  async getTokenUsageDetails(input) {
    if (!input.sessionId) return queryWebTokenUsageDetails(input);
    const session = findSession(input.sessionId);
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
    findSession(sessionId);
    if (!cancelActiveStream(sessionId)) return;
    updateSession(sessionId, { lifecycleState: "stopped" });
  },

  async openAgentTerminal(sessionId: string, size: AgentTerminalSize) {
    const session = findSession(sessionId);
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
    updateSession(sessionId, { lifecycleState: "running", runtimeSessionId });
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
    updateSession(terminal.sessionId, { lifecycleState: "stopped" });
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
    sessionEventSubscribers.add(handler);
    return () => sessionEventSubscribers.delete(handler);
  },

  async listSkills(input): Promise<SkillListResult> {
    const skills = webSkills.filter((skill) => skillScopeMatches(skill, input)).map(hydrateSkillBindings);
    return { skills, stats: skillStats(skills) };
  },

  async getSkillOverview(input): Promise<SkillOverview> {
    const { skills, stats } = await this.listSkills(input);
    const apiAgentBindings = Object.fromEntries(
      skills.map((skill) => [
        skill.id,
        webSkillApiAgentBindings
          .filter(
            (binding) =>
              binding.skillId === skill.id &&
              binding.scope === skill.scope &&
              binding.workspacePath === skill.workspacePath,
          )
          .map((binding) => binding.agentId),
      ]),
    );
    return {
      skills,
      stats,
      mountPaths: webSkillMountPaths.map((path) => ({ ...path })),
      agents: mockAgents.map((agent) => ({
        id: agent.id,
        displayName: agent.displayName,
        kind: agent.launch.kind === "api" ? "api" : "cli",
      })),
      apiAgentBindings,
      drift: await this.detectSkillDrift(input),
      restoreCandidates: input.scope === "global" ? [...deletedBuiltinSkillIds].sort() : [],
    };
  },

  async listSkillMountPaths() {
    return webSkillMountPaths.map((path) => ({ ...path }));
  },

  async updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport> {
    requireAgentKind(agentId, "cli");
    mountPath = validateMountPath(mountPath);
    const existing = webSkillMountPaths.find((path) => path.agentId === agentId);
    const oldMountPath = existing?.mountPath ?? mountPathForAgent(agentId);
    webSkillMountPaths = webSkillMountPaths.map((path) =>
      path.agentId === agentId ? { agentId, mountPath, isDefault: false } : path,
    );
    if (!existing) {
      webSkillMountPaths = [...webSkillMountPaths, { agentId, mountPath, isDefault: false }];
    }
    const migrated = webSkills
      .filter((skill) => skill.boundAgentIds.includes(agentId) && skill.enabled)
      .map((skill) => skill.id);
    return {
      agentId,
      oldMountPath,
      newMountPath: mountPath,
      migrated,
      removed: migrated.map((skillId) => `${oldMountPath}/${skillId}`),
      overwritten: [],
      backedUp: [],
      failed: [],
    };
  },

  async createSkill(input) {
    const normalized = validateWebSkillMutation(input, "user");
    if (webSkills.some((skill) => skill.id === normalized.id && skillScopeMatches(skill, normalized))) {
      throw new Error(`Skill already exists: ${normalized.id}`);
    }
    for (const agentId of normalized.boundAgentIds) requireAgentKind(agentId, "cli");
    const skill = mutationToSkill(normalized);
    return hydrateSkillBindings(upsertWebSkill(skill));
  },

  async updateSkill(skillId, input: SkillUpdateInput) {
    validateWebSkillMetadata(input.metadata);
    if (input.metadata.id !== skillId) {
      throw new Error(tr("web.error.skillIdImmutable"));
    }
    const current = findWebSkill(skillId, input);
    if (current.contentHash !== input.expectedContentHash) {
      throw new Error(`Skill changed since it was loaded: ${skillId}`);
    }
    webSkillDocuments.set(skillDocumentKey(current), input.body);
    const updated: Skill = {
      ...current,
      metadata: {
        ...input.metadata,
        aliases: input.metadata.aliases ?? [],
        type: input.metadata.type ?? "role",
        delivery: input.metadata.delivery ?? "eager",
        compatibilityDefaults: input.metadata.compatibilityDefaults ?? {
          skillType: input.metadata.type == null,
          delivery: input.metadata.delivery == null,
        },
      },
      availability: input.metadata.type === "utility" ? "unsupported" : "available",
      delegationCapability: input.metadata.type === "utility"
        ? { supported: false, reason: "native-runtime-unavailable" }
        : { supported: false, reason: "not-utility" },
      contentHash: nextWebSkillHash(skillId),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async deleteSkill(skillId, input) {
    const current = findWebSkill(skillId, input);
    if (current.source === "builtin") {
      deletedBuiltinSkillIds.add(skillId);
    }
    webSkills = webSkills.filter((skill) => !(skill.id === skillId && skillScopeMatches(skill, input)));
    webSkillDocuments.delete(skillDocumentKey(current));
    webSkillApiAgentBindings = webSkillApiAgentBindings.filter(
      (binding) =>
        !(
          binding.skillId === current.id &&
          binding.scope === current.scope &&
          binding.workspacePath === current.workspacePath
        ),
    );
  },

  async restoreBuiltinSkill(skillId) {
    const seed = builtinSkillSeeds.find((candidate) => candidate.id === skillId);
    if (!seed) {
      throw new Error(`Unknown built-in Skill: ${skillId}`);
    }
    if (!deletedBuiltinSkillIds.has(skillId)) {
      throw new Error(`Built-in Skill is not eligible for restore: ${skillId}`);
    }
    if (webSkills.some((skill) => skill.id === skillId && skill.scope === "global")) {
      throw new Error(`Skill already exists: ${skillId}`);
    }
    deletedBuiltinSkillIds.delete(skillId);
    const restored = {
      ...mutationToSkill({
      id: seed.id,
      scope: "global",
      workspacePath: null,
      metadata: {
        id: seed.id,
        name: seed.name,
        description: seed.description,
        category: seed.category,
        version: "1.0.0",
        triggers: seed.triggers,
      },
      body: `Web mock restored content for ${seed.id}.`,
      enabled: true,
      boundAgentIds: [],
      source: "builtin",
      }),
      layer: "system" as const,
      origin: "shipped" as const,
      immutable: true,
    };
    return hydrateSkillBindings(upsertWebSkill(restored));
  },

  async setSkillEnabled(skillId, input, enabled) {
    const current = findWebSkill(skillId, input);
    const availability = !enabled
      ? "disabled" as const
      : current.metadata.type === "utility" ? "unsupported" as const : "available" as const;
    const updated = { ...current, enabled, availability, updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async setSkillAgentBindings(skillId, input, agentIds) {
    for (const agentId of agentIds) requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    const updated = { ...current, boundAgentIds: [...agentIds], updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async bindSkillToCliAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    if (current.boundAgentIds.includes(agentId)) return hydrateSkillBindings(current);
    const updated = {
      ...current,
      boundAgentIds: [...current.boundAgentIds, agentId].sort(),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async unbindSkillFromCliAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "cli");
    const current = findWebSkill(skillId, input);
    const updated = {
      ...current,
      boundAgentIds: current.boundAgentIds.filter((id) => id !== agentId),
      updatedAt: nowIso(),
    };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async bindSkillToApiAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "api");
    const skill = findWebSkill(skillId, input);
    const alreadyBound = webSkillApiAgentBindings.some(
      (binding) =>
        binding.skillId === skill.id &&
        binding.scope === skill.scope &&
        binding.workspacePath === skill.workspacePath &&
        binding.agentId === agentId,
    );
    if (!alreadyBound) {
      webSkillApiAgentBindings = [
        ...webSkillApiAgentBindings,
        { skillId: skill.id, scope: skill.scope, workspacePath: skill.workspacePath, agentId },
      ];
    }
  },

  async unbindSkillFromApiAgent(skillId, input, agentId) {
    requireAgentKind(agentId, "api");
    const skill = findWebSkill(skillId, input);
    webSkillApiAgentBindings = webSkillApiAgentBindings.filter(
      (binding) =>
        !(
          binding.skillId === skill.id &&
          binding.scope === skill.scope &&
          binding.workspacePath === skill.workspacePath &&
          binding.agentId === agentId
        ),
    );
  },

  async listSkillApiAgentBindings(skillId, input) {
    const skill = findWebSkill(skillId, input);
    return webSkillApiAgentBindings
      .filter(
        (binding) =>
          binding.skillId === skill.id &&
          binding.scope === skill.scope &&
          binding.workspacePath === skill.workspacePath,
      )
      .map((binding) => binding.agentId);
  },

  async previewSkill(skillId, input): Promise<SkillPreview> {
    const skill = hydrateSkillBindings(findWebSkill(skillId, input));
    return {
      id: skill.id,
      scope: skill.scope,
      workspacePath: skill.workspacePath,
      path: skill.skillMdPath,
      content: buildSkillContent(skill),
      layer: skill.layer,
      origin: skill.origin,
      availability: skill.availability,
      immutable: skill.immutable,
      shadowedDefinitions: skill.shadowedDefinitions.map((definition) => ({ ...definition })),
    };
  },

  async loadSkill(input: SkillLoadInput): Promise<SkillLoadOutcome> {
    const resolved = findProgressiveWebSkill(input);
    if ("status" in resolved) return resolved;
    const body = webSkillDocuments.get(skillDocumentKey(resolved)) ?? "";
    const baseUri = `skill://${resolved.id}/`;
    const expanded = body.replaceAll("{skill_base_dir}", baseUri);
    const characters = [...expanded];
    const timestamp = nowIso();
    upsertWebSkill({
      ...resolved,
      usage: {
        ...resolved.usage,
        viewCount: resolved.usage.viewCount + 1,
        lastViewedAt: timestamp,
        revisionWitness: `${resolved.usage.revisionWitness ?? "web-usage"}-view`,
      },
    });
    return {
      status: "loaded",
      result: {
        id: resolved.id,
        name: resolved.metadata.name,
        content: characters.slice(0, 12_000).join(""),
        truncated: characters.length > 12_000,
        revision: resolved.contentHash,
        baseUri,
        resources: webSkillResources(resolved.id),
      },
    };
  },

  async readSkillResource(input: SkillResourceReadInput): Promise<SkillResourceReadOutcome> {
    const match = /^skill:\/\/([a-z0-9]+(?:-[a-z0-9]+)*)\/(.+)$/.exec(input.uri);
    if (!match) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: null, reason: "invalid-uri", conflictingIds: [] },
      };
    }
    const skillId = match[1];
    const resolved = findProgressiveWebSkill({ idOrAlias: skillId, workspacePath: input.workspacePath });
    if ("status" in resolved) return resolved;
    if (resolved.contentHash !== input.revision) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: skillId, reason: "stale-revision", conflictingIds: [] },
      };
    }
    const content = webSkillResourceDocuments.get(input.uri);
    if (content == null || !webSkillResources(skillId).references.concat(
      webSkillResources(skillId).templates,
      webSkillResources(skillId).scripts,
      webSkillResources(skillId).assets,
    ).some((entry) => entry.uri === input.uri)) {
      return {
        status: "refused",
        refusal: { requested: input.uri, canonicalId: skillId, reason: "unindexed-resource", conflictingIds: [] },
      };
    }
    return {
      status: "read",
      result: {
        id: skillId,
        uri: input.uri,
        revision: input.revision,
        content,
        sizeBytes: new TextEncoder().encode(content).byteLength,
      },
    };
  },

  async importSkill(input: SkillImportInput) {
    const sourcePath = normalizeWebPath(input.sourcePath, "External Skill directory");
    const id = sourcePath.split("/").at(-1) ?? "";
    const location = normalizeWebSkillLocation(input);
    const destinationRoot = location.scope === "global"
      ? "~/.vanehub/skills"
      : `${location.workspacePath}/.vanehub/skills`;
    const destination = normalizeWebPath(`${destinationRoot}/${id}`, "Managed Skill destination");
    if (webPathsOverlap(sourcePath, destination)) {
      throw new Error("External Skill source overlaps the managed Skill destination");
    }
    const mutation = validateWebSkillMutation({
      id,
      scope: location.scope,
      workspacePath: location.workspacePath,
      metadata: {
        id,
        name: id,
        description: tr("web.skill.importedDescription"),
        category: "imported",
        version: "1.0.0",
        triggers: [],
      },
      body: tr("web.skill.importedBody"),
      enabled: input.enabled,
      boundAgentIds: input.boundAgentIds,
      source: "imported",
    }, "imported");
    if (webSkills.some((skill) => skill.id === id && skillScopeMatches(skill, mutation))) {
      throw new Error(`Skill already exists: ${id}`);
    }
    for (const agentId of mutation.boundAgentIds) requireAgentKind(agentId, "cli");
    return hydrateSkillBindings(upsertWebSkill(mutationToSkill(mutation)));
  },

  async detectSkillDrift(input): Promise<SkillDriftReport> {
    const location = normalizeWebSkillLocation(input);
    const issues: SkillDriftReport["issues"] = [];
    return {
      scope: location.scope,
      workspacePath: location.workspacePath ?? null,
      issues,
      driftHash: `web-${issues.length}`,
    };
  },

  async syncSkillDrift(input): Promise<SkillSyncResult> {
    const report = await this.detectSkillDrift(input);
    return {
      mounted: [],
      unmounted: [],
      overwritten: [],
      backedUp: [],
      restored: [],
      failed: [],
      resolvedFrom: report,
    };
  },

  async getSkillOverlaySummary(input) {
    return webSkillOverlayRuntime.getSummary(input);
  },

  async getSkillOverlayDetail(input) {
    return webSkillOverlayRuntime.getDetail(input);
  },

  async previewSkillOverlay(input) {
    return webSkillOverlayRuntime.preview(input);
  },

  async getSkillOverlayHistory(input) {
    return webSkillOverlayRuntime.getHistory(input);
  },

  async createSkillOverlayPatch(input) {
    return webSkillOverlayRuntime.createPatch(input);
  },

  async createSkillOverlayGuidance(input) {
    return webSkillOverlayRuntime.createGuidance(input);
  },

  async addSkillOverlayFile(input) {
    return webSkillOverlayRuntime.addFile(input);
  },

  async replaceSkillOverlayFile(input) {
    return webSkillOverlayRuntime.replaceFile(input);
  },

  async importSkillOverlay(input) {
    return webSkillOverlayRuntime.importOverlay(input);
  },

  async promoteSkillOverlay(input) {
    return webSkillOverlayRuntime.promote(input);
  },

  async disableSkillOverlayMutation(input) {
    return webSkillOverlayRuntime.disable(input);
  },

  async revertSkillOverlayMutation(input) {
    return webSkillOverlayRuntime.revert(input);
  },

  async previewSkillOverlayReconciliation(input) {
    return webSkillOverlayRuntime.previewReconciliation(input);
  },

  async reconcileSkillOverlay(input) {
    return webSkillOverlayRuntime.reconcile(input);
  },

  async listPromptHooks(): Promise<PromptHookListResult> {
    const hooks = listEffectivePromptHooks();
    return { hooks, stats: promptHookStats(hooks) };
  },

  async createPromptHook(input: PromptHookMutationInput): Promise<PromptHook> {
    const stored = readStoredPromptHooks();
    if (listEffectivePromptHooks().some((hook) => hook.id === input.id)) {
      throw new Error(`Prompt Hook already exists: ${input.id}`);
    }
    const created = mutationToPromptHook(input);
    const hook: PromptHook = {
      ...created,
      version: 0,
      publishedVersion: null,
      hasDraft: true,
      draftRevision: 1,
      enabled: false,
    };
    writeStoredPromptHooks({ ...stored, [hook.id]: hook });
    saveWebPromptHookDraft({
      hookId: hook.id,
      expectedRevision: null,
      draft: input,
    });
    return hook;
  },

  async updatePromptHook(hookId: string, input: PromptHookUpdateInput): Promise<PromptHook> {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      throw new Error("Built-in Prompt Hook content cannot be edited");
    }
    if (input.id !== hookId) {
      throw new Error("Prompt Hook id cannot be changed");
    }
    validatePromptHookInput(input);
    const history = webPromptHookHistory(current);
    const draft = saveWebPromptHookDraft({
      hookId,
      expectedRevision: history.draft?.revision ?? null,
      draft: input,
    });
    const updated: PromptHook = {
      ...current,
      hasDraft: true,
      draftRevision: draft.revision,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async deletePromptHook(hookId: string): Promise<void> {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      throw new Error("Built-in Prompt Hook cannot be deleted");
    }
    const stored = { ...readStoredPromptHooks() };
    delete stored[hookId];
    writeStoredPromptHooks(stored);
    deleteWebPromptHookVersionState(hookId);
  },

  async setPromptHookEnabled(hookId: string, enabled: boolean): Promise<PromptHook> {
    const current = findPromptHook(hookId);
    if (!enabled && !current.disableable) {
      throw new Error("Prompt Hook cannot be disabled");
    }
    const updated = { ...current, enabled, updatedAt: nowIso() };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async setPromptHookCliBindings(hookId: string, agentIds: string[]): Promise<PromptHook> {
    if (!agentIds.every(isManagedCliAgentId)) throw new Error("Unsupported Prompt Hook CLI binding");
    const current = findPromptHook(hookId);
    const cliBindings = Array.from(new Set(agentIds));
    const updated = { ...current, cliBindings, updatedAt: nowIso() };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [hookId]: updated });
    return updated;
  },

  async previewPromptHook(input: PromptHookPreviewInput): Promise<PromptHookPreview> {
    const hook = findPromptHook(input.hookId);
    const sampleInput = input.sampleInput ?? "Preview request";
    const renderedContent = renderPromptHookTemplate(hook.templateBody ?? "", {
      agentId: input.agentId,
      sampleInput,
    });
    const trace = [traceForHook(hook, hook.enabled ? "fired" : "disabled", hook.enabled ? renderedContent : null, input.agentId, hook.enabled ? undefined : "disabled")];
    writePromptHookTraces([...trace, ...readPromptHookTraces()]);
    return { hookId: hook.id, agentId: input.agentId, renderedContent, trace };
  },

  async previewPromptAssembly(input: PromptAssemblyPreviewInput): Promise<PromptHookPreview> {
    return assemblePromptHooks(input);
  },

  async listPromptHookTraces(limit = 25): Promise<PromptHookTraceSummary[]> {
    return readPromptHookTraces().slice(0, limit);
  },

  async listPromptHookVariables(): Promise<PromptHookVariableDefinition[]> {
    return webPromptHookVariables.map((variable) => ({ ...variable, aliases: [...variable.aliases] }));
  },

  async savePromptHookDraft(input: SavePromptHookDraftInput): Promise<PromptHookDraft> {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    return saveWebPromptHookDraft(input);
  },

  async publishPromptHook(input: PublishPromptHookInput): Promise<PromptHookVersion> {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    const result = publishWebPromptHook(input, current);
    const updated: PromptHook = {
      ...current,
      ...result.published,
      version: result.version.version,
      publishedVersion: result.version.version,
      hasDraft: false,
      draftRevision: null,
      updatedAt: result.version.publishedAt,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [current.id]: updated });
    return result.version;
  },

  async getPromptHookVersionHistory(hookId: string): Promise<PromptHookVersionHistory> {
    const current = findPromptHook(hookId);
    if (current.source === "builtin") {
      return {
        hookId,
        publishedVersion: current.version,
        draft: null,
        versions: [{
          hookId,
          version: current.version,
          contentHash: `builtin-${hookId}-${current.version}`,
          publicationKind: "publish",
          rollbackFromVersion: null,
          publishedAt: current.updatedAt,
        }],
        evaluations: [],
      };
    }
    return webPromptHookHistory(current);
  },

  async rollbackPromptHook(input: RollbackPromptHookInput): Promise<PromptHookVersion> {
    const current = findPromptHook(input.hookId);
    if (current.source === "builtin") throw new Error("Built-in Prompt Hook content cannot be edited");
    const result = rollbackWebPromptHook(input, current);
    const history = webPromptHookHistory(current);
    const updated: PromptHook = {
      ...current,
      ...result.published,
      version: result.version.version,
      publishedVersion: result.version.version,
      hasDraft: history.draft !== null,
      draftRevision: history.draft?.revision ?? null,
      updatedAt: result.version.publishedAt,
    };
    writeStoredPromptHooks({ ...readStoredPromptHooks(), [current.id]: updated });
    return result.version;
  },

  async selectWorkspaceDirectory() {
    return "D:\\\\example-workspace";
  },
};
