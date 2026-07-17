import type { AgentService } from "./agent-service";
import { mockAgents, mockWorkflowState } from "./mock-agent-data";
import { i18n } from "../i18n";
import type {
  CliToolStatus,
  CreateSessionInput,
  InteractionMode,
  KnownProject,
  ProjectInspection,
  Session,
  SessionDetails,
  WorkflowState,
} from "../types/agent";
import type { ChatMessage, ChatStreamEvent } from "../types/chat";
import type { OperationTask } from "../types/operation";
import { createWebMockOperation } from "./web-operation-client";
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

function tr(key: string) {
  return i18n.t(key);
}

function webLocalCliDetectionMessage() {
  return tr("web.error.localCliDetection");
}

function webCliPackageOperationsMessage() {
  return tr("web.error.cliPackageOperations");
}

let workflowState: WorkflowState = { ...mockWorkflowState };
let nextSessionId = 1;
let nextMessageId = 1;
let activeSessionId: string | null = null;
let sessions: Session[] = [];
let knownProjects: KnownProject[] = [];
const messagesBySession = new Map<string, ChatMessage[]>();
const subscribersBySession = new Map<string, Set<(event: ChatStreamEvent) => void>>();
const activeStreams = new Map<string, { messageId: string; timeoutIds: Array<ReturnType<typeof setTimeout>> }>();

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
    installCommand: "npm install -g @anthropic-ai/claude-code@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
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
    installCommand: "npm install -g opencode-ai@latest",
    lastCheckedAt: null,
    lastError: webLocalCliDetectionMessage(),
    lastOperationId: null,
    versionCheckStatus: "unsupported",
  },
];

function nowIso() {
  return new Date().toISOString();
}

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
  return input.projectPath?.trim() || input.folder?.trim() || null;
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

function clearActiveStream(sessionId: string) {
  const activeStream = activeStreams.get(sessionId);
  activeStream?.timeoutIds.forEach((timeoutId) => clearTimeout(timeoutId));
  activeStreams.delete(sessionId);
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

export const webAgentClient: AgentService = {
  async listAgents(capabilityTag) {
    return capabilityTag
      ? mockAgents.filter((agent) => agent.capabilityTags.includes(capabilityTag))
      : mockAgents;
  },

  async listCliTools() {
    return webCliTools.map((tool) => ({
      ...tool,
      availableVersions: [...tool.availableVersions],
      lastError: webLocalCliDetectionMessage(),
    }));
  },

  async refreshCliDetections(): Promise<OperationTask> {
    const timestamp = nowIso();
    const message = webLocalCliDetectionMessage();
    const operationId = `web-cli-refresh-${timestamp}`;
    return createWebMockOperation({
      id: operationId,
      relatedEntityId: null,
      message,
      terminalStatus: "failed",
      error: message,
      result: { agentIds: webCliTools.map((tool) => tool.agentId) },
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
    return sortSessions(sessions.filter((session) => !session.archived));
  },

  async listArchivedSessions() {
    return sortSessions(sessions.filter((session) => session.archived));
  },

  async getActiveSession() {
    if (!activeSessionId) return null;
    return sessions.find((session) => session.id === activeSessionId) ?? null;
  },

  async listKnownProjects() {
    return knownProjects.map((project) => ({ ...project }));
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
    const projectPath = resolveProjectPath(input);
    const inspection = projectPath ? inspectMockProject(projectPath) : null;
    if (inspection) {
      upsertKnownProject(inspection);
    }
    let effectiveFolder = projectPath;
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
    const session: Session = {
      id: `web-session-${nextSessionId}`,
      title: input.title?.trim() || tr("createSession.sessionPlaceholder"),
      agentId: input.agentId,
      interactionMode: input.interactionMode,
      lifecycleState: "idle",
      folder: effectiveFolder,
      projectPath,
      worktreePath,
      worktreeName,
      worktreeBranch,
      pinned: false,
      archived: false,
      createdAt: timestamp,
      updatedAt: timestamp,
    };
    nextSessionId += 1;
    sessions = [session, ...sessions];
    activeSessionId = session.id;
    workflowState = {
      ...workflowState,
      activeAgentId: session.agentId,
      activeInteractionMode: session.interactionMode,
      lifecycleState: session.lifecycleState,
    };
    return createWebMockOperation({
      id: `web-session-create-${session.id}-${Date.now()}`,
      kind: "workspace",
      relatedEntityId: projectPath,
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
    sessions = sessions.filter((session) => session.id !== sessionId);
    if (activeSessionId === sessionId) {
      activeSessionId = null;
    }
  },

  async switchSession(sessionId: string) {
    const session = findSession(sessionId);
    if (session.archived) {
      throw new Error(`Cannot switch to archived session: ${sessionId}`);
    }
    activeSessionId = session.id;
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
    }
    return session;
  },

  async unarchiveSession(sessionId: string) {
    return updateSession(sessionId, { archived: false });
  },

  async sendMessage(input) {
    const session = findSession(input.sessionId);
    clearActiveStream(input.sessionId);
    const timestamp = nowIso();
    const userMessage: ChatMessage = {
      id: createMessageId(),
      sessionId: input.sessionId,
      role: "user",
      content: input.content.trim(),
      status: "completed",
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
    if (input.config.thinking) {
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

  async stopGeneration(sessionId: string) {
    findSession(sessionId);
    if (!cancelActiveStream(sessionId)) return;
    updateSession(sessionId, { lifecycleState: "stopped" });
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

  async selectWorkspaceDirectory() {
    return "D:\\\\example-workspace";
  },
};
