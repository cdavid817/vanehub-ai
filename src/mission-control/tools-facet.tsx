import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { ExecutionSpanRow } from "./execution-span-row";
import { useExecutionTimeline } from "./use-execution-timeline";

/**
 * The Tools facet: spans of kind `"tool"` only, from the same joined timeline the Timeline facet
 * shows in full.
 *
 * Deliberately excludes `"mcp"`-kind spans. `"tool"` and `"mcp"` are peer, independently-classified
 * values of `ExecutionSpanKind` (`types/execution-observability.ts`) — the native side chose to keep
 * them apart because a producer's own asserted kind is what that type exists to preserve rather than
 * re-infer (see that type's own doc comment). The backend also tracks their observation fidelity
 * separately: `ExecutionObservationCapability` carries `toolFidelity` and `mcpFidelity` as two
 * distinct dimensions, not one. Folding "mcp" into this facet would quietly erase a distinction the
 * native side went out of its way to keep, and this facet does not get to re-decide that behind a
 * filter. `files-facet.tsx` follows the identical one-kind-per-facet rule for `"file"`.
 *
 * 16.11: routes through the shared `AsyncBoundary`, same as `timeline-facet.tsx` — the kind filter
 * itself lives only in `isEmpty` and the `children` render callback, since the underlying
 * `AsyncViewState<ExecutionTimeline>` from `useExecutionTimeline` is unfiltered.
 */
export function ToolsFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const { reload, ...state } = useExecutionTimeline(run, t("missionControl.tools.empty"), t("missionControl.tools.error"));

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-tools-facet">
      <AsyncBoundary
        emptyState={{ title: t("missionControl.tools.noSpans") }}
        isEmpty={(timeline) => timeline.spans.every((span) => span.kind !== "tool")}
        onRetry={reload}
        state={state}
        unavailableState={{ title: t("missionControl.tools.empty") }}
      >
        {(timeline) => (
          <ul className="space-y-1" data-testid="mission-control-tools-spans">
            {timeline.spans.filter((span) => span.kind === "tool").map((span) => (
              <ExecutionSpanRow key={span.spanId} showKind={false} span={span} />
            ))}
          </ul>
        )}
      </AsyncBoundary>
    </div>
  );
}
