import { useCallback, useEffect, useState } from "react";
import { Check, Loader2, MessageSquareMore, Pause, Play, Square, X } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { applyLoopRunUpdate, loopQueryKeys } from "../hooks/loop-query";
import { useLoopAdmission, type LoopAdmissionTarget } from "../hooks/use-loop-admission";
import { agentService } from "../services/runtime-agent-client";
import type { LoopControlEnvelope, LoopRun } from "../types/loop";
import { LoopAuditAcknowledgement } from "./loop-scope-assessment";

export type LoopControlAction = "pause" | "resume" | "cancel" | "accept" | "continue" | "reject";
type ConfirmAction = "pause" | "cancel" | "reject";

export function availableLoopActions(run: Pick<LoopRun, "status">): LoopControlAction[] {
  if (run.status === "queued" || run.status === "running") return ["pause", "cancel"];
  if (run.status === "paused") return ["resume", "cancel"];
  if (run.status === "awaiting-acceptance") return ["accept", "continue", "reject"];
  return [];
}

/** A run whose binding is gone or whose host changed cannot regain authority by resuming. */
export function resumeBlockedReason(run: Pick<LoopRun, "terminalReason" | "scope">): "binding" | "capability" | null {
  if (run.terminalReason === "scope-binding-missing" || run.scope?.bindingStatus === "missing" || run.scope?.bindingStatus === "legacy") return "binding";
  if (run.terminalReason === "scope-capability-changed") return "capability";
  return null;
}

