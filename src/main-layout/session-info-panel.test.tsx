import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { readFileSync } from "node:fs";
import { renderToString } from "react-dom/server";
import { beforeEach, describe, expect, it } from "vitest";
import "../i18n";
import { activateAppLanguage } from "../i18n";
import type { Session } from "../types/agent";
import type { SessionUsageSummary } from "../types/chat";
import type { TokenUsageSummary, UsageMeasure, UsageQualityTotals } from "../types/token-usage";
import type { Skill } from "../types/skill";
import { SessionInfoPanel } from "./session-info-panel";

function session(): Session {
  return {
    id: "session-info-fixture",
    title: "CLI work",
    agentId: "codex-cli",
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "running",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
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
    layer: scope === "workspace" ? "project" : "user",
    origin: "created",
    trust: "trusted",
    availability: "available",
    immutable: false,
    shadowedDefinitions: [],
    usage: { viewCount: 0, useCount: 0, lastViewedAt: null, lastUsedAt: null, revisionWitness: null },
  };
}

function renderPanel(
  usage: SessionUsageSummary,
  overrideSession: Partial<Session> = {},
  currentSpeakerSeatId: string | null = null,
) {
  const activeSession = { ...session(), ...overrideSession };
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } });
  queryClient.setQueryData(["session-chat-config", activeSession.id], {
    agentId: "codex-cli",
    interactionMode: "cli",
    executionMode: "execute",
    providerId: "openai",
    modelId: "gpt-5-5",
    reasoningDepth: "high",
    streaming: true,
    thinking: true,
    longContext: false,
  });
  queryClient.setQueryData(["token-usage-summary", "session", activeSession.id], ledgerSummary(usage));
  queryClient.setQueryData(["skill-overview", { scope: "global", workspacePath: null }], {
    skills: [skill("global-codex", true, ["codex-cli"], "global")],
    stats: { total: 1, enabled: 1, mounted: 1 },
    agents: [{ id: "codex-cli", displayName: "Codex CLI", kind: "cli" }],
    apiAgentBindings: {},
    mountPaths: [],
    restoreCandidates: [],
    drift: { scope: "global", workspacePath: null, issues: [], driftHash: "clean" },
  });
  queryClient.setQueryData(["skill-overview", { scope: "workspace", workspacePath: activeSession.worktreePath }], {
    skills: [
      skill("project-codex", true, ["codex-cli"], "workspace"),
      skill("project-disabled", false, ["codex-cli"], "workspace"),
    ],
    stats: { total: 2, enabled: 1, mounted: 1 },
    agents: [{ id: "codex-cli", displayName: "Codex CLI", kind: "cli" }],
    apiAgentBindings: {},
    mountPaths: [],
    restoreCandidates: [],
    drift: { scope: "workspace", workspacePath: activeSession.worktreePath, issues: [], driftHash: "clean" },
  });

  return renderToString(
    <QueryClientProvider client={queryClient}>
      <SessionInfoPanel activeSession={activeSession} currentSpeakerSeatId={currentSpeakerSeatId} />
    </QueryClientProvider>,
  );
}

function ledgerMeasure(unit: "tokens" | "characters", total: number, calls: number): UsageMeasure {
  return { unit, dimensions: { input: 0, output: 0, cachedInput: 0, cacheWriteInput: 0, reasoningOutput: 0, providerTotal: total }, headlineTotal: total, callCount: calls, observationCount: calls };
}

function ledgerQuality(usage: SessionUsageSummary): UsageQualityTotals {
  return {
    reported: ledgerMeasure("tokens", usage.reported.totalTokens, usage.coverage.reportedResponses),
    reportedDerived: ledgerMeasure("tokens", 0, 0),
    estimated: ledgerMeasure("characters", usage.estimated.totalCharacters, usage.coverage.estimatedResponses),
  };
}

function ledgerSummary(usage: SessionUsageSummary): TokenUsageSummary {
  const totals = ledgerQuality(usage);
  return {
    schemaVersion: 1,
    totals,
    userResponse: totals,
    internal: {
      reported: ledgerMeasure("tokens", 0, 0),
      reportedDerived: ledgerMeasure("tokens", 0, 0),
      estimated: ledgerMeasure("characters", 0, 0),
    },
    counts: { calls: usage.responseCount, generations: usage.responseCount, sessions: usage.responseCount > 0 ? 1 : 0 },
    daily: [],
    breakdowns: [{ dimension: "purpose", entries: usage.responseCount > 0 ? [{ key: "assistant-initial", totals, counts: { calls: usage.responseCount, generations: usage.responseCount, sessions: 1 } }] : [] }],
    generatedAt: usage.generatedAt,
  };
}

