import { useTranslation } from "react-i18next";
import type { LoopInspectionTarget, LoopRun } from "../types/loop";
import { LoopIterationRow } from "./loop-iteration-row";
import { LoopAcceptancePanel } from "./loop-acceptance-panel";
import { LoopRunHeader } from "./loop-run-header";
import { PhaseStepper } from "./phase-stepper";

export function LoopTimeline({ onInspect, refreshing = false, run }: { onInspect?: (target: LoopInspectionTarget) => void; refreshing?: boolean; run: LoopRun }) {
  const { t } = useTranslation();
  return (
    <div className="grid w-full gap-5">
      <LoopRunHeader refreshing={refreshing} run={run} />
      <PhaseStepper phase={run.phase} status={run.status} />
      <LoopAcceptancePanel onInspect={onInspect} run={run} />
      <section className="mx-auto w-full max-w-4xl">
        <h3 className="mb-2 text-xs font-semibold uppercase text-muted-foreground">{t("loops.iterations.title")}</h3>
        <ol className="grid gap-2">
          {run.iterations.map((iteration, index) => <LoopIterationRow iteration={iteration} key={iteration.id} onInspect={onInspect} open={index === run.iterations.length - 1} previousIteration={index > 0 ? run.iterations[index - 1] : null} />)}
          {run.iterations.length === 0 ? <p className="rounded-md border border-dashed border-border px-3 py-8 text-center text-xs text-muted-foreground">{t("loops.iterations.empty")}</p> : null}
        </ol>
      </section>
    </div>
  );
}
