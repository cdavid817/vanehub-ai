import type { ExecutionObservabilityService } from "../services/execution-observability-service";
import type { ExecutionRunSummary } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";

// A session realistically accumulates only a handful of execution-observability runs, so a bounded
// scan is plenty — this is not meant to be a paged search.
const SESSION_EXECUTION_RUN_SCAN_LIMIT = 20;

/**
 * The session id a Mission Control run is attached to, read from its own projected navigation
 * target rather than from `AgentRun.links` directly — `MissionControlRunSummary` never carries the
 * raw links array, only what the backend already resolved from it. A "review" navigation still
 * carries the session id (when the run is also session-linked) in its own `sessionId` field, which
 * is why both branches are checked rather than only `kind === "session"`.
 */
function sessionIdOfRun(run: MissionControlRunSummary): string | null {
  const navigation = run.navigation;
  if (!navigation) return null;
  if (navigation.kind === "session") return navigation.id;
  return navigation.sessionId ?? null;
}

/** `[start, end]` in epoch milliseconds, with an open end treated as extending to now. */
function windowOf(startedAt: string, endedAt: string | null | undefined): readonly [number, number] {
  return [Date.parse(startedAt), endedAt ? Date.parse(endedAt) : Date.now()];
}

function overlapMs(a: readonly [number, number], b: readonly [number, number]): number {
  return Math.max(0, Math.min(a[1], b[1]) - Math.max(a[0], b[0]));
}

/**
 * Joins a Mission Control run to the execution-observability run that most plausibly produced it.
 *
 * The two systems do not share an id space — `AgentRun.id` and `ExecutionRunId` are independent
 * native newtypes — so this is a heuristic join, not a lookup: every execution run recorded for the
 * same session is fetched, and whichever one overlaps the Mission Control run's own
 * `[createdAt, endedAt ?? now]` window the most is treated as the match. Zero overlapping candidates
 * (or no resolvable session at all) means there is nothing to join, reported as `null` rather than
 * guessed at — callers must not fall back to "most recent run regardless of overlap".
 *
 * Known, accepted limitation: a session with two genuinely concurrent execution runs (e.g. two
 * agent invocations racing in the same session) can still resolve to the wrong one, since overlap
 * alone cannot distinguish which of two simultaneous runs produced a given Mission Control run.
 * This is a deliberate, bounded-effort resolution — the backend does not yet expose an exact join,
 * and this does not pretend to invent one.
 */
export async function resolveSessionExecutionContext(
  run: MissionControlRunSummary,
  service: ExecutionObservabilityService,
): Promise<ExecutionRunSummary | null> {
  const sessionId = sessionIdOfRun(run);
  if (!sessionId) return null;

  const page = await service.listRuns({ sessionId, limit: SESSION_EXECUTION_RUN_SCAN_LIMIT });
  const runWindow = windowOf(run.createdAt, run.endedAt);

  let best: ExecutionRunSummary | null = null;
  let bestOverlap = 0;
  // Defensive re-filter: the join is only meaningful within the requested session even if an
  // adapter's own filtering were ever inexact, since a mismatched candidate here would be picked
  // silently rather than surfaced as a fetch error.
  for (const candidate of page.items.filter((item) => item.sessionId === sessionId)) {
    const overlap = overlapMs(runWindow, windowOf(candidate.startedAt, candidate.endedAt));
    if (overlap > bestOverlap) {
      best = candidate;
      bestOverlap = overlap;
    }
  }
  return best;
}
