import { useCallback, useMemo, useState } from "react";
import { Download } from "lucide-react";
import { useTranslation } from "react-i18next";
import { DataTable } from "../ui/data-table/DataTable";
import { StatusBadge, type StatusTone } from "../ui/status/StatusBadge";
import type { DataTableColumn, DataTableRowMeta, DataTableSort } from "../ui/data-table/types";
import type { EvaluationArena, EvaluationAttempt, EvaluationMetric, EvaluationOutcome } from "../types/evaluation";

export interface EvaluationResultRow {
  arena: EvaluationArena;
  attempt: EvaluationAttempt;
}

export interface EvaluationResultsTableProps {
  rows: EvaluationResultRow[];
  filter: string;
  onFilterChange: (value: string) => void;
  onSelectAttempt: (attemptId: string) => void;
  onExportArena: (arena: EvaluationArena) => void;
}

// benchmark_error is an infra/harness failure, not a verdict on the Agent or the task, so it gets
// its own "needs an operator" tone rather than being lumped in with a deterministic task/Agent
// failure (task_failed/agent_failed).
const OUTCOME_TONE: Record<EvaluationOutcome, StatusTone> = {
  queued: "neutral",
  running: "running",
  succeeded: "success",
  task_failed: "danger",
  agent_failed: "danger",
  timed_out: "warning",
  stuck: "warning",
  cancelled: "neutral",
  benchmark_error: "attention",
};

// Ascending puts a passing result first and reads as "best first" — cancelled sits last since it
// is neither a pass nor a verdict on the Agent, not a rank between the two failure clusters.
const OUTCOME_RANK: Record<EvaluationOutcome, number> = {
  succeeded: 0,
  running: 1,
  queued: 2,
  timed_out: 3,
  stuck: 4,
  task_failed: 5,
  agent_failed: 6,
  benchmark_error: 7,
  cancelled: 8,
};

function findMetric(attempt: EvaluationAttempt, name: string): EvaluationMetric | undefined {
  return attempt.metrics.find((item) => item.name === name);
}

// Milliseconds are what the runtime reports and seconds are what a benchmark is read in; a
// six-digit `duration` in a table column next to a four-digit token count invites the wrong
// comparison.
function formatMetric(attempt: EvaluationAttempt, name: string): string {
  const value = findMetric(attempt, name);
  if (value?.value == null) return "—";
  return value.unit === "ms" ? `${(value.value / 1_000).toFixed(1)} s` : `${value.value} ${value.unit}`;
}

function metricValue(attempt: EvaluationAttempt, name: string): number | null {
  return findMetric(attempt, name)?.value ?? null;
}

function checkRatio(attempt: EvaluationAttempt): number {
  // No checks ran at all carries less information than any real ratio, including 0/1 — sorts last.
  return attempt.checks.length === 0 ? -1 : attempt.checks.filter((item) => item.passed).length / attempt.checks.length;
}

function nullsLast(a: number | null, b: number | null): number {
  if (a == null && b == null) return 0;
  if (a == null) return 1;
  if (b == null) return -1;
  return a - b;
}

const COLUMN_COMPARATORS: Record<string, (a: EvaluationResultRow, b: EvaluationResultRow) => number> = {
  agent: (a, b) => a.attempt.agent.agentId.localeCompare(b.attempt.agent.agentId),
  outcome: (a, b) => OUTCOME_RANK[a.attempt.outcome] - OUTCOME_RANK[b.attempt.outcome],
  tests: (a, b) => checkRatio(a.attempt) - checkRatio(b.attempt),
  tokens: (a, b) => nullsLast(metricValue(a.attempt, "input_tokens"), metricValue(b.attempt, "input_tokens")),
  time: (a, b) => nullsLast(metricValue(a.attempt, "duration"), metricValue(b.attempt, "duration")),
};

// Undefined (no header clicked yet) must stay a no-op: the desktop WDIO suite asserts the table's
// default order matches the ranked arena's own attempt order verbatim (agent-evaluation e2e,
// "the table rendered the attempts in a different order than the ranked arena reports").
function sortRows(rows: readonly EvaluationResultRow[], sort: DataTableSort | undefined): EvaluationResultRow[] {
  if (!sort) return [...rows];
  const compare = COLUMN_COMPARATORS[sort.columnId];
  if (!compare) return [...rows];
  const direction = sort.direction === "asc" ? 1 : -1;
  return [...rows].sort((a, b) => compare(a, b) * direction);
}

