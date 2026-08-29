import { useCallback, useEffect, useMemo, useState } from "react";
import { agentService } from "../services/runtime-agent-client";
import type {
  ActivitySeverity,
  SystemActivityHealth,
  SystemActivityReadState,
  SystemActivitySession,
  SystemActivityTimelineEntry,
  SystemActivityDashboardSummary,
} from "../services/system-activity-service";

export interface SystemActivityModel {
  sessions: SystemActivitySession[];
  selectedSessionId: string | null;
  entries: SystemActivityTimelineEntry[];
  nextCursor: string | null;
  staleGeneration: boolean;
  readState: SystemActivityReadState | null;
  health: SystemActivityHealth | null;
  dashboard: SystemActivityDashboardSummary[];
  loading: boolean;
  error: string | null;
  severityFilter: ActivitySeverity | null;
  searchText: string;
  selectSession: (sessionId: string) => void;
  loadMore: () => void;
  markReadThroughNewest: () => void;
  setSeverityFilter: (severity: ActivitySeverity | null) => void;
  setSearchText: (text: string) => void;
  refresh: () => void;
}

function errorCode(error: unknown): string {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  if (typeof error === "object" && error !== null && "code" in error) {
    return String((error as { code: unknown }).code);
  }
  return "system-activity-storage-unavailable";
}

export function useSystemActivity(): SystemActivityModel {
  const [sessions, setSessions] = useState<SystemActivitySession[]>([]);
  const [selectedSessionId, setSelectedSessionId] = useState<string | null>(null);
  const [entries, setEntries] = useState<SystemActivityTimelineEntry[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [staleGeneration, setStaleGeneration] = useState(false);
  const [readState, setReadState] = useState<SystemActivityReadState | null>(null);
  const [health, setHealth] = useState<SystemActivityHealth | null>(null);
  const [dashboard, setDashboard] = useState<SystemActivityDashboardSummary[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [severityFilter, setSeverityFilter] = useState<ActivitySeverity | null>(null);
  const [searchText, setSearchText] = useState("");
  const [refreshToken, setRefreshToken] = useState(0);

  const refresh = useCallback(() => setRefreshToken((token) => token + 1), []);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    Promise.all([agentService.listSystemActivitySessions(), agentService.getSystemActivityHealth()])
      .then(([listed, projectionHealth]) => {
        if (cancelled) return;
        setSessions(listed);
        setHealth(projectionHealth);
        setError(null);
        setSelectedSessionId((current) => current ?? listed[0]?.sessionId ?? null);
      })
      .catch((cause: unknown) => {
        if (!cancelled) setError(errorCode(cause));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [refreshToken]);

  useEffect(() => {
    if (!selectedSessionId) return;
    let cancelled = false;
    const selected = sessions.find((session) => session.sessionId === selectedSessionId);
    agentService
      .querySystemActivityTimeline({
        sessionId: selectedSessionId,
        severities: severityFilter ? [severityFilter] : undefined,
        searchText: searchText.trim() === "" ? undefined : searchText.trim(),
      })
      .then((result) => {
        if (cancelled) return;
        if (result.kind === "staleGeneration") {
          setStaleGeneration(true);
          setEntries([]);
          setNextCursor(null);
          return;
        }
        setStaleGeneration(false);
        setEntries(result.entries);
        setNextCursor(result.nextCursor);
      })
      .catch((cause: unknown) => setError(errorCode(cause)));
    agentService
      .getSystemActivityReadState(selectedSessionId)
      .then((state) => {
        if (!cancelled) setReadState(state);
      })
      .catch(() => undefined);
    if (selected) {
      agentService
        .getSystemActivityDashboard(selected.scopeKind, selected.canonicalScopeId)
        .then((summaries) => {
          if (!cancelled) setDashboard(summaries);
        })
        .catch(() => undefined);
    }
    return () => {
      cancelled = true;
    };
  }, [selectedSessionId, severityFilter, searchText, sessions, refreshToken]);

  const loadMore = useCallback(() => {
    if (!selectedSessionId || !nextCursor) return;
    agentService
      .querySystemActivityTimeline({
        sessionId: selectedSessionId,
        severities: severityFilter ? [severityFilter] : undefined,
        searchText: searchText.trim() === "" ? undefined : searchText.trim(),
        cursor: nextCursor,
      })
      .then((result) => {
        if (result.kind !== "page") {
          setStaleGeneration(true);
          return;
        }
        setEntries((current) => [...current, ...result.entries]);
        setNextCursor(result.nextCursor);
      })
      .catch((cause: unknown) => setError(errorCode(cause)));
  }, [selectedSessionId, nextCursor, severityFilter, searchText]);

  const markReadThroughNewest = useCallback(() => {
    if (!selectedSessionId || !readState || entries.length === 0) return;
    const newest = entries[entries.length - 1].sequence;
    agentService
      .advanceSystemActivityReadCursor(selectedSessionId, newest, readState.revision)
      .then((state) => {
        setReadState(state);
        setSessions((current) =>
          current.map((session) =>
            session.sessionId === selectedSessionId
              ? { ...session, unreadCount: state.unreadCount }
              : session,
          ),
        );
      })
      .catch((cause: unknown) => setError(errorCode(cause)));
  }, [selectedSessionId, readState, entries]);

  const selectSession = useCallback((sessionId: string) => {
    setSelectedSessionId(sessionId);
    setSeverityFilter(null);
    setSearchText("");
  }, []);

  return useMemo(
    () => ({
      sessions,
      selectedSessionId,
      entries,
      nextCursor,
      staleGeneration,
      readState,
      health,
      dashboard,
      loading,
      error,
      severityFilter,
      searchText,
      selectSession,
      loadMore,
      markReadThroughNewest,
      setSeverityFilter,
      setSearchText,
      refresh,
    }),
    [
      sessions,
      selectedSessionId,
      entries,
      nextCursor,
      staleGeneration,
      readState,
      health,
      dashboard,
      loading,
      error,
      severityFilter,
      searchText,
      selectSession,
      loadMore,
      markReadThroughNewest,
      refresh,
    ],
  );
}
