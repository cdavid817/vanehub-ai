import { Activity, ChevronDown, LoaderCircle, RefreshCw } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import type { AgentService } from "../../../services/agent-service";
import { agentService as defaultAgentService } from "../../../services/runtime-agent-client";
import type {
  ContextQualityAssessment,
  ContextQualityRangeDays,
  ContextQualitySummary,
} from "../../../types/context-quality";
import { contextQualityRangeDaysOptions } from "../../../types/context-quality";
import { contextQualityRetentionDaysOptions } from "../../../types/settings";
import { useSettings } from "../../settings-provider";
import { ContextInspector } from "./context-inspector";

function Distribution({ label, values }: { label: string; values: Record<string, number> }) {
  const entries = Object.entries(values);
  return <div className="rounded-lg border border-border/70 p-3">
    <h4 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{label}</h4>
    {entries.length ? <dl className="mt-2 space-y-1.5 text-sm">{entries.map(([name, count]) => (
      <div className="flex justify-between gap-3" key={name}><dt className="truncate">{name}</dt><dd className="font-mono tabular-nums">{count}</dd></div>
    ))}</dl> : <p className="mt-2 text-sm text-muted-foreground">—</p>}
  </div>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="rounded-lg border border-border/70 bg-background/70 p-3">
    <dt className="text-xs text-muted-foreground">{label}</dt>
    <dd className="mt-1 text-lg font-semibold tabular-nums">{value}</dd>
  </div>;
}

