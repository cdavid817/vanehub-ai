import type { ReactNode } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { EvaluationOutcome } from "../types/evaluation";
import { StatusBadge } from "../ui/status/StatusBadge";
import { OUTCOME_TONE } from "./evaluation-arena-summary";
import { formatSigned, VerdictBadge } from "./evaluation-comparison-result";
import { attemptOptionLabel, type ComparisonMatrix, type ComparisonMatrixColumn, type ComparisonMatrixMetricCell } from "./evaluation-comparison-matrix";
import { checkRatio } from "./evaluation-results-table";

export interface EvaluationComparisonMatrixViewProps {
  matrix: ComparisonMatrix;
}

function OutcomeCell({ outcome, t }: { outcome: EvaluationOutcome; t: TFunction }) {
  return <StatusBadge label={t(`evaluation.outcome.${outcome}`)} tone={OUTCOME_TONE[outcome]} />;
}

/** Screen-reader-discoverable, not just a visual dash -- the same real reason is also shown once,
 *  visibly, in that column's own header row, so this never repeats the full sentence down every row. */
function Dash({ t }: { t: TFunction }) {
  return (
    <span className="text-muted-foreground">
      <span aria-hidden="true">—</span>
      <span className="sr-only">{t("evaluation.comparison.notComparable")}</span>
    </span>
  );
}

/**
 * One row of the matrix: a label cell, the baseline's own reference value, then one cell per
 * candidate column (keyed by the column's real attempt id, never array index). `renderCell` closes
 * over the row's own per-column data, keeping this primitive itself data-agnostic and reusable for
 * every row kind below.
 */
function MatrixRow({ baselineCell, columns, label, renderCell }: {
  label: ReactNode;
  baselineCell: ReactNode;
  columns: ComparisonMatrixColumn[];
  renderCell: (column: ComparisonMatrixColumn, index: number) => ReactNode;
}) {
  return (
    <div className="flex items-start gap-3 border-b border-border/60 py-1.5 last:border-b-0" role="row">
      <div className="w-28 shrink-0 pt-0.5 text-xs font-medium text-muted-foreground" role="rowheader">{label}</div>
      <div className="min-w-0 flex-1 text-xs" role="cell">{baselineCell}</div>
      {columns.map((column, index) => (
        <div className="min-w-0 flex-1 text-xs" key={column.attempt.id} role="cell">{renderCell(column, index)}</div>
      ))}
    </div>
  );
}

function outcomeRowCell(column: ComparisonMatrixColumn, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  const { outcomeTier } = column.result;
  return <span className="flex flex-wrap items-center gap-1"><OutcomeCell outcome={outcomeTier.candidateOutcome} t={t} /><VerdictBadge verdict={outcomeTier.verdict} /></span>;
}

function configurationRowCell(column: ComparisonMatrixColumn, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  return <span>{t(column.result.sameAgentConfiguration ? "evaluation.comparison.sameConfiguration" : "evaluation.comparison.differentConfiguration")}</span>;
}

function reliabilityRowCell(column: ComparisonMatrixColumn, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  const { reliability } = column.result;
  if (!reliability.available) return <span className="text-muted-foreground">{t("evaluation.comparison.reliabilityUnavailable")}</span>;
  return <span className="flex flex-wrap items-center gap-1">{Math.round(reliability.candidateRatio * 100)}%<VerdictBadge verdict={reliability.verdict} /></span>;
}

function checksRowCell(column: ComparisonMatrixColumn, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  const { evidence } = column.result;
  return <span>{evidence.candidateChecksCount} ({formatSigned(evidence.checksCountDelta, 0)})</span>;
}

function artifactsRowCell(column: ComparisonMatrixColumn, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  const { evidence } = column.result;
  return <span>{evidence.candidateArtifactsCount} ({formatSigned(evidence.artifactsCountDelta, 0)})</span>;
}

