import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { SessionLogEntry, SessionLogLevel } from "../types/session-workspace";
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
  seatId: string | null;
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
  search,
  seatId,
  sessionId,
}: SessionLogsScope): SessionLogsState {
  const [entries, setEntries] = useState<SessionLogEntry[]>([]);
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
      const page = await agentService.listSessionLogs({ sessionId, levels, search, seatId, cursor: null });
      if (attempt !== generation.current) return;
      setEntries(page.items);
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
  }, [levels, search, seatId, sessionId]);

  useEffect(() => {
    generation.current += 1;
    setEntries([]);
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
      const page = await agentService.listSessionLogs({ sessionId, levels, search, seatId, cursor });
      if (attempt !== generation.current) return;
      setEntries((current) => appendUniqueLogs(current, page.items));
      setCursor(page.nextCursor);
      setHasMore(page.truncated);
    } catch (reason: unknown) {
      // The continuation boundary is unchanged, so Retry can resume from the same cursor.
      if (attempt === generation.current) setPageError(workspaceErrorKey(reason));
    } finally {
      if (attempt === generation.current) setLoading(false);
    }
  }, [cursor, levels, loading, search, seatId, seeking, sessionId]);

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
          seatId,
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
  }, [cursor, entries, hasMore, levels, search, seatId, sessionId]);

  return {
    entries,
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
