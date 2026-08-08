import { Bot, Loader2, Sparkles } from "lucide-react";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import { planService } from "../services/runtime-plan-client";
import { subscribePlanRunPolling } from "../services/plan-run-polling";
import type { PlanControlKind, PlanDraft, PlanRunDetail } from "../types/plan";
import { PlanDraftEditor, validatePlanDraftReview } from "./plan-draft-editor";
import { PlanRunView } from "./plan-run-view";

const inputClass = "ucd-input w-full rounded-md px-3 py-2 text-sm outline-hidden focus-visible:ring-2 focus-visible:ring-ring";

export function PlanCenter() {
  const { t } = useTranslation();
  const [goal, setGoal] = useState("");
  const [projectPath, setProjectPath] = useState("");
  const [baseRef, setBaseRef] = useState("main");
  const [draft, setDraft] = useState<PlanDraft | null>(null);
  const [run, setRun] = useState<PlanRunDetail | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const runId = run?.id;
  const runStatus = run?.status;

  useEffect(() => {
    if (!runId || !runStatus || ["completed", "cancelled", "failed"].includes(runStatus)) return;
    return subscribePlanRunPolling(() => planService.getPlanRun(runId), setRun, 1_500);
  }, [runId, runStatus]);

  const perform = async (action: () => Promise<void>) => {
    setBusy(true);
    setError(null);
    try { await action(); } catch (reason: unknown) { setError(reason instanceof Error ? reason.message : String(reason)); }
    finally { setBusy(false); }
  };
  const generate = () => perform(async () => {
    if (!goal.trim() || !projectPath.trim() || !baseRef.trim()) throw new Error(t("plans.validation.goalFields"));
    setDraft(await planService.generatePlanDraft({ planId: null, version: 1, goal, projectPath, baseRef, availableTools: ["shell", "file", "grep", "glob", "edit", "remember", "recall"] }));
  });
  const approve = () => perform(async () => {
    if (!draft) return;
    const issue = validatePlanDraftReview(draft);
    if (issue) throw new Error(t(issue));
    await planService.savePlanDraft(draft);
    const approved = await planService.approvePlan(draft.id);
    await planService.startPlanRun(approved.runId);
    setRun(await planService.getPlanRun(approved.runId));
  });
  const refresh = async () => run && setRun(await planService.getPlanRun(run.id));
  const control = (kind: PlanControlKind) => perform(async () => {
    if (!run) return;
    const result = kind === "recover" ? await planService.recoverPlanRun(run.id) : await planService.requestPlanControl(run.id, kind);
    setRun(result.run);
  });

  return (
    <section aria-labelledby="plan-center-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg">
      <header className="flex shrink-0 items-center justify-between gap-3 border-b border-border px-4 py-3">
        <div className="flex min-w-0 items-center gap-3"><span className="grid h-9 w-9 shrink-0 place-items-center rounded-lg bg-primary/10 text-primary"><Bot aria-hidden="true" className="h-5 w-5" /></span><div><h1 className="truncate text-sm font-semibold" id="plan-center-title">{t("plans.title")}</h1><p className="truncate text-xs text-muted-foreground">{t("plans.subtitle")}</p></div></div>
        {draft || run ? <Button disabled={busy} onClick={() => { setDraft(null); setRun(null); setError(null); }} size="sm" type="button" variant="outline">{t("plans.newPlan")}</Button> : null}
      </header>
      <div aria-atomic="true" aria-live="polite" className="min-h-0 flex-1 overflow-y-auto p-3 md:p-5">
        {error ? <div className="mb-4 rounded-md border border-destructive/50 bg-destructive/10 px-3 py-2 text-sm text-destructive" role="alert">{error}</div> : null}
        {run ? <PlanRunView busy={busy} onAccept={() => void perform(async () => setRun((await planService.acceptPlanRun(run.id)).run))} onControl={(kind) => void control(kind)} onExecute={() => void perform(async () => { await planService.executeNextAttempt(run.id); await refresh(); })} onInspectEvidence={(attemptId) => planService.getPlanAttemptEvidence(attemptId)} onRetry={(taskId) => void perform(async () => setRun((await planService.retryPlanSubtask(run.id, taskId)).run))} run={run} /> : draft ? <div className="mx-auto grid max-w-5xl gap-4"><PlanDraftEditor draft={draft} onChange={setDraft} /><div className="sticky bottom-0 flex justify-end gap-2 border-t border-border bg-background/95 py-3 backdrop-blur"><Button disabled={busy} onClick={() => setDraft(null)} type="button" variant="outline">{t("plans.review.back")}</Button><Button disabled={busy} onClick={() => void approve()} type="button">{busy ? <Loader2 aria-hidden="true" className="animate-spin" /> : <Sparkles aria-hidden="true" />}{t("plans.review.approve")}</Button></div></div> : <GoalForm baseRef={baseRef} busy={busy} goal={goal} onBaseRef={setBaseRef} onGenerate={() => void generate()} onGoal={setGoal} onProjectPath={setProjectPath} projectPath={projectPath} />}
      </div>
    </section>
  );
}

function GoalForm({ baseRef, busy, goal, onBaseRef, onGenerate, onGoal, onProjectPath, projectPath }: { baseRef: string; busy: boolean; goal: string; onBaseRef: (value: string) => void; onGenerate: () => void; onGoal: (value: string) => void; onProjectPath: (value: string) => void; projectPath: string }) {
  const { t } = useTranslation();
  return <form className="mx-auto grid w-full max-w-2xl gap-5 py-4 md:py-10" onSubmit={(event) => { event.preventDefault(); onGenerate(); }}>
    <div className="text-center">
      <span className="mx-auto grid h-14 w-14 place-items-center rounded-2xl bg-primary/10 text-primary"><Sparkles aria-hidden="true" className="h-7 w-7" /></span>
      <h2 className="mt-4 text-xl font-semibold">{t("plans.goal.title")}</h2>
      <p className="mt-2 text-sm text-muted-foreground">{t("plans.goal.description")}</p>
    </div>
    <div className="grid gap-1.5">
      <label className="text-sm font-medium" htmlFor="plan-goal">{t("plans.goal.goal")}</label>
      <textarea aria-describedby="plan-goal-hint" className={`${inputClass} min-h-32`} id="plan-goal" onChange={(event) => onGoal(event.target.value)} required value={goal} />
      <span className="text-xs text-muted-foreground" id="plan-goal-hint">{t("plans.goal.hint")}</span>
    </div>
    <div className="grid gap-3 sm:grid-cols-[1fr_12rem]">
      <div className="grid gap-1.5">
        <label className="text-sm font-medium" htmlFor="plan-project">{t("plans.goal.project")}</label>
        <input className={inputClass} id="plan-project" onChange={(event) => onProjectPath(event.target.value)} required value={projectPath} />
      </div>
      <div className="grid gap-1.5">
        <label className="text-sm font-medium" htmlFor="plan-base">{t("plans.goal.baseRef")}</label>
        <input className={inputClass} id="plan-base" onChange={(event) => onBaseRef(event.target.value)} required value={baseRef} />
      </div>
    </div>
    <Button className="w-full" disabled={busy} type="submit">{busy ? <Loader2 aria-hidden="true" className="animate-spin" /> : <Sparkles aria-hidden="true" />}{t("plans.goal.generate")}</Button>
  </form>;
}