function metricRowCell(column: ComparisonMatrixColumn, cell: ComparisonMatrixMetricCell, t: TFunction) {
  if (!column.result.eligible) return <Dash t={t} />;
  if (cell.kind === "compared") {
    const percent = cell.delta.percentChange == null ? "" : ` (${formatSigned(cell.delta.percentChange, 1)}%)`;
    return <span>{cell.delta.candidateValue} {cell.delta.unit} · Δ {formatSigned(cell.delta.delta, 2)}{percent}</span>;
  }
  if (cell.kind === "uncompared") return <span className="text-muted-foreground">{t(`evaluation.comparison.uncomparedReason.${cell.reason}`)}</span>;
  return <span className="text-muted-foreground">{t("evaluation.comparison.matrixMetricNotInColumn")}</span>;
}

function baselineMetricCell(matrix: ComparisonMatrix, name: string, t: TFunction) {
  const found = matrix.baseline.metrics.find((item) => item.name === name);
  if (!found || found.value == null || found.quality === "unavailable") return <span className="text-muted-foreground">{t("evaluation.unavailable")}</span>;
  return <span>{found.value} {found.unit}</span>;
}

/**
 * Renders a `ComparisonMatrix` (18.11) as aligned rows -- one per comparison dimension -- and
 * columns -- the baseline plus every candidate -- built entirely from `evaluation-comparison.ts`'s
 * existing, already-tested pairwise deltas (see `evaluation-comparison-matrix.ts`'s own doc
 * comment). Deliberately built from `role="table"/"row"/"rowheader"/"cell"` `<div>`s, never a real
 * `<table>`: `evaluation-center.test.tsx` asserts a whole-document `tbody tr` count that belongs to
 * `EvaluationResultsTable` alone (the same hazard `evaluation-comparison-result.tsx`'s own comment
 * already documents for the single-pair view) -- a second `<tbody>` here would silently inflate it.
 */
export function EvaluationComparisonMatrixView({ matrix }: EvaluationComparisonMatrixViewProps) {
  const { t } = useTranslation();
  const { baseline, columns, metricRows } = matrix;

  return (
    <div className="grid gap-0.5" data-testid="evaluation-comparison-matrix" role="table">
      <MatrixRow
        baselineCell={<span className="font-medium">{t("evaluation.comparison.baselineLabel")}: {attemptOptionLabel(baseline, t)}</span>}
        columns={columns}
        label={t("evaluation.comparison.matrixTitle")}
        renderCell={(column) => (
          <span className="font-medium">
            {attemptOptionLabel(column.attempt, t)}
            {!column.result.eligible ? (
              <span className="mt-0.5 block font-normal text-muted-foreground" data-testid="evaluation-comparison-matrix-ineligible">
                {t(`evaluation.comparison.reason.${column.result.reason}`)}
              </span>
            ) : null}
          </span>
        )}
      />
      <MatrixRow baselineCell={<OutcomeCell outcome={baseline.outcome} t={t} />} columns={columns} label={t("evaluation.comparison.outcomeTier")} renderCell={(column) => outcomeRowCell(column, t)} />
      <MatrixRow baselineCell={<span className="font-mono">{baseline.agent.configurationFingerprint}</span>} columns={columns} label={t("evaluation.comparison.matrixConfigurationRow")} renderCell={(column) => configurationRowCell(column, t)} />
      <MatrixRow
        baselineCell={checkRatio(baseline) < 0 ? <span className="text-muted-foreground">{t("evaluation.comparison.reliabilityUnavailable")}</span> : <span>{Math.round(checkRatio(baseline) * 100)}%</span>}
        columns={columns}
        label={t("evaluation.comparison.reliability")}
        renderCell={(column) => reliabilityRowCell(column, t)}
      />
      <MatrixRow baselineCell={<span>{baseline.checks.length}</span>} columns={columns} label={t("evaluation.comparison.checksCount")} renderCell={(column) => checksRowCell(column, t)} />
      <MatrixRow baselineCell={<span>{baseline.artifactIds.length}</span>} columns={columns} label={t("evaluation.comparison.artifactsCount")} renderCell={(column) => artifactsRowCell(column, t)} />
      {metricRows.map((row) => (
        <MatrixRow
          baselineCell={baselineMetricCell(matrix, row.name, t)}
          columns={columns}
          key={row.name}
          label={row.name}
          renderCell={(column, index) => metricRowCell(column, row.cells[index] ?? { kind: "notInColumn" }, t)}
        />
      ))}
    </div>
  );
}
