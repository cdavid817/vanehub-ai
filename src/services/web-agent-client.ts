import type { AgentService, SessionStateEvent } from "./agent-service";
import { mockAgents, mockWorkflowState } from "./mock-agent-data";
import { i18n } from "../i18n";
import type {
  AssignSessionCategoryInput,
  AutomaticArchivalSettings,
  AgentTerminalEvent,
  AgentTerminalSession,
  AgentTerminalSize,
  CliParameterSelections,
  CliToolStatus,
  CreateSessionCategoryInput,
  CreateSessionInput,
  CreateScheduledTaskInput,
  ExportSessionInput,
  InteractionMode,
  KnownRemoteWorkspace,
  KnownProject,
  ProjectInspection,
  RemoteWorkspace,
  RenameSessionCategoryInput,
  ScheduledTask,
  SetScheduledTaskEnabledInput,
  Session,
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
import { defaultSessionTitleFromPath, normalizeDisplayPath } from "../lib/session-path";
import type { ChatConfig, ChatMessage, ChatStreamEvent } from "../types/chat";
import type { UsageStatistics, UsageStatisticsRange } from "../types/chat";
import type { OperationTask } from "../types/operation";
import type {
  CoordinationRun,
  StartCoordinationInput,
  StartCoordinationResult,
} from "../types/coordination";
import {
  createCoordinationRun,
  executeCoordinationRun,
  requestCoordinationCancellation,
  validateCoordinationInput,
} from "./coordination-runtime";
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
import { createWebMockOperation, settleWebOperation } from "./web-operation-client";
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
import {
  createCliParameterProfile,
  defaultCliParameterSelections,
  normalizeCliParameterSelections,
} from "./cli-parameter-catalog";
import { aggregateSessionUsageRecords, aggregateUsageRecords, type UsageRecord } from "./usage-statistics";
import { webSessionWorkspaceClient } from "./web-session-workspace-client";
import { defaultChatConfigForSession, normalizeChatConfigForSession } from "./chat-configuration";
import { computeNextScheduledRun, validateScheduledTaskFrequency } from "../lib/scheduled-task-recurrence";

function tr(key: string, values?: Record<string, string | number>) {
  return i18n.t(key, values);
}

function webLocalCliDetectionMessage() {
  return tr("web.error.localCliDetection");
}

function webCliPackageOperationsMessage() {
  return tr("web.error.cliPackageOperations");
}

const webRetainedTerminalTranscriptBytes = 1_000_000;

let workflowState: WorkflowState = { ...mockWorkflowState };
let nextSessionId = 1;
const cliParameterStorageKey = "vanehub.cli-parameter-profiles.v1";
let memoryCliParameterSelections: Partial<Record<ManagedCliAgentId, CliParameterSelections>> = {};

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
let activeSessionId: string | null = null;
let sessions: Session[] = [];
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
let coordinationRuns: CoordinationRun[] = [];
let nextCoordinationRunId = 1;
const coordinationTimers = new Map<string, ReturnType<typeof setTimeout>>();
const coordinationAttempts = new Map<string, AbortController>();
const loopSubscribers = new Map<string, Set<(event: LoopEvent) => void>>();
const loopTimers = new Map<string, ReturnType<typeof setTimeout>>();
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

export function seedWebImSessionForTest(connector: ImSessionConnector): Session {
  const timestamp = nowIso();
  const session: Session = {
    id: `web-im-session-${nextSessionId++}`,
    title: `IM ${connector}`,
    agentId: "codex-cli",
    interactionMode: "cli",
    lifecycleState: "idle",
    folder: "D:\\example\\im-project",
    projectPath: "D:\\example\\im-project",
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
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
            : ".vanehub/skills",
  isDefault: true,
}));

let webSkills: Skill[] = builtinSkillSeeds.map((seed) => {
  const timestamp = nowIso();
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
    },
    boundAgentIds: ["claude-code", "codex-cli"],
    bindings: [],
    createdAt: timestamp,
    updatedAt: timestamp,
  };
});

const deletedBuiltinSkillIds = new Set<string>();

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
];

function nowIso() {
  return new Date().toISOString();
}

