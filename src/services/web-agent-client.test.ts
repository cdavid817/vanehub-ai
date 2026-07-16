import { afterEach, describe, expect, it, vi } from "vitest";
import { webAgentClient } from "./web-agent-client";
import type { ChatStreamEvent } from "../types/chat";

afterEach(() => {
  vi.useRealTimers();
});

describe("webAgentClient", () => {
  it("lists agents and filters by capability tag", async () => {
    const allAgents = await webAgentClient.listAgents();
    const browserAgents = await webAgentClient.listAgents("browser");

    expect(allAgents.length).toBeGreaterThan(0);
    expect(browserAgents.every((agent) => agent.capabilityTags.includes("browser"))).toBe(true);
  });

  it("does not fake local CLI installation status in Web runtime", async () => {
    const cliTools = await webAgentClient.listCliTools();

    expect(cliTools.map((tool) => tool.agentId)).toEqual(["claude-code", "codex-cli", "gemini-cli", "opencode"]);
    expect(cliTools.every((tool) => tool.installed === null)).toBe(true);
    expect(cliTools.every((tool) => tool.versionCheckStatus === "unsupported")).toBe(true);
    await expect(webAgentClient.refreshCliDetections()).resolves.toMatchObject({ status: "failed" });
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
    const first = await webAgentClient.createSession({
      agentId: "gemini-cli",
      interactionMode: "browser",
    });
    const second = await webAgentClient.createSession({
      agentId: "codex-cli",
      interactionMode: "cli",
      title: "Codex work",
    });

    expect(first.title).toBe("New Session");
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

  it("stores messages and emits mock streaming events", async () => {
    vi.useFakeTimers();
    const session = await webAgentClient.createSession({
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
    expect(await webAgentClient.listMessages({ sessionId: session.id })).toHaveLength(2);

    await vi.advanceTimersByTimeAsync(3_000);
    const messages = await webAgentClient.listMessages({ sessionId: session.id });
    const completedAssistant = messages.find((message) => message.id === assistant.id);

    expect(events.some((event) => event.type === "token")).toBe(true);
    expect(events.some((event) => event.type === "completed")).toBe(true);
    expect(completedAssistant?.status).toBe("completed");
    expect(completedAssistant?.content).toContain("Mock codex-cli response");
    unsubscribe();
  });

  it("cancels active mock generation and preserves partial content", async () => {
    vi.useFakeTimers();
    const session = await webAgentClient.createSession({
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
  });
});
