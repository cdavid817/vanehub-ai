// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import {
  decodeReturnToken,
  defaultWorkbenchLocation,
  extractReturnTo,
  legacyWorkbenchRedirectPath,
  parseWorkbenchLocation,
  recallWorkbenchPath,
  rememberWorkbenchLocation,
  withReturnTo,
  workbenchPath,
  type WorkbenchLocation,
} from "./workbench-route";

function parse(path: string): WorkbenchLocation {
  const [pathname, search] = path.split("?");
  return parseWorkbenchLocation(pathname, new URLSearchParams(search));
}

describe("workbenchPath / parseWorkbenchLocation round trip", () => {
  const cases: [string, WorkbenchLocation][] = [
    ["/workspace/sessions", { destination: "sessions", sessionId: null, creatingSession: false }],
    ["/workspace/sessions/new", { destination: "sessions", sessionId: null, creatingSession: true }],
    ["/workspace/sessions/abc-123", { destination: "sessions", sessionId: "abc-123", creatingSession: false }],
    ["/workspace/projects", { destination: "projects", projectId: undefined }],
    ["/workspace/projects/proj-1", { destination: "projects", projectId: "proj-1" }],
    ["/workspace/runs/attention", { destination: "runs", section: "attention", runId: undefined }],
    ["/workspace/runs/attention/run-1", { destination: "runs", section: "attention", runId: "run-1" }],
    ["/workspace/runs/active/run-2", { destination: "runs", section: "active", runId: "run-2" }],
    ["/workspace/runs/history/run-3", { destination: "runs", section: "history", runId: "run-3" }],
    ["/workspace/runs/loops", { destination: "runs", section: "loops", definitionId: undefined, loopRunId: undefined }],
    ["/workspace/runs/loops/def-1", { destination: "runs", section: "loops", definitionId: "def-1", loopRunId: undefined }],
    ["/workspace/runs/loops/def-1/loop-run-1", { destination: "runs", section: "loops", definitionId: "def-1", loopRunId: "loop-run-1" }],
    ["/workspace/runs/schedules", { destination: "runs", section: "schedules", scheduleId: undefined }],
    ["/workspace/runs/schedules/sched-1", { destination: "runs", section: "schedules", scheduleId: "sched-1" }],
    ["/workspace/plan/board", { destination: "plan", section: "board", viewId: undefined, workItemId: undefined }],
    ["/workspace/plan/board/item-1", { destination: "plan", section: "board", viewId: undefined, workItemId: "item-1" }],
    ["/workspace/plan/board?view=view-1", { destination: "plan", section: "board", viewId: "view-1", workItemId: undefined }],
    ["/workspace/plan/goals/goal-1", { destination: "plan", section: "goals", goalId: "goal-1" }],
    ["/workspace/quality/evaluations/exp-1", { destination: "quality", section: "evaluations", experimentId: "exp-1", comparisonIds: undefined }],
    [
      "/workspace/quality/evaluations/exp-1?compare=a,b",
      { destination: "quality", section: "evaluations", experimentId: "exp-1", comparisonIds: ["a", "b"] },
    ],
  ];

  it.each(cases)("round-trips %s", (path, location) => {
    expect(parse(path)).toEqual(location);
    expect(workbenchPath(location)).toBe(path);
  });

  it("decodes and encodes path-segment ids that contain reserved characters", () => {
    const location: WorkbenchLocation = { destination: "sessions", sessionId: "a/b c", creatingSession: false };
    const path = workbenchPath(location);
    expect(path).not.toContain("a/b c");
    expect(parse(path)).toEqual(location);
  });
});

