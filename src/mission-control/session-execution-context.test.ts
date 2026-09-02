import { afterEach, describe, expect, it, vi } from "vitest";
import type { ExecutionObservabilityService } from "../services/execution-observability-service";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { resolveSessionExecutionContext } from "./session-execution-context";

afterEach(() => vi.useRealTimers());

function summary(overrides: Partial<MissionControlRunSummary> = {}): MissionControlRunSummary {
  return {
    runId: "run-1", version: 1, ownerType: "agent", ownerId: "owner-1", agentId: "claude-code",
    title: "Run 1", state: "running", createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:00:00.000Z",
    endedAt: null, projectId: null, workspace: null, phase: null, attention: null, reasonCode: null,
    verification: "unavailable", tokens: null, cost: null, actions: [],
    navigation: { kind: "session", id: "session-1", sessionId: null },
    runner: null,
    ...overrides,
  };
}

function execRun(overrides: Partial<ExecutionRunSummary> = {}): ExecutionRunSummary {
  return {
    runId: "exec-1", traceId: "trace-1", rootSpanId: "span-1", source: "desktop", status: "succeeded",
    startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:05:00.000Z", sessionId: "session-1",
    ...overrides,
  };
}

// Only `listRuns` is exercised by the resolver; the rest of the interface is wired to a stub that
// fails loudly if a future change accidentally starts calling it, rather than one that silently
// returns plausible-looking nothing.
function fakeService(listRuns: ExecutionObservabilityService["listRuns"]): ExecutionObservabilityService {
  const unimplemented = () => { throw new Error("not implemented in this fake"); };
  return {
    getSettings: unimplemented, updateSettings: unimplemented, listRuns,
    getRun: unimplemented, getTimeline: unimplemented, getObservationCapabilities: unimplemented,
  };
}

describe("resolveSessionExecutionContext", () => {
  it("picks the candidate with the greatest window overlap among several", async () => {
    const run = summary({ createdAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:10:00.000Z" });
    const barelyOverlaps = execRun({ runId: "exec-a", startedAt: "2026-08-15T23:59:00.000Z", endedAt: "2026-08-16T00:00:30.000Z" });
    const mostlyOverlaps = execRun({ runId: "exec-b", startedAt: "2026-08-16T00:01:00.000Z", endedAt: "2026-08-16T00:09:00.000Z" });
    const noOverlap = execRun({ runId: "exec-c", startedAt: "2026-08-16T01:00:00.000Z", endedAt: "2026-08-16T02:00:00.000Z" });
    const listRuns = vi.fn().mockResolvedValue({ items: [barelyOverlaps, mostlyOverlaps, noOverlap], nextPageToken: null });

    const resolved = await resolveSessionExecutionContext(run, fakeService(listRuns));

    expect(resolved?.runId).toBe("exec-b");
    expect(listRuns).toHaveBeenCalledWith({ sessionId: "session-1", limit: 20 });
  });

  it("returns null when no candidate overlaps the run's window at all", async () => {
    const run = summary({ createdAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:10:00.000Z" });
    const listRuns = vi.fn().mockResolvedValue({
      items: [execRun({ startedAt: "2026-08-10T00:00:00.000Z", endedAt: "2026-08-10T00:05:00.000Z" })],
      nextPageToken: null,
    });

    expect(await resolveSessionExecutionContext(run, fakeService(listRuns))).toBeNull();
  });

  it("returns null without calling the service when the run has no session link", async () => {
    const run = summary({ navigation: null });
    const listRuns = vi.fn();

    expect(await resolveSessionExecutionContext(run, fakeService(listRuns))).toBeNull();
    expect(listRuns).not.toHaveBeenCalled();
  });

  it("resolves the session id from a review navigation target's own sessionId field", async () => {
    const run = summary({ navigation: { kind: "review", id: "review-1", sessionId: "session-9" } });
    const listRuns = vi.fn().mockResolvedValue({
      items: [execRun({ sessionId: "session-9", startedAt: "2026-08-16T00:00:00.000Z", endedAt: "2026-08-16T00:05:00.000Z" })],
      nextPageToken: null,
    });

    const resolved = await resolveSessionExecutionContext(run, fakeService(listRuns));

    expect(resolved?.sessionId).toBe("session-9");
    expect(listRuns).toHaveBeenCalledWith({ sessionId: "session-9", limit: 20 });
  });

  it("treats a still-in-progress Mission Control run's window as extending to now, not to updatedAt", async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2026-08-16T00:20:00.000Z"));
    // `updatedAt` is stale (minute 1); if the window used it instead of "now", this candidate —
    // which starts at minute 15 — would never overlap and this assertion would fail.
    const run = summary({ createdAt: "2026-08-16T00:00:00.000Z", updatedAt: "2026-08-16T00:01:00.000Z", endedAt: null });
    const stillRunning = execRun({ runId: "exec-still-running", startedAt: "2026-08-16T00:15:00.000Z", endedAt: null });
    const listRuns = vi.fn().mockResolvedValue({ items: [stillRunning], nextPageToken: null });

    const resolved = await resolveSessionExecutionContext(run, fakeService(listRuns));

    expect(resolved?.runId).toBe("exec-still-running");
  });

  it("ignores a candidate outside the requested session even if the adapter returned it", async () => {
    const listRuns = vi.fn().mockResolvedValue({ items: [execRun({ sessionId: "other-session" })], nextPageToken: null });

    expect(await resolveSessionExecutionContext(summary(), fakeService(listRuns))).toBeNull();
  });
});
