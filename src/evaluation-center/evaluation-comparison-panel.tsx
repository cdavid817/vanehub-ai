import { useMemo, useState } from "react";
import type { TFunction } from "i18next";
import { useTranslation } from "react-i18next";
import type { EvaluationAttempt } from "../types/evaluation";
import { compareEvaluationAttempts } from "./evaluation-comparison";
import { EvaluationComparisonResultView } from "./evaluation-comparison-result";

export interface EvaluationComparisonPanelProps {
  attempts: EvaluationAttempt[];
}

const selectClass = "h-9 w-full rounded-md border border-input bg-background px-2 text-sm";

function attemptOptionLabel(attempt: EvaluationAttempt, t: TFunction): string {
  return t("evaluation.comparison.attemptOption", {
    agent: attempt.agent.agentId,
    outcome: t(`evaluation.outcome.${attempt.outcome}`),
    task: attempt.taskId,
    version: attempt.taskVersion,
  });
}

/**
 * 18.8: the two pickers below never restrict either dropdown to only "eligible" options against
 * the other's current pick -- the same judgment call `evaluation-agent-selector.tsx` already made
 * for incompatible Agents (18.5): hiding or disabling options would silently look like fewer
 * results exist, when the honest signal is *why* a pair is not comparable
 * (`EvaluationComparisonResultView`'s own not-comparable state), not that the pair can never be
 * picked. Both dropdowns list every currently loaded attempt (`arenas.flatMap`, not the results
 * table's own text-filtered subset) -- comparison is independent of what the table happens to be
 * filtered to.
 */
export function EvaluationComparisonPanel({ attempts }: EvaluationComparisonPanelProps) {
  const { t } = useTranslation();
  const [baselineId, setBaselineId] = useState("");
  const [candidateId, setCandidateId] = useState("");
  const baseline = useMemo(() => attempts.find((attempt) => attempt.id === baselineId) ?? null, [attempts, baselineId]);
  const candidate = useMemo(() => attempts.find((attempt) => attempt.id === candidateId) ?? null, [attempts, candidateId]);
  const result = baseline && candidate ? compareEvaluationAttempts(baseline, candidate) : null;

  return (
    <section className="border-t border-border p-3" data-testid="evaluation-comparison">
      <h2 className="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">{t("evaluation.comparison.title")}</h2>
      {attempts.length < 2 ? (
        <p className="text-sm text-muted-foreground">{t("evaluation.comparison.needsTwoResults")}</p>
      ) : (
        <div className="grid gap-3">
          <p className="text-xs text-muted-foreground">{t("evaluation.comparison.description")}</p>
          <div className="grid gap-2 sm:grid-cols-2">
            <label className="grid gap-1 text-xs font-medium text-muted-foreground">
              {t("evaluation.comparison.baselineLabel")}
              <select
                aria-label={t("evaluation.comparison.baselineLabel")}
                className={selectClass}
                data-testid="evaluation-comparison-baseline"
                onChange={(event) => setBaselineId(event.target.value)}
                value={baselineId}
              >
                <option value="">{t("evaluation.comparison.choosePlaceholder")}</option>
                {attempts.map((attempt) => <option key={attempt.id} value={attempt.id}>{attemptOptionLabel(attempt, t)}</option>)}
              </select>
            </label>
            <label className="grid gap-1 text-xs font-medium text-muted-foreground">
              {t("evaluation.comparison.candidateLabel")}
              <select
                aria-label={t("evaluation.comparison.candidateLabel")}
                className={selectClass}
                data-testid="evaluation-comparison-candidate"
                onChange={(event) => setCandidateId(event.target.value)}
                value={candidateId}
              >
                <option value="">{t("evaluation.comparison.choosePlaceholder")}</option>
                {attempts.map((attempt) => <option key={attempt.id} value={attempt.id}>{attemptOptionLabel(attempt, t)}</option>)}
              </select>
            </label>
          </div>
          {result ? (
            // Capped and internally scrollable, mirroring evaluation-agent-selector.tsx's own
            // `max-h-72 overflow-y-auto` roster list -- the result breakdown's height is
            // data-dependent (metric count varies per attempt), and this section is the last,
            // non-flexed child of `EvaluationCenter`'s own `overflow-hidden flex-col` root
            // (`quality-destination.tsx` bounds it to `h-full`): left uncapped, a long breakdown
            // would grow past the page's fixed height and get silently clipped at the bottom
            // rather than scrolled to.
            <div className="max-h-96 overflow-y-auto">
              <EvaluationComparisonResultView result={result} />
            </div>
          ) : <p className="text-sm text-muted-foreground">{t("evaluation.comparison.selectBoth")}</p>}
        </div>
      )}
    </section>
  );
}
