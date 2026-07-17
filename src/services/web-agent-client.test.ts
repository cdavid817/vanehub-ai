import { afterEach, describe, expect, it, vi } from "vitest";
import { webAgentClient } from "./web-agent-client";
import { webOperationClient } from "./web-operation-client";
import type { CreateSessionInput, Session } from "../types/agent";
import type { ChatStreamEvent } from "../types/chat";

afterEach(() => {
  vi.useRealTimers();
});

describe("webAgentClient", () => {
  async function createMockSession(input: CreateSessionInput): Promise<Session> {
    vi.useFakeTimers();
    const operation = await webAgentClient.createSession(input);
    await vi.advanceTimersByTimeAsync(950);
    const completed = await webOperationClient.getOperationStatus(operation.id);
    expect(completed.status).toBe("succeeded");
    return completed.result as unknown as Session;
  }

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
    const operation = await webAgentClient.refreshCliDetections();
    expect(operation).toMatchObject({ status: "queued" });

    await vi.advanceTimersByTimeAsync(950);
    await expect(webOperationClient.getOperationStatus(operation.id)).resolves.toMatchObject({ status: "failed" });
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

    expect(first.title).toBe("新会话");
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
    expect(events.some((event) => event.type === "completed")).toBe(true);
    expect(completedAssistant?.status).toBe("completed");
    expect(completedAssistant?.content).toContain("Mock codex-cli response");
    expect((await webAgentClient.getActiveSession())?.lifecycleState).toBe("idle");
    unsubscribe();
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
});