describe("SessionInfoPanel", () => {
  beforeEach(async () => {
    await activateAppLanguage("en");
  });

  it("renders the session information panel with IM management and selected model", () => {
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
    expect(html).toContain('id="info-tab-im"');
    expect(html).toContain('data-testid="session-im-access"');
    expect(html).not.toContain(">Files<");
    expect(html).not.toContain(">Changes<");
    expect(html).not.toContain(">Logs<");
    expect(html).toContain("GPT-5.5");
    expect(html).toContain("Codex CLI");
    expect(html).not.toContain('data-testid="session-roster-editor"');
    expect(html).not.toContain('id="info-tab-members"');
    expect(html).toContain("grid-cols-4");
    expect(html).not.toContain("Collapse");
    expect(html).not.toContain("Expand info panel");
  });

  it("shows the independent membership card only after a session has held multiple participants", () => {
    const usage: SessionUsageSummary = {
      sessionId: "session-info-fixture",
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 0, totalResponses: 0, reportedPercent: 0 },
      responseCount: 0,
      generatedAt: "2026-07-20T00:00:00.000Z",
    };
    const html = renderPanel(usage, {
      seats: [
        { seatId: "seat-architect", agentId: "codex-cli", roleId: "architect", joinedAt: "2026-07-20T00:00:00.000Z" },
        { seatId: "seat-implementer", agentId: "claude-code", roleId: "implementer", joinedAt: "2026-07-20T00:00:00.000Z" },
      ],
    }, "seat-implementer");

    expect(html).toContain('data-testid="session-roster-editor"');
    expect(html).toContain("Member Information");
    expect(html).toContain('id="info-tab-members"');
    expect(html).toContain('data-testid="info-pane-members"');
    expect(html).toContain("grid-cols-5");
    const memberPane = html.indexOf('data-testid="info-pane-members"');
    const editor = html.indexOf('data-testid="session-roster-editor"');
    const basicPane = html.indexOf('data-testid="info-pane-basic"');
    expect(memberPane).toBeLessThan(editor);
    expect(editor).toBeLessThan(basicPane);
    expect(html).toContain('data-speaking="true"');
    expect(html).toContain('aria-current="true"');
    expect(html).toContain("starting");
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

    expect(html).toContain("Estimated characters");
    expect(html).toContain("Invocation details");
    expect(html).not.toContain("Code Index");
    expect(html).toContain("2,000");
  });

  it("adds a session-scoped code-index tab for a local OnePiece session", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 0 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 0, estimatedResponses: 0, totalResponses: 0, reportedPercent: 0 },
      responseCount: 0,
      generatedAt: "2026-07-20T00:00:00.000Z",
    }, { agentId: "onepiece", interactionMode: "api" });

    expect(html).toContain("Code Index");
    expect(html).toContain("grid-cols-5");
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

  it("shows backend-reported token totals directly, independent of live message state", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 12, outputTokens: 34, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 46 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
      responseCount: 1,
      generatedAt: "2026-07-20T00:00:00.000Z",
    });

    expect(html).toContain("46");
  });

  it("keeps Effective, Global, and Project Skill views mounted", () => {
    const html = renderPanel({
      sessionId: "session-info-fixture",
      reported: { inputTokens: 1, outputTokens: 1, cacheReadTokens: 0, cacheCreationTokens: 0, totalTokens: 2 },
      estimated: { inputCharacters: 0, outputCharacters: 0, totalCharacters: 0 },
      coverage: { reportedResponses: 1, estimatedResponses: 0, totalResponses: 1, reportedPercent: 100 },
      responseCount: 1,
      generatedAt: "2026-07-20T00:00:00.000Z",
    });

    expect(html).toContain("Effective");
    expect(html).toContain("Global");
    expect(html).toContain("Project");
    expect(html).toContain("global-codex");
    expect(html).toContain("project-codex");
    expect(html).toContain("project-disabled");
  });

  it("uses a contiguous themed surface without branching on registered style ids", () => {
    const source = ["./session-info-panel.tsx", "./session-skills-pane.tsx"]
      .map((path) => readFileSync(new URL(path, import.meta.url), "utf8"))
      .join("\n");

    expect(source).toContain("bg-[hsl(var(--panel-muted))]");
    expect(source).not.toContain('className={cn("ucd-panel');
    expect(source).toContain("ucd-muted-panel");
    expect(source).toContain("ucd-segmented");
    expect(source).not.toMatch(/theme\s*===\s*["'](?:minimal|futuristic)/);
    expect(source).not.toContain("invoke(");
  });
});
