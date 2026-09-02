import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
import { ExecutionSpanRow } from "./execution-span-row";
import { useExecutionTimeline } from "./use-execution-timeline";

/**
 * The Timeline facet: every span from the joined execution-observability run, unfiltered — the
 * "shows everything" sibling of the Tools/Files facets below it, which each filter to one
 * `ExecutionSpanKind` (see `tools-facet.tsx` for which kind, and why `"mcp"` is not folded in).
 */
export function TimelineFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const state = useExecutionTimeline(run);

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-timeline-facet">
      {state.status === "loading" ? <p className="text-xs text-muted-foreground">{t("missionControl.timeline.loading")}</p> : null}
      {state.status === "error" ? <p className="text-xs text-destructive">{t("missionControl.timeline.error")}</p> : null}
      {state.status === "empty" ? <p className="text-xs text-muted-foreground">{t("missionControl.timeline.empty")}</p> : null}
      {state.status === "ready" ? (
        state.timeline.spans.length ? (
          <ul className="space-y-1" data-testid="mission-control-timeline-spans">
            {state.timeline.spans.map((span) => <ExecutionSpanRow key={span.spanId} showKind span={span} />)}
          </ul>
        ) : <p className="text-xs text-muted-foreground">{t("missionControl.timeline.noSpans")}</p>
      ) : null}
    </div>
  );
}
