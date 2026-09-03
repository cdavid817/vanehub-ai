import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview } from "../types/code-review";
import type { MissionControlRunSummary } from "../types/mission-control";
import type { AsyncViewState } from "../ui/async/async-view-state";

/**
 * The Review facet's own fetch: unlike Timeline/Tools/Files/Usage/Logs, this needs no heuristic
 * join at all. `MissionControlRunSummary.navigation` is already `{ kind: "review", id, sessionId }`
 * whenever the backend's own facet-availability derivation marks "review" available — both read the
 * same run link (`link.linkType === "review"`, confirmed identical in `webMissionSummary`'s own
 * `review`/`facets` derivation, `web-mission-control-client.ts`), so `navigation.id` is already the
 * exact `reviewId` a single `agentService.getCodeReview` call needs. The backend's own gating
 * mounting this facet only when available makes `navigation.kind !== "review"` here a defensive
 * fallback for a data-integrity edge case, not a real, reachable path today.
 *
 * A single record by id is trivially bounded (16.12) — no list, no pagination, and this facet's own
 * render stays a summary (status/decision/counts), not the full diff/comment/finding content the
 * dedicated Review Center surface already owns.
 */
export function useMissionControlReview(
  run: MissionControlRunSummary,
  noReviewMessage: string,
  errorMessage: string,
): AsyncViewState<CodeReview> & { reload: () => void } {
  const [state, setState] = useState<AsyncViewState<CodeReview>>({
    data: undefined, initialLoading: true, refreshing: false, stale: false,
  });
  const generation = useRef(0);

  const load = useCallback(() => {
    const attempt = ++generation.current;
    setState((current) => ({
      data: current.data, initialLoading: current.data === undefined, refreshing: current.data !== undefined, stale: false,
    }));
    const reviewId = run.navigation?.kind === "review" ? run.navigation.id : null;
    if (!reviewId) {
      setState({
        data: undefined, initialLoading: false, refreshing: false, stale: false,
        error: { kind: "unavailable", message: noReviewMessage, retryable: false },
      });
      return;
    }
    void agentService.getCodeReview(reviewId).then((review) => {
      if (attempt !== generation.current) return;
      setState({ data: review, initialLoading: false, refreshing: false, stale: false });
    }).catch(() => {
      if (attempt !== generation.current) return;
      setState((current) => ({
        data: current.data, initialLoading: false, refreshing: false, stale: false,
        error: { kind: "error", message: errorMessage, retryable: true },
      }));
    });
  }, [run, noReviewMessage, errorMessage]);

  useEffect(() => { load(); }, [load]);

  return { ...state, reload: load };
}
