import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { ExecutionSpanRow } from "./execution-span-row";
import { useExecutionTimeline } from "./use-execution-timeline";

/**
 * The Timeline facet: every span from the joined execution-observability run, unfiltered — the
 * "shows everything" sibling of the Tools/Files facets below it, which each filter to one
 * `ExecutionSpanKind` (see `tools-facet.tsx` for which kind, and why `"mcp"` is not folded in).
 *
 * 16.11: the loading/unavailable/error/ready states all route through the shared `AsyncBoundary`
 * now, via `useExecutionTimeline`'s own `AsyncViewState<ExecutionTimeline>` — `reload` wires a real
 * retry affordance into the generic error state that did not exist before this pass.
 */
export function TimelineFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const { reload, ...state } = useExecutionTimeline(run, t("missionControl.timeline.empty"), t("missionControl.timeline.error"));

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-timeline-facet">
      <AsyncBoundary
        emptyState={{ title: t("missionControl.timeline.noSpans") }}
        isEmpty={(timeline) => timeline.spans.length === 0}
        onRetry={reload}
        state={state}
        unavailableState={{ title: t("missionControl.timeline.empty") }}
      >
        {(timeline) => (
          <ul className="space-y-1" data-testid="mission-control-timeline-spans">
            {timeline.spans.map((span) => <ExecutionSpanRow key={span.spanId} showKind span={span} />)}
          </ul>
        )}
      </AsyncBoundary>
    </div>
  );
}
