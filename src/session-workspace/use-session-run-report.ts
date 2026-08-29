import { keepPreviousData, useQuery } from "@tanstack/react-query";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import { isEvidenceUnavailableError } from "../services/native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  ReportGroupBy,
  SessionRunReport,
} from "../types/session-workspace-evidence";
import { evidenceQueryKeys } from "./evidence-query-keys";

/**
 * What the reader has narrowed the report to.
 *
 * Held apart from the report itself because the two change for different reasons: a scope changes
 * when somebody picks a control, and a report changes when the backend answers. Folding them into
 * one value would make a stale report look like a stale scope.
 */
export interface ReportScopeSelection {
  runIds: readonly EvidenceRunId[];
  seatIds: readonly EvidenceSeatId[];
  from?: string;
  to?: string;
  groupBy: ReportGroupBy;
}

export const WHOLE_SESSION_REPORT_SCOPE: ReportScopeSelection = {
  runIds: [],
  seatIds: [],
  groupBy: "run",
};

export type ReportState = "loading" | "ready" | "unavailable";

export interface SessionRunReportResult {
  report: SessionRunReport | undefined;
  state: ReportState;
  /**
   * True while a newer answer is in flight and the previous one is still on screen.
   *
   * Distinct from `loading`, which means there is nothing to show. A panel that treated a refresh
   * as loading would blank a report the reader is still reading every time they touch a control.
   */
  isRefreshing: boolean;
  /** The backend's stable refusal code, when it refused. Never a message. */
  reasonCode: string | null;
}

export function useSessionRunReport({
  isVisible = true,
  scope,
  service = defaultAgentService,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  scope: ReportScopeSelection;
  service?: SessionWorkspaceEvidenceService;
  sessionId: EvidenceSessionId | null;
}): SessionRunReportResult {
  const query = useQuery({
    enabled: sessionId !== null && isVisible,
    queryKey: evidenceQueryKeys.report(
      // Unreachable while `enabled` holds. The key still has to be built, and a placeholder that
      // collided with a real session's entry would serve one session's report to another.
      sessionId ?? ("" as EvidenceSessionId),
      scope.runIds,
      scope.seatIds,
      scope.from,
      scope.to,
      scope.groupBy,
    ),
    queryFn: () => {
      if (sessionId === null) throw new Error("A session-run report needs a session.");
      return service.getSessionRunReport({
        sessionId,
        runIds: [...scope.runIds],
        seatIds: [...scope.seatIds],
        from: scope.from,
        to: scope.to,
        groupBy: scope.groupBy,
      });
    },
    // The previous report stays on screen while a narrower one is fetched. Every control on this
    // panel changes the key, so without this each click empties the page and refills it.
    placeholderData: keepPreviousData,
  });

  const state: ReportState = query.data ? "ready" : query.isError ? "unavailable" : "loading";
  return {
    isRefreshing: state === "ready" && query.isFetching,
    reasonCode: refusalCode(query.error),
    report: query.data,
    state,
  };
}

/**
 * A refusal the panel can translate, or nothing.
 *
 * Anything that is not a typed evidence refusal is reported as a generic code rather than surfaced:
 * its text is untranslated and may name internals, and a panel showing it would be showing the
 * reader something no locale file has a string for.
 */
function refusalCode(error: unknown): string | null {
  if (error === null || error === undefined) return null;
  return isEvidenceUnavailableError(error) ? error.reasonCode : "evidence_unavailable";
}
