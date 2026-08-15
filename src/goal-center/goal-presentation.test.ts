import { describe, expect, it } from "vitest";
import type { Goal, GoalLink } from "../contracts/goal";
import {
  blockingLinks, blockingReason, canAccept, groupLinks, progressLabel, unresolvableLinks,
} from "./goal-presentation";

function goal(overrides: Partial<Goal> = {}): Goal {
  const links = overrides.links ?? [];
  return {
    id: "goal-1", title: "发布目标系统", description: "", acceptanceNotes: "",
    status: "active", derivedStatus: "active", projectPath: null,
    createdAt: "2026-08-15T00:00:00Z", updatedAt: "2026-08-15T00:00:00Z",
    counted: 0, terminal: 0, unresolvable: 0, ...overrides, links,
  };
}

const link = (overrides: Partial<GoalLink> = {}): GoalLink => ({
  targetKind: "plan", targetId: "plan-1", progress: "active", ...overrides,
});

describe("goal presentation", () => {
  it("groups links by kind and drops kinds with nothing linked", () => {
    const groups = groupLinks([
      link({ targetKind: "loop", targetId: "loop-1" }),
      link({ targetKind: "plan", targetId: "plan-1" }),
      link({ targetKind: "plan", targetId: "plan-2" }),
    ]);

    expect(groups.map((group) => group.kind)).toEqual(["plan", "loop"]);
    expect(groups[0].links).toHaveLength(2);
  });

  it("counts only children that can block acceptance", () => {
    const links = [
      link({ targetId: "plan-1", progress: "active" }),
      link({ targetKind: "session", targetId: "session-1", progress: "active" }),
      link({ targetKind: "loop", targetId: "loop-1", progress: "unresolvable" }),
    ];

    expect(blockingLinks(goal({ links })).map((item) => item.targetId)).toEqual(["plan-1"]);
    expect(unresolvableLinks(goal({ links })).map((item) => item.targetId)).toEqual(["loop-1"]);
  });

  it("names why a goal is not ready instead of leaving a stalled bar unexplained", () => {
    expect(blockingReason(goal({ derivedStatus: "awaiting_acceptance" }))).toBe("none");
    expect(blockingReason(goal({ status: "draft", derivedStatus: "draft" }))).toBe("not-active");
    expect(blockingReason(goal({ counted: 0 }))).toBe("no-children");
    expect(blockingReason(goal({ counted: 2, terminal: 1 }))).toBe("children-running");
  });

  it("allows acceptance only from the derived awaiting state", () => {
    expect(canAccept(goal({ derivedStatus: "awaiting_acceptance" }))).toBe(true);
    expect(canAccept(goal({ derivedStatus: "active" }))).toBe(false);
    // A stored status is never awaiting acceptance, so it can never authorise it.
    expect(canAccept(goal({ status: "active", derivedStatus: "active", terminal: 3, counted: 3 }))).toBe(false);
  });

  it("reports progress against the counted children, not the raw link total", () => {
    expect(progressLabel(goal({ counted: 2, terminal: 1, unresolvable: 3 }))).toBe("1/2");
  });
});
