import { Check, Circle, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopInspectionTarget, LoopRun, LoopRunPhase } from "../types/loop";
import { LoopIterationDetails } from "./loop-iteration-details";
import { LoopAcceptancePanel } from "./loop-acceptance-panel";
import { LoopRunHeader } from "./loop-run-header";

const phases: LoopRunPhase[] = ["preparing", "acting", "verifying", "deciding", "finalizing"];

export function LoopTimeline({ onInspect, refreshing = false, run }: { onInspect?: (target: LoopInspectionTarget) => void; refreshing?: boolean; run: LoopRun }) {
  const { t } = useTranslation();
  const currentPhase = phases.indexOf(run.phase);
  return (
    <div className="grid w-full gap-5">
      <LoopRunHeader refreshing={refreshing} run={run} />
      <ol aria-label={t("loops.phases.title")} className="mx-auto grid w-full max-w-4xl grid-cols-5 gap-2">
        {phases.map((phase, index) => {
          const complete = index < currentPhase || ["succeeded", "failed", "cancelled", "awaiting-acceptance"].includes(run.status);
          const active = index === currentPhase && !["succeeded", "failed", "cancelled"].includes(run.status);
          const Icon = complete ? Check : active ? LoaderCircle : Circle;
          return <li className={cn("flex min-w-0 flex-col items-center gap-1 border-t-2 pt-2 text-center", complete || active ? "border-primary text-foreground" : "border-border text-muted-foreground")} key={phase}><Icon aria-hidden="true" className={cn("h-4 w-4", active && "animate-spin")} /><span className="w-full truncate text-[11px]">{t(`loops.phase.${phase}`)}</span></li>;
        })}
      </ol>
      <LoopAcceptancePanel onInspect={onInspect} run={run} />
      <section className="mx-auto w-full max-w-4xl">
        <h3 className="mb-2 text-xs font-semibold uppercase text-muted-foreground">{t("loops.iterations.title")}</h3>
        <div className="grid gap-2">
          {run.iterations.map((iteration, index) => <LoopIterationDetails iteration={iteration} key={iteration.id} onInspect={onInspect} open={index === run.iterations.length - 1} previousIteration={index > 0 ? run.iterations[index - 1] : null} />)}
          {run.iterations.length === 0 ? <p className="rounded-md border border-dashed border-border px-3 py-8 text-center text-xs text-muted-foreground">{t("loops.iterations.empty")}</p> : null}
        </div>
      </section>
    </div>
  );
}
