import { useCallback, useEffect, useRef, useState } from "react";
import { executionObservabilityService } from "../services/runtime-execution-observability-client";
import type { ExecutionTimeline } from "../types/execution-observability";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { AsyncViewState } from "../ui/async/async-view-state";
import { resolveSessionExecutionContext } from "./session-execution-context";

/**
 * Shared by the Timeline/Tools/Files facets — all three render the same underlying
 * `ExecutionTimeline` fetch (joined via `resolveSessionExecutionContext`, exactly as the Usage facet
 * joins it), just filtered to a different `ExecutionSpanKind` or not filtered at all for Timeline.
 * Only the fetch and its state live here; each facet owns its own filter and row rendering, since
 * which spans matter — and how to label them — differs per facet.
 *
 * `unavailableMessage`/`errorMessage` are already-translated strings from the calling facet, not
 * i18n keys resolved in here — Timeline/Tools/Files each want their own wording ("Could not load
 * tool calls…" vs. "…file operations…") for the identical fetch, so the copy has to stay the
 * caller's, the same way `ScheduledTaskHistory`'s own `AsyncBoundary` caller supplies its slots
 * rather than a shared hook guessing generic text. Both are plain strings, not an options object, so
 * a facet's own inline literal cannot recreate this hook's `useCallback` dependency identity on every
 * render the way a fresh object argument would (see memory: renderHook fresh-object-arg infinite loop).
 *
 * 16.11: returns `AsyncViewState<ExecutionTimeline>` (plus `reload`) instead of a bespoke
 * loading/empty/error/ready union, so every facet routes through the same `AsyncBoundary` the rest
 * of this codebase's list-shaped fetches already use — mirrors `useScheduledTaskHistory`'s own
 * `AsyncViewState<T> & { reload }` shape and its generation-guard against a stale response from a
 * run this view has since navigated away from landing after a newer one already resolved.
 *
 * The resolver finding nothing to join (no session link, or no execution run overlaps the Mission
 * Control run's own window) is reported as `error: { kind: "unavailable" }` — a real, honest "no
 * evidence to show," not a fetch failure — mirroring `AsyncBoundary`'s own vocabulary for the
 * backend's own facet-availability gating one level up (`mission-control-facets.tsx`). A resolved
 * run whose timeline genuinely has zero spans is real, valid data and stays in `data`, distinct from
 * this — a fetch that succeeded and found nothing is not the same situation as a join that found
 * nothing to fetch from.
 */
export function useExecutionTimeline(
  run: MissionControlRunSummary,
  unavailableMessage: string,
  errorMessage: string,
): AsyncViewState<ExecutionTimeline> & { reload: () => void } {
  const [state, setState] = useState<AsyncViewState<ExecutionTimeline>>({
    data: undefined, initialLoading: true, refreshing: false, stale: false,
  });
  const generation = useRef(0);

  const load = useCallback(() => {
    const attempt = ++generation.current;
    setState((current) => ({
      data: current.data, initialLoading: current.data === undefined, refreshing: current.data !== undefined, stale: false,
    }));
    void (async () => {
      const resolved = await resolveSessionExecutionContext(run, executionObservabilityService);
      if (!resolved) {
        if (attempt !== generation.current) return;
        setState({
          data: undefined, initialLoading: false, refreshing: false, stale: false,
          error: { kind: "unavailable", message: unavailableMessage, retryable: false },
        });
        return;
      }
      const timeline = await executionObservabilityService.getTimeline(resolved.runId);
      if (attempt !== generation.current) return;
      setState({ data: timeline, initialLoading: false, refreshing: false, stale: false });
    })().catch(() => {
      if (attempt !== generation.current) return;
      setState((current) => ({
        data: current.data, initialLoading: false, refreshing: false, stale: false,
        error: { kind: "error", message: errorMessage, retryable: true },
      }));
    });
  }, [run, unavailableMessage, errorMessage]);

  useEffect(() => { load(); }, [load]);

  return { ...state, reload: load };
}
