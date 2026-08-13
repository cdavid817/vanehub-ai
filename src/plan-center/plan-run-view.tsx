import { Check, CircleStop, ListChecks, Pause, Play, RefreshCw, RotateCcw } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import type { PlanAttemptEvidence, PlanControlKind, PlanRunDetail, PlanSubTaskAttempt, PlanSubTaskRun, SubTaskRunStatus } from "../types/plan";

const statusTone: Record<SubTaskRunStatus, "default" | "success" | "warning" | "danger" | "muted"> = {
  pending: "muted", ready: "default", dispatching: "default", running: "default", verifying: "warning",
  succeeded: "success", failed: "danger", cancelled: "muted", interrupted: "warning", blocked: "danger", skipped: "muted",
};

export function PlanRunView({ busy, onAccept, onControl, onInspectEvidence, onRetry, onReturnPlanning, run }: {
  busy: boolean;
  onAccept: () => void;
  onControl: (kind: PlanControlKind) => void;
  onInspectEvidence: (attemptId: string) => Promise<PlanAttemptEvidence[]>;
  onRetry: (taskId: string) => void;
  onReturnPlanning?: () => void;
  run: PlanRunDetail;
}) {
  const { t } = useTranslation();
  const percent = run.totalTasks === 0 ? 0 : Math.round((run.completedTasks / run.totalTasks) * 100);
  return (
    <div className="grid min-h-0 gap-4 overflow-y-auto p-1">
      <header className="ucd-card grid gap-3 rounded-lg p-4">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div aria-live="polite" role="status"><div className="flex items-center gap-2"><h2 className="text-base font-semibold">{t("plans.run.title")}</h2><Badge tone={run.status === "failed" ? "danger" : run.status === "completed" ? "success" : "default"}>{t(`plans.status.${run.status}`)}</Badge>{run.simulated ? <Badge tone="warning">{t("plans.run.simulated")}</Badge> : null}</div><p className="mt-1 text-xs text-muted-foreground">{t("plans.run.progress", { complete: run.completedTasks, total: run.totalTasks })}</p></div>
          <div className="flex flex-wrap gap-2">
            {run.availableControls.includes("pause") ? <Button disabled={busy} onClick={() => onControl("pause")} size="sm" type="button" variant="outline"><Pause aria-hidden="true" />{t("plans.controls.pause")}</Button> : null}
            {run.availableControls.includes("resume") ? <Button disabled={busy} onClick={() => onControl("resume")} size="sm" type="button"><Play aria-hidden="true" />{t("plans.controls.resume")}</Button> : null}
            {run.availableControls.includes("recover") ? <Button disabled={busy} onClick={() => onControl("recover")} size="sm" type="button"><RotateCcw aria-hidden="true" />{t("plans.controls.recover")}</Button> : null}
            {run.availableControls.includes("retry") ? <Button disabled={busy} onClick={() => onControl("retry")} size="sm" type="button"><RefreshCw aria-hidden="true" />{t("plans.controls.retryFinal")}</Button> : null}
            {run.status === "action_required" && onReturnPlanning ? <Button disabled={busy} onClick={onReturnPlanning} size="sm" type="button" variant="outline"><ListChecks aria-hidden="true" />{t("plans.controls.returnPlanning")}</Button> : null}
            {run.availableControls.includes("cancel") ? <Button disabled={busy} onClick={() => onControl("cancel")} size="sm" type="button" variant="outline"><CircleStop aria-hidden="true" />{t("plans.controls.cancel")}</Button> : null}
            {run.availableControls.includes("accept") ? <Button disabled={busy} onClick={onAccept} size="sm" type="button"><Check aria-hidden="true" />{t("plans.controls.accept")}</Button> : null}
          </div>
        </div>
        <progress aria-label={t("plans.run.progressLabel")} className="h-2 w-full overflow-hidden rounded-full accent-primary" max={100} value={percent} />
        {run.worktreePath ? <div className="grid gap-1 rounded-md border border-border bg-muted/20 p-3 text-xs"><span className="font-medium">{t("plans.run.retainedWorktree")}</span><code className="break-all text-muted-foreground">{run.worktreePath}</code><span className="text-muted-foreground">{t("plans.run.noAutomaticGit")}</span></div> : null}
      </header>
      {run.finalization ? <FinalizationCard finalization={run.finalization} /> : null}
      <ol className="grid gap-3" aria-label={t("plans.run.tasks")}>{run.tasks.map((task) => <TaskCard busy={busy} key={task.id} onInspectEvidence={onInspectEvidence} onRetry={onRetry} task={task} />)}</ol>
    </div>
  );
}

