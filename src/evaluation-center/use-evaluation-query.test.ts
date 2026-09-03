// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import { getActivePendingTimerCount } from "../testing/resource-tracking";
import type { EvaluationArena } from "../types/evaluation";
import { useEvaluationQuery } from "./use-evaluation-query";

function arena(outcome: EvaluationArena["attempts"][number]["outcome"]): EvaluationArena {
  return {
    id: "arena-1", operationId: "operation-1", taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "deterministic-v2",
    attempts: [{
      id: "attempt-1", arenaId: "arena-1", canonicalRunId: "run-1", taskId: "fix-null-auth-token", taskVersion: 1,
      agent: { agentId: "onepiece", providerId: "onepiece", modelId: null, interactionMode: "api", configurationFingerprint: "safe" },
      outcome, checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
    }],
  };
}

/**
 * Task 21.16: `use-evaluation-query.ts`'s reconcile-poll effect (18.2/18.6) had no direct hook-level
 * test before this -- only observed indirectly through the full `EvaluationCenter` page
 * (`evaluation-center.test.tsx`'s "pauses polling"/"stops polling once unmounted" tests, both of
 * which use a real 1.5s wall-clock wait and a mock-call-count proxy, not a live resource count).
 * This proves the actual `setInterval` handle count, at the hook boundary, using 21.15's
 * instrumentation -- same technique as use-mission-control-polling.test.ts's and
 * loop-run-polling.test.ts's own unmount tests.
 */
describe("useEvaluationQuery reconcile-poll interval (21.16)", () => {
  afterEach(() => {
    vi.useRealTimers();
    vi.restoreAllMocks();
  });

  it("arms exactly one interval while a non-terminal arena is loaded, and releases it on unmount", async () => {
    vi.useFakeTimers();
    vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationTasks").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [arena("queued")], nextCursor: null });

    const { result, unmount } = renderHook(() => useEvaluationQuery());
    // Flushes the initial load's Promise.all -- fake timers do not mock microtask scheduling, only
    // setInterval/setTimeout, so the already-resolved mocks above settle without advancing time.
    // Not `waitFor`: its own internal polling needs real time to pass, which fake timers never
    // supply on their own -- `act` alone is enough once the awaited microtasks resolve.
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.arenas).toHaveLength(1);

    expect(getActivePendingTimerCount()).toBe(1);

    unmount();
    expect(getActivePendingTimerCount()).toBe(0);
  });

  it("arms no interval at all once every loaded arena is terminal", async () => {
    vi.useFakeTimers();
    vi.spyOn(agentService, "listAgents").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationTasks").mockResolvedValue([]);
    vi.spyOn(agentService, "listEvaluationArenas").mockResolvedValue({ items: [arena("succeeded")], nextCursor: null });

    const { result } = renderHook(() => useEvaluationQuery());
    await act(async () => {
      await vi.advanceTimersByTimeAsync(0);
    });
    expect(result.current.arenas).toHaveLength(1);

    // No live run to reconcile -- the effect's own early return means it never even calls
    // `setInterval`, not merely "clears it again immediately".
    expect(getActivePendingTimerCount()).toBe(0);
  });
});
