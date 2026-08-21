import type { AgentService } from "./agent-service";
import { mockAgents } from "./mock-agent-data";
import {
  createWebPendingApproval,
  getWebDefaultPolicyTemplate,
  isAgentAutoApproved,
  webPrincipalTemplates,
} from "./web-permissions-mock-state";
import type { ImSessionConnector, Session } from "../types/agent";
import { readWebAppSettings } from "./web-settings-client";
import type { ChatMessage } from "../types/chat";
import { webEvaluationClient } from "./web-evaluation-client";
import { webPromptHookClient } from "./web-prompt-hook-client";
import { webApiAgentClient } from "./web-api-agent-client";
import { webOnePieceProviderClient } from "./web-onepiece-provider-client";
import { webOnePieceProfileClient } from "./web-onepiece-profile-client";
import { webHybridRoutingClient } from "./web-hybrid-routing-client";
import { deleteWebApiAgentProviderConfig } from "./web-api-provider-state";
import { webCodeIndexClient } from "./web-code-index-client";
import { webCliToolClient } from "./web-cli-tool-client";
import { webCliParameterClient } from "./web-cli-parameter-client";
import { webCliConfigClient } from "./web-cli-config-client";
import { webScheduledTaskClient } from "./web-scheduled-task-client";
import { webContextQualityClient } from "./web-context-quality-client";
import { webSkillGovernanceClient } from "./web-skill-governance-client";
import { webSkillEvidenceClient } from "./web-skill-evidence-client";
import { webAgentRegistryClient } from "./web-agent-registry-client";
import { normalizeWebPath } from "./web-skill-location";

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
import { webSessionWorkspaceClient } from "./web-session-workspace-client";
import { webLspClient } from "./web-lsp-client";
import { normalizeChatConfigForSession, withEffectiveExecutionPolicy } from "./chat-configuration";
import { createWebMcpToolSimulationPlan } from "./web-mcp-tool-simulation";
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
import { webAgentMemoryClient } from "./web-agent-memory-client";

export { resetWebAgentMemoriesForTest } from "./web-agent-memory-state";
import {
  createWebAgentMemory,
  listWebAgentMemories,
  simulateWebMemoryIndexInjection,
} from "./web-agent-memory-state";
import { webChatClient } from "./web-chat-client";

export {
  resolveWebMockToolApproval,
  WEB_MOCK_PLAN_EXIT_TRIGGER,
  WEB_MOCK_QUESTION_TRIGGER,
} from "./web-chat-client";
import { WEB_MOCK_PLAN_EXIT_TRIGGER, WEB_MOCK_QUESTION_TRIGGER } from "./web-chat-client";
import {
  createWebMessageId,
  emitWebChatEvent,
  getWebSessionMessages,
  hasWebActiveStream,
  publishWebChatEvent,
  setWebActiveStream,
  setWebSessionMessages,
} from "./web-chat-state";
import { webSessionChatConfigClient } from "./web-session-chat-config-client";
import { webSessionRecoveryClient } from "./web-session-recovery-client";

export {
  resetWebRecoverySessionsForTest,
  seedWebRecoverySessionForTest,
} from "./web-session-recovery-state";
import { prependWebAgentRun, setWebAgentRunEvents } from "./web-agent-run-state";

export {
  resetWebMissionControlRunsForTest,
  seedWebMissionControlRunsForTest,
} from "./web-agent-run-state";
import { webKnownWorkspaceClient } from "./web-known-workspace-client";
import {
  findWebSession,
  getWebActiveSessionId,
  getWebWorkflowState,
  listWebSessions,
  nextWebSessionSequence,
  prependWebSession,
  setWebActiveSessionId,
  subscribeWebSessionStateEvents,
  updateWebSession,
} from "./web-session-state";
import { webSessionQueryClient } from "./web-session-query-client";
import { webSessionLifecycleClient } from "./web-session-lifecycle-client";
import { webRoutedSeatId, webSessionSeatClient } from "./web-session-seat-client";
import { selectWebRunner, webRunRunner } from "./web-agent-runner";
import type { Skill } from "../types/skill";
import {
  listWebSkillApiAgentBindings,
  listWebSkillMountPaths,
  listWebSkills,
  replaceWebSkillApiAgentBindings,
  replaceWebSkillMountPaths,
  replaceWebSkills,
} from "./web-skill-state";

/** Mirrors the desktop runtime's character-count compaction trigger (see `add-agent-context-compaction`), scaled down for deterministic mock sessions. */
const mockCompactionTriggerCharacters = 2_000;

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


const webCodeReviewClient = createWebCodeReviewClient(webSessionWorkspaceClient);

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

  ...webAgentMemoryClient,

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
    const existingMessages = getWebSessionMessages(input.sessionId);
    const firstSpeakerSeatId = webRoutedSeatId(activeSeats, existingMessages, input.content.trim());
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
          const memory = createWebAgentMemory(
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
        const memory = createWebAgentMemory(
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
    const hadExistingMemories = listWebAgentMemories().length > 0;
    if (memoryEnabled && hadExistingMemories) {
      // `add-two-tier-memory-recall`: the index is what every request carries, so the mock reports
      // how many memories it names. Neither this nor the selection below depends on an embedding
      // source being configured — memory has to work on an installation without retrieval.
      const injected = simulateWebMemoryIndexInjection();
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
          const memory = createWebAgentMemory(
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