function FinalizationCard({ finalization }: { finalization: NonNullable<PlanRunDetail["finalization"]> }) {
  const { t } = useTranslation();
  return <section aria-labelledby="final-verification-title" className="ucd-card rounded-lg p-3"><h3 className="text-sm font-semibold" id="final-verification-title">{t("plans.run.finalVerification")}</h3><p className="mt-1 text-xs text-muted-foreground">{t("plans.run.finalizationAttempt", { sequence: finalization.sequence, status: finalization.status })}</p>{finalization.evidence.map((item) => <div className="mt-2 border-t border-border pt-2 text-xs" key={item.id}><span className="font-medium">{item.commandId}: {item.status}</span>{item.outputSummary ? <pre className="mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-muted p-2">{item.outputSummary}</pre> : null}</div>)}{finalization.repairAttempts.length ? <div className="mt-3 border-t border-border pt-2"><h4 className="text-xs font-semibold">{t("plans.run.repairHistory")}</h4>{finalization.repairAttempts.map((attempt) => <p className="mt-1 text-xs text-muted-foreground" key={attempt.id}>{t("plans.run.finalRepairAttempt", { sequence: attempt.sequence, status: attempt.status, tokens: attempt.tokenUsage, tools: attempt.toolCallCount })}</p>)}</div> : null}</section>;
}

function TaskCard({ busy, onInspectEvidence, onRetry, task }: { busy: boolean; onInspectEvidence: (attemptId: string) => Promise<PlanAttemptEvidence[]>; onRetry: (taskId: string) => void; task: PlanSubTaskRun }) {
  const { t } = useTranslation();
  return <li className="ucd-card rounded-lg p-3"><div className="flex flex-wrap items-center justify-between gap-2"><div className="flex min-w-0 items-center gap-2"><span className="grid h-7 w-7 shrink-0 place-items-center rounded-full bg-muted text-xs font-semibold">{task.ordinal + 1}</span><h3 className="truncate text-sm font-medium">{task.title}</h3><Badge tone={statusTone[task.status]}>{t(`plans.taskStatus.${task.status}`)}</Badge></div>{["failed", "interrupted"].includes(task.status) ? <Button disabled={busy} onClick={() => onRetry(task.id)} size="sm" type="button" variant="outline"><RefreshCw aria-hidden="true" />{t("plans.controls.retry")}</Button> : null}</div>{task.resultSummary ? <p className="mt-2 text-xs text-muted-foreground">{task.resultSummary}</p> : null}{task.verificationSummary ? <p className="mt-1 text-xs text-muted-foreground">{task.verificationSummary}</p> : null}{task.changedFiles.length ? <p className="mt-1 text-xs text-muted-foreground">{t("plans.run.changedFiles", { count: task.changedFiles.length })}</p> : null}{task.attempts.map((attempt) => <AttemptDetails attempt={attempt} key={attempt.id} onInspectEvidence={onInspectEvidence} />)}</li>;
}

function AttemptDetails({ attempt, onInspectEvidence }: { attempt: PlanSubTaskAttempt; onInspectEvidence: (attemptId: string) => Promise<PlanAttemptEvidence[]> }) {
  const { t } = useTranslation();
  const [evidence, setEvidence] = useState<PlanAttemptEvidence[] | null>(null);
  const [loading, setLoading] = useState(false);
  const [failed, setFailed] = useState(false);
  const inspect = async () => {
    if (evidence !== null || loading) return;
    setLoading(true);
    setFailed(false);
    try { setEvidence(await onInspectEvidence(attempt.id)); }
    catch { setFailed(true); }
    finally { setLoading(false); }
  };
  return <details className="mt-2 rounded-md border border-border px-3 py-2 text-xs" onToggle={(event) => { if (event.currentTarget.open) void inspect(); }}><summary className="cursor-pointer font-medium">{t("plans.run.attempt", { sequence: attempt.sequence })} · {attempt.status}</summary><dl className="mt-2 grid gap-1 text-muted-foreground"><div>{t("plans.run.session")}: {attempt.sessionId ?? "—"}</div><div>{t("plans.run.usage", { tokens: attempt.tokenUsage, tools: attempt.toolCallCount })}</div>{attempt.errorClass ? <div>{t("plans.run.errorClass")}: {attempt.errorClass}</div> : null}</dl>{loading ? <p className="mt-2 text-muted-foreground">{t("plans.run.evidenceLoading")}</p> : null}{failed ? <p className="mt-2 text-destructive" role="alert">{t("plans.run.evidenceFailed")}</p> : null}{evidence?.length === 0 ? <p className="mt-2 text-muted-foreground">{t("plans.run.noEvidence")}</p> : null}{evidence?.map((item) => <div className="mt-2 border-t border-border pt-2" key={item.id}><span className="font-medium">{item.commandId}: {item.status}</span>{item.outputSummary ? <pre className="mt-1 max-h-40 overflow-auto whitespace-pre-wrap rounded bg-muted p-2">{item.outputSummary}</pre> : null}</div>)}</details>;
}
