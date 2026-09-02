import { describe, expect, it } from "vitest";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import {
  isTerminalMissionControlRunState,
  mergeMissionControlOverview,
  patchMissionControlRun,
  preferAuthoritativeRun,
} from "./mission-control-run-precedence";

function run(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:05:00.000Z",
    endedAt: null, projectId: null, workspace: null, phase: "running",
    attention: null, reasonCode: null, verification: "unavailable", tokens: null, cost: null,
    actions: ["open", "cancel"], navigation: null, runner: null,
    ...overrides,
  };
}

function overview(overrides: Partial<MissionControlOverview> = {}): MissionControlOverview {
  const counts = { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 };
  return {
    counts,
    attention: { items: [], nextCursor: null },
    active: { items: [], nextCursor: null },
    recent: { items: [], nextCursor: null },
    ...overrides,
  };
}

describe("isTerminalMissionControlRunState", () => {
  it("treats exactly completed, failed, and cancelled as terminal", () => {
    expect(isTerminalMissionControlRunState("completed")).toBe(true);
    expect(isTerminalMissionControlRunState("failed")).toBe(true);
    expect(isTerminalMissionControlRunState("cancelled")).toBe(true);
    for (const state of ["created", "preparing", "running", "waiting_approval", "waiting_user", "paused", "retrying", "blocked", "stuck", "verifying"] as const) {
      expect(isTerminalMissionControlRunState(state)).toBe(false);
    }
  });
});

describe("preferAuthoritativeRun", () => {
  it("accepts the incoming run when nothing is known yet", () => {
    const incoming = run({ version: 1 });
    expect(preferAuthoritativeRun(undefined, incoming)).toBe(incoming);
  });

  it("keeps a known terminal run over a later read reporting it as non-terminal", () => {
    const existing = run({ state: "cancelled", version: 5 });
    // A stale poll response fetched before the cancel landed, describing the run as still running.
    const staleIncoming = run({ state: "running", version: 3 });
    expect(preferAuthoritativeRun(existing, staleIncoming)).toBe(existing);
  });

  it("prefers the higher version when neither side is a stale terminal regression", () => {
    const existing = run({ state: "running", version: 3 });
    const newerIncoming = run({ state: "paused", version: 4 });
    expect(preferAuthoritativeRun(existing, newerIncoming)).toBe(newerIncoming);
  });

  it("keeps the existing run when an older-versioned response lands late", () => {
    const existing = run({ state: "paused", version: 4 });
    const olderIncoming = run({ state: "running", version: 3 });
    expect(preferAuthoritativeRun(existing, olderIncoming)).toBe(existing);
  });

  it("accepts an equal version (idempotent refetch of the same state)", () => {
    const existing = run({ state: "running", version: 3 });
    const sameVersion = run({ state: "running", version: 3 });
    expect(preferAuthoritativeRun(existing, sameVersion)).toBe(sameVersion);
  });
});

describe("mergeMissionControlOverview", () => {
  it("passes the incoming overview through unchanged when nothing was previously known", () => {
    const fresh = overview({ active: { items: [run({ version: 2 })], nextCursor: null } });
    expect(mergeMissionControlOverview(null, fresh)).toEqual(fresh);
  });

  it("guards a run already known terminal against a slower poll response landing late", () => {
    // The page already applied a cancel action's own receipt: version 6, terminal.
    const previous = overview({ active: { items: [run({ runId: "run-1", state: "cancelled", version: 6 })], nextCursor: null } });
    // A 2-second poll request that was in flight *before* the cancel resolved lands afterward,
    // still reporting the pre-cancel snapshot -- exactly the race task 16.15 calls out.
    const stalePoll = overview({ active: { items: [run({ runId: "run-1", state: "running", version: 5 })], nextCursor: null } });
    const merged = mergeMissionControlOverview(previous, stalePoll);
    expect(merged.active.items[0].state).toBe("cancelled");
    expect(merged.active.items[0].version).toBe(6);
  });

  it("accepts a genuinely newer poll response", () => {
    const previous = overview({ active: { items: [run({ runId: "run-1", state: "running", version: 3 })], nextCursor: null } });
    const newerPoll = overview({ active: { items: [run({ runId: "run-1", state: "paused", version: 4 })], nextCursor: null } });
    const merged = mergeMissionControlOverview(previous, newerPoll);
    expect(merged.active.items[0].state).toBe("paused");
    expect(merged.active.items[0].version).toBe(4);
  });

  it("always takes counts and page cursors from the incoming fetch", () => {
    const previous = overview({ counts: { running: 1, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 } });
    const incoming = overview({
      counts: { running: 2, waitingApproval: 1, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 },
      active: { items: [], nextCursor: "20" },
    });
    const merged = mergeMissionControlOverview(previous, incoming);
    expect(merged.counts.running).toBe(2);
    expect(merged.active.nextCursor).toBe("20");
  });
});

