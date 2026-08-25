import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type {
  SessionLogCorrelationFilters,
  SessionLogCoverage,
  SessionLogEntry,
  SessionLogLevel,
} from "../types/session-workspace";
import {
  appendUniqueLogs,
  isTimestampNewerThanLogs,
  parseTimestampInput,
  seekLogsByTimestamp,
} from "./log-list-utils";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

export type SessionLogSeekStatus = "continue" | "invalid" | "not-found" | null;

export interface SessionLogsScope {
  sessionId: string | null;
  /**
   * Every correlation narrowing the read, as one value.
   *
   * One object rather than a parameter each, because the set grows: a new correlation added as its
   * own parameter is one the callers can forget to pass, and forgetting it widens the query
   * silently — the list gets bigger and nothing says why.
   */
  scope: SessionLogCorrelationFilters;
  levels: SessionLogLevel[];
  search: string;
  /** False while the panel stays mounted behind another tab. Defers reads, keeps rows. */
  isVisible?: boolean;
}

/**
 * Log page state, kept out of the view so a failure can be attributed to the request that failed.
 *
 * `initialError` blocks, because there is nothing to look at yet. `pageError` does not: a page
 * append or refresh that fails must leave the rows the user is already reading on screen, which
 * the previous single-error state could not express — one failed Load more replaced the whole
 * list with an error panel.
 */
export interface SessionLogsState {
  entries: SessionLogEntry[];
  /**
   * What the index was willing to claim about the rows below, as of the read that produced them.
   *
   * Kept beside the entries rather than fetched on its own, so a reader can never be shown rows
   * from one moment under a coverage claim from another. `undefined` until a page has answered,
   * which the view renders as `unavailable` — a coverage nobody reported is not a complete one.
   */
  coverage: SessionLogCoverage | undefined;
  hasMore: boolean;
  initialError: WorkspaceErrorKey | null;
  loading: boolean;
  pageError: WorkspaceErrorKey | null;
  pendingFocusId: string | null;
  seekStatus: SessionLogSeekStatus;
  seeking: boolean;
  stale: boolean;
  clearPendingFocus: () => void;
  clearSeekStatus: () => void;
  loadMore: () => Promise<void>;
  locateTimestamp: (draft: string) => Promise<void>;
  refresh: () => Promise<void>;
  retryInitial: () => Promise<void>;
}

