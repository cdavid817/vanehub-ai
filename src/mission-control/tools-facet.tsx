import { useTranslation } from "react-i18next";
import type { MissionControlRunSummary } from "../types/mission-control";
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
 */
export function ToolsFacet({ run }: { run: MissionControlRunSummary }) {
  const { t } = useTranslation();
  const state = useExecutionTimeline(run);
  const spans = state.status === "ready" ? state.timeline.spans.filter((span) => span.kind === "tool") : [];

  return (
    <div className="mt-4 space-y-2" data-testid="mission-control-tools-facet">
      {state.status === "loading" ? <p className="text-xs text-muted-foreground">{t("missionControl.tools.loading")}</p> : null}
      {state.status === "error" ? <p className="text-xs text-destructive">{t("missionControl.tools.error")}</p> : null}
      {state.status === "empty" ? <p className="text-xs text-muted-foreground">{t("missionControl.tools.empty")}</p> : null}
      {state.status === "ready" ? (
        spans.length ? (
          <ul className="space-y-1" data-testid="mission-control-tools-spans">
            {spans.map((span) => <ExecutionSpanRow key={span.spanId} showKind={false} span={span} />)}
          </ul>
        ) : <p className="text-xs text-muted-foreground">{t("missionControl.tools.noSpans")}</p>
      ) : null}
    </div>
  );
}
