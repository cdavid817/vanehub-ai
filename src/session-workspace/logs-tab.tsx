import { useEffect, useMemo, useRef, useState } from "react";
import { Clock3, Download, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import {
  MeasuredVirtualList,
  type MeasuredVirtualListHandle,
} from "../components/measured-virtual-list";
import { cn } from "../lib/utils";
import { agentService } from "../services/runtime-agent-client";
import type { SessionLogEntry, SessionLogLevel } from "../types/session-workspace";
import { LogEntryArticle } from "./log-entry-article";
import { useSessionLogs } from "./use-session-logs";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

const logLevels: SessionLogLevel[] = ["error", "warn", "info", "debug"];
type VirtualLogItem =
  | { kind: "entry"; entry: SessionLogEntry }
  | { kind: "load-more" };

export function LogsTab({ seatId = null, sessionId }: { seatId?: string | null; sessionId: string | null }) {
  const { i18n, t } = useTranslation();
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const [levels, setLevels] = useState<SessionLogLevel[]>(logLevels);
  const [searchDraft, setSearchDraft] = useState("");
  const [search, setSearch] = useState("");
  const [timestampDraft, setTimestampDraft] = useState("");
  const [exportMessage, setExportMessage] = useState<string | null>(null);
  const [exportError, setExportError] = useState<WorkspaceErrorKey | null>(null);
  const logs = useSessionLogs({ levels, search, seatId, sessionId });

  useEffect(() => {
    setExportMessage(null);
    setExportError(null);
    listRef.current?.scrollToStart();
  }, [levels, search, seatId, sessionId]);

  useEffect(() => {
    if (!logs.pendingFocusId) return;
    const index = logs.entries.findIndex((entry) => entry.id === logs.pendingFocusId);
    if (index >= 0) listRef.current?.scrollToIndex(index, "center");
  }, [logs.entries, logs.pendingFocusId]);

  const virtualItems = useMemo<VirtualLogItem[]>(() => [
    ...logs.entries.map((entry) => ({ kind: "entry" as const, entry })),
    ...(logs.hasMore || logs.pageError ? [{ kind: "load-more" as const }] : []),
  ], [logs.entries, logs.hasMore, logs.pageError]);

  function toggleLevel(level: SessionLogLevel) {
    setLevels((current) => current.includes(level)
      ? current.filter((item) => item !== level)
      : [...current, level]);
  }

  async function exportLogs() {
    if (!sessionId) return;
    try {
      const result = await agentService.exportSessionLogs({ sessionId, levels, search, seatId });
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
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {logLevels.map((level) => (
          <button
            aria-pressed={levels.includes(level)}
            className={cn(
              "h-7 rounded border border-border px-2 text-xs uppercase",
              levels.includes(level) ? "bg-primary text-primary-foreground" : "bg-background text-muted-foreground",
            )}
            key={level}
            onClick={() => toggleLevel(level)}
            type="button"
          >
            {t(`sessionTabs.logs.level.${level}`)}
          </button>
        ))}
        <form
          className="ml-auto flex min-w-48 flex-1 items-center gap-1 sm:max-w-sm"
          onSubmit={(event) => {
            event.preventDefault();
            setSearch(searchDraft.trim());
          }}
        >
          <input
            aria-label={t("sessionTabs.logs.search")}
            className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm"
            onChange={(event) => setSearchDraft(event.target.value)}
            placeholder={t("sessionTabs.logs.search")}
            value={searchDraft}
          />
          <button className="flex h-8 w-8 items-center justify-center rounded border border-border hover:bg-muted" title={t("sessionTabs.logs.search")} type="submit">
            <Search className="h-4 w-4" aria-hidden="true" />
          </button>
        </form>
        <form
          className="flex min-w-64 flex-1 items-center gap-1 sm:max-w-md"
          onSubmit={(event) => {
            event.preventDefault();
            void logs.locateTimestamp(timestampDraft);
          }}
        >
          <input
            aria-label={t("sessionTabs.logs.timestamp")}
            className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm"
            onChange={(event) => {
              setTimestampDraft(event.target.value);
              logs.clearSeekStatus();
            }}
            type="datetime-local"
            value={timestampDraft}
          />
          <button
            className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted"
            disabled={logs.seeking}
            type="submit"
          >
            <Clock3 className="h-3.5 w-3.5" aria-hidden="true" />
            {logs.seeking ? t("sessionTabs.logs.seeking") : t("sessionTabs.logs.locate")}
          </button>
        </form>
        <button className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted" onClick={() => void exportLogs()} type="button">
          <Download className="h-3.5 w-3.5" aria-hidden="true" />
          {t("sessionTabs.logs.export")}
        </button>
      </div>
      {logs.seekStatus ? (
        <p className={cn("rounded border px-2 py-1 text-xs", logs.seekStatus === "invalid" ? "ucd-status-warning" : "border-border bg-muted text-muted-foreground")} role="status">
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
