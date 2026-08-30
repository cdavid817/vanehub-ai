import { Activity, Play, RefreshCw, Workflow } from "lucide-react";
import { useState, type KeyboardEvent } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../../../components/ui/badge";
import { Button } from "../../../components/ui/button";
import { agentService } from "../../../services/runtime-agent-client";
import type { SkillEvolutionOrchestrationService } from "../../../services/skill-evolution-orchestration-service";
import { SkillEvolutionBreakerPanel } from "./skill-evolution-breaker-panel";
import { SkillEvolutionDecisionsPanel } from "./skill-evolution-decisions-panel";
import { SkillEvolutionPolicyPanel } from "./skill-evolution-policy-panel";
import { SkillEvolutionRunsPanel } from "./skill-evolution-runs-panel";
import { useSkillEvolutionOrchestration } from "./use-skill-evolution-orchestration";

type View = "runs" | "policy" | "decisions" | "safety";

export interface SkillEvolutionNavigation {
  applicationId?: string;
  breakerId?: string;
  probationId?: string;
  runId?: string;
  workspaceId?: string;
}

export function SkillEvolutionOrchestrationWorkspace({
  initial,
  onOpenCurator,
  service = agentService,
}: {
  initial?: SkillEvolutionNavigation;
  onOpenCurator: (workspaceId: string) => void;
  service?: SkillEvolutionOrchestrationService;
}) {
  const { t } = useTranslation();
  const [draft, setDraft] = useState(initial?.workspaceId ?? "");
  const [workspaceId, setWorkspaceId] = useState(initial?.workspaceId ?? "");
  const [view, setView] = useState<View>(initialView(initial));
  const state = useSkillEvolutionOrchestration(workspaceId, service);
  const loading = [state.overview, state.policy, state.runs, state.eligibility, state.applications, state.probations, state.breakers].some((query) => query.isLoading);
  const error = [state.overview, state.policy, state.runs, state.eligibility, state.applications, state.probations, state.breakers].find((query) => query.isError)?.error;
  const overview = state.overview.data;
  return <section aria-labelledby="skill-evolution-title" className="space-y-4">
      <header className="overflow-hidden rounded-xl border border-border bg-gradient-to-br from-cyan-500/15 via-background to-violet-500/10 p-4 shadow-sm sm:p-5"><div className="flex flex-wrap items-start justify-between gap-3"><div><div className="flex items-center gap-2"><Workflow className="h-5 w-5 text-cyan-500" /><h2 className="text-lg font-semibold" id="skill-evolution-title">{t("skills.evolution.orchestrationTitle")}</h2></div><p className="mt-1 max-w-3xl text-xs leading-5 text-muted-foreground">{t("skills.evolution.orchestrationDescription")}</p></div>{overview ? <Badge tone={overview.mockProvenance ? "warning" : "success"}>{t(overview.mockProvenance ? "skills.evolution.capability.web" : "skills.evolution.capability.desktop")}</Badge> : null}</div>
      <form className="mt-4 flex flex-col gap-2 sm:flex-row" onSubmit={(event) => { event.preventDefault(); setWorkspaceId(draft.trim()); }}><label className="min-w-0 flex-1 text-xs text-muted-foreground"><span>{t("skills.evolution.workspace")}</span><input className="mt-1 h-9 w-full rounded-md border border-border bg-background px-3 text-sm text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring" onChange={(event) => setDraft(event.target.value)} placeholder={t("skills.evolution.workspacePlaceholder")} value={draft} /></label><Button className="sm:mt-5" type="submit">{t("skills.evolution.openWorkspace")}</Button></form>
    </header>
    {!workspaceId ? <Empty text={t("skills.evolution.workspaceRequired")} /> : <>
      {loading ? <p className="rounded-xl border border-border p-4 text-sm text-muted-foreground" role="status">{t("skills.evolution.orchestrationLoading")}</p> : null}
      {error ? <div className="rounded-xl border border-destructive/40 bg-destructive/10 p-4 text-sm text-destructive" role="alert"><p>{error.message}</p><Button className="mt-2" onClick={() => void state.refresh()} size="sm" variant="outline"><RefreshCw />{t("featureLoad.retry")}</Button></div> : null}
      {overview ? <SchedulerOverview onRun={() => state.requestRun.mutate()} overview={overview} requesting={state.requestRun.isPending} requestError={state.requestRun.error?.message ?? null} /> : null}
      <EvolutionViewTabs onChange={setView} view={view} />
      <div aria-labelledby={`evolution-tab-${view}`} id={`evolution-panel-${view}`} role="tabpanel">{view === "runs" && state.runs.data ? <SkillEvolutionRunsPanel cancelling={state.cancelRun.isPending} initialRunId={initial?.runId} onCancel={(run) => state.cancelRun.mutate(run)} runs={state.runs.data.items} service={service} /> : view === "policy" && state.policy.data ? <SkillEvolutionPolicyPanel error={state.updatePolicy.error?.message ?? null} onSave={(input) => state.updatePolicy.mutate(input)} policy={state.policy.data} saving={state.updatePolicy.isPending} /> : view === "decisions" && state.eligibility.data && state.applications.data && state.probations.data ? <SkillEvolutionDecisionsPanel applications={state.applications.data.items} eligibility={state.eligibility.data.items} initialApplicationId={initial?.applicationId} initialProbationId={initial?.probationId} onOpenCurator={() => onOpenCurator(workspaceId)} probations={state.probations.data.items} /> : view === "safety" && state.breakers.data ? <SkillEvolutionBreakerPanel acknowledging={state.acknowledgeBreaker.isPending} breakers={state.breakers.data.items} initialBreakerId={initial?.breakerId} onAcknowledge={(breaker) => state.acknowledgeBreaker.mutate({ breakerId: breaker.breakerId, revision: breaker.revision })} onOpenCurator={() => onOpenCurator(workspaceId)} /> : null}</div>
    </>}
  </section>;
}

