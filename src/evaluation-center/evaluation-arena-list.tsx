import { useMemo } from "react";
import { useTranslation } from "react-i18next";
import type { EvaluationArena, EvaluationTask } from "../types/evaluation";
import { StatusBadge } from "../ui/status/StatusBadge";
import {
  ARENA_STATE_LABEL_KEY,
  ARENA_STATE_TONE,
  OUTCOME_TONE,
  deriveAgentSet,
  deriveArenaState,
  deriveOutcomeTally,
  findTaskPrompt,
} from "./evaluation-arena-summary";

export interface EvaluationArenaListProps {
  arenas: EvaluationArena[];
  tasks: EvaluationTask[];
}

/**
 * 18.3: one row per experiment (`EvaluationArena`), distinct from the attempt-level
 * `EvaluationResultsTable` rendered below/beside it on the same page -- see `evaluation-center.tsx`
 * for how the two are composed together.
 *
 * Deliberately not built on the shared `DataTable` primitive: `DataTable` renders a literal
 * `<table>/<tbody>` outside its (ResizeObserver-gated) compact mode, and `evaluation-center.test.tsx`
 * -- frozen for this pass -- asserts against whole-document `document.querySelectorAll("tbody
 * tr")` counts that belong to the *existing* results table alone; a second `<tbody>` would
 * silently inflate that count and break an unrelated, already-passing test. A plain card list
 * sidesteps that hazard entirely while still matching `DataTableBody`'s own compact-card visual
 * language (`ucd-card`, the same `<dl>` label/value grid).
 *
 * Every value shown here is prefixed by its own field label (e.g. the state badge always reads
 * "State: <value>", never a bare "<value>") so it can never collide, as exact rendered text, with
 * an attempt-level badge the results table shows for the same underlying outcome word.
 *
 * "Regression state" and "updated time" render an honest `evaluation.unavailable` rather than
 * fabricated data: no baseline/comparison concept exists anywhere yet (tasks.md 18.8-18.10 are
 * unbuilt), and none of `EvaluationArena`, `EvaluationAttempt`, or `EvaluationTimelineItem` carry a
 * wall-clock timestamp field anywhere (every field on all three was checked). A real timestamp
 * does exist one hop away -- `OperationTask.updatedAt` via
 * `operationService.getOperationStatus(arena.operationId)` -- but wiring that join is a new,
 * uncached, one-call-per-row data-fetching concern of its own, squarely inside the query model
 * 18.2 already extracted (`use-evaluation-query.ts`) and which this task was told not to touch;
 * left for whoever picks up real "updated time" wiring.
 */
export function EvaluationArenaList({ arenas, tasks }: EvaluationArenaListProps) {
  const { t } = useTranslation();
  const rows = useMemo(
    () => arenas.map((arena) => ({
      arena,
      state: deriveArenaState(arena.attempts),
      agentSet: deriveAgentSet(arena.attempts),
      tally: deriveOutcomeTally(arena.attempts),
      taskPrompt: findTaskPrompt(tasks, arena.taskId, arena.taskVersion),
    })),
    [arenas, tasks],
  );

  return (
    <section className="border-b border-border p-3" data-testid="evaluation-arena-list">
      <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.experiments")}</h2>
      {rows.length === 0 ? (
        <p className="p-3 text-center text-sm text-muted-foreground">{t("evaluation.empty")}</p>
      ) : (
        <ul className="flex flex-col gap-2">
          {rows.map(({ arena, state, agentSet, tally, taskPrompt }) => (
            <li className="ucd-card rounded-lg p-3" data-arena-id={arena.id} data-arena-state={state} data-testid="evaluation-arena-row" key={arena.id}>
              <div className="flex flex-wrap items-start justify-between gap-2">
                <div className="min-w-0">
                  <p className="text-sm font-medium">{arena.taskId} v{arena.taskVersion}</p>
                  {taskPrompt ? <p className="mt-0.5 line-clamp-1 text-xs text-muted-foreground">{taskPrompt}</p> : null}
                </div>
                <StatusBadge label={`${t("evaluation.state")}: ${t(ARENA_STATE_LABEL_KEY[state])}`} tone={ARENA_STATE_TONE[state]} />
              </div>
              <dl className="mt-2 grid grid-cols-[minmax(0,auto)_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
                <div className="contents">
                  <dt className="text-muted-foreground">{t("evaluation.agents")}</dt>
                  <dd className="min-w-0">{agentSet.map((agent) => agent.agentId).join(", ")}</dd>
                </div>
                <div className="contents">
                  <dt className="text-muted-foreground">{t("evaluation.outcomeSummary")}</dt>
                  <dd className="flex flex-wrap gap-1">
                    {tally.map(({ outcome, count }) => (
                      <StatusBadge key={outcome} label={`${count} · ${t(`evaluation.outcome.${outcome}`)}`} tone={OUTCOME_TONE[outcome]} />
                    ))}
                  </dd>
                </div>
                <div className="contents">
                  <dt className="text-muted-foreground">{t("evaluation.regressionState")}</dt>
                  <dd data-testid="evaluation-arena-regression">{t("evaluation.unavailable")}</dd>
                </div>
                <div className="contents">
                  <dt className="text-muted-foreground">{t("evaluation.updatedTime")}</dt>
                  <dd data-testid="evaluation-arena-updated">{t("evaluation.unavailable")}</dd>
                </div>
              </dl>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
