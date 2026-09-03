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
    // Bare list routes for the two destinations whose detail case was already covered above —
    // these are the section's own index, not a degenerate/malformed form, so they get the same
    // full parse+serialize round-trip guarantee as every other shape.
    ["/workspace/plan/goals", { destination: "plan", section: "goals", goalId: undefined }],
    ["/workspace/plan/goals/goal-1", { destination: "plan", section: "goals", goalId: "goal-1" }],
    [
      "/workspace/quality/evaluations",
      { destination: "quality", section: "evaluations", experimentId: undefined, comparisonIds: undefined },
    ],
    ["/workspace/quality/evaluations/exp-1", { destination: "quality", section: "evaluations", experimentId: "exp-1", comparisonIds: undefined }],
    [
      "/workspace/quality/evaluations/exp-1?compare=a,b",
      { destination: "quality", section: "evaluations", experimentId: "exp-1", comparisonIds: ["a", "b"] },
    ],
    // A single id with no comma is still a real, distinct shape from the two-id case above — the
    // split/join round trip must not silently require at least one comma to work.
    [
      "/workspace/quality/evaluations/exp-1?compare=a",
      { destination: "quality", section: "evaluations", experimentId: "exp-1", comparisonIds: ["a"] },
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

  // 21.3: every shape above already proves parse(path) and serialize(location) individually;
  // this proves the composition is stable too — feeding a parsed location back through
  // serialization and re-parsing must not drift, which matters because nothing about
  // `parseWorkbenchLocation`'s implementation guarantees it structurally (it is a hand-written
  // segment/search parser, not a schema with a derived inverse).
  it.each(cases)("stays stable across a second parse → serialize → parse cycle for %s", (path) => {
    const once = parse(path);
    const reparsed = parse(workbenchPath(once));
    expect(reparsed).toEqual(once);
  });

  it.each(["/workspace/sessions/", "/workspace/plan/board/", "/workspace/runs/loops/def-1/"])(
    "tolerates a trailing slash, parsing %s the same as without it",
    (pathWithTrailingSlash) => {
      expect(parse(pathWithTrailingSlash)).toEqual(parse(pathWithTrailingSlash.slice(0, -1)));
    },
  );
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

  it("ignores an empty compare query value instead of producing an empty-array comparisonIds", () => {
    expect(parse("/workspace/quality/evaluations/exp-1?compare=")).toEqual({
      destination: "quality",
      section: "evaluations",
      experimentId: "exp-1",
      comparisonIds: undefined,
    });
  });
});

/**
 * 21.3: `parseRunsSection`/`parsePlanSection` each have their own fallback for a sub-section id
 * they don't recognize — distinct from `parseWorkbenchLocation`'s own top-level fallback to
 * Sessions (above), and not covered by it: a syntactically valid `/workspace/<known-destination>/…`
 * path never reaches that top-level branch at all, since the destination itself *is* recognized.
 * Pinning this down matters because neither function documents it, and both resolve to a
 * same-destination default section rather than crashing or producing `undefined`/partial state —
 * confirmed here as real, current behavior (read from `workbench-route.ts` directly), not assumed.
 */
describe("parseWorkbenchLocation — unrecognized sub-section within a known destination", () => {
  it("falls back to the runs destination's own attention section for an unrecognized runs sub-route", () => {
    expect(parse("/workspace/runs/not-a-real-section/xyz")).toEqual({
      destination: "runs",
      section: "attention",
      runId: "xyz",
    });
  });

  it("falls back to the plan destination's own board section for an unrecognized plan sub-route", () => {
    expect(parse("/workspace/plan/not-a-real-section/xyz")).toEqual({
      destination: "plan",
      section: "board",
      viewId: undefined,
      workItemId: "xyz",
    });
  });

  it("resolves any quality sub-route to its one real section, evaluations", () => {
    // Quality has exactly one section today (QualitySection's only variant), so
    // parseQualitySection does not branch on the sub-route segment at all — any value here, valid
    // or not, resolves the same way. This pins that down as intentional rather than an oversight
    // that happens to look harmless only because there is nothing else to fall back to yet.
    expect(parse("/workspace/quality/not-a-real-section/xyz")).toEqual({
      destination: "quality",
      section: "evaluations",
      experimentId: "xyz",
      comparisonIds: undefined,
    });
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

  it("joins with & rather than overwriting when the target path already has its own query string", () => {
    const from: WorkbenchLocation = { destination: "plan", section: "goals", goalId: "goal-1" };
    const path = withReturnTo("/workspace/plan/board?view=view-1", from);
    expect(path).toBe(`/workspace/plan/board?view=view-1&returnTo=${encodeURIComponent("/workspace/plan/goals/goal-1")}`);
    // The target's own param must still be readable — a returnTo token is never allowed to
    // clobber the query string it rides along on.
    const [, search] = path.split("?");
    const params = new URLSearchParams(search);
    expect(params.get("view")).toBe("view-1");
    expect(extractReturnTo(params)).toEqual(from);
  });

  /**
   * The other cases in this block build a `URLSearchParams` from a manually `.split("?")` string,
   * matching every other helper in this file — safe here only because `encodeURIComponent` never
   * emits a raw `?`. This one instead goes through a real `URL` object end to end (construct →
   * `.pathname`/`.search` → parse), the same shape react-router hands `App.tsx` in production
   * (`location.pathname`/`location.search`, see `App.tsx`'s own `extractReturnTo` call site) — real
   * evidence the token survives actual URL embedding, not just this file's own string-splitting
   * helper.
   */
  it("survives being embedded in and re-parsed from a real URL object, including reserved characters in the target id", () => {
    const from: WorkbenchLocation = { destination: "sessions", sessionId: "run & <escape> #frag = x", creatingSession: false };
    const target = withReturnTo(workbenchPath({ destination: "runs", section: "active", runId: "run-1" }), from);
    const url = new URL(target, "https://vanehub.local");
    expect(extractReturnTo(url.searchParams)).toEqual(from);
    expect(parseWorkbenchLocation(url.pathname, url.searchParams)).toEqual({ destination: "runs", section: "active", runId: "run-1" });
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
