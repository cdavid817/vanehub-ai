import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
import { ExecutionSpanRow } from "./execution-span-row";
import { useExecutionTimeline } from "./use-execution-timeline";

/**
 * The Files facet: spans of kind `"file"` only — the same one-kind-per-facet rule `tools-facet.tsx`
 * documents, applied to file operations instead of tool calls.
 */
export function FilesFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const state = useExecutionTimeline(run);
  const spans = state.status === "ready" ? state.timeline.spans.filter((span) => span.kind === "file") : [];

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-files-facet">
      {state.status === "loading" ? <p className="text-xs text-muted-foreground">{t("missionControl.files.loading")}</p> : null}
      {state.status === "error" ? <p className="text-xs text-destructive">{t("missionControl.files.error")}</p> : null}
      {state.status === "empty" ? <p className="text-xs text-muted-foreground">{t("missionControl.files.empty")}</p> : null}
      {state.status === "ready" ? (
        spans.length ? (
          <ul className="space-y-1" data-testid="mission-control-files-spans">
            {spans.map((span) => <ExecutionSpanRow key={span.spanId} showKind={false} span={span} />)}
          </ul>
        ) : <p className="text-xs text-muted-foreground">{t("missionControl.files.noSpans")}</p>
      ) : null}
    </div>
  );
}
