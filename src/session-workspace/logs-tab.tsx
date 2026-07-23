import { useCallback, useEffect, useState } from "react";
import { Download, Search } from "lucide-react";
import { useTranslation } from "react-i18next";
import { agentService } from "../services/runtime-agent-client";
import type { SessionLogEntry, SessionLogLevel } from "../types/session-workspace";
import { cn } from "../lib/utils";
import { WorkspaceState } from "./workspace-state";
import { workspaceErrorKey, type WorkspaceErrorKey } from "./workspace-error";

const logLevels: SessionLogLevel[] = ["error", "warn", "info", "debug"];

export function LogsTab({ sessionId }: { sessionId: string | null }) {
  const { i18n, t } = useTranslation();
  const [levels, setLevels] = useState<SessionLogLevel[]>(logLevels);
  const [searchDraft, setSearchDraft] = useState("");
  const [search, setSearch] = useState("");
  const [entries, setEntries] = useState<SessionLogEntry[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [hasMore, setHasMore] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<WorkspaceErrorKey | null>(null);
  const [exportMessage, setExportMessage] = useState<string | null>(null);

  const load = useCallback(async (append: boolean, pageCursor: string | null) => {
    if (!sessionId) return;
    setLoading(true); setError(null);
    try {
      const page = await agentService.listSessionLogs({ sessionId, levels, search, cursor: append ? pageCursor : null });
      setEntries((current) => append ? [...current, ...page.items.filter((entry) => !current.some((item) => item.id === entry.id))] : page.items);
      setCursor(page.nextCursor); setHasMore(page.truncated);
    } catch (reason) { setError(workspaceErrorKey(reason)); }
    finally { setLoading(false); }
  }, [levels, search, sessionId]);

  useEffect(() => { setEntries([]); setCursor(null); setHasMore(false); setExportMessage(null); void load(false, null); }, [load]);

  function toggleLevel(level: SessionLogLevel) {
    setLevels((current) => current.includes(level) ? current.filter((item) => item !== level) : [...current, level]);
  }

  async function exportLogs() {
    if (!sessionId) return;
    try {
      const result = await agentService.exportSessionLogs({ sessionId, levels, search });
      setExportMessage(result.status === "exported" && result.path ? t("sessionTabs.logs.exported", { path: result.path }) : result.status === "unavailable" ? t("sessionTabs.logs.exportUnavailable") : null);
    } catch (reason) { setError(workspaceErrorKey(reason)); }
  }

  if (!sessionId) return <WorkspaceState kind="unavailable" />;
  return (
    <div className="flex h-full min-h-0 flex-col gap-3">
      <div className="flex flex-wrap items-center gap-2 rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {logLevels.map((level) => <button aria-pressed={levels.includes(level)} className={cn("h-7 rounded border border-border px-2 text-xs uppercase", levels.includes(level) ? "bg-primary text-primary-foreground" : "bg-background text-muted-foreground")} key={level} onClick={() => toggleLevel(level)} type="button">{t(`sessionTabs.logs.level.${level}`)}</button>)}
        <form className="ml-auto flex min-w-48 flex-1 items-center gap-1 sm:max-w-sm" onSubmit={(event) => { event.preventDefault(); setSearch(searchDraft.trim()); }}>
          <input aria-label={t("sessionTabs.logs.search")} className="ucd-input h-8 min-w-0 flex-1 rounded px-2 text-sm" onChange={(event) => setSearchDraft(event.target.value)} placeholder={t("sessionTabs.logs.search")} value={searchDraft} />
          <button className="flex h-8 w-8 items-center justify-center rounded border border-border hover:bg-muted" title={t("sessionTabs.logs.search")} type="submit"><Search className="h-4 w-4" /></button>
        </form>
        <button className="flex h-8 items-center gap-1 rounded border border-border px-2 text-xs hover:bg-muted" onClick={() => void exportLogs()} type="button"><Download className="h-3.5 w-3.5" />{t("sessionTabs.logs.export")}</button>
      </div>
      {exportMessage ? <p className="rounded border border-border bg-muted px-2 py-1 text-xs text-muted-foreground">{exportMessage}</p> : null}
      <div className="min-h-0 flex-1 overflow-y-auto rounded-lg border border-border bg-[hsl(var(--panel-muted))] p-2">
        {error ? <WorkspaceState kind="error" message={t(error)} /> : entries.length === 0 && loading ? <WorkspaceState kind="loading" /> : entries.length === 0 ? <WorkspaceState kind="empty" message={t("sessionTabs.logs.empty")} /> : <div className="grid gap-2">{entries.map((entry) => <article className="rounded border border-border bg-background p-2" key={entry.id}><div className="flex items-center justify-between gap-2 text-xs"><span className={cn("font-semibold uppercase", entry.level === "error" && "text-destructive", entry.level === "warn" && "text-primary")}>{entry.level}</span><time className="text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "short", timeStyle: "medium" }).format(new Date(entry.timestamp))}</time></div><p className="mt-1 text-xs text-muted-foreground">{entry.category}</p><p className="mt-1 whitespace-pre-wrap text-sm">{entry.message}</p>{Object.keys(entry.context).length > 0 ? <pre className="mt-2 overflow-auto rounded bg-muted p-2 text-xs">{JSON.stringify(entry.context, null, 2)}</pre> : null}</article>)}{hasMore ? <button className="mx-auto h-8 rounded border border-border px-3 text-xs hover:bg-muted" disabled={loading} onClick={() => void load(true, cursor)} type="button">{loading ? t("sessionTabs.state.loading") : t("sessionTabs.logs.loadMore")}</button> : null}</div>}
      </div>
    </div>
  );
}
