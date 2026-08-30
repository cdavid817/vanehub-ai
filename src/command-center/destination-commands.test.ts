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
});
