import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type {
  SessionLogCorrelationFilters,
  SessionLogCoverage,
  SessionLogEntry,
  SessionLogLevel,
} from "../types/session-workspace";
import type { SessionLogNotice } from "../types/session-log-notice";
import { decideLiveNotice, type LiveNoticeDecision } from "./log-live-policy";
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
  /**
   * Set when a live notice arrived that the current filters could not be judged against.
   *
   * Not an error, and not a stale marker: the rows on screen are correct, and something happened
   * that this view cannot place among them. Refreshing resolves it; guessing would not.
   */
  firstPageInvalidated: boolean;
  clearPendingFocus: () => void;
  clearSeekStatus: () => void;
  /**
   * Feeds one live notice through the insertion policy.
   *
   * Returns what was done, so a caller can count what it is withholding without re-deriving the
   * decision — two answers to "was this row added" is exactly the drift the shared policy exists to
   * prevent.
   */
  applyLiveNotice: (notice: SessionLogNotice) => Promise<LiveNoticeDecision>;
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
  const [firstPageInvalidated, setFirstPageInvalidated] = useState(false);
  // Read inside async callbacks so a late response cannot resurrect a previous scope's rows.
  const generation = useRef(0);

  // The scope is compared by value, never by identity.
  //
  // Depending on the object would make every caller responsible for memoising it, and a caller
  // that passed an inline object would not get a warning — it would get an infinite render loop,
  // because the reset effect below would fire on every render and set state each time. Requiring
  // that discipline of every call site is a trap; deriving a key here is not.
  const scopeKey = JSON.stringify([
    scope.seatId ?? null,
    scope.runId ?? null,
    scope.traceId ?? null,
    scope.spanId ?? null,
    scope.operationId ?? null,
    scope.agentId ?? null,
  ]);
  // Levels are a filter too, and an inline array has the same identity problem.
  const levelsKey = [...levels].sort().join(",");

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
      setFirstPageInvalidated(false);
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
    setFirstPageInvalidated(false);
    if (!sessionId) return;
    setPendingRead(true);
  }, [levelsKey, scopeKey, search, sessionId]);

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

  const applyLiveNotice = useCallback(async (notice: SessionLogNotice): Promise<LiveNoticeDecision> => {
    const decision = decideLiveNotice(notice, { correlation: scope, levels, search, sessionId });
    if (decision === "ignore") return decision;
    if (decision === "invalidate") {
      setFirstPageInvalidated(true);
      return decision;
    }
    const attempt = generation.current;
    try {
      // The notice named a row; it did not carry one. Fetching by id keeps a single authoritative
      // shape for a record instead of two that can disagree about what it says.
      const record = await agentService.getSessionLogRecord(notice.recordId);
      if (!record || attempt !== generation.current) return decision;
      setEntries((current) => current.some((entry) => entry.id === record.id)
        ? current
        : [record, ...current]);
    } catch {
      // A row that could not be fetched is one this view cannot place. Saying so is the honest
      // outcome; silently dropping it would leave the list short with nothing to explain it.
      if (attempt === generation.current) setFirstPageInvalidated(true);
    }
    return decision;
  }, [levels, scope, search, sessionId]);

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
    firstPageInvalidated,
    applyLiveNotice,
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
