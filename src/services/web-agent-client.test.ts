import { afterEach, describe, expect, it, vi } from "vitest";
import {
  resetWebLoopsForTest,
  seedWebImSessionForTest,
  simulateWebLoopRestartForTest,
  webAgentClient,
} from "./web-agent-client";
import { webOperationClient } from "./web-operation-client";
import type { CreateSessionInput, Session } from "../types/agent";
import type { ChatStreamEvent } from "../types/chat";
import type { PromptHookMutationInput } from "../types/prompt-hook";
import type { SaveLoopDefinitionInput } from "../types/loop";
import { i18n } from "../i18n";

afterEach(() => {
  resetWebLoopsForTest();
  vi.useRealTimers();
});

const loopDefinitionInput: SaveLoopDefinitionInput = {
  name: "Web Loop",
  enabled: true,
  projectPath: "D:/example/project",
  baseBranch: "main",
  goal: "Improve coverage",
  acceptanceCriteria: ["Tests pass"],
  allowedPaths: ["src"],
  protectedPaths: [".git"],
  workerAgentId: "codex-cli",
  verifierAgentId: "claude-code",
  verificationCommands: [{
    id: "tests",
    program: "npm",
    args: ["test"],
    workingDirectory: null,
    timeoutSeconds: 60,
    required: true,
  }],
  limits: {
    maxIterations: 3,
    stepTimeoutSeconds: 60,
    totalTimeoutSeconds: 600,
    maxConsecutiveRuntimeErrors: 2,
    maxConsecutiveNoProgress: 2,
  },
};

