import { Check, Circle, LoaderCircle } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopRunPhase, LoopRunStatus } from "../types/loop";

/** Task 17.2's fixed run-lifecycle order -- not derived from data, every run walks these five in
 *  sequence. Exported so a caller that only has a `LoopRun` can still pass the canonical order
 *  explicitly rather than this module silently owning an undocumented default. */
export const loopRunPhaseOrder: LoopRunPhase[] = ["preparing", "acting", "verifying", "deciding", "finalizing"];

const pastPhaseStatuses: LoopRunStatus[] = ["succeeded", "failed", "cancelled", "awaiting-acceptance"];
const terminalStatuses: LoopRunStatus[] = ["succeeded", "failed", "cancelled"];

/**
 * Task 17.9: extracted from `loop-timeline.tsx` (previously 9 lines inlined there) into its own
 * independently reusable/testable component. Behavior and markup are unchanged from the inline
 * version -- this is a structural move, not a redesign.
 */
export function PhaseStepper({ phase, phases = loopRunPhaseOrder, status }: { phase: LoopRunPhase; phases?: LoopRunPhase[]; status: LoopRunStatus }) {
  const { t } = useTranslation();
  const currentPhase = phases.indexOf(phase);
  return (
    <ol aria-label={t("loops.phases.title")} className="mx-auto grid w-full max-w-4xl grid-cols-5 gap-2">
      {phases.map((value, index) => {
        const complete = index < currentPhase || pastPhaseStatuses.includes(status);
        const active = index === currentPhase && !terminalStatuses.includes(status);
        const Icon = complete ? Check : active ? LoaderCircle : Circle;
        return <li className={cn("flex min-w-0 flex-col items-center gap-1 border-t-2 pt-2 text-center", complete || active ? "border-primary text-foreground" : "border-border text-muted-foreground")} key={value}><Icon aria-hidden="true" className={cn("h-4 w-4", active && "animate-spin")} /><span className="w-full truncate text-[11px]">{t(`loops.phase.${value}`)}</span></li>;
      })}
    </ol>
  );
}
