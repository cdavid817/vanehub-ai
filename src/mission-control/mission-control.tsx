import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { agentService } from "../services/runtime-agent-client";
import type { MissionControlAction, MissionControlFacet, MissionControlNavigationTarget, MissionControlOverview, MissionControlRunDetail, MissionControlRunSummary, MissionControlSort } from "../types/mission-control";

const states = ["", "running", "waiting_approval", "waiting_user", "retrying", "stuck", "failed", "completed"] as const;
const facets = ["overview", "plan", "timeline", "tools", "files", "review", "verification", "context", "usage", "logs"] as const;

export function MissionControl({ onNavigate }: { onNavigate?: (target: MissionControlNavigationTarget) => void }) {
  const { t } = useTranslation();
  const [overview, setOverview] = useState<MissionControlOverview | null>(null);
  const [selected, setSelected] = useState<MissionControlRunDetail | null>(null);
  const [status, setStatus] = useState("");
  const [agentId, setAgentId] = useState("");
  const [projectId, setProjectId] = useState("");
  const [runner, setRunner] = useState<"" | "local" | "ssh">("");
  const [sort, setSort] = useState<MissionControlSort>("attention");
  const [cursor, setCursor] = useState<string | null>(null);
  const [activeFacet, setActiveFacet] = useState<MissionControlFacet>("overview");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const load = useCallback(async () => {
    setLoading(true);
    try {
      setOverview(await agentService.getMissionControlOverview({ agentId: agentId || undefined, cursor, limit: 20, projectId: projectId || undefined, runner: runner || undefined, sort, states: status ? [status as MissionControlRunSummary["state"]] : undefined }));
      setError(null);
    } catch { setError(t("missionControl.loadError")); } finally { setLoading(false); }
  }, [agentId, cursor, projectId, runner, sort, status, t]);

  useEffect(() => { void load(); }, [load]);
  useEffect(() => {
    const reconcile = () => { if (document.visibilityState === "visible") void load(); };
    const polling = window.setInterval(reconcile, 2_000);
    window.addEventListener("focus", reconcile); document.addEventListener("visibilitychange", reconcile);
    return () => { window.clearInterval(polling); window.removeEventListener("focus", reconcile); document.removeEventListener("visibilitychange", reconcile); };
  }, [load]);

  async function inspect(run: MissionControlRunSummary) {
    try { setSelected(await agentService.getMissionControlRun(run.runId)); setActiveFacet("overview"); } catch { setError(t("missionControl.loadError")); }
  }
  async function act(run: MissionControlRunSummary, action: MissionControlAction) {
    if (action === "open" || action === "approval" || action === "review") {
      if (run.navigation) onNavigate?.(run.navigation);
      return;
    }
    try {
      const receipt = await agentService.performMissionControlAction({ runId: run.runId, version: run.version, action });
      setSelected((current) => current ? { ...current, run: receipt.run } : current); await load();
    } catch { setError(t("missionControl.actionError")); }
  }

  const counts = overview?.counts;
  return <div className="ucd-panel flex min-h-0 flex-1 flex-col overflow-hidden rounded-lg" data-testid="mission-control">
    <header className="flex flex-wrap items-center gap-2 border-b border-border p-3">
      <div className="min-w-48 flex-1"><h1 className="text-sm font-semibold">{t("missionControl.title")}</h1><p className="text-xs text-muted-foreground">{t("missionControl.description")}</p></div>
      <input aria-label={t("missionControl.filterAgent")} className="h-8 w-32 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => { setAgentId(event.target.value); setCursor(null); }} placeholder={t("missionControl.filterAgent")} value={agentId} />
      <input aria-label={t("missionControl.filterProject")} className="h-8 w-32 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => { setProjectId(event.target.value); setCursor(null); }} placeholder={t("missionControl.filterProject")} value={projectId} />
      <select aria-label={t("missionControl.filterStatus")} className="h-8 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => { setStatus(event.target.value); setCursor(null); }} value={status}>{states.map((state) => <option key={state || "all"} value={state}>{state ? t(`missionControl.state.${state}`) : t("missionControl.allStatuses")}</option>)}</select>
      <select aria-label={t("missionControl.filterRunner")} className="h-8 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => { setRunner(event.target.value as "" | "local" | "ssh"); setCursor(null); }} value={runner}><option value="">{t("missionControl.allRunners")}</option><option value="local">{t("runner.kind.local")}</option><option value="ssh">{t("runner.kind.ssh")}</option></select>
      <select aria-label={t("missionControl.sort")} className="h-8 rounded-md border border-input bg-background px-2 text-xs" onChange={(event) => { setSort(event.target.value as MissionControlSort); setCursor(null); }} value={sort}><option value="attention">{t("missionControl.sortAttention")}</option><option value="newest">{t("missionControl.sortNewest")}</option><option value="oldest">{t("missionControl.sortOldest")}</option></select>
      <button aria-label={t("missionControl.refresh")} className="ucd-interactive grid h-8 w-8 place-items-center rounded-md border border-input" onClick={() => void load()} title={t("missionControl.refresh")} type="button"><RefreshCw aria-hidden="true" className={`h-4 w-4 ${loading ? "animate-spin" : ""}`} /></button>
    </header>
    {error ? <p aria-live="polite" className="m-3 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs text-destructive">{error}</p> : null}
    <div className="flex gap-2 overflow-x-auto border-b border-border p-2">{counts ? Object.entries(counts).map(([key, count]) => <div className="min-w-28 rounded-md border border-border bg-muted/30 px-3 py-2" key={key}><p className="text-[11px] text-muted-foreground">{t(`missionControl.count.${key}`)}</p><p className="text-lg font-semibold tabular-nums">{count}</p></div>) : null}</div>
    <div className="grid min-h-0 flex-1 grid-cols-1 overflow-hidden min-[900px]:grid-cols-[minmax(0,1.4fr)_minmax(280px,1fr)]">
      <div className="min-h-0 overflow-y-auto p-3">
        <RunSection title={t("missionControl.attention")} runs={overview?.attention.items ?? []} onAct={act} onInspect={inspect} urgent />
        <RunSection title={t("missionControl.active")} runs={overview?.active.items ?? []} onAct={act} onInspect={inspect} />
        <RunSection title={t("missionControl.recent")} runs={overview?.recent.items ?? []} onAct={act} onInspect={inspect} />
        {overview && [overview.attention.nextCursor, overview.active.nextCursor, overview.recent.nextCursor].some(Boolean) ? <button className="rounded-md border border-input px-3 py-1.5 text-xs" onClick={() => setCursor(overview.attention.nextCursor ?? overview.active.nextCursor ?? overview.recent.nextCursor)} type="button">{t("missionControl.nextPage")}</button> : null}
        {!loading && overview && overview.attention.items.length + overview.active.items.length + overview.recent.items.length === 0 ? <p className="p-8 text-center text-sm text-muted-foreground">{t("missionControl.empty")}</p> : null}
      </div>
      <aside className="min-h-0 overflow-y-auto border-t border-border p-3 min-[900px]:border-l min-[900px]:border-t-0">
        <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("missionControl.detail")}</h2>
        {selected ? <><RunCard run={selected.run} onAct={act} onInspect={inspect} /><div className="mt-3 flex gap-1 overflow-x-auto" role="tablist">{facets.map((facet) => { const availability = selected.facets.find((item) => item.facet === facet)?.state ?? "unavailable"; return <button aria-disabled={availability !== "available"} aria-selected={activeFacet === facet} className="shrink-0 rounded-md border border-input px-2 py-1 text-xs disabled:opacity-50" disabled={availability !== "available"} key={facet} onClick={() => setActiveFacet(facet)} role="tab" type="button">{t(`missionControl.facet.${facet}`)}{availability === "available" ? null : ` · ${t(`missionControl.availability.${availability}`)}`}</button>; })}</div><p className="mt-4 text-xs text-muted-foreground">{t("missionControl.facetSelected", { facet: t(`missionControl.facet.${activeFacet}`) })} · {t("missionControl.lazyDetail")}</p></> : <p className="text-sm text-muted-foreground">{t("missionControl.selectRun")}</p>}
      </aside>
    </div>
  </div>;
}

