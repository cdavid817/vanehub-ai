import { forwardRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import type { LoopInspectionTarget, LoopRun } from "../types/loop";
import { LoopInspectionActions } from "./loop-inspection-actions";
import { latestLoopOperationEvidence, useLoopElapsed } from "./loop-monitoring";
import { LoopRunControls } from "./loop-run-controls";

interface LoopInspectorProps {
  className?: string;
  id?: string;
  loading: boolean;
  onClose?: () => void;
  onInspect?: (target: LoopInspectionTarget) => void;
  run: LoopRun | null;
}

export const LoopInspector = forwardRef<HTMLElement, LoopInspectorProps>(function LoopInspector({ className, id, loading, onClose, onInspect, run }, ref) {
  const { i18n, t } = useTranslation();
  const elapsed = useLoopElapsed(run);
  const operationEvidence = run ? latestLoopOperationEvidence(run) : null;
  const latestIteration = run?.iterations.at(-1) ?? null;
  const latestDecision = latestIteration?.decisionReason ?? null;
  const inspectionSessionId = latestIteration?.workerSessionId ?? latestIteration?.verifierSessionId ?? null;
  return (
    <aside aria-label={t("loops.inspector.title")} className={cn("min-h-0 min-w-0 overflow-y-auto bg-[hsl(var(--panel-glass))] p-3", className)} id={id} ref={ref} tabIndex={-1}>
      <header className="mb-3 flex min-h-8 items-center justify-between gap-2">
        <h2 className="text-xs font-semibold uppercase text-muted-foreground">{t("loops.inspector.title")}</h2>
        {onClose ? (
          <button aria-label={t("loops.inspector.close")} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring min-[1024px]:hidden" onClick={onClose} title={t("loops.inspector.close")} type="button">
            <X aria-hidden="true" className="h-4 w-4" />
          </button>
        ) : null}
      </header>
      {loading ? <p className="text-xs text-muted-foreground">{t("loops.states.loading")}</p> : null}
      {!loading && !run ? <p className="text-xs text-muted-foreground">{t("loops.states.noSelection")}</p> : null}
      {run ? (
        <div className="grid gap-5">
          <InspectorSection title={t("loops.inspector.run")}>
            <Field label={t("loops.inspector.status")} value={t(`loops.status.${run.status}`)} />
            <Field label={t("loops.inspector.phase")} value={t(`loops.phase.${run.phase}`)} />
            <Field label={t("loops.monitor.elapsed")} value={elapsed} />
            <Field label={t("loops.inspector.iteration")} value={`${run.currentIteration} / ${run.definitionSnapshot.limits.maxIterations}`} />
            <Field label={t("loops.monitor.operation")} value={run.activeOperationId && ["queued", "running"].includes(run.status) ? t("loops.operation.active") : operationEvidence ? t(`loops.evidence.status.${operationEvidence.status}`) : t("loops.operation.none")} />
            {run.activeOperationId || operationEvidence?.operationId ? <Field label={t("loops.monitor.operationId")} value={run.activeOperationId ?? operationEvidence?.operationId ?? ""} /> : null}
            {run.activeOperationId || operationEvidence?.operationId ? <LoopInspectionActions onInspect={onInspect} sessionId={inspectionSessionId} surfaces={["logs"]} /> : null}
            {latestDecision ? <Field label={t("loops.iterations.decision")} value={latestDecision} /> : null}
            {run.terminalReason ? <Field label={t("loops.inspector.reason")} value={t(`loops.reason.${run.terminalReason}`)} /> : null}
          </InspectorSection>
          <LoopRunControls run={run} />
          <InspectorSection title={t("loops.inspector.limits")}>
            <Field label={t("loops.editor.field.stepTimeoutSeconds")} value={t("loops.inspector.seconds", { seconds: run.definitionSnapshot.limits.stepTimeoutSeconds })} />
            <Field label={t("loops.editor.field.totalTimeoutSeconds")} value={t("loops.inspector.seconds", { seconds: run.definitionSnapshot.limits.totalTimeoutSeconds })} />
            <Field label={t("loops.editor.field.maxConsecutiveRuntimeErrors")} value={`${run.consecutiveRuntimeErrors} / ${run.definitionSnapshot.limits.maxConsecutiveRuntimeErrors}`} />
            <Field label={t("loops.editor.field.maxConsecutiveNoProgress")} value={`${run.consecutiveNoProgress} / ${run.definitionSnapshot.limits.maxConsecutiveNoProgress}`} />
          </InspectorSection>
          <InspectorSection title={t("loops.inspector.workspace")}>
            <Field label={t("loops.inspector.project")} value={run.projectPath} />
            <Field label={t("loops.inspector.branch")} value={run.worktreeBranch ?? run.definitionSnapshot.baseBranch} />
            <Field label={t("loops.inspector.worktree")} value={run.worktreePath ?? t("loops.inspector.pending")} />
            {run.worktreePath ? <LoopInspectionActions onInspect={onInspect} sessionId={inspectionSessionId} surfaces={["changes", "files"]} /> : null}
            <Field label={t("loops.inspector.updated")} value={new Date(run.updatedAt).toLocaleString(i18n.resolvedLanguage)} />
          </InspectorSection>
        </div>
      ) : null}
    </aside>
  );
});

function InspectorSection({ children, title }: { children: ReactNode; title: string }) {
  return <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-muted-foreground">{title}</h3><dl className="grid gap-3">{children}</dl></section>;
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 border-b border-border/60 pb-3 last:border-0">
      <dt className="text-[11px] text-muted-foreground">{label}</dt>
      <dd className="mt-1 break-words text-xs text-foreground">{value}</dd>
    </div>
  );
}