/**
 * 18.6/18.7: the shared `DataTable` migration for Evaluation's result list. Service-side pagination
 * and row selection are deliberately not wired — there is no `offset`/`limit` on
 * `EvaluationService.listEvaluationArenas()` today (the Tauri command hard-codes `api.list(0, 100)`
 * server-side with nothing exposed to the frontend to change it), and no bulk row action exists yet
 * for a selection checkbox column to drive. Every current column (Agent, Outcome, Tests, Tokens,
 * Time) is core at-a-glance information, not a raw id or a low-frequency fingerprint, so none of
 * them are hidden behind `visibleColumnIds` — the primitive has no way to mark a column
 * non-hideable, and blanket-enabling the menu would let a reader hide Agent/Outcome identity
 * columns along with the rest.
 */
export function EvaluationResultsTable({ rows, filter, onFilterChange, onSelectAttempt, onExportArena }: EvaluationResultsTableProps) {
  const { t } = useTranslation();
  const [sort, setSort] = useState<DataTableSort | undefined>(undefined);
  const sorted = useMemo(() => sortRows(rows, sort), [rows, sort]);

  const columns = useMemo<DataTableColumn<EvaluationResultRow>[]>(() => [
    { id: "agent", header: t("evaluation.agent"), sortable: true, cell: (row) => <span className="font-medium">{row.attempt.agent.agentId}</span> },
    { id: "outcome", header: t("evaluation.outcome"), sortable: true, cell: (row) => <StatusBadge label={t(`evaluation.outcome.${row.attempt.outcome}`)} tone={OUTCOME_TONE[row.attempt.outcome]} /> },
    { id: "tests", header: t("evaluation.tests"), sortable: true, align: "end", cell: (row) => `${row.attempt.checks.filter((item) => item.passed).length}/${row.attempt.checks.length}` },
    { id: "tokens", header: t("evaluation.tokens"), sortable: true, align: "end", cell: (row) => formatMetric(row.attempt, "input_tokens") },
    { id: "time", header: t("evaluation.time"), sortable: true, align: "end", cell: (row) => formatMetric(row.attempt, "duration") },
    {
      id: "export",
      header: t("evaluation.export"),
      align: "end",
      cell: (row) => (
        <button
          aria-label={t("evaluation.export")}
          className="rounded p-1 hover:bg-muted"
          data-testid="evaluation-export"
          onClick={(event) => { event.stopPropagation(); onExportArena(row.arena); }}
          type="button"
        >
          <Download aria-hidden="true" className="h-4 w-4" />
        </button>
      ),
    },
  ], [t, onExportArena]);

  const getRowMeta = useCallback((row: EvaluationResultRow): DataTableRowMeta => ({
    attributes: { "data-testid": "evaluation-row", "data-attempt-id": row.attempt.id, "data-outcome": row.attempt.outcome },
    onClick: () => onSelectAttempt(row.attempt.id),
  }), [onSelectAttempt]);

  return (
    <section className="min-w-0 border-r border-border p-3">
      <div className="mb-2 flex items-center justify-between gap-2">
        <h2 className="text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.results")}</h2>
        <input
          aria-label={t("evaluation.filter")}
          className="h-8 w-44 rounded-md border border-input bg-background px-2 text-xs"
          data-testid="evaluation-filter"
          onChange={(event) => onFilterChange(event.target.value)}
          placeholder={t("evaluation.filter")}
          value={filter}
        />
      </div>
      <DataTable
        ariaLabel={t("evaluation.results")}
        columns={columns}
        emptyState={<p className="p-6 text-center text-sm text-muted-foreground">{t("evaluation.empty")}</p>}
        getRowMeta={getRowMeta}
        onSortChange={setSort}
        rowKey={(row) => row.attempt.id}
        rows={sorted}
        sort={sort}
      />
    </section>
  );
}
