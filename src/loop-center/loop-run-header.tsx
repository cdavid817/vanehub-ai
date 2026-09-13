import { RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import type { LoopRun } from "../types/loop";
import { formatLoopDuration, useLoopElapsed } from "./loop-monitoring";
import { selectCurrentLoopActivity, selectLoopBudget } from "./loop-presentation";
import { LoopRunControls } from "./loop-run-controls";
import { LoopScopeAssessment } from "./loop-scope-assessment";

const scopeReasons = new Set(["scope-violation", "scope-binding-missing", "scope-unverifiable", "scope-capability-changed"]);

export function LoopRunHeader({ refreshing, run }: { refreshing: boolean; run: LoopRun }) {
  const { t } = useTranslation();
  const elapsed = useLoopElapsed(run);
  const budget = selectLoopBudget(run, Date.now());
  const activity = selectCurrentLoopActivity(run) ?? t(`loops.phase.${run.phase}`);
  const scope = run.scope;
  const coverage = scope?.assessment ? (scope.assessment.satisfiesRequestedMode ? "satisfied" : "blocked") : "unknown";
  return (
    <header className="sticky -top-3 z-20 -mx-3 grid gap-3 border-b border-border bg-background/95 px-3 pb-3 pt-1 backdrop-blur-sm motion-reduce:transition-none sm:-top-4 sm:-mx-4 sm:px-4">
      <div className="flex min-w-0 items-start justify-between gap-3">
        <div className="min-w-0"><p className="truncate text-xs text-muted-foreground">{run.definitionSnapshot.name}</p><h2 className="mt-0.5 truncate text-base font-semibold">{run.definitionSnapshot.goal}</h2></div>
        <div className="flex shrink-0 items-center gap-2">
          {refreshing ? <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground"><RefreshCw aria-hidden="true" className="h-3 w-3 animate-spin" />{t("loops.states.refreshing")}</span> : null}
          {run.simulated ? <span className="rounded border border-warning/50 px-2 py-1 text-[11px] font-medium text-warning">{t("loops.simulated")}</span> : null}
        </div>
      </div>
      <dl className="grid grid-cols-2 border-y border-border/70 sm:grid-cols-3 lg:grid-cols-6">
        <Metric label={t("loops.inspector.status")} value={t(`loops.status.${run.status}`)} />
        <Metric label={t("loops.inspector.phase")} value={t(`loops.phase.${run.phase}`)} />
        <Metric label={t("loops.monitor.iteration")} value={`${run.currentIteration} / ${run.definitionSnapshot.limits.maxIterations}`} />
        <Metric label={t("loops.monitor.elapsed")} value={elapsed} />
        <Metric label={t("loops.monitor.remaining")} value={formatLoopDuration(budget.remainingMs)} />
        <Metric label={t("loops.monitor.activity")} value={activity} />
      </dl>
      <ul aria-label={t("loops.scope.title")} className="flex flex-wrap gap-2 text-[11px]">
        <Badge tone={scope?.requestedMode ? "neutral" : "warning"}>{t("loops.scope.mode")}: {scope?.requestedMode ? t(`loops.mode.${scope.requestedMode}`) : t("loops.mode.legacy")}</Badge>
        <Badge tone={coverage === "satisfied" ? "success" : coverage === "blocked" ? "destructive" : "warning"}>{t("loops.scope.coverage")}: {t(`loops.scope.coverageState.${coverage}`)}</Badge>
        <Badge tone={scope?.bindingStatus === "bound" ? "success" : "destructive"}>{t("loops.scope.binding")}: {t(`loops.scope.bindingStatus.${scope?.bindingStatus ?? "missing"}`)}</Badge>
        {scope?.sealedEvidenceId ? <Badge tone="success">{t("loops.scope.sealed")}</Badge> : null}
        {scope?.acceptanceOperationId ? <Badge tone="warning">{t("loops.scope.acceptancePending")}</Badge> : null}
        {scope?.scopeDigest ? <li className="max-w-full truncate rounded border border-border px-2 py-1 text-muted-foreground" title={scope.scopeDigest}>{t("loops.scope.digest")}: {scope.scopeDigest}</li> : null}
      </ul>
      {run.terminalReason ? <p className="text-xs text-warning">{t(`loops.reason.${run.terminalReason}`)}{run.terminalReason === "recovery-required" ? ` · ${t("loops.monitor.recoveryGuidance")}` : ""}{scopeReasons.has(run.terminalReason) ? ` · ${t(`loops.scope.reason.${run.terminalReason}`)}` : ""}</p> : null}
      {scope?.assessment && (coverage === "blocked" || scope.assessment.limitations.length > 0) ? <details className="text-xs"><summary className="cursor-pointer text-muted-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring">{t("loops.scope.showAssessment")}</summary><div className="mt-2"><LoopScopeAssessment assessment={scope.assessment} requestedMode={scope.requestedMode} /></div></details> : null}
      {run.status !== "awaiting-acceptance" ? <LoopRunControls run={run} /> : null}
    </header>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 border-b border-r border-border/60 px-2 py-2 last:border-r-0 sm:border-b-0"><dt className="truncate text-[10px] uppercase text-muted-foreground">{label}</dt><dd className="mt-1 truncate text-xs font-medium" title={value}>{value}</dd></div>;
}

function Badge({ children, tone }: { children: React.ReactNode; tone: "neutral" | "success" | "warning" | "destructive" }) {
  const toneClass = tone === "success" ? "border-success/50 text-success" : tone === "warning" ? "border-warning/50 text-warning" : tone === "destructive" ? "border-destructive/50 text-destructive" : "border-border text-muted-foreground";
  return <li className={`rounded border px-2 py-1 font-medium ${toneClass}`}>{children}</li>;
}
