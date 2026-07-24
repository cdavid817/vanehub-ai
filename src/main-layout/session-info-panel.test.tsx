import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { readFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { beforeEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { i18n } from "../i18n";
import type { Session } from "../types/agent";
import type { ChatMessage, SessionUsageSummary } from "../types/chat";
import type { Skill } from "../types/skill";
import { SessionInfoPanel } from "./session-info-panel";

function session(): Session {
  return {
    id: "session-info-fixture",
    title: "CLI work",
    agentId: "codex-cli",
    interactionMode: "cli",
    lifecycleState: "running",
    folder: "D:\\code\\vanehub-ai",
    projectPath: "D:\\code\\vanehub-ai",
    worktreePath: "D:\\code\\vanehub-ai-feature",
    worktreeName: "feature",
    worktreeBranch: "feature/info-panel",
  remoteWorkspace: null,
  remoteSshConnectionId: null,
  remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-07-20T00:00:00.000Z",
    updatedAt: "2026-07-20T00:00:00.000Z",
  };
}

function skill(id: string, enabled: boolean, boundAgentIds: string[], scope: "global" | "workspace"): Skill {
  const workspacePath = scope === "workspace" ? "D:\\code\\vanehub-ai-feature" : null;
  return {
    id,
    scope,
    workspacePath,
    source: "user",
    enabled,
    skillDir: `${workspacePath ?? "~"}/skills/${id}`,
    skillMdPath: `${workspacePath ?? "~"}/skills/${id}/SKILL.md`,
    contentHash: id,
    metadata: {
      id,
      name: id,
      description: `${id} description`,
      category: "testing",
      version: "1.0.0",
      triggers: [],
    },
    boundAgentIds,
    bindings: boundAgentIds.map((agentId) => ({
      agentId,
      mountPath: ".codex/skills",
      mountedPath: `.codex/skills/${id}`,
      mounted: enabled,
    })),
    createdAt: "2026-07-20T00:00:00.000Z",
    updatedAt: "2026-07-20T00:00:00.000Z",
  };
}

function renderPanel(usage: SessionUsageSummary, overrideSession: Partial<Session> = {}, messages: ChatMessage[] = []) {
  const activeSession = { ...session(), ...overrideSession };
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  queryClient.setQueryData(["session-chat-config", activeSession.id], {
    agentId: "codex-cli",
    interactionMode: "cli",
    permissionMode: "agent",
    providerId: "openai",
    modelId: "gpt-5-5",
    reasoningDepth: "high",
    streaming: true,
    thinking: true,
    longContext: false,
  });
  queryClient.setQueryData(["session-usage-summary", activeSession.id], usage);
  queryClient.setQueryData(["skills", "global", activeSession.id], {
    skills: [skill("global-codex", true, ["codex-cli"], "global")],
    stats: { total: 1, enabled: 1, mounted: 1 },
  });
  queryClient.setQueryData(["skills", "workspace", activeSession.worktreePath], {
    skills: [
      skill("project-codex", true, ["codex-cli"], "workspace"),
      skill("project-disabled", false, ["codex-cli"], "workspace"),
    ],
    stats: { total: 2, enabled: 1, mounted: 1 },
  });

  return renderToString(
    <QueryClientProvider client={queryClient}>
      <SessionInfoPanel activeSession={activeSession} collapsed={false} messages={messages} onCollapsedChange={vi.fn()} />
    </QueryClientProvider>,
  );
}

describe("SessionInfoPanel", () => {
  beforeEach(async () => {
    await i18n.changeLanguage("en");
  });

  it("renders the optimized three-tab information panel and selected model", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 10, outputTokens: 20, cacheReadTokens: 3, cacheCreationTokens: 2, totalTokens: 35 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
      responseCount: 1,
      generatedAt: "2026-07-20T00:00:00.000Z",
    });

    expect(html).toContain("Basic Info");
    expect(html).toContain("Token Usage");
    expect(html).toContain("Skill");
    expect(html).not.toContain(">Files<");
    expect(html).not.toContain(">Changes<");
    expect(html).not.toContain(">Logs<");
    expect(html).toContain("gpt-5-5");
    expect(html).toContain("Codex CLI");
  });

  it("keeps reported tokens primary and shows estimated fallback context separately", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 1200, outputCharacters: 800, totalCharacters: 2000 },
      coverage: { reportedResponses: 0, estimatedResponses: 2, totalResponses: 2, reportedPercent: 0 },
      responseCount: 2,
      generatedAt: "2026-07-20T00:00:00.000Z",
    });

    expect(html).toContain("No reported tokens yet");
    expect(html).toContain("Estimated Responses");
    expect(html).toContain("2,000");
  });

  it("normalizes Windows extended-length workspace paths for display", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 0, totalResponses: 0, reportedPercent: 0 },
      responseCount: 0,
      generatedAt: "2026-07-20T00:00:00.000Z",
    }, {
      projectPath: "\\\\?\\D:\\cdavid\\Documents\\code\\claude-code",
      worktreePath: null,
    });

    expect(html).toContain("D:\\cdavid\\Documents\\code\\claude-code");
    expect(html).not.toContain("\\\\?\\D:");
  });

  it("uses live message token usage while the session summary refreshes", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 0, totalResponses: 0, reportedPercent: 0 },
      responseCount: 0,
      generatedAt: "2026-07-20T00:00:00.000Z",
    }, {}, [{
      id: "assistant-1",
      sessionId: "session-info-fixture",
      role: "assistant",
      content: "done",
      status: "completed",
      tokenUsage: { input: 12, output: 34 },
      createdAt: "2026-07-20T00:00:00.000Z",
      updatedAt: "2026-07-20T00:00:01.000Z",
    }]);

    expect(html).toContain("12");
    expect(html).toContain("34");
    expect(html).toContain("46");
  });

  it("groups available CLI Skills separately from project Skills", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 1, outputTokens: 1, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 2 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
      responseCount: 1,
      generatedAt: "2026-07-20T00:00:00.000Z",
    });

    expect(html).toContain("Available Skills");
    expect(html).toContain("Project Skills");
    expect(html).toContain("global-codex");
    expect(html).toContain("project-codex");
    expect(html).toContain("project-disabled");
  });

  it("uses shared theme tokens without branching on registered style ids", () => {
    const source = readFileSync(new URL("./session-info-panel.tsx", import.meta.url), "utf8");

    expect(source).toContain("ucd-panel");
    expect(source).toContain("ucd-muted-panel");
    expect(source).toContain("ucd-segmented");
    expect(source).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
  });
});