export function useSessionLogs({
  isVisible = true,
  levels,
  scope,
  search,
  sessionId,
}: SessionLogsScope): SessionLogsState {
  const [entries, setEntries] = useState<SessionLogEntry[]>([]);
  const [coverage, setCoverage] = useState<SessionLogCoverage | undefined>(undefined);
  // A read the scope asked for that visibility has deferred. Without it, becoming visible again
  // would either re-read logs that are already on screen or leave the panel showing rows from a
  // filter the user has since changed.
  const [pendingRead, setPendingRead] = useState(false);
  const [cursor, setCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [seeking, setSeeking] = useState(false);
  const [initialError, setInitialError] = useState<WorkspaceErrorKey | null>(null);
  const [pageError, setPageError] = useState<WorkspaceErrorKey | null>(null);
  const [stale, setStale] = useState(false);
  const [pendingFocusId, setPendingFocusId] = useState<string | null>(null);
  const [seekStatus, setSeekStatus] = useState<SessionLogSeekStatus>(null);
  // Read inside async callbacks so a late response cannot resurrect a previous scope's rows.
  const generation = useRef(0);

  const loadFirstPage = useCallback(async (replaceOnFailure: boolean) => {
    if (!sessionId) return;
    const attempt = generation.current;
    setLoading(true);
    setPageError(null);
    if (replaceOnFailure) setInitialError(null);
    try {
      const page = await agentService.listSessionLogs({ sessionId, levels, search, ...scope, cursor: null });
      if (attempt !== generation.current) return;
      setEntries(page.items);
      setCoverage(page.coverage);
      setCursor(page.nextCursor);
      setHasMore(page.truncated);
      setInitialError(null);
      setStale(false);
    } catch (reason: unknown) {
      if (attempt !== generation.current) return;
      if (replaceOnFailure) setInitialError(workspaceErrorKey(reason));
      else {
        // Rows are already on screen; degrade to a stale marker instead of blanking them.
        setPageError(workspaceErrorKey(reason));
        setStale(true);
      }
    } finally {
      if (attempt === generation.current) setLoading(false);
    }
  }, [levels, scope, search, sessionId]);

  useEffect(() => {
    generation.current += 1;
    setEntries([]);
    setCoverage(undefined);
    setCursor(null);
    setHasMore(false);
    setInitialError(null);
    setPageError(null);
    setStale(false);
    setSeekStatus(null);
    setPendingFocusId(null);
    if (!sessionId) return;
    setPendingRead(true);
  }, [loadFirstPage, sessionId]);

  useEffect(() => {
    // Deferred rather than dropped: a hidden panel does not read logs, and the read it owed is
    // issued the moment it is on screen again.
    if (!pendingRead || !isVisible || !sessionId) return;
    setPendingRead(false);
    void loadFirstPage(true);
  }, [isVisible, loadFirstPage, pendingRead, sessionId]);

  const loadMore = useCallback(async () => {
    if (!sessionId || !cursor || loading || seeking) return;
    const attempt = generation.current;
    setLoading(true);
    setPageError(null);
    try {
      const page = await agentService.listSessionLogs({ sessionId, levels, search, ...scope, cursor });
      if (attempt !== generation.current) return;
      setEntries((current) => appendUniqueLogs(current, page.items));
      // The newest page is the one that describes the corpus; an older page appended below it
      // reports the same corpus and is allowed to update the claim.
      setCoverage(page.coverage);
      setCursor(page.nextCursor);
      setHasMore(page.truncated);
    } catch (reason: unknown) {
      // The continuation boundary is unchanged, so Retry can resume from the same cursor.
      if (attempt === generation.current) setPageError(workspaceErrorKey(reason));
    } finally {
      if (attempt === generation.current) setLoading(false);
    }
  }, [cursor, levels, loading, scope, search, seeking, sessionId]);

  const locateTimestamp = useCallback(async (draft: string) => {
    const target = parseTimestampInput(draft);
    if (target === null) {
      setSeekStatus("invalid");
      return;
    }
    if (!sessionId || entries.length === 0 || isTimestampNewerThanLogs(entries, target)) {
      setSeekStatus("not-found");
      return;
    }
    const attempt = generation.current;
    setSeeking(true);
    setSeekStatus(null);
    setPageError(null);
    try {
      const result = await seekLogsByTimestamp({
        entries,
        hasMore,
        nextCursor: cursor,
        targetTimestamp: target,
        loadPage: (pageCursor) => agentService.listSessionLogs({
          sessionId,
          levels,
          search,
          ...scope,
          cursor: pageCursor,
        }),
      });
      if (attempt !== generation.current) return;
      setEntries(result.entries);
      setCursor(result.nextCursor);
      setHasMore(result.hasMore);
      if (result.status === "found") setPendingFocusId(result.entries[result.matchIndex].id);
      else setSeekStatus(result.status);
    } catch (reason: unknown) {
      if (attempt === generation.current) setPageError(workspaceErrorKey(reason));
    } finally {
      if (attempt === generation.current) setSeeking(false);
    }
  }, [cursor, entries, hasMore, levels, scope, search, sessionId]);

  return {
    entries,
    coverage,
    hasMore,
    initialError,
    loading,
    pageError,
    pendingFocusId,
    seekStatus,
    seeking,
    stale,
    clearPendingFocus: () => setPendingFocusId(null),
    clearSeekStatus: () => setSeekStatus(null),
    loadMore,
    locateTimestamp,
    refresh: () => loadFirstPage(entries.length === 0),
    retryInitial: () => loadFirstPage(true),
  };
}
