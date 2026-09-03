import { afterEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import { workBoardService } from "../services/runtime-work-board-client";
import type { Session } from "../types/agent";
import type { LoopDefinition } from "../types/loop";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import type { WorkItem } from "../types/work-board";
import { executionTargetSearchProviders } from "./execution-target-providers";

afterEach(() => vi.restoreAllMocks());

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1", personalizationMode: "standard", title: "Fix null auth token", agentId: "claude-code",
    interactionMode: "cli", lifecycleState: "running", recoveryStatus: "clean", recoveryRevision: 0,
    stateRevision: 0, historyRevision: 0, activeExecutionRunId: null, folder: null,
    projectPath: "D:\\code\\vanehub", worktreePath: null, worktreeName: null, worktreeBranch: null,
    remoteWorkspace: null, remoteSshConnectionId: null, remoteSshConnectionRevision: null,
    runtimeSessionId: null, categoryId: null, pinned: false, archived: false,
    createdAt: "2026-08-14T00:00:00.000Z", updatedAt: "2026-08-14T01:00:00.000Z",
    ...overrides,
  };
}

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "session", ownerId: "session-1", agentId: "claude-code",
    title: "Fix null auth token", state: "running", createdAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T01:00:00.000Z", endedAt: null, projectId: null, workspace: "D:\\code\\vanehub",
    phase: null, attention: null, reasonCode: null, verification: "unavailable", tokens: null, cost: null,
    actions: ["open"], navigation: null, runner: null,
    ...overrides,
  };
}

function overview(sections: Partial<{ attention: MissionControlRunSummary[]; active: MissionControlRunSummary[]; recent: MissionControlRunSummary[] }>): MissionControlOverview {
  return {
    counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 },
    attention: { items: sections.attention ?? [], nextCursor: null },
    active: { items: sections.active ?? [], nextCursor: null },
    recent: { items: sections.recent ?? [], nextCursor: null },
  };
}

function loopDefinition(overrides: Partial<LoopDefinition> = {}): LoopDefinition {
  return {
    id: "loop-1", name: "Fix auth loop", enabled: true, projectPath: "D:\\code\\vanehub", baseBranch: "main",
    goal: "Fix the bug", acceptanceCriteria: [], allowedPaths: [], protectedPaths: [], workerAgentId: "claude-code",
    verifierAgentId: "claude-code", verificationCommands: [],
    limits: { maxIterations: 5, stepTimeoutSeconds: 60, totalTimeoutSeconds: 600, maxConsecutiveRuntimeErrors: 2, maxConsecutiveNoProgress: 2 },
    version: 1, createdAt: "2026-08-14T00:00:00.000Z", updatedAt: "2026-08-14T01:00:00.000Z",
    ...overrides,
  };
}

function workItem(overrides: Partial<WorkItem> = {}): WorkItem {
  return {
    id: "item-1", title: "Fix auth token", description: "", stage: "planned", priority: "none", rank: 1,
    projectPath: "D:\\code\\vanehub", dueAt: null, archived: false,
    createdAt: "2026-08-14T00:00:00.000Z", updatedAt: "2026-08-14T01:00:00.000Z", sources: [],
    ...overrides,
  };
}

describe("executionTargetSearchProviders.session", () => {
  it("maps title, project path, and status key/tone", async () => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([{ session: session(), matches: [] }]);
    const options = await executionTargetSearchProviders.session("auth");
    expect(options).toEqual([{
      id: "session-1", title: "Fix null auth token", projectPath: "D:\\code\\vanehub",
      statusKey: "layout.lifecycle.running", statusTone: "running",
    }]);
  });

  it("forwards the query and limit to the real search service", async () => {
    const spy = vi.spyOn(agentService, "searchSessions").mockResolvedValue([]);
    await executionTargetSearchProviders.session("auth");
    expect(spy).toHaveBeenCalledWith({ query: "auth", limit: 20 });
  });

  it("never surfaces a message match's excerpt text", async () => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([
      { session: session(), matches: [{ kind: "message", excerpt: "SECRET_CONTENT should never leak", messageId: "m1" }] },
    ]);
    const options = await executionTargetSearchProviders.session("auth");
    expect(JSON.stringify(options)).not.toContain("SECRET_CONTENT");
  });
});

