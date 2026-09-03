import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
import { AsyncBoundary } from "../ui/async/AsyncBoundary";
import { ExecutionSpanRow } from "./execution-span-row";
import { useExecutionTimeline } from "./use-execution-timeline";

/**
 * The Files facet: spans of kind `"file"` only — the same one-kind-per-facet rule `tools-facet.tsx`
 * documents, applied to file operations instead of tool calls.
 *
 * 16.11: routes through the shared `AsyncBoundary`, same as `timeline-facet.tsx`/`tools-facet.tsx`.
 */
export function FilesFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const { reload, ...state } = useExecutionTimeline(run, t("missionControl.files.empty"), t("missionControl.files.error"));

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-files-facet">
      <AsyncBoundary
        emptyState={{ title: t("missionControl.files.noSpans") }}
        isEmpty={(timeline) => timeline.spans.every((span) => span.kind !== "file")}
        onRetry={reload}
        state={state}
        unavailableState={{ title: t("missionControl.files.empty") }}
      >
        {(timeline) => (
          <ul className="space-y-1" data-testid="mission-control-files-spans">
            {timeline.spans.filter((span) => span.kind === "file").map((span) => (
              <ExecutionSpanRow key={span.spanId} showKind={false} span={span} />
            ))}
          </ul>
        )}
      </AsyncBoundary>
    </div>
  );
}
