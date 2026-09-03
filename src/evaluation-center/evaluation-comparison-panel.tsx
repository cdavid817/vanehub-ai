import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import type { EvaluationAttempt } from "../types/evaluation";
import { compareEvaluationAttempts } from "./evaluation-comparison";
import { attemptOptionLabel, buildComparisonMatrix, MAX_ADDITIONAL_CANDIDATES } from "./evaluation-comparison-matrix";
import { EvaluationComparisonMatrixView } from "./evaluation-comparison-matrix-view";
import { EvaluationComparisonResultView } from "./evaluation-comparison-result";

export interface EvaluationComparisonPanelProps {
  attempts: EvaluationAttempt[];
}

const selectClass = "h-9 w-full rounded-md border border-input bg-background px-2 text-sm";

/**
 * 18.8: the two pickers below never restrict either dropdown to only "eligible" options against
 * the other's current pick -- the same judgment call `evaluation-agent-selector.tsx` already made
 * for incompatible Agents (18.5): hiding or disabling options would silently look like fewer
 * results exist, when the honest signal is *why* a pair is not comparable
 * (`EvaluationComparisonResultView`'s own not-comparable state), not that the pair can never be
 * picked. Both dropdowns list every currently loaded attempt (`arenas.flatMap`, not the results
 * table's own text-filtered subset) -- comparison is independent of what the table happens to be
 * filtered to.
 *
 * 18.11: baseline + candidate stays the original 2-attempt flow, completely unchanged in shape and
 * behavior (`result`/`EvaluationComparisonResultView` below). On top of it, once a third attempt
 * exists, a reader can additionally check up to `MAX_ADDITIONAL_CANDIDATES` more attempts as extra
 * candidates -- 2 to 4 attempts total -- which switches the rendered output from the single-pair
 * view to `EvaluationComparisonMatrixView`'s aligned rows/columns instead (never both at once, so
 * the same information is never shown twice). `effectiveAdditional` re-derives from `attempts`
 * every render rather than trusting `additionalCandidateIds` directly, so a stale id that now
 * coincides with a freshly re-picked baseline/candidate silently drops out on its own instead of
 * needing a reconciliation effect.
 */
export function EvaluationComparisonPanel({ attempts }: EvaluationComparisonPanelProps) {
  const { t } = useTranslation();
  const [baselineId, setBaselineId] = useState("");
  const [candidateId, setCandidateId] = useState("");
  const [additionalCandidateIds, setAdditionalCandidateIds] = useState<string[]>([]);
  const baseline = useMemo(() => attempts.find((attempt) => attempt.id === baselineId) ?? null, [attempts, baselineId]);
  const candidate = useMemo(() => attempts.find((attempt) => attempt.id === candidateId) ?? null, [attempts, candidateId]);
  const result = baseline && candidate ? compareEvaluationAttempts(baseline, candidate) : null;

  const additionalOptions = useMemo(
    () => attempts.filter((attempt) => attempt.id !== baselineId && attempt.id !== candidateId),
    [attempts, baselineId, candidateId],
  );
  const effectiveAdditional = useMemo(
    () => additionalOptions.filter((attempt) => additionalCandidateIds.includes(attempt.id)).slice(0, MAX_ADDITIONAL_CANDIDATES),
    [additionalOptions, additionalCandidateIds],
  );
  const atAdditionalCapacity = effectiveAdditional.length >= MAX_ADDITIONAL_CANDIDATES;
  const matrix = baseline && candidate && effectiveAdditional.length > 0
    ? buildComparisonMatrix(baseline, [candidate, ...effectiveAdditional])
    : null;

  function toggleAdditional(id: string) {
    setAdditionalCandidateIds((current) => (
      current.includes(id) ? current.filter((item) => item !== id) : atAdditionalCapacity ? current : [...current, id]
    ));
  }

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

          {baseline && candidate && additionalOptions.length > 0 ? (
            <div className="grid gap-1.5">
              <p className="text-xs font-medium text-muted-foreground">{t("evaluation.comparison.additionalCandidatesLabel")}</p>
              <ul className="grid max-h-40 gap-1 overflow-y-auto" data-testid="evaluation-comparison-additional-list">
                {additionalOptions.map((attempt) => {
                  const checked = effectiveAdditional.some((item) => item.id === attempt.id);
                  return (
                    <li key={attempt.id}>
                      <label className="flex items-center gap-2 text-xs">
                        <input
                          checked={checked}
                          data-testid={`evaluation-comparison-additional-${attempt.id}`}
                          disabled={!checked && atAdditionalCapacity}
                          onChange={() => toggleAdditional(attempt.id)}
                          type="checkbox"
                        />
                        {attemptOptionLabel(attempt, t)}
                      </label>
                    </li>
                  );
                })}
              </ul>
              {atAdditionalCapacity ? <p className="text-xs text-muted-foreground">{t("evaluation.comparison.additionalCandidatesMax")}</p> : null}
            </div>
          ) : null}

          {matrix ? (
            // Data-dependent height (metric-row count varies per attempt); scroll-bound for the same
            // reason as the single-pair view below -- this section is the last, non-flexed child of
            // EvaluationCenter's own `overflow-hidden flex-col` root.
            <div className="max-h-96 overflow-y-auto"><EvaluationComparisonMatrixView matrix={matrix} /></div>
          ) : result ? (
            <div className="max-h-96 overflow-y-auto">
              <EvaluationComparisonResultView result={result} />
            </div>
          ) : <p className="text-sm text-muted-foreground">{t("evaluation.comparison.selectBoth")}</p>}
        </div>
      )}
    </section>
  );
}
