import { Loader2, Plus } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { Goal, GoalInput, GoalLinkTarget } from "../contracts/goal";
import { goalService } from "../services/runtime-goal-client";
import { GoalDetail } from "./goal-detail";
import { GoalForm } from "./goal-form";
import { progressLabel, progressRatio, statusTone } from "./goal-presentation";

function GoalProgressMeter({ goal }: { goal: Goal }) {
  const { t } = useTranslation();
  const ratio = progressRatio(goal);
  return (
    <span className="flex items-center gap-2">
      <span
        aria-label={t("goals.progress.title")}
        aria-valuemax={goal.counted}
        aria-valuemin={0}
        aria-valuenow={goal.terminal}
        className="h-1.5 min-w-0 flex-1 overflow-hidden rounded-full bg-muted"
        role="progressbar"
      >
        {/* Inline width because the value is data, not styling: it is the goal's own ratio. */}
        <span className="block h-full rounded-full bg-primary" style={{ width: `${Math.round(ratio * 100)}%` }} />
      </span>
      <span className="shrink-0 text-xs tabular-nums text-muted-foreground">{progressLabel(goal)}</span>
    </span>
  );
}

export function GoalCenter() {
  const { t } = useTranslation();
  const [goals, setGoals] = useState<Goal[]>([]);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState(false);
  const [busy, setBusy] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setBusy(true);
    try {
      setGoals(await goalService.listGoals());
      setError(null);
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
    } finally {
      setBusy(false);
    }
  }, []);
  useEffect(() => { void load(); }, [load]);

  const perform = async (action: () => Promise<unknown>) => {
    setBusy(true);
    try {
      await action();
      setError(null);
      await load();
    } catch (reason: unknown) {
      setError(reason instanceof Error ? reason.message : String(reason));
      setBusy(false);
    }
  };

  const selected = goals.find((goal) => goal.id === selectedId) ?? null;
  const create = (input: GoalInput) => perform(async () => {
    const goal = await goalService.createGoal(input);
    setSelectedId(goal.id);
    setCreating(false);
  });
  const update = (input: GoalInput) => perform(async () => {
    if (selected) await goalService.updateGoal(selected.id, input);
    setEditing(false);
  });
  const link = (targetKind: GoalLinkTarget, targetId: string) =>
    perform(() => goalService.linkGoalTarget(String(selectedId), targetKind, targetId));
  const unlink = (targetKind: GoalLinkTarget, targetId: string) =>
    perform(() => goalService.unlinkGoalTarget(String(selectedId), targetKind, targetId));
  const remove = () => perform(async () => {
    await goalService.deleteGoal(String(selectedId));
    setSelectedId(null);
  });

  return <section aria-labelledby="goal-center-title" className="ucd-panel flex h-full min-h-0 flex-1 flex-col overflow-hidden rounded-lg" id="goal-center">
    <header className="grid shrink-0 gap-3 border-b border-border p-3 md:p-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-base font-semibold" id="goal-center-title">{t("goals.title")}</h1>
          <p className="text-xs text-muted-foreground">{t("goals.subtitle")}</p>
        </div>
        <Button onClick={() => { setEditing(false); setCreating((value) => !value); }} size="sm" type="button">
          <Plus aria-hidden="true" />{t("goals.new")}
        </Button>
      </div>
      {creating ? <GoalForm busy={busy} onCancel={() => setCreating(false)} onSubmit={create} submitLabel={t("goals.actions.create")} /> : null}
      {editing && selected ? <GoalForm busy={busy} goal={selected} onCancel={() => setEditing(false)} onSubmit={update} submitLabel={t("goals.actions.save")} /> : null}
    </header>

    {error ? <p className="m-3 rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">{error}</p> : null}

    {busy && !goals.length
      ? <div className="grid flex-1 place-items-center"><Loader2 aria-label={t("goals.loading")} className="animate-spin" /></div>
      : <div className="grid min-h-0 flex-1 gap-3 overflow-y-auto p-3 md:grid-cols-[minmax(14rem,20rem)_1fr] md:overflow-hidden">
          <ul className="grid content-start gap-2 max-md:max-h-64 max-md:overflow-y-auto md:overflow-y-auto" aria-label={t("goals.listLabel")}>
            {goals.map((goal) => <li key={goal.id}>
              <button
                aria-current={goal.id === selectedId}
                className={`relative grid w-full gap-1.5 rounded-md border px-3 py-2.5 text-left transition-colors ${
                  goal.id === selectedId
                    ? "border-primary bg-[hsl(var(--nav-active-soft))] shadow-[0_0_0_1px_hsl(var(--primary))]"
                    : "border-border hover:bg-muted/40"
                }`}
                onClick={() => { setSelectedId(goal.id); setEditing(false); }}
                type="button"
              >
                {goal.id === selectedId ? <span aria-hidden="true" className="absolute inset-y-2 left-0 w-0.5 rounded bg-primary" /> : null}
                <span className="flex items-start justify-between gap-2">
                  <span className="min-w-0 flex-1 truncate text-sm font-semibold">{goal.title}</span>
                  <span className={`shrink-0 rounded px-1.5 py-0.5 text-[0.6875rem] ${statusTone(goal.derivedStatus)}`}>
                    {t(`goals.status.${goal.derivedStatus}`)}
                  </span>
                </span>
                <GoalProgressMeter goal={goal} />
              </button>
            </li>)}
            {goals.length === 0 ? <li className="rounded-md border border-dashed border-border p-4 text-center text-xs text-muted-foreground">{t("goals.empty")}</li> : null}
          </ul>

          <div className="min-h-0 rounded-md border border-border md:overflow-hidden">
            {selected
              ? <GoalDetail
                  busy={busy}
                  goal={selected}
                  onAbandon={() => void perform(() => goalService.abandonGoal(selected.id))}
                  onAccept={() => void perform(() => goalService.acceptGoal(selected.id))}
                  onActivate={() => void perform(() => goalService.activateGoal(selected.id))}
                  onDelete={() => void remove()}
                  onEdit={() => { setCreating(false); setEditing(true); }}
                  onLink={(kind, id) => void link(kind, id)}
                  onReopen={() => void perform(() => goalService.reopenGoal(selected.id))}
                  onUnlink={(kind, id) => void unlink(kind, id)}
                />
              : <p className="grid h-full place-items-center p-4 text-center text-xs text-muted-foreground">{t("goals.detail.empty")}</p>}
          </div>
        </div>}
  </section>;
}