describe("executionTargetSearchProviders.run", () => {
  it("maps title, workspace-as-projectPath, and status key/tone", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: [run()] }));
    const options = await executionTargetSearchProviders.run("auth");
    expect(options).toEqual([{
      id: "run-1", title: "Fix null auth token", projectPath: "D:\\code\\vanehub",
      statusKey: "missionControl.state.running", statusTone: "running",
    }]);
  });

  it("reports a null projectPath honestly when the run carries no workspace", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: [run({ workspace: null })] }));
    const options = await executionTargetSearchProviders.run("auth");
    expect(options[0].projectPath).toBeNull();
  });

  it("dedupes a run appearing in more than one section", async () => {
    const duplicated = run({ runId: "run-1" });
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ attention: [duplicated], active: [duplicated] }));
    const options = await executionTargetSearchProviders.run("");
    expect(options).toHaveLength(1);
  });

  it("filters by a case-insensitive substring of the title", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      active: [run({ runId: "run-1", title: "Fix null auth token" }), run({ runId: "run-2", title: "Refactor search" })],
    }));
    const options = await executionTargetSearchProviders.run("AUTH");
    expect(options.map((option) => option.id)).toEqual(["run-1"]);
  });

  it("shows every candidate when the query is empty", async () => {
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({
      active: [run({ runId: "run-1" }), run({ runId: "run-2", title: "Refactor search" })],
    }));
    const options = await executionTargetSearchProviders.run("");
    expect(options).toHaveLength(2);
  });

  // 21.11 picker-query budget: `RESULT_LIMIT` (execution-target-providers.ts) is applied via
  // `.slice(0, RESULT_LIMIT)` client-side for this provider (unlike session's, which forwards a
  // `limit` to the real search service instead -- see "forwards the query and limit to the real
  // search service" above). Only the session half had a candidate-count assertion before this
  // pass; this proves the client-side truncation itself actually caps a real overflow, not just
  // that the constant exists in source.
  it("caps candidates at 20 even when the overview reports more", async () => {
    const many = Array.from({ length: 35 }, (_unused, index) => run({ runId: `run-${index}`, title: `Run ${index}` }));
    vi.spyOn(agentService, "getMissionControlOverview").mockResolvedValue(overview({ active: many }));
    const options = await executionTargetSearchProviders.run("");
    expect(options).toHaveLength(20);
  });
});

describe("executionTargetSearchProviders.loop", () => {
  it("maps name-as-title and enabled/disabled status", async () => {
    vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue([loopDefinition()]);
    const options = await executionTargetSearchProviders.loop("");
    expect(options).toEqual([{
      id: "loop-1", title: "Fix auth loop", projectPath: "D:\\code\\vanehub",
      statusKey: "loops.definition.enabled", statusTone: "success",
    }]);
  });

  it("maps a disabled definition to the disabled key and a neutral tone", async () => {
    vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue([loopDefinition({ enabled: false })]);
    const options = await executionTargetSearchProviders.loop("");
    expect(options[0]).toMatchObject({ statusKey: "loops.definition.disabled", statusTone: "neutral" });
  });

  it("filters by a case-insensitive substring of the name", async () => {
    vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue([
      loopDefinition({ id: "loop-1", name: "Fix auth loop" }),
      loopDefinition({ id: "loop-2", name: "Refactor search" }),
    ]);
    const options = await executionTargetSearchProviders.loop("AUTH");
    expect(options.map((option) => option.id)).toEqual(["loop-1"]);
  });

  // 21.11 picker-query budget: see the run provider's identical case above for why this is
  // asserted separately from `RESULT_LIMIT`'s own declaration -- `listLoopDefinitions` has no
  // server-side limit of its own (unlike session search), so this client-side slice is the only
  // thing standing between a large real definition catalog and an unbounded picker result list.
  it("caps candidates at 20 even when the registry reports more", async () => {
    const many = Array.from({ length: 35 }, (_unused, index) => loopDefinition({ id: `loop-${index}`, name: `Loop ${index}` }));
    vi.spyOn(agentService, "listLoopDefinitions").mockResolvedValue(many);
    const options = await executionTargetSearchProviders.loop("");
    expect(options).toHaveLength(20);
  });
});

describe("executionTargetSearchProviders.work_item", () => {
  it("maps title and stage status, excluding archived items", async () => {
    const spy = vi.spyOn(workBoardService, "listWorkItems").mockResolvedValue([workItem()]);
    const options = await executionTargetSearchProviders.work_item("auth");
    expect(spy).toHaveBeenCalledWith({ archived: false, query: "auth" });
    expect(options).toEqual([{
      id: "item-1", title: "Fix auth token", projectPath: "D:\\code\\vanehub",
      statusKey: "todoBoard.stage.planned", statusTone: "neutral",
    }]);
  });

  // 21.11 picker-query budget: `listWorkItems` takes the raw query but this provider still slices
  // its response client-side -- whatever the backend's own matching behavior returns, a reader
  // never sees more than 20 rows to pick from.
  it("caps candidates at 20 even when the backend reports more", async () => {
    const many = Array.from({ length: 35 }, (_unused, index) => workItem({ id: `item-${index}` }));
    vi.spyOn(workBoardService, "listWorkItems").mockResolvedValue(many);
    const options = await executionTargetSearchProviders.work_item("");
    expect(options).toHaveLength(20);
  });

  it.each([
    ["inbox", "neutral"],
    ["planned", "neutral"],
    ["in_progress", "running"],
    ["review", "attention"],
    ["done", "success"],
  ] as const)("maps stage %s to tone %s", async (stage, tone) => {
    vi.spyOn(workBoardService, "listWorkItems").mockResolvedValue([workItem({ stage })]);
    const options = await executionTargetSearchProviders.work_item("");
    expect(options[0].statusTone).toBe(tone);
  });
});