describe("patchMissionControlRun", () => {
  it("replaces a run in place within whichever section already shows it", () => {
    const before = overview({ active: { items: [run({ runId: "run-1", state: "running", version: 1 })], nextCursor: null } });
    const fresh = run({ runId: "run-1", state: "paused", version: 2, actions: ["open", "cancel", "resume"] });
    const patched = patchMissionControlRun(before, fresh);
    expect(patched.active.items).toEqual([fresh]);
  });

  it("drops a run from a section it no longer belongs to once it turns terminal, without inserting it elsewhere", () => {
    const before = overview({
      attention: { items: [run({ runId: "run-1", state: "waiting_approval", attention: "approval", version: 1 })], nextCursor: null },
      active: { items: [run({ runId: "run-1", state: "waiting_approval", attention: "approval", version: 1 })], nextCursor: null },
    });
    const fresh = run({ runId: "run-1", state: "cancelled", attention: null, version: 2, actions: ["open"] });
    const patched = patchMissionControlRun(before, fresh);
    expect(patched.attention.items).toEqual([]);
    expect(patched.active.items).toEqual([]);
    // "recent" is where a cancelled run belongs, but the client never fetched that page for this
    // run, so it must not fabricate an insertion there -- it stays absent until the next load().
    expect(patched.recent.items).toEqual([]);
  });

  it("moves a run into recent in place when it was already loaded there (e.g. a second action after the first landed)", () => {
    const before = overview({ recent: { items: [run({ runId: "run-1", state: "failed", version: 1 })], nextCursor: null } });
    const fresh = run({ runId: "run-1", state: "cancelled", version: 2 });
    const patched = patchMissionControlRun(before, fresh);
    expect(patched.recent.items).toEqual([fresh]);
  });

  it("leaves every section untouched when the run is not part of the currently loaded page", () => {
    const before = overview({ active: { items: [run({ runId: "run-other" })], nextCursor: null } });
    const fresh = run({ runId: "run-1", state: "cancelled", version: 2 });
    expect(patchMissionControlRun(before, fresh)).toEqual(before);
  });

  it("never regresses a section entry to a stale version even during a single-run patch", () => {
    const before = overview({ active: { items: [run({ runId: "run-1", state: "paused", version: 4 })], nextCursor: null } });
    // Defensive: a conflict refetch or receipt should never itself be older than what's shown, but
    // the merge still protects against it if one somehow were.
    const stale = run({ runId: "run-1", state: "running", version: 3 });
    const patched = patchMissionControlRun(before, stale);
    expect(patched.active.items[0].version).toBe(4);
  });

  it("leaves counts untouched -- corrected only by the next natural load()", () => {
    const before = overview({ counts: { running: 3, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 }, active: { items: [run({ runId: "run-1" })], nextCursor: null } });
    const fresh = run({ runId: "run-1", state: "cancelled", version: 2 });
    const patched = patchMissionControlRun(before, fresh);
    expect(patched.counts.running).toBe(3);
  });
});