function waitForSimulatedCoordinationAttempt(signal: AbortSignal) {
  return new Promise<boolean>((resolve) => {
    if (signal.aborted) {
      resolve(false);
      return;
    }
    const complete = () => {
      signal.removeEventListener("abort", cancel);
      resolve(true);
    };
    const timer = setTimeout(complete, 50);
    const cancel = () => {
      clearTimeout(timer);
      signal.removeEventListener("abort", cancel);
      resolve(false);
    };
    signal.addEventListener("abort", cancel, { once: true });
  });
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

function skillScopeMatches(skill: Skill, input: SkillScopeInput) {
  return (
    skill.scope === input.scope &&
    (input.scope === "global" || skill.workspacePath === (input.workspacePath?.trim() || null))
  );
}

function mountPathForAgent(agentId: string) {
  return webSkillMountPaths.find((path) => path.agentId === agentId)?.mountPath ?? ".vanehub/skills";
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
  return `---\nid: ${skill.metadata.id}\nname: ${skill.metadata.name}\ndescription: ${skill.metadata.description}\ncategory: ${skill.metadata.category}\nversion: ${skill.metadata.version}\ntriggers:\n${triggers}\n---\n\n# ${skill.metadata.name}\n\nWeb mock SKILL.md content for ${skill.id}.\n`;
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
  const timestamp = nowIso();
  const root = input.scope === "global" ? "~/.vanehub/skills" : `${input.workspacePath}/.vanehub/skills`;
  return {
    id: input.id,
    scope: input.scope,
    workspacePath: input.scope === "workspace" ? (input.workspacePath ?? null) : null,
    source: input.source ?? "user",
    enabled: input.enabled,
    skillDir: `${root}/${input.id}`,
    skillMdPath: `${root}/${input.id}/SKILL.md`,
    contentHash: `web-${input.id}-${timestamp}`,
    metadata: input.metadata,
    boundAgentIds: [...input.boundAgentIds],
    bindings: [],
    createdAt: timestamp,
    updatedAt: timestamp,
  };
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
  const records: UsageRecord[] = [...representativeUsageRecords];
  for (const [sessionId, messages] of messagesBySession.entries()) {
    const session = sessions.find((candidate) => candidate.id === sessionId);
    if (!session) continue;
    for (const message of messages) {
      if (message.role !== "assistant" || !message.tokenUsage) continue;
      records.push({
        messageId: message.id,
        sessionId,
        agentId: session.agentId,
        accountingKind: "estimated",
        inputCount: message.tokenUsage.input,
        outputCount: message.tokenUsage.output,
        cacheReadCount: 0,
        cacheCreationCount: 0,
        occurredAt: message.createdAt,
      });
    }
  }
  return aggregateUsageRecords(records, range);
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

function applyStreamEvent(event: ChatStreamEvent) {
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
    upsertMessage({ ...message, toolUse: [...(message.toolUse ?? []), event.toolUse], updatedAt: timestamp });
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
  } else if (event.type === "failed") {
    upsertMessage({ ...message, status: "failed", error: event.error, updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
  } else if (event.type === "cancelled") {
    upsertMessage({ ...message, status: "cancelled", updatedAt: timestamp });
    activeStreams.delete(event.sessionId);
  }
}

function publishChatEvent(event: ChatStreamEvent) {
  applyStreamEvent(event);
  emitChatEvent(event);
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
  if (!mockAgents.some((agent) => agent.id === input.agentId)) {
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

export function resetWebLoopsForTest() {
  loopTimers.forEach((timer) => clearTimeout(timer));
  loopTimers.clear();
  coordinationTimers.forEach((timer) => clearTimeout(timer));
  coordinationTimers.clear();
  coordinationAttempts.forEach((controller) => controller.abort());
  coordinationAttempts.clear();
  loopSubscribers.clear();
  sessions = sessions.filter((session) => !loopRoleSessionIds.has(session.id));
  loopRoleSessionIds.forEach((sessionId) => messagesBySession.delete(sessionId));
  loopRoleSessionIds.clear();
  loopDefinitions = [];
  loopRuns = [];
  coordinationRuns = [];
  nextLoopDefinitionId = 1;
  nextLoopRunId = 1;
  nextLoopEvidenceId = 1;
  nextCoordinationRunId = 1;
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
    folder: run.worktreePath,
    projectPath: run.projectPath,
    worktreePath: run.worktreePath,
    worktreeName: run.worktreeName,
    worktreeBranch: run.worktreeBranch,
    remoteWorkspace: null,
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
  }, 220);
  loopTimers.set(run.id, timeoutId);
}

export const webAgentClient: AgentService = {
  ...webSessionWorkspaceClient,
  async listAgents(capabilityTag) {
    return capabilityTag
      ? mockAgents.filter((agent) => agent.capabilityTags.includes(capabilityTag))
      : mockAgents;
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

  async startCoordination(input: StartCoordinationInput): Promise<StartCoordinationResult> {
    const order = validateCoordinationInput(input, new Set(mockAgents.map((agent) => agent.id)));
    const timestamp = nowIso();
    const runId = `web-coordination-${nextCoordinationRunId++}`;
    const operationId = `web-coordination-operation-${runId}`;
    const run = createCoordinationRun(input, runId, operationId, timestamp, true);
    coordinationRuns = [run, ...coordinationRuns];
    createWebMockOperation({
      id: operationId,
      relatedEntityId: runId,
      message: `Simulating Multi-Agent coordination ${run.name}.`,
      terminalStatus: "succeeded",
      error: null,
      result: { runId },
    });
    const timer = setTimeout(() => {
      coordinationTimers.delete(runId);
      const controller = new AbortController();
      coordinationAttempts.set(runId, controller);
      void executeCoordinationRun(run, order, async (request) => {
        const completed = await waitForSimulatedCoordinationAttempt(controller.signal);
        if (!completed) return { status: "cancelled", error: "Coordination was cancelled." };
        const agent = mockAgents.find((candidate) => candidate.id === request.agentId);
        if (!agent || agent.availabilityState === "unavailable" || agent.availabilityState === "needs-auth") {
          return { status: "failed", kind: "retryable", error: `Agent unavailable: ${request.agentId}` };
        }
        const contextNote = request.prerequisiteContext
          ? `\n\nReceived prerequisite context:\n${request.prerequisiteContext}`
          : "";
        return {
          status: "succeeded",
          content: `[Web mock ${request.agentId}] ${request.instruction}${contextNote}`,
        };
      }, nowIso).then(() => {
        const error = run.status === "failed" ? "One or more coordination nodes failed." : null;
        settleWebOperation(
          operationId,
          run.status === "succeeded" ? "succeeded" : run.status === "cancelled" ? "cancelled" : "failed",
          error,
          { runId },
        );
      }).finally(() => {
        coordinationAttempts.delete(runId);
      });
    }, 100);
    coordinationTimers.set(runId, timer);
    return { runId, operationId };
  },

  async listCoordinationRuns() {
    return structuredClone(coordinationRuns);
  },

  async getCoordinationRun(runId: string) {
    const run = coordinationRuns.find((candidate) => candidate.id === runId);
    if (!run) throw new Error(`Coordination run not found: ${runId}`);
    return structuredClone(run);
  },

  async cancelCoordinationRun(runId: string) {
    const run = coordinationRuns.find((candidate) => candidate.id === runId);
    if (!run) throw new Error(`Coordination run not found: ${runId}`);
    const timer = coordinationTimers.get(runId);
    if (timer) {
      clearTimeout(timer);
      coordinationTimers.delete(runId);
    }
    requestCoordinationCancellation(run, nowIso());
    coordinationAttempts.get(runId)?.abort();
    if (run.status === "cancelled") {
      settleWebOperation(run.operationId, "cancelled", "Coordination was cancelled.", { runId });
    }
    return structuredClone(run);
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
    return stored ? normalizeChatConfigForSession(session, stored) : defaultChatConfigForSession(session);
  },

  async saveSessionChatConfig(sessionId, config) {
    const session = findSession(sessionId);
    const normalized = normalizeChatConfigForSession(session, config);
    writeChatConfigs({ ...readChatConfigs(), [sessionId]: normalized });
    emitSessionEvent({ kind: "configuration-changed", sessionId });
    return normalized;
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
    const remoteWorkspace = input.remoteWorkspace ? normalizeRemoteWorkspace(input.remoteWorkspace) : null;
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
    const session: Session = {
      id: `web-session-${nextSessionId}`,
      title: input.title?.trim() || defaultSessionTitleFromPath(titleSource) || tr("createSession.sessionPlaceholder"),
      agentId: input.agentId,
      interactionMode: input.interactionMode,
      lifecycleState: "idle",
      folder: effectiveFolder,
      projectPath,
      worktreePath,
      worktreeName,
      worktreeBranch,
      remoteWorkspace,
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
    const config = normalizeChatConfigForSession(session, input.config);
    if (activeStreams.has(input.sessionId)) {
      throw new Error("A generation is already active for this session.");
    }
    const timestamp = nowIso();
    const userMessage: ChatMessage = {
      id: createMessageId(),
      sessionId: input.sessionId,
      role: "user",
      content: input.content.trim(),
      status: "completed",
      fileReferences: input.fileReferences,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    const assistantMessage: ChatMessage = {
      id: createMessageId(),
      sessionId: input.sessionId,
      role: "assistant",
      content: "",
      status: "streaming",
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    setSessionMessages(input.sessionId, [...getSessionMessages(input.sessionId), userMessage, assistantMessage]);
    updateSession(input.sessionId, { lifecycleState: "running" });

    const responseText = `Mock ${session.agentId} response: I received "${userMessage.content}". This is a streaming preview in Web mode.`;
    const tokens = responseText.match(/.{1,6}/g) ?? [responseText];
    const timeoutIds: Array<ReturnType<typeof setTimeout>> = [];
    const startTimeoutId = setTimeout(() => {
      emitChatEvent({ type: "started", sessionId: input.sessionId, messageId: assistantMessage.id });
    }, 80);
    timeoutIds.push(startTimeoutId);
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
        tokenUsage: { input: userMessage.content.length, output: responseText.length },
      });
      updateSession(input.sessionId, { lifecycleState: "idle" });
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

  async getUsageStatistics(input) {
    return aggregateWebUsageStatistics(input.range);
  },

  async getSessionUsageSummary(sessionId: string) {
    findSession(sessionId);
    const generated = aggregateSessionUsageRecords(representativeUsageRecords, sessionId);
    if (generated.responseCount > 0) return generated;
    const messages = getSessionMessages(sessionId);
    const records: UsageRecord[] = messages
      .filter((message) => message.role === "assistant" && message.status === "completed" && message.tokenUsage)
      .map((message) => ({
        messageId: message.id,
        sessionId,
        agentId: findSession(sessionId).agentId,
        accountingKind: "reported",
        inputCount: message.tokenUsage?.input ?? 0,
        outputCount: message.tokenUsage?.output ?? 0,
        cacheReadCount: 0,
        cacheCreationCount: 0,
        occurredAt: message.updatedAt,
      }));
    return aggregateSessionUsageRecords(records, sessionId);
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
      emitTerminalEvent({
        type: "output",
        terminalId: terminal.terminalId,
        sessionId,
        content: `Web mock Agent Terminal for ${session.agentId}\r\nLocal CLI execution is unavailable in Web mode.\r\n`,
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

  async listSkillMountPaths() {
    return webSkillMountPaths.map((path) => ({ ...path }));
  },

  async updateSkillMountPath(agentId: string, mountPath: string): Promise<SkillMountMigrationReport> {
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
    const skill = mutationToSkill(input);
    return hydrateSkillBindings(upsertWebSkill(skill));
  },

  async updateSkill(skillId, input: SkillUpdateInput) {
    if (input.metadata.id !== skillId) {
      throw new Error(tr("web.error.skillIdImmutable"));
    }
    const current = findWebSkill(skillId, input);
    const updated: Skill = {
      ...current,
      metadata: input.metadata,
      enabled: input.enabled,
      boundAgentIds: [...input.boundAgentIds],
      contentHash: `web-${skillId}-${nowIso()}`,
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
  },

  async restoreBuiltinSkill(skillId) {
    const seed = builtinSkillSeeds.find((candidate) => candidate.id === skillId);
    if (!seed) {
      throw new Error(`Unknown built-in Skill: ${skillId}`);
    }
    deletedBuiltinSkillIds.delete(skillId);
    const restored = mutationToSkill({
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
    });
    return hydrateSkillBindings(upsertWebSkill(restored));
  },

  async setSkillEnabled(skillId, input, enabled) {
    const current = findWebSkill(skillId, input);
    const updated = { ...current, enabled, updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async setSkillAgentBindings(skillId, input, agentIds) {
    const current = findWebSkill(skillId, input);
    const updated = { ...current, boundAgentIds: [...agentIds], updatedAt: nowIso() };
    return hydrateSkillBindings(upsertWebSkill(updated));
  },

  async previewSkill(skillId, input): Promise<SkillPreview> {
    const skill = hydrateSkillBindings(findWebSkill(skillId, input));
    return {
      id: skill.id,
      scope: skill.scope,
      workspacePath: skill.workspacePath,
      path: skill.skillMdPath,
      content: buildSkillContent(skill),
    };
  },

  async importSkill(input: SkillImportInput) {
    const id = input.sourcePath.split(/[\\/]/).filter(Boolean).pop() ?? `imported-${Date.now()}`;
    return this.createSkill({
      id,
      scope: input.scope,
      workspacePath: input.workspacePath,
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
    });
  },

  async detectSkillDrift(input): Promise<SkillDriftReport> {
    const issues = [...deletedBuiltinSkillIds].map((skillId) => ({
      skillId,
      type: "deleted-builtin" as const,
      agentId: null,
      path: null,
      message: tr("web.skill.restoreMessage"),
    }));
    return {
      scope: input.scope,
      workspacePath: input.scope === "workspace" ? (input.workspacePath ?? null) : null,
      issues,
      driftHash: `web-${issues.length}`,
    };
  },

  async syncSkillDrift(input): Promise<SkillSyncResult> {
    const report = await this.detectSkillDrift(input);
    const restored: string[] = [];
    for (const issue of report.issues) {
      if (issue.type === "deleted-builtin") {
        await this.restoreBuiltinSkill(issue.skillId);
        restored.push(issue.skillId);
      }
    }
    return {
      mounted: [],
      unmounted: [],
      overwritten: [],
      backedUp: [],
      restored,
      failed: [],
      resolvedFrom: report,
    };
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
