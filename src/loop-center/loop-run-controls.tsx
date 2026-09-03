import { useEffect, useState, type ReactNode } from "react";
import { Check, Loader2, MessageSquareMore, Pause, Play, Square, X } from "lucide-react";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { applyLoopRunUpdate, loopQueryKeys } from "../hooks/loop-query";
import { agentService } from "../services/runtime-agent-client";
import { ActionMenu, type ActionMenuItem } from "../ui/actions/ActionMenu";
import type { LoopRun } from "../types/loop";

export type LoopControlAction = "pause" | "resume" | "cancel" | "accept" | "continue" | "reject";
// `resume` is exempt from any confirm step: it reverses pause rather than committing the run
// further, so it keeps executing immediately. Of the rest, only `pause` and `accept` -- each the
// one primary, always-visible action for its status (17.8) -- plus `continue` stay on this
// hand-rolled preview-then-confirm block. `cancel` and `reject` moved into `More` (`ActionMenu`) as
// of 17.8 and now use its own built-in `confirmation` instead: both are a fixed
// title/description/confirm-button shape with no live input, exactly what that primitive is for, so
// keeping a second, parallel confirm mechanism here for them too would just be the same job done
// twice. `pause`/`accept` cannot make that same move -- as the primary action they render outside
// any menu, and `ActionMenu`'s `confirmation` only wraps menu items. `continue` could technically
// move (it *is* a `More` item, not primary), but its confirm step has to keep this run's feedback
// `<textarea>` (rendered below, always visible whenever `continue` is offered) reachable and
// editable while the reader decides, re-validating that value if it changes before they confirm --
// see the guard in the confirm block below. `ActionMenu`'s own confirmation renders a
// focus-trapped, full-screen `ApplicationDialog`, which would cover and lock that textarea
// entirely, silently losing the "keep typing, Confirm disables itself if you clear it" behavior the
// guard exists to protect. So `continue`'s `More` entry only opens this same hand-rolled block,
// same as before -- it just opens from inside the menu now instead of from its own button.
type ConfirmAction = "pause" | "accept" | "continue";

