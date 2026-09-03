import type { ComponentType } from "react";
import { useTranslation } from "react-i18next";
import type { MissionControlFacet, MissionControlRunDetail, MissionControlRunSummary } from "../types/mission-control";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import type { AsyncViewState, DisplayableErrorKind } from "../ui/async/async-view-state";
import { EmptyState } from "../ui/empty-state/EmptyState";
import { FilesFacet } from "./files-facet";
import { LogsFacet } from "./logs-facet";
import { OverviewFacet } from "./overview-facet";
import { ReviewFacet } from "./review-facet";
import { TimelineFacet } from "./timeline-facet";
import { ToolsFacet } from "./tools-facet";
import { UsageFacet } from "./usage-facet";

/**
 * Facets with a real, built component as of this pass — 7 of 9. `verification`/`context` are
 * intentionally absent, not an oversight:
 *
 * - `verification`: real verification results are iteration-scoped (`LoopIteration`, keyed by
 *   `(loop_run_id, iteration_id)` — `loop_verification.rs`'s own `LoopVerificationRequest` requires
 *   both), while a Mission Control Run is the whole loop run aggregate. Even for a loop-owned Run
 *   (`ownerType === "loop_run"`, `ownerId` resolvable to the same `loop_run_id`), there is no single
 *   iteration to attribute a facet-level result to without a second, undocumented heuristic layered
 *   on top of the session-execution one Timeline/Usage already carry — and it would still be
 *   meaningless for every non-loop-owned Run. Genuinely blocked, not merely unbuilt.
 * - `context`: `ContextQualityAssessmentRecord.session_correlation` is unconditionally `None` at its
 *   only real construction site (`api_process_adapter/compaction.rs`), and `ContextQualityQueryService
 *   ::list` takes no session/run filter at all (`range_days`/`cursor`/`limit` only) — so even a
 *   client-side filter would mean scanning an unbounded, unrelated history to find nothing, which
 *   16.12's own bounded-query discipline rules out. Genuinely blocked, not merely unbuilt.
 *
 * Both were investigated by reading the actual Rust source directly, not assumed.
 */
const FACET_COMPONENTS: Partial<Record<MissionControlFacet, ComponentType<{ run: MissionControlRunSummary }>>> = {
  overview: OverviewFacet,
  usage: UsageFacet,
  timeline: TimelineFacet,
  tools: ToolsFacet,
  files: FilesFacet,
  logs: LogsFacet,
  review: ReviewFacet,
};

/**
 * Honest, and honestly distinct from the backend's own "Unavailable"/"Restricted" tone
 * (`missionControl.availability.*`, used just below): this is not the backend reporting missing
 * data for a Run, it is this client not having a detail view for the facet at all yet, regardless
 * of what any backend would say.
 */
function FacetNotBuilt() {
  const { t } = useTranslation();
  return (
    <EmptyState
      className="mt-4"
      description={t("missionControl.facetNotBuilt.description")}
      title={t("missionControl.facetNotBuilt.title")}
      variant="unsupported"
    />
  );
}

/**
 * 16.10/16.11: routes every one of the nine facets through the shared `AsyncBoundary`/
 * `AsyncViewState` vocabulary — no facet ever reaches the old universal `lazyDetail` placeholder
 * again, deleted along with this dispatcher's previous unconditional fallback.
 *
 * Two gates, in order:
 * 1. Does this facet have a real component at all (`FACET_COMPONENTS`)? If not, `FacetNotBuilt` —
 *    a client-side fact, not an async result, so it renders directly rather than through a
 *    fabricated `AsyncViewState` for an operation that does not exist.
 * 2. Has the backend actually marked this facet `"available"` (`detail.facets`)? Re-checked here
 *    rather than trusted from the caller — the tab strip already disables selecting an unavailable
 *    facet, but defaulting back to an honest state for anything the backend has not actually marked
 *    available keeps this component correct even if that ever stops holding. Not available routes
 *    through `AsyncBoundary` itself (a real `AsyncViewState` whose `error.kind` is literally the
 *    backend's own `"unavailable"`/`"restricted"` value), so the same primitive Timeline/Tools/
 *    Files/Usage/Logs/Review use internally for their own resolver-level gaps also renders the
 *    dispatcher-level one — one visual vocabulary for "no evidence," wherever the gap is found.
 */
export function MissionControlFacetPanel({ detail, facet }: { detail: MissionControlRunDetail; facet: MissionControlFacet }) {
  const { t } = useTranslation();
  const Component = FACET_COMPONENTS[facet];
  if (!Component) return <FacetNotBuilt />;

  const backendState = detail.facets.find((item) => item.facet === facet)?.state ?? "unavailable";
  if (backendState !== "available") {
    const kind: DisplayableErrorKind = backendState;
    const label = t(`missionControl.availability.${backendState}`);
    const gated: AsyncViewState<never> = {
      initialLoading: false, refreshing: false, stale: false,
      error: { kind, message: label, retryable: false },
    };
    return (
      <AsyncBoundary
        className="mt-4"
        restrictedState={{ title: label }}
        state={gated}
        unavailableState={{ title: label }}
      >
        {() => null}
      </AsyncBoundary>
    );
  }

  return <Component run={detail.run} />;
}
