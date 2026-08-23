import { useQuery, useQueryClient, type QueryClient } from "@tanstack/react-query";
import { useEffect, useRef } from "react";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type { EvidenceSessionId, WorkspaceEvidenceSummary } from "../types/session-workspace-evidence";
import { evidenceQueryKeys } from "./evidence-query-keys";
import type { WorkspaceSummaryState } from "./workspace-evidence-badges";
import {
  createEvidenceInvalidationBuffer,
  EVIDENCE_NOTICE_WINDOW_MS,
  type EvidenceInvalidation,
} from "./workspace-evidence-notices";

/**
 * The one summary read the workspace makes.
 *
 * Every badge and the Basic Info health block come from this. The alternative — each panel
 * mounting its own query so its tab can show a number — costs one request per tab on every session
 * open, and produces six answers measured at six different instants that then disagree.
 *
 * Disabled while hidden is not a case here: the tab bar is always on screen when a session is.
 */
export function useWorkspaceEvidenceSummary(
  sessionId: EvidenceSessionId | null,
  service: SessionWorkspaceEvidenceService = defaultAgentService,
): { summary: WorkspaceEvidenceSummary | undefined; state: WorkspaceSummaryState } {
  const query = useQuery({
    enabled: sessionId !== null,
    queryKey: evidenceQueryKeys.summary(sessionId),
    queryFn: () => {
      // Unreachable while `enabled` holds; throwing rather than inventing an id keeps the branded
      // session type meaning what it says.
      if (sessionId === null) throw new Error("A workspace evidence summary needs a session.");
      return service.getWorkspaceEvidenceSummary({ sessionId });
    },
  });

  // A failed summary is not an empty summary. Reporting `unavailable` keeps every badge a
  // placeholder instead of a confident zero.
  const state: WorkspaceSummaryState = query.data
    ? "ready"
    : query.isError
      ? "unavailable"
      : "loading";
  return { state, summary: query.data };
}

/**
 * One notice subscription per workspace, driving cache invalidation and nothing else.
 *
 * Notices carry identifiers, a sequence, and counts — never a command line, a log message, or a
 * diff. Keeping that true means they must not become React state: they are read here, folded into
 * a set of query keys, and dropped. Nothing this hook touches is rendered.
 */
export function useWorkspaceEvidenceNotices(
  sessionId: EvidenceSessionId | null,
  service: SessionWorkspaceEvidenceService = defaultAgentService,
  windowMs: number = EVIDENCE_NOTICE_WINDOW_MS,
): void {
  const queryClient = useQueryClient();
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (sessionId === null) return;
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    let released = false;
    let unsubscribe: (() => void) | null = null;

    const flush = () => {
      timer.current = null;
      const invalidation = buffer.drain();
      if (invalidation !== null && !released) {
        applyEvidenceInvalidation(queryClient, sessionId, invalidation);
      }
    };

    void service
      .subscribeExecutionEvidence({ sessionId }, (notice) => {
        // A notice for another session is dropped rather than applied: refetching the right keys
        // for the wrong reason hides that the subscription is pointed at the wrong place.
        if (!buffer.accept(notice) || released) return;
        if (timer.current === null) timer.current = setTimeout(flush, windowMs);
      })
      .then((release) => {
        if (released) release();
        else unsubscribe = release;
      })
      // A workspace whose live channel is unavailable still reads on demand; it just does not
      // learn about changes it did not cause. Failing the render would be worse.
      .catch(() => undefined);

    return () => {
      released = true;
      if (timer.current !== null) {
        clearTimeout(timer.current);
        timer.current = null;
      }
      unsubscribe?.();
    };
  }, [queryClient, service, sessionId, windowMs]);
}

/**
 * Turns one folded invalidation into query invalidations, and nothing broader than it earned.
 *
 * `broad` is the honest fallback: a sequence gap or an overflowed record set means the workspace
 * cannot say which rows moved, so claiming a narrow list would be claiming knowledge it lost.
 */
export function applyEvidenceInvalidation(
  queryClient: QueryClient,
  sessionId: EvidenceSessionId,
  invalidation: EvidenceInvalidation,
): void {
  const root = evidenceQueryKeys.all();
  if (invalidation.broad) {
    void queryClient.invalidateQueries({
      predicate: (query) =>
        query.queryKey[0] === root[0] && query.queryKey.flat(2).includes(sessionId),
    });
    return;
  }

  for (const family of invalidation.families) {
    if (family === "summary") {
      void queryClient.invalidateQueries({ queryKey: evidenceQueryKeys.summary(sessionId) });
      continue;
    }
    if (family === "record-detail") {
      for (const recordId of invalidation.recordIds) {
        void queryClient.invalidateQueries({
          queryKey: evidenceQueryKeys.recordDetail(sessionId, recordId),
        });
      }
      continue;
    }
    void queryClient.invalidateQueries({
      predicate: (query) =>
        query.queryKey[0] === root[0] &&
        query.queryKey[1] === family &&
        query.queryKey.flat(2).includes(sessionId),
    });
  }
}
