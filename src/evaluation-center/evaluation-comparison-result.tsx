import { CircleHelp, Minus, TrendingDown, TrendingUp, TriangleAlert } from "lucide-react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import { OUTCOME_TONE } from "./evaluation-arena-summary";
import { StatusBadge, type StatusTone } from "../ui/status/StatusBadge";
import type { DeltaVerdict, EvaluationComparisonResult as ComparisonResult, MetricDelta, UncomparedMetric } from "./evaluation-comparison";

export interface EvaluationComparisonResultViewProps {
  result: ComparisonResult;
}

// 18.10: regressions/improvements are never color-only -- every verdict pairs a distinct icon with
// a required visible label (`StatusBadge`'s own contract), the same primitive already used for
// every other status signal on this page (18.3's arena state, 18.7's outcome column).
const VERDICT_TONE: Record<DeltaVerdict, StatusTone> = { improved: "success", regressed: "danger", unchanged: "neutral", notRankable: "information" };
const VERDICT_ICON = { improved: TrendingUp, regressed: TrendingDown, unchanged: Minus, notRankable: CircleHelp };

/** Exported: `evaluation-comparison-matrix-view.tsx` (18.11) reuses this for every candidate
 *  column's own verdict rather than reimplementing the icon/tone/label mapping a second time. */
export function VerdictBadge({ verdict }: { verdict: DeltaVerdict }) {
  const { t } = useTranslation();
  return <StatusBadge icon={VERDICT_ICON[verdict]} label={t(`evaluation.comparison.verdict.${verdict}`)} tone={VERDICT_TONE[verdict]} />;
}

/** Signed, fixed-precision formatting shared by every numeric delta shown below -- e.g. "+12.5",
 *  "-3", "0" (no sign for exactly zero: there is nothing to signal a direction for). Exported:
 *  `evaluation-comparison-matrix-view.tsx` (18.11) reuses this for the same metric-delta formatting. */
export function formatSigned(value: number, digits: number): string {
  const rounded = Number(value.toFixed(digits));
  return rounded > 0 ? `+${rounded}` : String(rounded);
}

function MetricRow({ metric }: { metric: MetricDelta }) {
  const percent = metric.percentChange == null ? "" : ` (${formatSigned(metric.percentChange, 1)}%)`;
  return (
    <li className="text-xs" data-testid="evaluation-comparison-metric">
      <span className="font-medium">{metric.name}</span>: {metric.baselineValue} {"→"} {metric.candidateValue} {metric.unit}
      {" · Δ "}
      {formatSigned(metric.delta, 2)}
      {percent}
    </li>
  );
}

function UncomparedMetricRow({ item, t }: { item: UncomparedMetric; t: TFunction }) {
  return (
    <li className="text-xs text-muted-foreground" data-testid="evaluation-comparison-uncompared-metric">
      {item.name}: {t(`evaluation.comparison.uncomparedReason.${item.reason}`)}
    </li>
  );
}

function IneligibleView({ reason, t }: { reason: string; t: TFunction }) {
  return (
    <div className="flex items-start gap-2 rounded-md border border-border bg-muted/30 p-3 text-xs" data-testid="evaluation-comparison-ineligible">
      <TriangleAlert aria-hidden="true" className="mt-0.5 h-3.5 w-3.5 shrink-0 text-[hsl(var(--warning))]" />
      <span>
        <span className="block font-medium">{t("evaluation.comparison.notComparable")}</span>
        <span className="text-muted-foreground">{t(`evaluation.comparison.reason.${reason}`)}</span>
      </span>
    </div>
  );
}

/**
 * Renders one `EvaluationComparisonResult` (18.9's deltas, 18.10's icon/text/reason marking).
 * Deliberately built from `<div>`/`<dl>`/`<ul>`, never a `<table>` -- `evaluation-center.test.tsx`
 * asserts a whole-document `tbody tr` count that belongs to `EvaluationResultsTable` alone
 * (`evaluation-arena-list.tsx`'s own comment documents the same hazard); a second `<tbody>` here
 * would silently inflate that count.
 */
