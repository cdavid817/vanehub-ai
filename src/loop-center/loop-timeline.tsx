import { Check, Circle, LoaderCircle, RefreshCw } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopInspectionTarget, LoopRun, LoopRunPhase } from "../types/loop";
import { LoopIterationDetails } from "./loop-iteration-details";
import { latestLoopOperationEvidence, useLoopElapsed } from "./loop-monitoring";

const phases: LoopRunPhase[] = ["preparing", "acting", "verifying", "deciding", "finalizing"];

export function LoopTimeline({ onInspect, refreshing = false, run }: { onInspect?: (target: LoopInspectionTarget) => void; refreshing?: boolean; run: LoopRun }) {
  const { t } = useTranslation();
  const elapsed = useLoopElapsed(run);
  const currentPhase = phases.indexOf(run.phase);
  const operationEvidence = latestLoopOperationEvidence(run);
  const operationId = run.activeOperationId ?? operationEvidence?.operationId ?? null;
  const operationState = run.activeOperationId && ["queued", "running"].includes(run.status)
    ? t("loops.operation.active")
    : operationEvidence ? t(`loops.evidence.status.${operationEvidence.status}`) : t("loops.operation.pending");
  return (
    <div className="mx-auto flex w-full max-w-4xl flex-col gap-5">
      <header className="flex min-w-0 items-start justify-between gap-4">
        <div className="min-w-0"><p className="truncate text-xs text-muted-foreground">{run.definitionSnapshot.name}</p><h2 className="mt-1 text-lg font-semibold">{run.definitionSnapshot.goal}</h2></div>
        <div className="flex shrink-0 items-center gap-2">
          {refreshing ? <span className="inline-flex items-center gap-1 text-[11px] text-muted-foreground"><RefreshCw aria-hidden="true" className="h-3 w-3 animate-spin" />{t("loops.states.refreshing")}</span> : null}
          {run.simulated ? <span className="rounded-md border border-warning/40 px-2 py-1 text-[11px] font-medium text-warning">{t("loops.simulated")}</span> : null}
        </div>
      </header>
      <dl className="grid grid-cols-2 border-y border-border/70 sm:grid-cols-3 lg:grid-cols-6">
        <Metric label={t("loops.inspector.status")} value={t(`loops.status.${run.status}`)} />
        <Metric label={t("loops.inspector.phase")} value={t(`loops.phase.${run.phase}`)} />
        <Metric label={t("loops.monitor.iteration")} value={`${run.currentIteration} / ${run.definitionSnapshot.limits.maxIterations}`} />
        <Metric label={t("loops.monitor.elapsed")} value={elapsed} />
        <Metric label={t("loops.monitor.operation")} value={operationState} />
        <Metric label={t("loops.monitor.operationId")} value={operationId ?? t("loops.operation.none")} />
      </dl>
      <ol aria-label={t("loops.phases.title")} className="grid grid-cols-5 gap-2">
        {phases.map((phase, index) => {
          const complete = index < currentPhase || ["succeeded", "failed", "cancelled", "awaiting-acceptance"].includes(run.status);
          const active = index === currentPhase && !["succeeded", "failed", "cancelled"].includes(run.status);
          const Icon = complete ? Check : active ? LoaderCircle : Circle;
          return <li className={cn("flex min-w-0 flex-col items-center gap-1 border-t-2 pt-2 text-center", complete || active ? "border-primary text-foreground" : "border-border text-muted-foreground")} key={phase}><Icon aria-hidden="true" className={cn("h-4 w-4", active && "animate-spin")} /><span className="w-full truncate text-[11px]">{t(`loops.phase.${phase}`)}</span></li>;
        })}
      </ol>
      <section>
        <h3 className="mb-2 text-xs font-semibold uppercase text-muted-foreground">{t("loops.iterations.title")}</h3>
        <div className="grid gap-2">
          {run.iterations.map((iteration, index) => <LoopIterationDetails iteration={iteration} key={iteration.id} onInspect={onInspect} open={index === run.iterations.length - 1} />)}
          {run.iterations.length === 0 ? <p className="rounded-md border border-dashed border-border px-3 py-8 text-center text-xs text-muted-foreground">{t("loops.iterations.empty")}</p> : null}
        </div>
      </section>
    </div>
  );
}

function Metric({ label, value }: { label: string; value: string }) {
  return <div className="min-w-0 border-b border-r border-border/60 px-2 py-2 last:border-r-0 sm:border-b-0"><dt className="truncate text-[10px] uppercase text-muted-foreground">{label}</dt><dd className="mt-1 truncate text-xs font-medium" title={value}>{value}</dd></div>;
}
