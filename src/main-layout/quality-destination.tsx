import { LazyFeature, type LazyFeatureLoader } from "../components/lazy-feature";

const loadEvaluationCenter: LazyFeatureLoader<Record<string, never>> = () => import("../evaluation-center/evaluation-center")
  .then((module) => ({ default: module.EvaluationCenter }));

/**
 * Only one section (`evaluations`) exists today, so there is no secondary navigation to render —
 * unlike Runs/Plan. `EvaluationCenter` is zero-prop and self-contained (confirmed by reading it
 * directly); `experimentId`/`comparisonIds` on `QualitySection` are not consumed here for the same
 * reason as Plan's `workItemId`/`goalId` — no injectable initial-selection prop exists yet, and
 * the component's own "selected" concept (a run attempt) does not map cleanly onto "experiment"
 * without a real design decision this shell should not make on its own.
 */
export function QualityDestination() {
  return (
    <div className="h-full min-h-0 min-w-0 flex-1 p-2">
      <LazyFeature className="h-full min-h-0" componentProps={{}} loader={loadEvaluationCenter} />
    </div>
  );
}