function RunSection({ onAct, onInspect, runs, title, urgent = false }: { onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void; onInspect: (run: MissionControlRunSummary) => void; runs: MissionControlRunSummary[]; title: string; urgent?: boolean }) {
  if (!runs.length) return null;
  return <section className="mb-4"><h2 className="mb-2 flex items-center gap-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{urgent ? <AlertTriangle aria-hidden="true" className="h-3.5 w-3.5 text-warning" /> : null}{title}</h2><div className="grid gap-2">{runs.map((run) => <RunCard key={run.runId} onAct={onAct} onInspect={onInspect} run={run} />)}</div></section>;
}

function RunCard({ onAct, onInspect, run }: { onAct: (run: MissionControlRunSummary, action: MissionControlAction) => void; onInspect: (run: MissionControlRunSummary) => void; run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const ended = run.endedAt ?? run.updatedAt;
  const elapsed = Math.max(0, Date.parse(ended) - Date.parse(run.createdAt));
  return <article className="rounded-md border border-border bg-card p-3" data-testid={`mission-run-${run.runId}`}><button className="w-full text-left focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" onClick={() => void onInspect(run)} type="button"><div className="flex flex-wrap items-center gap-2"><span className="min-w-0 flex-1 truncate text-sm font-medium">{run.title}</span>{run.runner ? <span className="inline-flex max-w-44 items-center gap-1 rounded border border-primary/30 bg-primary/5 px-1.5 py-0.5 text-[11px] text-primary" data-runner={run.runner.kind}><span>{t(`runner.kind.${run.runner.kind}`)}</span>{run.runner.hostLabel ? <span className="truncate text-muted-foreground">· {run.runner.hostLabel}</span> : null}</span> : null}<span className="rounded border border-border px-1.5 py-0.5 text-[11px]">{t(`missionControl.state.${run.state}`)}</span></div><p className="mt-1 text-xs text-muted-foreground">{run.agentId ?? run.ownerType} · {t("missionControl.elapsed", { seconds: Math.round(elapsed / 1000) })} · {t(`missionControl.verification.${run.verification}`)}</p>{run.reasonCode ? <p className="mt-1 text-xs text-warning">{t(`runner.reason.${run.reasonCode}`, { defaultValue: run.reasonCode })}</p> : null}</button><div className="mt-2 flex flex-wrap gap-1">{run.actions.map((action) => <button className="rounded-md border border-input px-2 py-1 text-xs hover:bg-muted focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring" data-action={action} key={action} onClick={() => void onAct(run, action)} type="button">{t(`missionControl.action.${action}`)}</button>)}</div></article>;
}
