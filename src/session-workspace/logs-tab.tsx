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
import {
  appendUniqueLogs,
  isTimestampNewerThanLogs,
  parseTimestampInput,
  seekLogsByTimestamp,
} from "./log-list-utils";
import { LogEntryArticle } from "./log-entry-article";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

const logLevels: SessionLogLevel[] = ["error", "warn", "info", "debug"];
type SeekStatus = "continue" | "invalid" | "not-found" | null;
type VirtualLogItem =
  | { kind: "entry"; entry: SessionLogEntry }
  | { kind: "load-more" };

export function LogsTab({ sessionId }: { sessionId: string | null }) {
  const { i18n, t } = useTranslation();
  const listRef = useRef<MeasuredVirtualListHandle>(null);
  const [levels, setLevels] = useState<SessionLogLevel[]>(logLevels);
  const [searchDraft, setSearchDraft] = useState("");
  const [search, setSearch] = useState("");
  const [timestampDraft, setTimestampDraft] = useState("");
  const [entries, setEntries] = useState<SessionLogEntry[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [seeking, setSeeking] = useState(false);
  const [pendingFocusId, setPendingFocusId] = useState<string | null>(null);
  const [seekStatus, setSeekStatus] = useState<SeekStatus>(null);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);

  useEffect(() => {
    let active = true;
    setEntries([]);
    setCursor(null);
    setHasMore(false);
    setExportMessage(null);
    setSeekStatus(null);
    setPendingFocusId(null);
    listRef.current?.scrollToStart();
    if (!sessionId) return () => { active = false; };

    setLoading(true);
    setError(null);
    void agentService.listSessionLogs({ sessionId, levels, search, cursor: null })
      .then((page) => {
        if (!active) return;
        setEntries(page.items);
        setCursor(page.nextCursor);
        setHasMore(page.truncated);
      })
      .catch((reason: unknown) => {
        if (active) setError(workspaceErrorKey(reason));
      })
      .finally(() => {
        if (active) setLoading(false);
      });

    return () => { active = false; };
  }, [levels, search, sessionId]);

  useEffect(() => {
    if (!pendingFocusId) return;
    const index = entries.findIndex((entry) => entry.id === pendingFocusId);
    if (index >= 0) listRef.current?.scrollToIndex(index, "center");
  }, [entries, pendingFocusId]);

  const virtualItems = useMemo<VirtualLogItem[]>(() => [
    ...entries.map((entry) => ({ kind: "entry" as const, entry })),
    ...(hasMore ? [{ kind: "load-more" as const }] : []),
  ], [entries, hasMore]);

  function toggleLevel(level: SessionLogLevel) {
    setLevels((current) => current.includes(level)
      ? current.filter((item) => item !== level)
      : [...current, level]);
  }

  async function loadMore() {
    if (!sessionId || !cursor || loading || seeking) return;
    setLoading(true);
    setError(null);
    try {
      const page = await agentService.listSessionLogs({ sessionId, levels, search, cursor });
      setEntries((current) => appendUniqueLogs(current, page.items));
      setCursor(page.nextCursor);
      setHasMore(page.truncated);
    } catch (reason: unknown) {
      setError(workspaceErrorKey(reason));
    } finally {
      setLoading(false);
    }
  }

  async function locateTimestamp() {
    const target = parseTimestampInput(timestampDraft);
    if (target === null) {
      setSeekStatus("invalid");
      return;
    }
    if (!sessionId || entries.length === 0 || isTimestampNewerThanLogs(entries, target)) {
      setSeekStatus("not-found");
      return;
    }

    setSeeking(true);
    setSeekStatus(null);
    setError(null);
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
          cursor: pageCursor,
        }),
      });
      setEntries(result.entries);
      setCursor(result.nextCursor);
      setHasMore(result.hasMore);
      if (result.status === "found") {
        setPendingFocusId(result.entries[result.matchIndex].id);
      } else {
        setSeekStatus(result.status);
      }
    } catch (reason: unknown) {
      setError(workspaceErrorKey(reason));
    } finally {
      setSeeking(false);
    }
  }

  async function exportLogs() {
    if (!sessionId) return;
    try {
      const result = await agentService.exportSessionLogs({ sessionId, levels, search });
      setExportMessage(result.status === "exported" && result.path
        ? t("sessionTabs.logs.exported", { path: result.path })
        : result.status === "unavailable"
          ? t("sessionTabs.logs.exportUnavailable")
          : null);
    } catch (reason: unknown) {
      setError(workspaceErrorKey(reason));
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
            void locateTimestamp();
          }}
        >
          <input
            aria-label={t("sessionTabs.logs.timestamp")}
            className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm"
            onChange={(event) => {
              setTimestampDraft(event.target.value);
              setSeekStatus(null);
            }}
            type="datetime-local"
            value={timestampDraft}
          />
          <button
            className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted"
            disabled={seeking}
            type="submit"
          >
            <Clock3 className="h-3.5 w-3.5" aria-hidden="true" />
            {seeking ? t("sessionTabs.logs.seeking") : t("sessionTabs.logs.locate")}
          </button>
        </form>
        <button className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted" onClick={() => void exportLogs()} type="button">
          <Download className="h-3.5 w-3.5" aria-hidden="true" />
          {t("sessionTabs.logs.export")}
        </button>
      </div>
      {seekStatus ? (
        <p className={cn("rounded border px-2 py-1 text-xs", seekStatus === "invalid" ? "ucd-status-warning" : "border-border bg-muted text-muted-foreground")} role="status">
          {t(`sessionTabs.logs.seek.${seekStatus}`)}
        </p>
      ) : null}
      {exportMessage ? <p className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground">{exportMessage}</p> : null}
      {error ? (
        <div className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
          <WorkspaceState kind="error" message={t(error)} />
        </div>
      ) : entries.length === 0 && loading ? (
        <div className="min-h-0 flex-1 rounded-lg border border-border bg-[hsl(var(--panel-muted))]">
          <WorkspaceState kind="loading" />
        </div>
      ) : entries.length === 0 ? (
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
              focused={item.entry.id === pendingFocusId}
              language={i18n.language}
              onFocused={() => setPendingFocusId(null)}
              position={index + 1}
              total={entries.length}
            />
          ) : (
            <div className="flex justify-center pb-2" role="listitem">
              <button
                className="h-8 rounded border border-border bg-background px-3 text-xs hover:bg-muted"
                disabled={loading || seeking}
                onClick={() => void loadMore()}
                type="button"
              >
                {loading ? t("sessionTabs.state.loading") : t("sessionTabs.logs.loadMore")}
              </button>
            </div>
          )}
          testId="session-log-virtual-list"
        />
      )}
    </div>
  );
}