export function OnePieceContextHealthSection({ service = defaultAgentService }: { service?: AgentService }) {
  const { t, i18n } = useTranslation();
  const { saveSetting, savingKey, settings } = useSettings();
  const [rangeDays, setRangeDays] = useState<ContextQualityRangeDays>(30);
  const [summary, setSummary] = useState<ContextQualitySummary | null>(null);
  const [history, setHistory] = useState<ContextQualityAssessment[]>([]);
  const [nextCursor, setNextCursor] = useState<string | null>(null);
  const [summaryError, setSummaryError] = useState<string | null>(null);
  const [historyError, setHistoryError] = useState<string | null>(null);
  const [retentionError, setRetentionError] = useState<string | null>(null);
  const [summaryLoading, setSummaryLoading] = useState(true);
  const [historyLoading, setHistoryLoading] = useState(true);

  function loadSummary() {
    setSummaryLoading(true);
    setSummaryError(null);
    void service.getContextQualitySummary({ rangeDays })
      .then(setSummary)
      .catch((error: unknown) => setSummaryError(error instanceof Error ? error.message : String(error)))
      .finally(() => setSummaryLoading(false));
  }

  function loadHistory(cursor: string | null, append: boolean) {
    setHistoryLoading(true);
    setHistoryError(null);
    void service.listContextQualityHistory({ rangeDays, cursor, limit: 10 })
      .then((page) => {
        setHistory((current) => append ? [...current, ...page.items] : page.items);
        setNextCursor(page.nextCursor);
      })
      .catch((error: unknown) => setHistoryError(error instanceof Error ? error.message : String(error)))
      .finally(() => setHistoryLoading(false));
  }

  async function saveRetention(value: 7 | 30 | 90) {
    setRetentionError(null);
    try {
      await saveSetting("contextQualityRetentionDays", value);
      setHistory([]);
      setNextCursor(null);
      loadSummary();
      loadHistory(null, false);
    } catch (error) {
      setRetentionError(error instanceof Error ? error.message : String(error));
    }
  }

  useEffect(() => {
    setHistory([]);
    setNextCursor(null);
    loadSummary();
    loadHistory(null, false);
    // The selected range is the complete request identity for both independent resources.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rangeDays, service]);

  const number = (value: number) => new Intl.NumberFormat(i18n.language).format(value);
  const coverage = summary ? `${(summary.qualityCoverage.tokenCoverageBasisPoints / 100).toFixed(0)}%` : "—";

  return <section aria-labelledby="onepiece-context-health-heading" className="ucd-panel rounded-lg p-4">
    <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
      <div>
        <div className="flex items-center gap-2"><Activity aria-hidden="true" className="h-4 w-4 text-primary" /><h3 className="text-sm font-semibold" id="onepiece-context-health-heading">{t("onepiece.contextHealth.title")}</h3></div>
        <p className="mt-2 max-w-3xl text-sm leading-6 text-muted-foreground">{t("onepiece.contextHealth.description")}</p>
      </div>
      <div className="flex flex-wrap gap-2" role="group" aria-label={t("onepiece.contextHealth.rangeLabel")}>
        {contextQualityRangeDaysOptions.map((days) => <button aria-pressed={rangeDays === days} className={`min-h-11 rounded-md border px-3 text-xs font-medium focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring ${rangeDays === days ? "border-primary bg-primary text-primary-foreground" : "border-border bg-background text-muted-foreground"}`} key={days} onClick={() => setRangeDays(days)} type="button">{t("onepiece.contextHealth.days", { count: days })}</button>)}
      </div>
    </div>

    {summaryLoading ? <p className="mt-4 flex items-center gap-2 text-sm text-muted-foreground" role="status"><LoaderCircle className="h-4 w-4 animate-spin" />{t("onepiece.contextHealth.loadingSummary")}</p> : null}
    {summaryError ? <div className="mt-4 flex items-center justify-between gap-3 rounded-md border p-3 text-sm ucd-status-warning" role="alert"><span>{summaryError}</span><button aria-label={t("onepiece.contextHealth.retrySummary")} className="flex h-11 w-11 items-center justify-center rounded-md focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={loadSummary} type="button"><RefreshCw className="h-4 w-4" /></button></div> : null}
    {summary ? <>
      <dl className="mt-4 grid grid-cols-2 gap-2 lg:grid-cols-4">
        <Metric label={t("onepiece.contextHealth.evaluated")} value={number(summary.evaluated)} />
        <Metric label={t("onepiece.contextHealth.savedCharacters")} value={number(summary.savedCharacters)} />
        <Metric label={t("onepiece.contextHealth.savedTokens")} value={number(summary.savedTokens)} />
        <Metric label={t("onepiece.contextHealth.tokenCoverage")} value={coverage} />
      </dl>
      <div className="mt-3 grid gap-2 md:grid-cols-2 xl:grid-cols-4">
        <Distribution label={t("onepiece.contextHealth.outcomes")} values={summary.outcomes} />
        <Distribution label={t("onepiece.contextHealth.paths")} values={summary.paths} />
        <Distribution label={t("onepiece.contextHealth.qualities")} values={summary.qualities} />
        <Distribution label={t("onepiece.contextHealth.policyVersions")} values={summary.policyVersions} />
      </div>
    </> : null}

    <div className="mt-4 flex flex-col gap-3 border-t border-border/70 pt-4 sm:flex-row sm:items-center sm:justify-between">
      <div><h4 className="text-sm font-semibold">{t("onepiece.contextHealth.recent")}</h4><p className="mt-1 text-xs text-muted-foreground">{t("onepiece.contextHealth.disclosure")}</p></div>
      <label className="flex items-center gap-2 text-xs text-muted-foreground">{t("onepiece.contextHealth.retention")}<span className="relative"><select aria-label={t("onepiece.contextHealth.retention")} className="ucd-input h-11 appearance-none rounded-md pl-3 pr-8 text-sm" disabled={savingKey !== null} onChange={(event) => {
        const value = Number(event.target.value) as 7 | 30 | 90;
        void saveRetention(value);
      }} value={settings.contextQualityRetentionDays}>{contextQualityRetentionDaysOptions.map((days) => <option key={days} value={days}>{t("onepiece.contextHealth.days", { count: days })}</option>)}</select><ChevronDown className="pointer-events-none absolute right-2 top-2.5 h-4 w-4" /></span></label>
    </div>
    {retentionError ? <p className="mt-2 text-sm ucd-status-warning" role="alert">{retentionError}</p> : null}
    {historyError ? <div className="mt-3 flex items-center justify-between gap-3 rounded-md border p-3 text-sm ucd-status-warning" role="alert"><span>{historyError}</span><button className="min-h-11 rounded-md px-3 focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => loadHistory(null, false)} type="button">{t("onepiece.contextHealth.retry")}</button></div> : null}
    {!historyLoading && !historyError && history.length === 0 ? <p className="mt-3 rounded-lg border border-dashed p-6 text-center text-sm text-muted-foreground">{t("onepiece.contextHealth.empty")}</p> : null}
    {history.length ? <ul className="mt-3 divide-y divide-border/70 rounded-lg border border-border/70">{history.map((item) => <li className="grid gap-1 p-3 text-sm sm:grid-cols-[minmax(0,1fr)_auto_auto] sm:items-center sm:gap-4" key={item.attemptId}><div className="min-w-0"><p className="truncate font-medium">{item.outcome} · {item.path ?? item.reason ?? "—"}</p><p className="truncate text-xs text-muted-foreground">{new Intl.DateTimeFormat(i18n.language, { dateStyle: "medium", timeStyle: "short" }).format(new Date(item.recordedAt))}</p></div><span className="text-xs text-muted-foreground">{item.measurementQuality}</span><span className="font-mono text-xs tabular-nums">−{number(item.savedTokens ?? item.savedCharacters)} {item.savedTokens == null ? t("onepiece.contextHealth.characters") : t("onepiece.contextHealth.tokens")}</span></li>)}</ul> : null}
    {historyLoading ? <p className="mt-3 flex items-center gap-2 text-sm text-muted-foreground" role="status"><LoaderCircle className="h-4 w-4 animate-spin" />{t("onepiece.contextHealth.loadingHistory")}</p> : null}
    {nextCursor && !historyLoading ? <button className="mt-3 min-h-11 w-full rounded-md border border-border px-3 text-sm font-medium hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => loadHistory(nextCursor, true)} type="button">{t("onepiece.contextHealth.loadMore")}</button> : null}
    <ContextInspector service={service} />
  </section>;
}
