import { useEffect, useState } from "react";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import { resolveSessionExecutionContext } from "./session-execution-context";

export type ExecutionTimelineState =
  | { status: "loading" }
  | { status: "empty" }
  | { status: "error" }
  | { status: "ready"; timeline: ExecutionTimeline };

/**
 * Shared by the Timeline/Tools/Files facets — all three render the same underlying
 * `ExecutionTimeline` fetch (joined via `resolveSessionExecutionContext`, exactly as the Usage facet
 * joins it), just filtered to a different `ExecutionSpanKind` or not filtered at all for Timeline.
 * Only the fetch and its state union live here; each facet owns its own filter and row rendering,
 * since which spans matter — and how to label them — differs per facet.
 *
 * Unlike Usage (which needs the resolved run's `sessionId` to query token usage), the timeline fetch
 * needs the resolved run's own `runId` — an `ExecutionRunId` — passed to
 * `executionObservabilityService.getTimeline`.
 *
 * `"empty"` mirrors the Usage facet's own empty semantics exactly: the resolver could not find an
 * execution-observability run for this Mission Control run at all (no session link, or no window
 * overlap — see `session-execution-context.ts`). That is a different situation from a successful
 * fetch whose timeline happens to have zero spans (or zero spans of a facet's own kind) — that case
 * is real, valid data and stays inside `"ready"`, left for each facet's own render to decide, rather
 * than this hook collapsing it into `"empty"`.
 */
export function useExecutionTimeline(run: MissionControlRunSummary): ExecutionTimelineState {
  const [state, setState] = useState<ExecutionTimelineState>({ status: "loading" });

  useEffect(() => {
    let cancelled = false;
    setState({ status: "loading" });
    void (async () => {
      const resolved = await resolveSessionExecutionContext(run, executionObservabilityService);
      if (!resolved) { if (!cancelled) setState({ status: "empty" }); return; }
      const timeline = await executionObservabilityService.getTimeline(resolved.runId);
      if (!cancelled) setState({ status: "ready", timeline });
    })().catch(() => { if (!cancelled) setState({ status: "error" }); });
    return () => { cancelled = true; };
  }, [run]);

  return state;
}
