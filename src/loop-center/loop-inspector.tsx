import { forwardRef, type ReactNode } from "react";
import { X } from "lucide-react";
import { useTranslation } from "react-i18next";
import { cn } from "../lib/utils";
import { AgentRunOwnerStatus } from "../components/ui/agent-run-owner-status";
import type { LoopInspectionTarget, LoopRun } from "../types/loop";
import { LoopInspectionActions } from "./loop-inspection-actions";
import { latestLoopOperationEvidence } from "./loop-monitoring";

interface LoopInspectorProps {
  className?: string;
  id?: string;
  loading: boolean;
  onClose?: () => void;
  onInspect?: (target: LoopInspectionTarget) => void;
  run: LoopRun | null;
}

export const LoopInspector = forwardRef<HTMLElement, LoopInspectorProps>(function LoopInspector({ className, id, loading, onClose, onInspect, run }, ref) {
  const { t } = useTranslation();
  return (
    <aside aria-label={t("loops.inspector.title")} className={cn("ucd-panel min-h-0 min-w-0 overflow-y-auto rounded-lg p-3", className)} id={id} ref={ref} tabIndex={-1}>
      <header className="mb-3 flex min-h-8 items-center justify-between gap-2">
        <h2 className="text-xs font-semibold uppercase text-muted-foreground">{t("loops.inspector.title")}</h2>
        {onClose ? (
          <button aria-label={t("loops.inspector.close")} className="grid h-8 w-8 shrink-0 place-items-center rounded-md text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:outline-hidden focus-visible:ring-2 focus-visible:ring-ring min-[1024px]:hidden" onClick={onClose} title={t("loops.inspector.close")} type="button">
            <X aria-hidden="true" className="h-4 w-4" />
          </button>
        ) : null}
      </header>
      {loading ? <p className="text-xs text-muted-foreground">{t("loops.states.loading")}</p> : null}
      {!loading && !run ? (
        <div className="grid gap-1 py-6 text-center">
          <p className="text-xs font-medium">{t("loops.states.noSelection")}</p>
          <p className="text-[11px] leading-5 text-muted-foreground">{t("loops.states.noSelectionDescription")}</p>
        </div>
      ) : null}
      {run ? <LoopInspectorBody onInspect={onInspect} run={run} /> : null}
    </aside>
  );
});

/**
 * The run/limits/workspace content only, extracted from `LoopInspector` above (17.3) so the
 * registered `loop-iteration` Inspector provider (loop-iteration-inspector-provider.tsx) can
 * reuse the exact same data-derivation and markup inside the shared `Inspector` shell, instead of
 * this file's own `<aside>`/header/close button -- `Inspector` already renders an equivalent
 * header of its own. `LoopInspector` above still renders this unchanged, so its own byte-level
 * output (asserted directly by loop-center-states.test.tsx) is unaffected by this split.
 */
export function LoopInspectorBody({ onInspect, run }: { onInspect?: (target: LoopInspectionTarget) => void; run: LoopRun }) {
  const { i18n, t } = useTranslation();
  const operationEvidence = latestLoopOperationEvidence(run);
  const latestIteration = run.iterations.at(-1) ?? null;
  const inspectionSessionId = latestIteration?.workerSessionId ?? latestIteration?.verifierSessionId ?? null;
  return (
    <div className="grid gap-5">
      <InspectorSection title={t("loops.inspector.run")}>
        <AgentRunOwnerStatus ownerId={run.id} ownerType="loop_run" />
        <Field label={t("loops.monitor.operation")} value={run.activeOperationId && ["queued", "running"].includes(run.status) ? t("loops.operation.active") : operationEvidence ? t(`loops.evidence.status.${operationEvidence.status}`) : t("loops.operation.none")} />
        {run.activeOperationId || operationEvidence?.operationId ? <Field label={t("loops.monitor.operationId")} value={run.activeOperationId ?? operationEvidence?.operationId ?? ""} /> : null}
        {run.activeOperationId || operationEvidence?.operationId ? <LoopInspectionActions onInspect={onInspect} sessionId={inspectionSessionId} surfaces={["logs"]} /> : null}
        {run.terminalReason ? <Field label={t("loops.inspector.reason")} value={t(`loops.reason.${run.terminalReason}`)} /> : null}
      </InspectorSection>
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
  );
}

function InspectorSection({ children, title }: { children: ReactNode; title: string }) {
  return <section><h3 className="mb-2 text-[11px] font-semibold uppercase text-muted-foreground">{title}</h3><dl className="grid gap-3">{children}</dl></section>;
}

function Field({ label, value }: { label: string; value: string }) {
  return (
    <div className="min-w-0 border-b border-border/60 pb-3 last:border-0">
      <dt className="text-[11px] text-muted-foreground">{label}</dt>
      <dd className="mt-1 wrap-break-word text-xs text-foreground">{value}</dd>
    </div>
  );
}
