import { AlertTriangle, Loader2, Plus } from "lucide-react";
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Button } from "../components/ui/button";
import type { Goal } from "../contracts/goal";
import type { MutationState } from "../ui/async/mutation-state";
import { GoalDetail } from "./goal-detail";
import { GoalForm } from "./goal-form";
import { progressLabel, progressRatio, statusTone } from "./goal-presentation";
import { CREATE_MUTATION_KEY, useGoalCenterActions } from "./use-goal-center-actions";

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

/**
 * A quiet, decorative cue that this row's own goal has an in-flight mutation or a leftover
 * error, even while a different goal's detail pane (or none) is showing. Goal Center is
 * Master-Detail -- only the selected goal's actions and errors render in `GoalDetail` -- but
 * every row in this list stays visible regardless of selection, and every mutation here is
 * reconcile-only (see use-goal-center-actions.ts), so nothing about a pending goal's row changes
 * on its own until the response lands. Without this cue a background mutation on a non-selected
 * goal would look like nothing happened until the user re-selects it.
 *
 * Deliberately `aria-hidden` with a `title` instead of an `aria-label`/`role="status"`: folding
 * live status text into the row's own accessible name would make that name change while the
 * mutation is pending, and the row button's name is also how tests and assistive tech find it by
 * title text (see goal-center.test.tsx). The fully accessible version of this same state
 * (`role="status"` / `role="alert"`, dismissible) already renders in `GoalDetail` once the goal
 * is selected; this badge is a sighted-user hint layered on top, not a duplicate announcement.
 */
function GoalRowMutationBadge({ mutation }: { mutation: MutationState | undefined }) {
  const { t } = useTranslation();
  // `title` goes on a wrapping <span>, not the icon itself: lucide-react's own props don't
  // include `title`, and putting the hover text on a plain element also keeps it out of the
  // accessibility tree cleanly via this span's own `aria-hidden`, rather than relying on the
  // icon's internal SVG structure.
  if (mutation?.pending) {
    return <span aria-hidden="true" className="shrink-0" title={t("workbenchUi.mutation.pending")}>
      <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
    </span>;
  }
  if (mutation?.error) {
    return <span aria-hidden="true" className="shrink-0" title={mutation.error.message}>
      <AlertTriangle className="h-3 w-3 text-destructive" />
    </span>;
  }
  return null;
}

export function GoalCenter() {
  const { t } = useTranslation();
  const {
    abandon, accept, activate, create, error, goals, link, loading, mutations, remove, reopen, unlink, update,
  } = useGoalCenterActions();
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [creating, setCreating] = useState(false);
  const [editing, setEditing] = useState(false);

  const selected = goals.find((goal) => goal.id === selectedId) ?? null;

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
      {creating
        ? <GoalForm
            mutation={mutations.get(CREATE_MUTATION_KEY)}
            onCancel={() => setCreating(false)}
            onSubmit={(input) => void create(input, (goal) => { setSelectedId(goal.id); setCreating(false); })}
            submitLabel={t("goals.actions.create")}
          />
        : null}
      {editing && selected
        ? <GoalForm
            goal={selected}
            mutation={mutations.get(selected.id)}
            onCancel={() => setEditing(false)}
            onSubmit={(input) => void update(selected, input, () => setEditing(false))}
            submitLabel={t("goals.actions.save")}
          />
        : null}
    </header>

    {error ? <p className="m-3 rounded border border-destructive/50 bg-destructive/10 p-2 text-sm text-destructive" role="alert">{error}</p> : null}

    {loading && !goals.length
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
                  <span className="flex shrink-0 items-center gap-1">
                    <GoalRowMutationBadge mutation={mutations.get(goal.id)} />
                    <span className={`rounded px-1.5 py-0.5 text-[0.6875rem] ${statusTone(goal.derivedStatus)}`}>
                      {t(`goals.status.${goal.derivedStatus}`)}
                    </span>
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
                  goal={selected}
                  mutation={mutations.get(selected.id)}
                  onAbandon={() => void abandon(selected)}
                  onAccept={() => void accept(selected)}
                  onActivate={() => void activate(selected)}
                  onDelete={() => void remove(selected, () => setSelectedId(null))}
                  onDismissError={() => mutations.clear(selected.id)}
                  onEdit={() => { setCreating(false); setEditing(true); }}
                  onLink={(kind, id) => void link(selected, kind, id)}
                  onReopen={() => void reopen(selected)}
                  onUnlink={(kind, id) => void unlink(selected, kind, id)}
                />
              : <p className="grid h-full place-items-center p-4 text-center text-xs text-muted-foreground">{t("goals.detail.empty")}</p>}
          </div>
        </div>}
  </section>;
}