function SchedulerOverview({ onRun, overview, requestError, requesting }: { onRun: () => void; overview: Awaited<ReturnType<SkillEvolutionOrchestrationService["getEvolutionSchedulerOverview"]>>; requestError: string | null; requesting: boolean }) {
  const { t } = useTranslation();
  const counters = Object.entries(overview.triggerCounters).filter(([, count]) => count > 0);
  return <section aria-labelledby="evolution-scheduler-title" className="rounded-xl border border-border bg-background p-4 shadow-sm"><div className="flex flex-wrap items-start justify-between gap-3"><div><div className="flex items-center gap-2"><Activity className="h-4 w-4 text-primary" /><h3 className="font-semibold" id="evolution-scheduler-title">{t("skills.evolution.scheduler.title")}</h3></div><p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.scheduler.description")}</p></div><Button disabled={requesting} onClick={onRun} size="sm"><Play />{requesting ? t("skills.evolution.scheduler.requesting") : t("skills.evolution.scheduler.run")}</Button></div>
    <dl className="mt-4 grid grid-cols-2 gap-2 sm:grid-cols-4"><Metric label={t("skills.evolution.scheduler.mode")} value={t(`skills.evolution.mode.${overview.mode}`)} /><Metric label={t("skills.evolution.scheduler.idle")} value={t(`skills.evolution.idle.${overview.idleGate}`)} /><Metric label={t("skills.evolution.scheduler.pending")} value={String(overview.pendingTriggers)} /><Metric label={t("skills.evolution.scheduler.active")} value={overview.activeRunId ?? t("skills.evolution.notAvailable")} /></dl>
    {!overview.automaticMutationAvailable ? <p className="mt-3 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-xs text-muted-foreground">{t("skills.evolution.scheduler.mutationUnavailable")}</p> : null}
    {overview.idle.safeReasons.length ? <p className="mt-2 text-xs text-muted-foreground">{t("skills.evolution.scheduler.idleReasons", { reasons: overview.idle.safeReasons.join(", ") })}</p> : null}
    <div className="mt-3"><p className="text-xs font-medium">{t("skills.evolution.scheduler.triggers")}</p>{counters.length ? <ul className="mt-2 flex flex-wrap gap-2">{counters.map(([name, count]) => <li className="rounded-md border border-border bg-muted/30 px-2 py-1 text-xs" key={name}>{t(`skills.evolution.trigger.${name}`)} <span className="font-semibold tabular-nums">{count}</span></li>)}</ul> : <p className="mt-1 text-xs text-muted-foreground">{t("skills.evolution.scheduler.noTriggers")}</p>}</div>
    <p className="mt-3 text-xs text-muted-foreground">{t("skills.evolution.scheduler.manualGates")}</p>{requestError ? <p className="mt-2 text-sm text-destructive" role="alert">{requestError}</p> : null}
  </section>;
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 rounded-lg border border-border bg-muted/20 p-3"><dt className="text-[11px] text-muted-foreground">{label}</dt><dd className="mt-1 truncate text-sm font-semibold" title={value}>{value}</dd></div>;
}

function EvolutionViewTabs({ onChange, view }: { onChange: (view: View) => void; view: View }) {
  const { t } = useTranslation();
  const views: View[] = ["runs", "policy", "decisions", "safety"];
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault();
    const current = views.indexOf(view);
    const next = event.key === "Home" ? 0 : event.key === "End" ? views.length - 1
      : (current + (event.key === "ArrowRight" ? 1 : -1) + views.length) % views.length;
    onChange(views[next]);
    document.getElementById(`evolution-tab-${views[next]}`)?.focus();
  };
  return <div aria-label={t("skills.evolution.viewsLabel")} className="flex gap-1 overflow-x-auto rounded-lg border border-border bg-muted/30 p-1" onKeyDown={onKeyDown} role="tablist">{views.map((item) => <button aria-controls={`evolution-panel-${item}`} aria-selected={view === item} className={`shrink-0 rounded-md px-3 py-1.5 text-xs font-medium focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring ${view === item ? "bg-background text-foreground shadow-sm" : "text-muted-foreground hover:text-foreground"}`} id={`evolution-tab-${item}`} key={item} onClick={() => onChange(item)} role="tab" tabIndex={view === item ? 0 : -1} type="button">{t(`skills.evolution.view.${item}`)}</button>)}</div>;
}

function initialView(initial?: SkillEvolutionNavigation): View {
  if (initial?.breakerId) return "safety";
  if (initial?.applicationId || initial?.probationId) return "decisions";
  return "runs";
}

function Empty({ text }: { text: string }) {
  return <div className="rounded-xl border border-dashed border-border bg-muted/10 p-8 text-center text-sm text-muted-foreground">{text}</div>;
}