export function LoopRunControls({ run }: { run: LoopRun }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const actions = availableLoopActions(run);
  const [pending, setPending] = useState<LoopControlAction | null>(null);
  const [confirming, setConfirming] = useState<ConfirmAction | null>(null);
  const [feedback, setFeedback] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => { setConfirming(null); setError(null); }, [run.id, run.status]);

  const publish = useCallback((updated: LoopRun) => {
    queryClient.setQueryData(loopQueryKeys.run(updated.id), updated);
    queryClient.setQueriesData<LoopRun[]>({ queryKey: ["loops", "runs"] }, (current) => applyLoopRunUpdate(current, updated));
  }, [queryClient]);

  const admitted = useCallback(async (target: LoopAdmissionTarget, envelope: LoopControlEnvelope) => {
    setPending(target.action === "resume" ? "resume" : "continue");
    try {
      const updated = target.action === "resume"
        ? await agentService.resumeLoop(run.id, envelope)
        : await agentService.continueLoop({ runId: run.id, feedback: feedback.trim(), envelope });
      publish(updated);
      if (target.action === "continue") setFeedback("");
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : String(actionError));
    } finally {
      setPending(null);
    }
  }, [feedback, publish, run.id]);
  const admission = useLoopAdmission(admitted);

  async function execute(action: LoopControlAction) {
    if (pending || admission.busy || (action === "continue" && !feedback.trim())) return;
    setConfirming(null);
    setError(null);
    if (action === "resume" || action === "continue") {
      await admission.request({ action, runId: run.id, expectedRevision: run.revision });
      return;
    }
    setPending(action);
    try {
      const updated = action === "pause" ? await agentService.pauseLoop(run.id)
        : action === "cancel" ? await agentService.cancelLoop(run.id)
          : action === "accept" ? (await agentService.requestLoopAcceptance({ runId: run.id, expectedRevision: run.revision, expectedScopeDigest: run.scope?.scopeDigest ?? null })).run
            : await agentService.rejectLoop(run.id);
      publish(updated);
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : String(actionError));
    } finally {
      setPending(null);
    }
  }

  if (actions.length === 0) return null;
  const busy = pending !== null || admission.busy;
  const canContinue = run.currentIteration < run.definitionSnapshot.limits.maxIterations;
  const resumeBlocked = resumeBlockedReason(run);
  const acceptancePending = Boolean(run.scope?.acceptanceOperationId);
  const shownError = error ?? admission.error;
  return (
    <section aria-labelledby="loop-controls-title" className="grid gap-3 border-y border-border/70 py-3">
      <div><h3 className="text-[11px] font-semibold uppercase text-muted-foreground" id="loop-controls-title">{t("loops.controls.title")}</h3>{run.status === "paused" && !resumeBlocked ? <p className="mt-1 text-xs text-muted-foreground">{t("loops.controls.resumeBoundary", { phase: t(`loops.phase.${run.phase}`) })}</p> : null}{resumeBlocked && run.status === "paused" ? <p className="mt-1 text-xs text-destructive">{t(`loops.controls.resumeBlocked.${resumeBlocked}`)}</p> : null}{run.pauseRequested ? <p className="mt-1 text-xs text-warning">{t("loops.controls.pauseRequested")}</p> : null}</div>
      {actions.includes("pause") || actions.includes("resume") ? <div className="grid grid-cols-2 gap-2">
        {actions.includes("pause") ? <Button disabled={busy || run.pauseRequested} onClick={() => setConfirming("pause")} size="sm" type="button" variant="outline"><Pause aria-hidden="true" />{t("loops.controls.pause")}</Button> : null}
        {actions.includes("resume") ? <Button disabled={busy || resumeBlocked !== null || admission.pending !== null} onClick={() => void execute("resume")} size="sm" type="button"><Play aria-hidden="true" />{t("loops.controls.resume")}</Button> : null}
        {actions.includes("cancel") ? <Button className="text-destructive hover:text-destructive" disabled={busy} onClick={() => setConfirming("cancel")} size="sm" type="button" variant="outline"><Square aria-hidden="true" />{t("loops.controls.stop")}</Button> : null}
      </div> : null}
      {actions.includes("accept") ? <div className="grid gap-2">
        <Button disabled={busy || acceptancePending} onClick={() => void execute("accept")} size="sm" type="button"><Check aria-hidden="true" />{t("loops.controls.accept")}</Button>
        {acceptancePending ? <p aria-live="polite" className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />{t("loops.controls.acceptancePending")}</p> : null}
        <label className="grid gap-1.5"><span className="text-xs font-medium text-muted-foreground">{t("loops.controls.feedback")}</span><textarea className="ucd-input min-h-20 w-full rounded p-2 text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" disabled={busy || !canContinue || acceptancePending} onChange={(event) => setFeedback(event.target.value)} value={feedback} /></label>
        {!canContinue ? <p className="text-xs text-warning">{t("loops.controls.iterationLimitReached")}</p> : null}
        <div className="grid grid-cols-2 gap-2"><Button disabled={busy || !canContinue || !feedback.trim() || acceptancePending || admission.pending !== null} onClick={() => void execute("continue")} size="sm" type="button" variant="outline"><MessageSquareMore aria-hidden="true" />{t("loops.controls.continue")}</Button><Button className="text-destructive hover:text-destructive" disabled={busy || acceptancePending} onClick={() => setConfirming("reject")} size="sm" type="button" variant="outline"><X aria-hidden="true" />{t("loops.controls.reject")}</Button></div>
      </div> : null}
      {admission.pending ? <LoopAuditAcknowledgement admission={admission.pending} busy={busy} onAcknowledge={() => void admission.acknowledge()} onDismiss={admission.dismiss} /> : null}
      {confirming ? <div aria-describedby="loop-control-confirm-description" aria-labelledby="loop-control-confirm-title" className="grid gap-2 rounded-md border border-warning/50 bg-warning/5 p-3" role="alertdialog"><p className="text-xs font-medium" id="loop-control-confirm-title">{t(`loops.controls.confirm.${confirming}.title`)}</p><p className="text-xs text-muted-foreground" id="loop-control-confirm-description">{t(`loops.controls.confirm.${confirming}.description`)}</p><div className="grid grid-cols-2 gap-2"><Button disabled={busy} onClick={() => setConfirming(null)} size="sm" type="button" variant="ghost">{t("loops.controls.dismiss")}</Button><Button disabled={busy} onClick={() => void execute(confirming)} size="sm" type="button">{t("loops.controls.confirmAction")}</Button></div></div> : null}
      {pending ? <p aria-live="polite" className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />{t("loops.controls.pending", { action: t(`loops.controls.${pending}`) })}</p> : null}
      {shownError ? <p aria-live="assertive" className="whitespace-pre-line text-xs text-destructive">{shownError}</p> : null}
    </section>
  );
}