describe("parseWorkbenchLocation — missing, malformed, and unsupported input", () => {
  it("falls back to the default location for an empty path", () => {
    expect(parse("")).toEqual(defaultWorkbenchLocation);
  });

  it("falls back to the default location for a path outside /workspace", () => {
    expect(parse("/settings")).toEqual(defaultWorkbenchLocation);
  });

  it("falls back to the default location for an unsupported destination id", () => {
    // The stable-id set this repo shipped before the redesign — must not resolve to a blank panel.
    // App.tsx now redirects these specific five (legacyWorkbenchRedirectPath, below) before this
    // fallback is ever reached; the parser itself stays destination-agnostic on purpose, since it
    // cannot and should not know which unrecognized ids are legacy versus simply nonexistent.
    expect(parse("/workspace/mission-control")).toEqual(defaultWorkbenchLocation);
    expect(parse("/workspace/work-board")).toEqual(defaultWorkbenchLocation);
  });

  it("falls back to the default location for a bare /workspace with no destination", () => {
    expect(parse("/workspace")).toEqual(defaultWorkbenchLocation);
  });
});

describe("legacyWorkbenchRedirectPath", () => {
  it("maps each pre-redesign flat destination to its new home", () => {
    expect(legacyWorkbenchRedirectPath("/workspace/loops")).toBe("/workspace/runs/loops");
    expect(legacyWorkbenchRedirectPath("/workspace/work-board")).toBe("/workspace/plan/board");
    expect(legacyWorkbenchRedirectPath("/workspace/goals")).toBe("/workspace/plan/goals");
    expect(legacyWorkbenchRedirectPath("/workspace/evaluations")).toBe("/workspace/quality/evaluations");
    expect(legacyWorkbenchRedirectPath("/workspace/mission-control")).toBe("/workspace/runs/attention");
  });

  it("returns null for a current-scheme path", () => {
    expect(legacyWorkbenchRedirectPath("/workspace/runs/loops")).toBeNull();
    expect(legacyWorkbenchRedirectPath("/workspace/sessions")).toBeNull();
  });

  it("returns null for a never-issued id, rather than guessing", () => {
    expect(legacyWorkbenchRedirectPath("/workspace/does-not-exist")).toBeNull();
  });

  it("returns null outside /workspace, and for a bare /workspace", () => {
    expect(legacyWorkbenchRedirectPath("/settings")).toBeNull();
    expect(legacyWorkbenchRedirectPath("/workspace")).toBeNull();
  });

  it("returns null for a legacy id with an extra segment, since the old scheme never produced one", () => {
    expect(legacyWorkbenchRedirectPath("/workspace/loops/extra")).toBeNull();
  });
});

describe("returnTo token", () => {
  it("round-trips a valid internal location", () => {
    const from: WorkbenchLocation = { destination: "runs", section: "active", runId: "run-1" };
    const path = withReturnTo("/workspace/sessions/abc", from);
    const [, search] = path.split("?");
    expect(extractReturnTo(new URLSearchParams(search))).toEqual(from);
  });

  it("rejects a token that is not an internal /workspace path", () => {
    expect(decodeReturnToken("https://evil.example/steal")).toBeNull();
    expect(decodeReturnToken("//evil.example")).toBeNull();
    expect(decodeReturnToken("/settings")).toBeNull();
  });

  it("rejects a malformed percent-encoded token instead of throwing", () => {
    expect(decodeReturnToken("%")).toBeNull();
  });

  it("returns null for an absent token", () => {
    expect(extractReturnTo(new URLSearchParams())).toBeNull();
  });
});

describe("workbench location persistence", () => {
  afterEach(() => localStorage.clear());

  it("remembers and recalls a settled location, discarding an in-progress creation", () => {
    rememberWorkbenchLocation({ destination: "sessions", sessionId: null, creatingSession: true });
    expect(recallWorkbenchPath()).toBe("/workspace/sessions");
  });

  it("remembers a non-sessions destination with its section", () => {
    rememberWorkbenchLocation({ destination: "plan", section: "goals", goalId: "goal-1" });
    expect(recallWorkbenchPath()).toBe("/workspace/plan/goals/goal-1");
  });

  it("falls back to the default when nothing has been remembered yet", () => {
    expect(recallWorkbenchPath()).toBe(workbenchPath(defaultWorkbenchLocation));
  });

  it("falls back to the default for a corrupted stored value", () => {
    localStorage.setItem("vanehub.workbench.location.v1", "not-a-path");
    expect(recallWorkbenchPath()).toBe(workbenchPath(defaultWorkbenchLocation));
  });
});