describe("webAgentClient", () => {
  async function createMockSession(input: CreateSessionInput): Promise<Session> {
    vi.useFakeTimers();
    const operation = await webAgentClient.createSession(input);
    await vi.advanceTimersByTimeAsync(950);
    const completed = await webOperationClient.getOperationStatus(operation.id);
    expect(completed.status).toBe("succeeded");
    return completed.result as unknown as Session;
  }

  it("simulates fixed Loop phases, evidence, and human acceptance", async () => {
    vi.useFakeTimers();
    const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
    const started = await webAgentClient.startLoop(definition.id);

    expect(started.run).toMatchObject({ status: "queued", phase: "preparing", simulated: true });
    await vi.advanceTimersByTimeAsync(900);

    const awaiting = await webAgentClient.getLoopRun(started.run.id);
    expect(awaiting).toMatchObject({ status: "awaiting-acceptance", phase: "finalizing", simulated: true });
    expect(awaiting.iterations[0].evidence.map((item) => item.kind)).toEqual([
      "worker",
      "verification",
      "verifier",
      "decision",
    ]);
    await expect(webAgentClient.acceptLoop(awaiting.id)).resolves.toMatchObject({
      status: "succeeded",
      terminalReason: "goal-met",
    });
  });

  it("publishes cloned Loop events and releases the Web subscription boundary", async () => {
    vi.useFakeTimers();
    const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
    const started = await webAgentClient.startLoop(definition.id);
    const events: string[] = [];
    const unsubscribe = await webAgentClient.subscribeLoopEvents(started.run.id, (event) => {
      events.push(`${event.kind}:${event.run.phase}`);
      event.run.phase = "finalizing";
    });

    await vi.advanceTimersByTimeAsync(220);
    expect(events).toContain("evidence-added:acting");
    expect((await webAgentClient.getLoopRun(started.run.id)).phase).toBe("acting");

    unsubscribe();
    const eventCount = events.length;
    await vi.advanceTimersByTimeAsync(700);
    expect(events).toHaveLength(eventCount);
  });

  it("keeps Loop role sessions hidden while preserving direct inspection", async () => {
    vi.useFakeTimers();
    const activeBefore = await webAgentClient.getActiveSession();
    const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
    const started = await webAgentClient.startLoop(definition.id);
    await vi.advanceTimersByTimeAsync(900);

    const run = await webAgentClient.getLoopRun(started.run.id);
    const workerSessionId = run.iterations[0].workerSessionId ?? "";
    const verifierSessionId = run.iterations[0].verifierSessionId ?? "";
    await expect(webAgentClient.getSession(workerSessionId)).resolves.toMatchObject({
      id: workerSessionId,
      agentId: loopDefinitionInput.workerAgentId,
      worktreePath: run.worktreePath,
    });
    await expect(webAgentClient.getSession(verifierSessionId)).resolves.toMatchObject({
      id: verifierSessionId,
      agentId: loopDefinitionInput.verifierAgentId,
      worktreePath: run.worktreePath,
    });
    await expect(webAgentClient.listMessages({ sessionId: workerSessionId })).resolves.toEqual([]);
    await expect(webAgentClient.getSessionUsageSummary(verifierSessionId)).resolves.toBeDefined();

    const visibleIds = (await webAgentClient.listSessions()).map((session) => session.id);
    const searchIds = (await webAgentClient.searchSessions({ query: "Web Loop" })).map((result) => result.session.id);
    expect(visibleIds).not.toContain(workerSessionId);
    expect(visibleIds).not.toContain(verifierSessionId);
    expect(searchIds).not.toContain(workerSessionId);
    expect(searchIds).not.toContain(verifierSessionId);
    expect(await webAgentClient.getActiveSession()).toEqual(activeBefore);
  });

  it("localizes Web Loop evidence and control errors with the active locale", async () => {
    vi.useFakeTimers();
    const previousLanguage = i18n.resolvedLanguage ?? "en";
    try {
      await i18n.changeLanguage("zh-CN");
      const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
      const started = await webAgentClient.startLoop(definition.id);
      await vi.advanceTimersByTimeAsync(900);

      const run = await webAgentClient.getLoopRun(started.run.id);
      expect(run.iterations[0].workerSummary).toContain("模拟执行者");
      expect(run.iterations[0].verifierFindings).toContain("必需的模拟检查均已通过。");
      expect(run.iterations[0].decisionReason).toContain("独立验证者建议验收");
      await expect(webAgentClient.pauseLoop(run.id)).rejects.toThrow("只能暂停活动循环");
    } finally {
      await i18n.changeLanguage(previousLanguage);
    }
  });

  it("simulates pause, restart recovery, resume, and cancellation", async () => {
    vi.useFakeTimers();
    const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
    const started = await webAgentClient.startLoop(definition.id);
    const recovered = simulateWebLoopRestartForTest(started.run.id);

    expect(recovered).toMatchObject({ status: "paused", terminalReason: "recovery-required" });
    const resumed = await webAgentClient.resumeLoop(recovered.id);
    expect(resumed).toMatchObject({ status: "queued", terminalReason: null });
    await vi.advanceTimersByTimeAsync(220);
    await webAgentClient.pauseLoop(recovered.id);
    await vi.advanceTimersByTimeAsync(220);
    expect(await webAgentClient.getLoopRun(recovered.id)).toMatchObject({ status: "paused" });
    await webAgentClient.resumeLoop(recovered.id);
    await expect(webAgentClient.cancelLoop(recovered.id)).resolves.toMatchObject({
      status: "cancelled",
      terminalReason: "user-stopped",
    });
  });

  it("continues an awaiting Loop with feedback in a fresh revision", async () => {
    vi.useFakeTimers();
    const definition = await webAgentClient.createLoopDefinition(loopDefinitionInput);
    const started = await webAgentClient.startLoop(definition.id);
    await vi.advanceTimersByTimeAsync(900);

    const continued = await webAgentClient.continueLoop({
      runId: started.run.id,
      feedback: "Cover the boundary case",
    });
    expect(continued).toMatchObject({ status: "running", phase: "acting", currentIteration: 2 });
    expect(continued.iterations[1].userFeedback).toBe("Cover the boundary case");
    await vi.advanceTimersByTimeAsync(700);
    expect(await webAgentClient.getLoopRun(started.run.id)).toMatchObject({
      status: "awaiting-acceptance",
      currentIteration: 2,
    });
  });

  it("fails when a required simulated verification command fails", async () => {
    vi.useFakeTimers();
    const definition = await webAgentClient.createLoopDefinition({
      ...loopDefinitionInput,
      verificationCommands: [{ ...loopDefinitionInput.verificationCommands[0], program: "false" }],
    });
    const started = await webAgentClient.startLoop(definition.id);
    await vi.advanceTimersByTimeAsync(900);

    const failed = await webAgentClient.getLoopRun(started.run.id);
    expect(failed).toMatchObject({ status: "failed", terminalReason: "verification-failed" });
    expect(failed.iterations[0].evidence).toEqual(expect.arrayContaining([
      expect.objectContaining({ kind: "verification", status: "failed", exitCode: 1 }),
      expect.objectContaining({ kind: "decision", status: "failed" }),
    ]));
  });

  it("lists agents and filters by capability tag", async () => {
    const allAgents = await webAgentClient.listAgents();
    const browserAgents = await webAgentClient.listAgents("browser");

    expect(allAgents.length).toBeGreaterThan(0);
    expect(browserAgents.every((agent) => agent.capabilityTags.includes("browser"))).toBe(true);
  });

  it("does not fake local CLI installation status in Web runtime", async () => {
    vi.useFakeTimers();
    const cliTools = await webAgentClient.listCliTools();

    expect(cliTools.map((tool) => tool.agentId)).toEqual(["claude-code", "codex-cli", "gemini-cli", "opencode"]);
    expect(cliTools.every((tool) => tool.installed === null)).toBe(true);
    expect(cliTools.every((tool) => tool.versionCheckStatus === "unsupported")).toBe(true);
    expect(cliTools.every((tool) => tool.installations.length === 0 && tool.lifecycleEligibility === "unavailable")).toBe(true);
    const operation = await webAgentClient.refreshCliDetections("codex-cli");
    expect(operation).toMatchObject({ status: "queued", relatedEntityId: "codex-cli" });

    await vi.advanceTimersByTimeAsync(950);
    await expect(webOperationClient.getOperationStatus(operation.id)).resolves.toMatchObject({ status: "failed" });
  });

  it("persists and resets structured CLI parameter profiles", async () => {
    const initial = await webAgentClient.listCliParameterProfiles();
    expect(initial.map((profile) => profile.agentId)).toEqual(["claude-code", "codex-cli", "gemini-cli", "opencode"]);

    const saved = await webAgentClient.saveCliParameterProfile({
      agentId: "codex-cli",
      selections: { ...initial[1].selections, sandbox: "read-only", ephemeral: true },
    });
    expect(saved.previewArgs).toContain("--ephemeral");
    expect((await webAgentClient.listCliParameterProfiles())[1].selections.sandbox).toBe("read-only");

    const reset = await webAgentClient.resetCliParameterProfile("codex-cli");
    expect(reset.selections.sandbox).toBe("default");
    expect(reset.selections.ephemeral).toBe(false);
  });

  it("selects compatible agents and rejects unsupported interaction modes", async () => {
    await expect(webAgentClient.selectAgent("opencode", "browser")).rejects.toThrow("does not support");

    const workflow = await webAgentClient.selectAgent("gemini-cli", "browser");

    expect(workflow.activeAgentId).toBe("gemini-cli");
    expect(workflow.activeInteractionMode).toBe("browser");
  });

  it("reports browser readiness and session details", async () => {
    const readiness = await webAgentClient.checkBrowserReadiness("gemini-cli");
    const launch = await webAgentClient.launchActiveWorkflow();
    const details = await webAgentClient.getSessionDetails();

    expect(readiness.ready).toBe(true);
    expect(launch.workflow.lifecycleState).toBe("running");
    expect(details.adapter).toBe("browser");
  });

  it("manages sessions with Web runtime behavior", async () => {
    const first = await createMockSession({
      agentId: "gemini-cli",
      interactionMode: "browser",
      projectPath: "D:\\example\\mobile-app",
    });
    const second = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Codex work",
    });

    expect(first.title).toMatch(/^mobile-app-/);
    expect(first.folder).toBe("D:\\example\\mobile-app");
    expect(first.projectPath).toBe("D:\\example\\mobile-app");
    expect(first.worktreePath).toBeNull();
    expect(second.title).toBe("Codex work");

    await webAgentClient.pinSession(first.id);
    const sessions = await webAgentClient.listSessions();
    expect(sessions[0]?.id).toBe(first.id);

    const active = await webAgentClient.switchSession(second.id);
    expect(active.id).toBe(second.id);
    expect((await webAgentClient.getActiveSession())?.id).toBe(second.id);

    await webAgentClient.archiveSession(second.id);
    expect(await webAgentClient.getActiveSession()).toBeNull();
    expect((await webAgentClient.listSessions()).some((session) => session.id === second.id)).toBe(false);
    expect((await webAgentClient.listArchivedSessions()).some((session) => session.id === second.id)).toBe(true);
    await expect(webAgentClient.switchSession(second.id)).rejects.toThrow("archived");

    const restored = await webAgentClient.unarchiveSession(second.id);
    expect(restored.archived).toBe(false);

    await webAgentClient.deleteSession(second.id);
    expect((await webAgentClient.listSessions()).some((session) => session.id === second.id)).toBe(false);
  });

  it("keeps IM source metadata through standard session actions", async () => {
    const session = seedWebImSessionForTest("feishu");
    expect(session.source).toEqual({ kind: "im", connector: "feishu" });
    expect(JSON.stringify(session)).not.toContain("externalChat");

    await webAgentClient.renameSession(session.id, "Feishu task");
    await webAgentClient.pinSession(session.id);
    expect((await webAgentClient.switchSession(session.id)).source).toEqual(session.source);
    const archived = await webAgentClient.archiveSession(session.id);
    expect(archived).toMatchObject({ title: "Feishu task", pinned: true, source: session.source });

    const restored = await webAgentClient.unarchiveSession(session.id);
    expect(restored.source).toEqual(session.source);
    await expect(webAgentClient.listMessages({ sessionId: session.id })).resolves.toEqual([]);
    await webAgentClient.deleteSession(session.id);
    expect((await webAgentClient.listSessions()).some((item) => item.id === session.id)).toBe(false);
  });

  it("searches sessions by title, project, and message content", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Searchable planning session",
      projectPath: "D:\\example\\search-project",
    });
    const config = await webAgentClient.getSessionChatConfig(session.id);
    await webAgentClient.sendMessage({ sessionId: session.id, content: "Discuss roadmap anchors", config, fileReferences: [{ id: "ref-1", path: "D:\\example\\search-project\\README.md", name: "README.md" }] });
    await vi.advanceTimersByTimeAsync(3_000);

    const titleResults = await webAgentClient.searchSessions({ query: "planning" });
    const projectResults = await webAgentClient.searchSessions({ query: "search-project" });
    const contentResults = await webAgentClient.searchSessions({ query: "roadmap anchors" });
    const messages = await webAgentClient.listMessages({ sessionId: session.id });

    expect(titleResults.some((result) => result.session.id === session.id && result.matches.some((match) => match.kind === "title"))).toBe(true);
    expect(projectResults.some((result) => result.session.id === session.id && result.matches.some((match) => match.kind === "project"))).toBe(true);
    expect(contentResults.some((result) => result.session.id === session.id && result.matches.some((match) => match.kind === "message"))).toBe(true);
    expect(messages.find((message) => message.role === "user")?.fileReferences?.[0]?.name).toBe("README.md");
  });

  it("manages session categories and clears assignments on deletion", async () => {
    const session = await createMockSession({
      agentId: "gemini-cli",
      interactionMode: "browser",
      title: "Categorized session",
    });
    const category = await webAgentClient.createSessionCategory({ name: "Planning" });
    const assigned = await webAgentClient.assignSessionCategory({ sessionId: session.id, categoryId: category.id });
    const renamed = await webAgentClient.renameSessionCategory({ categoryId: category.id, name: "Delivery" });

    expect(assigned.categoryId).toBe(category.id);
    expect((await webAgentClient.listSessionCategories()).map((item) => item.name)).toContain("Delivery");
    expect(renamed.name).toBe("Delivery");

    await webAgentClient.deleteSessionCategory(category.id);
    expect((await webAgentClient.switchSession(session.id)).categoryId).toBeNull();
  });

  it("persists automatic archival settings and rejects invalid thresholds", async () => {
    expect(await webAgentClient.getAutomaticArchivalSettings()).toEqual({ enabled: true, inactiveDays: 10 });
    await expect(webAgentClient.saveAutomaticArchivalSettings({ enabled: true, inactiveDays: 0 })).rejects.toThrow("Invalid");

    const saved = await webAgentClient.saveAutomaticArchivalSettings({ enabled: false, inactiveDays: 30 });

    expect(saved).toEqual({ enabled: false, inactiveDays: 30 });
    expect(await webAgentClient.getAutomaticArchivalSettings()).toEqual(saved);
  });

  it("manages scheduled task creation, enabled state, and deletion in Web runtime", async () => {
    const created = await webAgentClient.createScheduledTask({
      name: " Daily summary ",
      content: " Summarize project progress ",
      agentId: "codex-cli",
      frequency: { kind: "daily", timeOfDay: "09:30" },
    });

    expect(created).toMatchObject({
      name: "Daily summary",
      content: "Summarize project progress",
      agentId: "codex-cli",
      enabled: true,
      latestStatus: "never-run",
      latestRunAt: null,
    });
    expect(created.nextRunAt).toBeTruthy();
    expect((await webAgentClient.listScheduledTasks()).some((task) => task.id === created.id)).toBe(true);

    const disabled = await webAgentClient.setScheduledTaskEnabled({ taskId: created.id, enabled: false });
    expect(disabled.enabled).toBe(false);

    const enabled = await webAgentClient.setScheduledTaskEnabled({ taskId: created.id, enabled: true });
    expect(enabled.enabled).toBe(true);
    expect(enabled.nextRunAt).toBeTruthy();

    await expect(
      webAgentClient.createScheduledTask({
        name: "Bad task",
        content: "Run it",
        agentId: "unknown-agent",
        frequency: { kind: "minutes", interval: 5 },
      }),
    ).rejects.toThrow("Unsupported Agent");

    await webAgentClient.deleteScheduledTask(created.id);
    expect((await webAgentClient.listScheduledTasks()).some((task) => task.id === created.id)).toBe(false);
  });

  it("exports sessions as JSON or Markdown in Web preview", async () => {
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Exportable session",
    });
    const json = await webAgentClient.exportSession({ sessionId: session.id, format: "json", destinationDirectory: "D:\\exports" });
    const markdown = await webAgentClient.exportSession({ sessionId: session.id, format: "markdown", destinationDirectory: "D:\\exports" });
    const cancelled = await webAgentClient.exportSession({ sessionId: session.id, format: "json", destinationDirectory: null });

    expect(json.status).toBe("exported");
    expect(json.path).toContain(`${session.id}.json`);
    expect(json.content).toContain("\"version\": 1");
    expect(markdown.path).toContain(`${session.id}.md`);
    expect(markdown.content).toContain("# Exportable session");
    expect(cancelled.status).toBe("cancelled");
  });

  it("tracks known projects and mock Git inspection", async () => {
    const inspection = await webAgentClient.inspectProject("D:\\example\\git-app");
    expect(inspection.isGit).toBe(true);

    await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      projectPath: inspection.path,
    });
    const projects = await webAgentClient.listKnownProjects();

    expect(projects[0]).toMatchObject({
      path: "D:\\example\\git-app",
      displayName: "git-app",
      isGit: true,
    });
  });

  it("creates mock worktree sessions with sibling path and branch metadata", async () => {
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      projectPath: "D:\\code\\app",
      worktree: { enabled: true, name: "feature-a" },
    });

    expect(session.folder).toBe("D:\\code\\app-feature-a");
    expect(session.projectPath).toBe("D:\\code\\app");
    expect(session.worktreePath).toBe("D:\\code\\app-feature-a");
    expect(session.worktreeName).toBe("feature-a");
    expect(session.worktreeBranch).toBe("vanehub/feature-a");
  });

  it("creates and searches mock remote workspace sessions", async () => {
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Remote workspace",
      remoteWorkspace: {
        host: "remote.example.test",
        user: "dev",
        path: "/work/app",
        displayName: "Remote App",
      },
    });

    expect(session.folder).toBe("ssh://dev@remote.example.test/work/app");
    expect(session.projectPath).toBeNull();
    expect(session.worktreePath).toBeNull();
    expect(session.remoteWorkspace).toMatchObject({
      host: "remote.example.test",
      user: "dev",
      path: "/work/app",
      displayName: "Remote App",
      uri: "ssh://dev@remote.example.test/work/app",
    });

    const workspaces = await webAgentClient.listKnownRemoteWorkspaces();
    expect(workspaces[0]).toMatchObject({ uri: "ssh://dev@remote.example.test/work/app" });

    const results = await webAgentClient.searchSessions({ query: "remote.example.test" });
    expect(results.some((result) => result.session.id === session.id && result.matches.some((match) => match.kind === "project"))).toBe(true);
  });

  it("rejects invalid or unavailable mock worktree requests", async () => {
    await expect(
      webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        projectPath: "D:\\code\\app",
        worktree: { enabled: true, name: "..\\bad" },
      }),
    ).rejects.toThrow("Invalid worktree name");

    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      projectPath: "D:\\code\\non-git",
    });
    expect(session.folder).toBe("D:\\code\\non-git");
    expect(session.worktreePath).toBeNull();

    await expect(
      webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        projectPath: "D:\\code\\non-git",
        worktree: { enabled: true, name: "feature-a" },
      }),
    ).rejects.toThrow("Git worktree unavailable");
  });

  it("rejects incomplete or mixed mock remote workspace requests", async () => {
    await expect(
      webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        remoteWorkspace: { host: "", path: "/work/app" },
      }),
    ).rejects.toThrow("Remote workspace requires host and path");

    await expect(
      webAgentClient.createSession({
        agentId: "codex-cli",
        interactionMode: "cli",
        remoteWorkspace: { host: "remote.example.test", path: "/work/app" },
        worktree: { enabled: true, name: "feature-a" },
      }),
    ).rejects.toThrow("Remote workspace cannot use Git worktree");
  });

  it("stores messages and emits mock streaming events", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Streaming test",
    });
    const events: ChatStreamEvent[] = [];
    const unsubscribe = await webAgentClient.subscribeMessageEvents(session.id, (event) => {
      events.push(event);
    });

    const assistant = await webAgentClient.sendMessage({
      sessionId: session.id,
      content: "hello agent",
      config: {
        agentId: session.agentId,
        interactionMode: session.interactionMode,
        permissionMode: "default",
        streaming: true,
        thinking: true,
        longContext: false,
      },
    });

    expect(assistant.role).toBe("assistant");
    expect(assistant.status).toBe("streaming");
    expect((await webAgentClient.getActiveSession())?.lifecycleState).toBe("running");
    expect(await webAgentClient.listMessages({ sessionId: session.id })).toHaveLength(2);

    await vi.advanceTimersByTimeAsync(3_000);
    const messages = await webAgentClient.listMessages({ sessionId: session.id });
    const completedAssistant = messages.find((message) => message.id === assistant.id);

    expect(events.some((event) => event.type === "token")).toBe(true);
    expect(events.some((event) => event.type === "rich_block")).toBe(true);
    expect(events.some((event) => event.type === "completed")).toBe(true);
    expect(completedAssistant?.status).toBe("completed");
    expect(completedAssistant?.content).toContain("Mock codex-cli response");
    expect(completedAssistant?.richBlocks?.map((block) => block.kind)).toEqual(["card", "checklist"]);
    expect((await webAgentClient.getActiveSession())?.lifecycleState).toBe("idle");
    unsubscribe();
  });

  it("persists chat configuration per session and keeps session identity authoritative", async () => {
    const first = await createMockSession({ agentId: "codex-cli", interactionMode: "cli", title: "Config one" });
    const second = await createMockSession({ agentId: "gemini-cli", interactionMode: "browser", title: "Config two" });
    const events: string[] = [];
    const unsubscribe = await webAgentClient.subscribeSessionEvents((event) => events.push(event.kind));

    const saved = await webAgentClient.saveSessionChatConfig(first.id, {
      agentId: "claude-code",
      interactionMode: "browser",
      permissionMode: "agent",
      providerId: "openai",
      modelId: "gpt-5-4",
      reasoningDepth: "medium",
      streaming: false,
      thinking: false,
      longContext: true,
    });

    expect(saved).toMatchObject({
      agentId: "codex-cli",
      interactionMode: "cli",
      modelId: "gpt-5-4",
      permissionMode: "agent",
    });
    expect(await webAgentClient.getSessionChatConfig(first.id)).toEqual(saved);
    expect((await webAgentClient.getSessionChatConfig(second.id)).agentId).toBe("gemini-cli");
    expect(events).toContain("configuration-changed");
    unsubscribe();
  });

  it("rejects a second generation while the same session is streaming", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({ agentId: "codex-cli", interactionMode: "cli", title: "Concurrency" });
    const config = await webAgentClient.getSessionChatConfig(session.id);
    await webAgentClient.sendMessage({ sessionId: session.id, content: "first", config });

    await expect(webAgentClient.sendMessage({ sessionId: session.id, content: "second", config }))
      .rejects.toThrow("already active");
    await webAgentClient.stopGeneration(session.id);
  });

  it("aggregates mock usage statistics from completed assistant messages", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Usage stats",
    });
    const before = await webAgentClient.getUsageStatistics({ range: "all" });
    const content = "usage statistics please";
    await webAgentClient.sendMessage({
      sessionId: session.id,
      content,
      config: {
        agentId: session.agentId,
        interactionMode: session.interactionMode,
        permissionMode: "default",
        streaming: true,
        thinking: false,
        longContext: false,
      },
    });

    await vi.advanceTimersByTimeAsync(3_000);
    const after = await webAgentClient.getUsageStatistics({ range: "all" });

    expect(after.estimated.inputCharacters - before.estimated.inputCharacters).toBe(content.length);
    expect(after.estimated.outputCharacters).toBeGreaterThan(before.estimated.outputCharacters);
    expect(after.estimated.totalCharacters).toBe(
      after.estimated.inputCharacters + after.estimated.outputCharacters,
    );
    expect(after.reported.totalTokens).toBe(before.reported.totalTokens);
    expect(after.coverage.estimatedResponses).toBe(before.coverage.estimatedResponses + 1);
    expect(after.countedSessions).toBeGreaterThanOrEqual(before.countedSessions);
  });

  it("cancels active mock generation and preserves partial content", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "gemini-cli",
      interactionMode: "browser",
      title: "Cancel test",
    });
    const assistant = await webAgentClient.sendMessage({
      sessionId: session.id,
      content: "stop soon",
      config: {
        agentId: session.agentId,
        interactionMode: session.interactionMode,
        permissionMode: "default",
        streaming: true,
        thinking: false,
        longContext: false,
      },
    });

    await vi.advanceTimersByTimeAsync(420);
    await webAgentClient.stopGeneration(session.id);
    const messages = await webAgentClient.listMessages({ sessionId: session.id });
    const cancelledAssistant = messages.find((message) => message.id === assistant.id);

    expect(cancelledAssistant?.status).toBe("cancelled");
    expect(cancelledAssistant?.content.length).toBeGreaterThan(0);
    expect((await webAgentClient.getActiveSession())?.lifecycleState).toBe("stopped");
  });

  it("archives a running mock session by cancelling active generation first", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Archive running",
    });
    const assistant = await webAgentClient.sendMessage({
      sessionId: session.id,
      content: "archive while running",
      config: {
        agentId: session.agentId,
        interactionMode: session.interactionMode,
        permissionMode: "default",
        streaming: true,
        thinking: false,
        longContext: false,
      },
    });

    const archived = await webAgentClient.archiveSession(session.id);
    const messages = await webAgentClient.listMessages({ sessionId: session.id });
    const cancelledAssistant = messages.find((message) => message.id === assistant.id);

    expect(archived.archived).toBe(true);
    expect(archived.lifecycleState).toBe("stopped");
    expect(cancelledAssistant?.status).toBe("cancelled");
    expect(await webAgentClient.getActiveSession()).toBeNull();
  });

  it("manages mock Skills, mount paths, drift, and built-in restore", async () => {
    const initial = await webAgentClient.listSkills({ scope: "global" });
    expect(initial.skills.some((skill) => skill.id === "tdd-discipline")).toBe(true);

    const migration = await webAgentClient.updateSkillMountPath("codex-cli", ".codex/custom-skills");
    expect(migration.agentId).toBe("codex-cli");
    expect(migration.newMountPath).toBe(".codex/custom-skills");

    const created = await webAgentClient.createSkill({
      scope: "workspace",
      workspacePath: "D:\\example",
      id: "workspace-helper",
      metadata: {
        id: "workspace-helper",
        name: "Workspace Helper",
        description: "Workspace-local test Skill.",
        category: "testing",
        version: "1.0.0",
        triggers: ["workspace"],
      },
      body: "Body",
      enabled: true,
      boundAgentIds: ["codex-cli"],
      source: "user",
    });
    expect(created.bindings[0]?.mountPath).toBe(".codex/custom-skills");

    await webAgentClient.deleteSkill("code-review", { scope: "global" });
    const drift = await webAgentClient.detectSkillDrift({ scope: "global" });
    expect(drift.issues.some((issue) => issue.type === "deleted-builtin")).toBe(true);

    const sync = await webAgentClient.syncSkillDrift({ scope: "global" });
    expect(sync.restored).toContain("code-review");
  });

  it("manages mock Prompt Hooks, previews content, and stores safe traces", async () => {
    const initial = await webAgentClient.listPromptHooks();
    expect(initial.hooks.map((hook) => hook.category)).toEqual(
      expect.arrayContaining(["bootstrap", "callback", "dynamic", "law", "navigation", "routing", "static"]),
    );
    expect(initial.stats.builtin).toBeGreaterThanOrEqual(7);

    await expect(webAgentClient.setPromptHookEnabled("law-runtime-boundary", false)).rejects.toThrow("cannot be disabled");

    const disabled = await webAgentClient.setPromptHookEnabled("navigation-project-hints", false);
    expect(disabled.enabled).toBe(false);

    const rebound = await webAgentClient.setPromptHookCliBindings("navigation-project-hints", ["codex-cli"]);
    expect(rebound.cliBindings).toEqual(["codex-cli"]);
    await expect(webAgentClient.setPromptHookCliBindings("navigation-project-hints", ["unknown-cli"])).rejects.toThrow(
      "Unsupported",
    );

    const input: PromptHookMutationInput = {
      id: "user-review-focus",
      name: "Review Focus",
      description: "Adds a review-focused instruction.",
      category: "dynamic",
      stage: "per-turn",
      order: 450,
      templateBody: "Review focus for {{agentId}}: {{sampleInput}}",
      enabled: true,
      cliBindings: ["codex-cli"],
      governance: {
        safetyTier: "editable",
        transparencyTier: "visible-by-default",
        governanceTier: "human-gated",
      },
    };
    const created = await webAgentClient.createPromptHook(input);
    expect(created.source).toBe("user");

    const preview = await webAgentClient.previewPromptHook({
      hookId: created.id,
      agentId: "codex-cli",
      sampleInput: "check regressions",
    });
    expect(preview.renderedContent).toContain("Review focus for codex-cli");
    expect(preview.trace[0]).toMatchObject({ hookId: created.id, status: "fired" });
    expect(preview.trace[0]?.contentHash).toBeTruthy();

    const assembly = await webAgentClient.previewPromptAssembly({
      agentId: "codex-cli",
      sampleInput: "implement feature",
    });
    expect(assembly.renderedContent).toContain("implement feature");
    expect(assembly.trace.some((trace) => trace.status === "fired")).toBe(true);

    const traces = await webAgentClient.listPromptHookTraces(5);
    expect(traces.length).toBeGreaterThan(0);
    expect(traces[0]).not.toHaveProperty("renderedContent");

    const updated = await webAgentClient.updatePromptHook(created.id, {
      ...input,
      version: 2,
      name: "Review Focus Updated",
    });
    expect(updated.version).toBe(2);
    expect(updated.name).toBe("Review Focus Updated");

    await webAgentClient.deletePromptHook(created.id);
    expect((await webAgentClient.listPromptHooks()).hooks.some((hook) => hook.id === created.id)).toBe(false);
  });

  it("simulates Agent Terminal without writing terminal output to chat messages", async () => {
    vi.useFakeTimers();
    const session = await createMockSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      projectPath: "D:\\example\\terminal-project",
    });
    const events: string[] = [];
    const unsubscribe = await webAgentClient.subscribeAgentTerminalEvents(session.id, (event) => {
      events.push(event.type);
    });

    const terminal = await webAgentClient.openAgentTerminal(session.id, { rows: 24, cols: 80 });
    await vi.advanceTimersByTimeAsync(40);

    expect(terminal).toMatchObject({
      sessionId: session.id,
      agentId: "codex-cli",
      state: "running",
      capability: "simulated",
      retained: true,
    });
    expect((await webAgentClient.getActiveSession())?.runtimeSessionId).toBe(`web-runtime-${session.id}`);
    expect(events).toEqual(["runtime_session_id", "output"]);
    expect(await webAgentClient.listMessages({ sessionId: session.id })).toEqual([]);

    await webAgentClient.sendAgentTerminalInput(terminal.terminalId, "hello\r\n");
    await webAgentClient.resizeAgentTerminal(terminal.terminalId, { rows: 40, cols: 120 });
    await expect(webAgentClient.stopAgentTerminal(terminal.terminalId)).resolves.toBe(true);
    expect((await webAgentClient.getActiveSession())?.lifecycleState).toBe("stopped");
    unsubscribe();
  });
});
