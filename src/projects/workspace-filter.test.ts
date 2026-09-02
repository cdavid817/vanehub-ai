import { describe, expect, it } from "vitest";
import { selectWorkspaceView } from "./workspace-filter";
import type { WorkspaceSummary } from "./workspace-summary";

function workspace(overrides: Partial<WorkspaceSummary> = {}): WorkspaceSummary {
  return {
    availability: "available", displayName: "app", displayPath: "D:\\repo\\app",
    kind: "local", lastOpenedAt: "2026-08-01T00:00:00.000Z", workspaceId: "D:\\repo\\app",
    ...overrides,
  };
}

describe("selectWorkspaceView", () => {
  it("sorts every view by most-recently-opened first", () => {
    const oldest = workspace({ lastOpenedAt: "2026-01-01T00:00:00.000Z", workspaceId: "oldest" });
    const newest = workspace({ lastOpenedAt: "2026-08-01T00:00:00.000Z", workspaceId: "newest" });
    expect(selectWorkspaceView([oldest, newest], "all").map((item) => item.workspaceId)).toEqual(["newest", "oldest"]);
  });

  it("keeps only non-available rows for the unavailable view", () => {
    const available = workspace({ availability: "available", workspaceId: "a" });
    const missing = workspace({ availability: "missing", workspaceId: "b" });
    const disconnected = workspace({ availability: "disconnected", workspaceId: "c" });
    const result = selectWorkspaceView([available, missing, disconnected], "unavailable");
    expect(result.map((item) => item.workspaceId).sort()).toEqual(["b", "c"]);
  });

  it("bounds the recent view without ever emptying it while the full list is non-empty", () => {
    const many = Array.from({ length: 20 }, (_, index) => workspace({
      lastOpenedAt: new Date(2026, 0, index + 1).toISOString(),
      workspaceId: `w-${index}`,
    }));
    const recent = selectWorkspaceView(many, "recent");
    expect(recent.length).toBeGreaterThan(0);
    expect(recent.length).toBeLessThan(many.length);
  });

  it("never produces the recent view empty when the underlying list is non-empty", () => {
    expect(selectWorkspaceView([workspace()], "recent")).toHaveLength(1);
  });
});
