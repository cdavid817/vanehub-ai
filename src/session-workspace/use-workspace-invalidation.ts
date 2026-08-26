import { useEffect } from "react";
import { useQueryClient } from "@tanstack/react-query";
import { agentService } from "../services/runtime-agent-client";
import type { WorkspaceInvalidationNotice } from "../types/session-workspace-inspection";
import { filterMatchesKey, invalidationFiltersFor } from "./workspace-invalidation-targets";

/**
 * Applies change notices to the query cache.
 *
 * One subscription for the whole workspace rather than one per panel. Panels mount and unmount as a
 * reader moves between tabs, and a subscription per panel would tear down and rebuild with them —
 * every switch a window in which a notice can be published to nobody, with nothing on screen to say
 * one went missing.
 *
 * Notices for other sessions are ignored here rather than filtered at the source. The native side
 * publishes for every session because it does not know which one is on screen, and a reader who
 * switches back to a session expects its panels to be refetched, not to be showing whatever they
 * held when they were last looked at.
 */
export function useWorkspaceInvalidation(sessionId: string | null): void {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!sessionId) return;
    let unsubscribe: (() => void) | null = null;
    let cancelled = false;

    const apply = (notice: WorkspaceInvalidationNotice) => {
      if (notice.sessionId !== sessionId) return;
      for (const filter of invalidationFiltersFor(notice)) {
        void queryClient.invalidateQueries(
          filter.pathWithin === undefined
            ? { queryKey: filter.queryKey }
            : {
                queryKey: filter.queryKey,
                predicate: (query) => filterMatchesKey(filter, query.queryKey),
              },
        );
      }
    };

    void agentService
      .subscribeWorkspaceInvalidation(apply)
      .then((release) => {
        // The effect can be torn down while `subscribe` is still in flight. Releasing immediately
        // in that case is the difference between a subscription that outlives its session and one
        // that does not; the alternative leaks a listener per session switch.
        if (cancelled) release();
        else unsubscribe = release;
      })
      .catch(() => {
        // A build with no notice channel is a build where nothing changes on its own. The panels
        // still read, refresh still works, and there is nothing here to report to a reader.
      });

    return () => {
      cancelled = true;
      unsubscribe?.();
    };
  }, [queryClient, sessionId]);
}
