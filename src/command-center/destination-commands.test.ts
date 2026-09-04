import { describe, expect, it, vi } from "vitest";
import { DESTINATION_COMMANDS } from "./destination-commands";
import type { WorkbenchCommandContext } from "./command-center-types";

function context(overrides: Partial<WorkbenchCommandContext> = {}): WorkbenchCommandContext {
  return {
    location: { destination: "sessions", sessionId: null, creatingSession: false },
    navigate: vi.fn(),
    onOpenSettings: vi.fn(),
    onNewSession: vi.fn(),
    onToggleNavigation: vi.fn(),
    onToggleInspector: vi.fn(),
    onToggleFocusMode: vi.fn(),
    ...overrides,
  };
}

function byIdKeywords(id: string): string[] {
  return DESTINATION_COMMANDS.find((command) => command.id === id)!.keywords;
}

describe("DESTINATION_COMMANDS", () => {
  it("covers exactly the eight distinct deep-link targets, each with a unique id", () => {
    expect(DESTINATION_COMMANDS).toHaveLength(8);
    expect(new Set(DESTINATION_COMMANDS.map((command) => command.id)).size).toBe(8);
  });

  it("is always available, regardless of current location", () => {
    for (const command of DESTINATION_COMMANDS) {
      expect(command.isAvailable(context())).toBe(true);
      expect(command.isAvailable(context({ location: { destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined } }))).toBe(true);
    }
  });

  it("goToRuns navigates to the attention section", () => {
    const ctx = context();
    DESTINATION_COMMANDS.find((command) => command.id === "goto-runs")!.run(ctx);
    expect(ctx.navigate).toHaveBeenCalledWith({ destination: "runs", section: "attention", runId: undefined });
  });

  it("goToLoops and goToSchedules route to distinct Runs sections, not the attention default", () => {
    const loopsCtx = context();
    DESTINATION_COMMANDS.find((command) => command.id === "goto-loops")!.run(loopsCtx);
    expect(loopsCtx.navigate).toHaveBeenCalledWith({ destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined });

    const schedulesCtx = context();
    DESTINATION_COMMANDS.find((command) => command.id === "goto-schedules")!.run(schedulesCtx);
    expect(schedulesCtx.navigate).toHaveBeenCalledWith({ destination: "runs", section: "schedules", scheduleId: undefined });
  });

  it("goToPlan and goToGoals route to distinct Plan sections", () => {
    const planCtx = context();
    DESTINATION_COMMANDS.find((command) => command.id === "goto-plan")!.run(planCtx);
    expect(planCtx.navigate).toHaveBeenCalledWith({ destination: "plan", section: "board", viewId: undefined, workItemId: undefined });

    const goalsCtx = context();
    DESTINATION_COMMANDS.find((command) => command.id === "goto-goals")!.run(goalsCtx);
    expect(goalsCtx.navigate).toHaveBeenCalledWith({ destination: "plan", section: "goals", goalId: undefined });
  });

  it("goToPlan's keywords index Board as a synonym rather than duplicating a command for it", () => {
    const planCommand = DESTINATION_COMMANDS.find((command) => command.id === "goto-plan")!;
    expect(planCommand.keywords).toContain("board");
    expect(DESTINATION_COMMANDS.some((command) => command.id === "goto-board")).toBe(false);
  });

  it("goToRuns' keywords index Mission Control as a synonym rather than duplicating a command for it", () => {
    const runsCommand = DESTINATION_COMMANDS.find((command) => command.id === "goto-runs")!;
    expect(runsCommand.keywords).toContain("mission control");
    expect(DESTINATION_COMMANDS.some((command) => command.id === "goto-mission-control")).toBe(false);
  });

  it("22.2: indexes every LEGACY_DESTINATION_REDIRECTS old destination's real former label so a reader who still remembers the pre-redesign name finds its new home", () => {
    // Mirrors workbench-route.ts's own (private) LEGACY_DESTINATION_REDIRECTS map — five old
    // `/workspace/<id>` path segments, hardcoded here the same way workbench-route.test.ts
    // hardcodes the literal old paths, since the map itself is not exported. Each left-hand label
    // below is the exact former `layout.activityBar.*` value (git show b3ba029a^:src/i18n/locales/
    // en.json), not a guess at what a reader might type.
    const oldLabelToCommandId: Record<string, string> = {
      loops: "goto-loops", // old label "Loops" — unchanged
      "todo board": "goto-plan", // old label "Todo Board"
      "goal center": "goto-goals", // old label "Goal Center" — unchanged
      evaluations: "goto-quality", // old label "Evaluations"
      "mission control": "goto-runs", // old label "Mission Control"
    };
    for (const [oldLabel, commandId] of Object.entries(oldLabelToCommandId)) {
      expect(byIdKeywords(commandId).some((keyword) => keyword.toLowerCase().includes(oldLabel))).toBe(true);
    }
  });

  it("22.2: also indexes the raw legacy URL segments (LEGACY_DESTINATION_REDIRECTS' own keys) as search terms", () => {
    expect(byIdKeywords("goto-plan")).toContain("work-board");
    expect(byIdKeywords("goto-runs")).toContain("mission-control");
    // "evaluations" is both the old label and the old path segment for Quality — one keyword
    // already covers both, asserted by the label-coverage test above.
  });

  it("22.2: indexes the old zh-CN activity-bar labels too, since a command's keywords mix locales in one flat array", () => {
    // Old labels from git show b3ba029a^:src/i18n/locales/zh-CN.json.
    expect(byIdKeywords("goto-plan")).toContain("任务看板"); // old label for Todo Board
    expect(byIdKeywords("goto-quality")).toContain("agent 评测"); // old label for Evaluations
  });
});