export function EvaluationComparisonResultView({ result }: EvaluationComparisonResultViewProps) {
  const { t } = useTranslation();

  if (!result.eligible) return <IneligibleView reason={result.reason} t={t} />;

  const { outcomeTier, metrics, uncomparedMetrics, reliability, evidence, sameAgentConfiguration } = result;
  const baselineOutcomeLabel = t(`evaluation.outcome.${outcomeTier.baselineOutcome}`);
  const candidateOutcomeLabel = t(`evaluation.outcome.${outcomeTier.candidateOutcome}`);

  return (
    <div className="grid gap-3" data-testid="evaluation-comparison-result">
      <section>
        <h3 className="text-xs font-semibold">{t("evaluation.comparison.outcomeTier")}</h3>
        <div className="mt-1 flex flex-wrap items-center gap-2">
          <StatusBadge label={baselineOutcomeLabel} tone={OUTCOME_TONE[outcomeTier.baselineOutcome]} />
          <span aria-hidden="true">{"→"}</span>
          <StatusBadge label={candidateOutcomeLabel} tone={OUTCOME_TONE[outcomeTier.candidateOutcome]} />
          <VerdictBadge verdict={outcomeTier.verdict} />
        </div>
        <p className="mt-1 text-xs text-muted-foreground">
          {t("evaluation.comparison.outcomeTierReason", { baseline: baselineOutcomeLabel, candidate: candidateOutcomeLabel })}
        </p>
      </section>

      <section>
        <h3 className="text-xs font-semibold">{t("evaluation.comparison.metricDeltas")}</h3>
        {metrics.length === 0
          ? <p className="mt-1 text-xs text-muted-foreground">{t("evaluation.unavailable")}</p>
          : <ul className="mt-1 flex flex-col gap-1">{metrics.map((metric) => <MetricRow key={metric.name} metric={metric} />)}</ul>}
        {uncomparedMetrics.length > 0 ? (
          <div className="mt-2">
            <h4 className="text-[0.6875rem] font-semibold uppercase text-muted-foreground">{t("evaluation.comparison.uncomparedMetrics")}</h4>
            <ul className="mt-1 flex flex-col gap-1">{uncomparedMetrics.map((item) => <UncomparedMetricRow item={item} key={item.name} t={t} />)}</ul>
          </div>
        ) : null}
      </section>

      <section data-testid="evaluation-comparison-reliability">
        <h3 className="text-xs font-semibold">{t("evaluation.comparison.reliability")}</h3>
        {reliability.available ? (
          <div className="mt-1 flex flex-wrap items-center gap-2 text-xs">
            <span>{Math.round(reliability.baselineRatio * 100)}% {"→"} {Math.round(reliability.candidateRatio * 100)}%</span>
            <VerdictBadge verdict={reliability.verdict} />
          </div>
        ) : <p className="mt-1 text-xs text-muted-foreground">{t("evaluation.comparison.reliabilityUnavailable")}</p>}
      </section>

      <section>
        <h3 className="text-xs font-semibold">{t("evaluation.comparison.evidence")}</h3>
        <dl className="mt-1 grid grid-cols-[minmax(0,auto)_minmax(0,1fr)] gap-x-3 gap-y-1 text-xs">
          <div className="contents">
            <dt className="text-muted-foreground">{t("evaluation.comparison.checksCount")}</dt>
            <dd>{evidence.baselineChecksCount} {"→"} {evidence.candidateChecksCount} ({formatSigned(evidence.checksCountDelta, 0)})</dd>
          </div>
          <div className="contents">
            <dt className="text-muted-foreground">{t("evaluation.comparison.artifactsCount")}</dt>
            <dd>{evidence.baselineArtifactsCount} {"→"} {evidence.candidateArtifactsCount} ({formatSigned(evidence.artifactsCountDelta, 0)})</dd>
          </div>
        </dl>
      </section>

      <p className="text-xs text-muted-foreground" data-testid="evaluation-comparison-configuration">
        {t(sameAgentConfiguration ? "evaluation.comparison.sameConfiguration" : "evaluation.comparison.differentConfiguration")}
      </p>
    </div>
  );
}
