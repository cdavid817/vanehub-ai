import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../components/measured-virtual-list";
import { agentService } from "../services/runtime-agent-client";
import type {
  SessionLogCorrelationFilters,
  SessionLogEntry,
  SessionLogLevel,
} from "../types/session-workspace";
import { LogEntryArticle } from "./log-entry-article";
import {
  LogsCoverageNotice,
  LogsScopeChips,
  type SessionLogCorrelationKey,
} from "./logs-coverage";
import { LogsToolbar, LOG_LEVELS } from "./logs-toolbar";
import { useLogFollow } from "./use-log-follow";
import { useSessionLogs } from "./use-session-logs";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

type VirtualLogItem =
  | { kind: "entry"; entry: SessionLogEntry }
  | { kind: "load-more" };

export function LogsTab({
  correlation,
  isVisible = true,
  seatId = null,
  sessionId,
}: {
  /**
   * The scope this panel was opened under, chosen somewhere else — a trace, a run, an operation.
   * Rendered as chips so a narrower list cannot be mistaken for a quieter session.
   */
  correlation?: SessionLogCorrelationFilters;
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  seatId?: string | null;
  sessionId: string | null;
}) {
  const { i18n, t } = useTranslation();
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const [levels, setLevels] = useState<SessionLogLevel[]>(LOG_LEVELS);
  const [searchDraft, setSearchDraft] = useState("");
  const [search, setSearch] = useState("");
  const [timestampDraft, setTimestampDraft] = useState("");
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [exportError, setExportError] = useState<WorkspaceErrorKey | null>(null);
  const [cleared, setCleared] = useState<SessionLogCorrelationKey[]>([]);
  const follow = useLogFollow();

  // The scope as it stands after whatever the reader dropped. Derived rather than stored, so a new
  // scope arriving from another panel is not silently overridden by an older set of dismissals.
  const scope = useMemo<SessionLogCorrelationFilters>(() => {
    const active: SessionLogCorrelationFilters = { seatId, ...correlation };
    for (const key of cleared) active[key] = null;
    return active;
  }, [cleared, correlation, seatId]);

  const logs = useSessionLogs({ isVisible, levels, scope, search, sessionId });

  useEffect(() => {
    setExportMessage(null);
    setExportError(null);
    setCleared([]);
  }, [correlation, seatId, sessionId]);

  useEffect(() => {
    // A filter change replaces the result set, so the newest edge is the only honest place to be.
    setExportMessage(null);
    setExportError(null);
    listRef.current?.scrollToStart();
    follow.resumeAtNewest();
    // `follow` is stable per render but not referentially, and depending on it would re-run this on
    // every viewport report — which is exactly the automatic movement 8.13 exists to stop.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [levels, scope, search, sessionId]);

  useEffect(() => {
    if (!logs.pendingFocusId) return;
    const index = logs.entries.findIndex((entry) => entry.id === logs.pendingFocusId);
    if (index >= 0) listRef.current?.scrollToIndex(index, "center");
  }, [logs.entries, logs.pendingFocusId]);

  const virtualItems = useMemo<VirtualLogItem[]>(() => [
    ...logs.entries.map((entry) => ({ kind: "entry" as const, entry })),
    ...(logs.hasMore || logs.pageError ? [{ kind: "load-more" as const }] : []),
  ], [logs.entries, logs.hasMore, logs.pageError]);

  const jumpToLatest = useCallback(() => {
    listRef.current?.scrollToStart();
    follow.resumeAtNewest();
    void logs.refresh();
  }, [follow, logs]);

  function toggleLevel(level: SessionLogLevel) {
    setLevels((current) => current.includes(level)
      ? current.filter((item) => item !== level)
      : [...current, level]);
  }

  async function exportLogs() {
    if (!sessionId) return;
    try {
      const result = await agentService.exportSessionLogs({ sessionId, levels, search, ...scope });
      setExportMessage(result.status === "exported" && result.path
        ? t("sessionTabs.logs.exported", { path: result.path })
        : result.status === "unavailable"
          ? t("sessionTabs.logs.exportUnavailable")
          : null);
    } catch (reason: unknown) {
      setExportError(workspaceErrorKey(reason));
    }
  }

  if (!sessionId) return <WorkspaceState kind="unavailable" />;

  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <LogsToolbar
        following={follow.following}
        levels={levels}
        onExport={() => void exportLogs()}
        onJumpToLatest={jumpToLatest}
        onLocate={() => void logs.locateTimestamp(timestampDraft)}
        onSearchDraftChange={setSearchDraft}
        onSubmitSearch={() => setSearch(searchDraft.trim())}
        onTimestampDraftChange={(value) => {
          setTimestampDraft(value);
          logs.clearSeekStatus();
        }}
        onToggleLevel={toggleLevel}
        onTogglePause={() => follow.setPaused(!follow.paused)}
        paused={follow.paused}
        pendingCount={follow.pendingCount}
        searchDraft={searchDraft}
        seeking={logs.seeking}
        timestampDraft={timestampDraft}
      />
      <LogsScopeChips
        correlation={scope}
        onClear={(key) => setCleared((current) => current.includes(key) ? current : [...current, key])}
      />
      <LogsCoverageNotice coverage={logs.coverage} />
      {logs.seekStatus ? (
        <p className={logs.seekStatus === "invalid"
          ? "ucd-status-warning rounded border px-2 py-1 text-xs"
          : "rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground"} role="status">
          {t(`sessionTabs.logs.seek.${logs.seekStatus}`)}
        </p>
      ) : null}
      {exportMessage ? <p className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground">{exportMessage}</p> : null}
      {exportError ? <p className="ucd-status-warning rounded border px-2 py-1 text-xs" role="status">{t(exportError)}</p> : null}
      {logs.stale ? <p className="ucd-status-warning rounded border px-2 py-1 text-xs" role="status">{t("sessionTabs.logs.stale")}</p> : null}
      {logs.initialError ? (
        <div className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
          <WorkspaceState kind="error" message={t(logs.initialError)} />
          <div className="flex justify-center pb-2">
            <button className="h-8 rounded border border-border bg-background px-3 text-xs hover:bg-muted" onClick={() => void logs.retryInitial()} type="button">
              {t("sessionTabs.logs.retry")}
            </button>
          </div>
        </div>
      ) : logs.entries.length === 0 && logs.loading ? (
        <div className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
          <WorkspaceState kind="loading" />
        </div>
      ) : logs.entries.length === 0 ? (
        <div className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
          <WorkspaceState kind="empty" message={t("sessionTabs.logs.empty")} />
        </div>
      ) : (
        <MeasuredVirtualList
          ariaLabel={t("sessionTabs.logs.list")}
          className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]"
          estimateSize={() => 132}
          getItemKey={(item) => item.kind === "entry" ? item.entry.id : "load-more"}
          itemClassName="px-2 pt-2"
          items={virtualItems}
          onAtStartChange={follow.noteViewport}
          overscan={10}
          ref={listRef}
          renderItem={(item, index) => item.kind === "entry" ? (
            <LogEntryArticle
              entry={item.entry}
              focused={item.entry.id === logs.pendingFocusId}
              language={i18n.language}
              onFocused={logs.clearPendingFocus}
              position={index + 1}
              total={logs.entries.length}
            />
          ) : (
            <div className="flex flex-col items-center gap-1 pb-2" role="listitem">
              {/* Inline, at the continuation boundary: the rows above stay readable. */}
              {logs.pageError ? (
                <p className="ucd-status-warning rounded border px-2 py-1 text-xs" role="status">
                  {t("sessionTabs.logs.pageFailed")}
                </p>
              ) : null}
              <button
                className="h-8 rounded border border-border bg-background px-3 text-xs hover:bg-muted"
                disabled={logs.loading || logs.seeking}
                onClick={() => void logs.loadMore()}
                type="button"
              >
                {logs.loading
                  ? t("sessionTabs.state.loading")
                  : logs.pageError
                    ? t("sessionTabs.logs.retry")
                    : t("sessionTabs.logs.loadMore")}
              </button>
            </div>
          )}
          testId="session-log-virtual-list"
        />
      )}
    </div>
  );
}