export function availableLoopActions(run: Pick<LoopRun, "status">): LoopControlAction[] {
  if (run.status === "queued" || run.status === "running") return ["pause", "cancel"];
  if (run.status === "paused") return ["resume", "cancel"];
  if (run.status === "awaiting-acceptance") return ["accept", "continue", "reject"];
  return [];
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

  async function execute(action: LoopControlAction) {
    if (pending || (action === "continue" && !feedback.trim())) return;
    setPending(action);
    setConfirming(null);
    setError(null);
    try {
      const updated = action === "pause" ? await agentService.pauseLoop(run.id)
        : action === "resume" ? await agentService.resumeLoop(run.id)
          : action === "cancel" ? await agentService.cancelLoop(run.id)
            : action === "accept" ? await agentService.acceptLoop(run.id)
              : action === "continue" ? await agentService.continueLoop({ runId: run.id, feedback: feedback.trim() })
                : await agentService.rejectLoop(run.id);
      queryClient.setQueryData(loopQueryKeys.run(updated.id), updated);
      queryClient.setQueriesData<LoopRun[]>({ queryKey: ["loops", "runs"] }, (current) => applyLoopRunUpdate(current, updated));
      if (action === "continue") setFeedback("");
    } catch (actionError) {
      setError(actionError instanceof Error ? actionError.message : String(actionError));
    } finally {
      setPending(null);
    }
  }

  if (actions.length === 0) return null;
  const busy = pending !== null;
  const canContinue = run.currentIteration < run.definitionSnapshot.limits.maxIterations;

  // Exactly one of pause/resume/accept is ever a candidate for a given status
  // (`availableLoopActions` never returns more than one of the three), so -- matching
  // `goal-detail.tsx`'s own `primaryAction` (15.3) -- each is guarded by a disjoint check against
  // `actions`, never a second candidate to weigh against another.
  let primaryAction: ReactNode = null;
  if (actions.includes("pause")) {
    primaryAction = <Button disabled={busy || run.pauseRequested} onClick={() => setConfirming("pause")} size="sm" type="button"><Pause aria-hidden="true" />{t("loops.controls.pause")}</Button>;
  } else if (actions.includes("resume")) {
    primaryAction = <Button disabled={busy} onClick={() => void execute("resume")} size="sm" type="button"><Play aria-hidden="true" />{t("loops.controls.resume")}</Button>;
  } else if (actions.includes("accept")) {
    primaryAction = <Button disabled={busy} onClick={() => setConfirming("accept")} size="sm" type="button"><Check aria-hidden="true" />{t("loops.controls.accept")}</Button>;
  }

  // Everything else this status allows, grouped into `More` instead of its own always-visible
  // button (17.8). `cancel`/`continue`/`reject` are the only actions that can ever land here --
  // see the `ConfirmAction` comment above for why `continue` still opens the hand-rolled block
  // below instead of using `ActionMenu`'s own `confirmation`.
  const moreItems: ActionMenuItem[] = [];
  if (actions.includes("cancel")) {
    moreItems.push({
      // `confirmLabel` reuses the same key the hand-rolled block's own Confirm button used for this
      // action before 17.8 (`loops.controls.confirmAction`), rather than `ActionMenu`'s generic
      // `confirmation.confirm` default -- the two read identically in en/zh-CN/zh-TW/ko but not ja
      // ("確認する" vs "確認"), so leaving it implicit would have silently changed the ja wording.
      confirmation: {
        confirmLabel: t("loops.controls.confirmAction"),
        description: t("loops.controls.confirm.cancel.description"),
        title: t("loops.controls.confirm.cancel.title"),
      },
      disabled: busy,
      icon: Square,
      id: "cancel",
      label: t("loops.controls.stop"),
      onSelect: () => void execute("cancel"),
      tone: "destructive",
    });
  }
  if (actions.includes("continue")) {
    moreItems.push({
      disabled: busy || !canContinue || !feedback.trim(),
      disabledReason: !canContinue ? t("loops.controls.iterationLimitReached") : undefined,
      icon: MessageSquareMore,
      id: "continue",
      label: t("loops.controls.continue"),
      onSelect: () => setConfirming("continue"),
    });
  }
  if (actions.includes("reject")) {
    moreItems.push({
      confirmation: {
        confirmLabel: t("loops.controls.confirmAction"),
        description: t("loops.controls.confirm.reject.description"),
        title: t("loops.controls.confirm.reject.title"),
      },
      disabled: busy,
      icon: X,
      id: "reject",
      label: t("loops.controls.reject"),
      onSelect: () => void execute("reject"),
      tone: "destructive",
    });
  }

  return (
    <section aria-labelledby="loop-controls-title" className="grid gap-3 border-y border-border/70 py-3">
      <div><h3 className="text-[11px] font-semibold uppercase text-muted-foreground" id="loop-controls-title">{t("loops.controls.title")}</h3>{run.status === "paused" ? <p className="mt-1 text-xs text-muted-foreground">{t("loops.controls.resumeBoundary", { phase: t(`loops.phase.${run.phase}`) })}</p> : null}{run.pauseRequested ? <p className="mt-1 text-xs text-warning">{t("loops.controls.pauseRequested")}</p> : null}</div>
      <div className="flex flex-wrap items-center gap-2">
        {primaryAction}
        {moreItems.length > 0 ? <ActionMenu items={moreItems} triggerLabel={t("workbenchUi.pageHeader.moreActions")} /> : null}
      </div>
      {actions.includes("accept") ? <div className="grid gap-2">
        <label className="grid gap-1.5"><span className="text-xs font-medium text-muted-foreground">{t("loops.controls.feedback")}</span><textarea className="ucd-input min-h-20 w-full rounded p-2 text-xs outline-hidden focus-visible:ring-2 focus-visible:ring-ring" disabled={busy || !canContinue} onChange={(event) => setFeedback(event.target.value)} value={feedback} /></label>
        {!canContinue ? <p className="text-xs text-warning">{t("loops.controls.iterationLimitReached")}</p> : null}
      </div> : null}
      {confirming ? <div aria-describedby="loop-control-confirm-description" aria-labelledby="loop-control-confirm-title" className="grid gap-2 rounded-md border border-warning/50 bg-warning/5 p-3" role="alertdialog"><p className="text-xs font-medium" id="loop-control-confirm-title">{t(`loops.controls.confirm.${confirming}.title`)}</p><p className="text-xs text-muted-foreground" id="loop-control-confirm-description">{t(`loops.controls.confirm.${confirming}.description`)}</p><div className="grid grid-cols-2 gap-2"><Button disabled={busy} onClick={() => setConfirming(null)} size="sm" type="button" variant="ghost">{t("loops.controls.dismiss")}</Button>{/* The feedback textarea above stays editable while confirming, so re-check continue's own precondition here too -- otherwise clearing it after opening this step would make Confirm a silent no-op instead of a disabled button. */}<Button disabled={busy || (confirming === "continue" && !feedback.trim())} onClick={() => void execute(confirming)} size="sm" type="button">{t("loops.controls.confirmAction")}</Button></div></div> : null}
      {pending ? <p aria-live="polite" className="flex items-center gap-2 text-xs text-muted-foreground"><Loader2 aria-hidden="true" className="h-3.5 w-3.5 animate-spin" />{t("loops.controls.pending", { action: t(`loops.controls.${pending}`) })}</p> : null}
      {error ? <p aria-live="assertive" className="text-xs text-destructive">{error}</p> : null}
    </section>
  );
}
