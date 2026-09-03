import { useCallback, useEffect, useRef, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type { SessionLogCoverage, SessionLogEntry } from "../types/session-workspace";
import type { SessionLogNotice } from "../types/session-log-notice";
import { decideLiveNotice, type LiveNoticeDecision } from "./log-live-policy";
import {
  appendUniqueLogs,
  isTimestampNewerThanLogs,
  parseTimestampInput,
  seekLogsByTimestamp,
} from "./log-list-utils";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

// Re-exported so every existing caller keeps importing the contract from the hook it belongs to.
// A split that made callers change their imports would be a rename wearing a refactor's clothes.
export type {
  SessionLogSeekStatus,
  SessionLogsScope,
  SessionLogsState,
} from "./session-logs-state";
import type {
  SessionLogSeekStatus,
  SessionLogsScope,
  SessionLogsState,
} from "./session-logs-state";

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
