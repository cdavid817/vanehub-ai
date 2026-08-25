import { useCallback, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { agentService as defaultAgentService } from "../services/runtime-agent-client";
import type { SessionWorkspaceEvidenceService } from "../services/session-workspace-evidence-service";
import type {
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
  ReportGroupBy,
} from "../types/session-workspace-evidence";
import { reportSectionTarget, type ReportSectionId } from "./report-evidence-links";
import {
  ReportChangesSection,
  ReportLatencySection,
  ReportOverviewSection,
  ReportUsageSection,
  ReportVerificationSection,
} from "./report-figures";
import {
  ReportScopeControls,
  reportRangeStart,
  type ReportExportState,
  type ReportRangeKey,
} from "./report-scope-controls";
import {
  ReportAgentsSection,
  ReportCommandsSection,
  ReportFailuresSection,
  ReportToolsSection,
} from "./report-tables";
import { useSessionRunReport, type ReportScopeSelection } from "./use-session-run-report";
import { useWorkspaceEvidenceScope } from "./workspace-evidence-scope";
import { WorkspaceState } from "./workspace-state";

/**
 * The session report, read from the backend rather than aggregated from mounted messages.
 *
 * The old panel summed whatever `ChatMessage[]` happened to be mounted, which made the report a
 * function of scrolling: paging older messages in changed every figure on the page, and a session
 * whose history had been trimmed reported a smaller session. Nothing about that was visible — the
 * numbers were confident either way. The backend counts what actually happened, over a scope the
 * reader chooses, and says per section how much of it it could see.
 */
export function ReportTab({
  isVisible = true,
  service,
  sessionId,
}: {
  /** False while the panel stays mounted behind another tab. */
  isVisible?: boolean;
  service?: SessionWorkspaceEvidenceService;
  sessionId: string | null;
}) {
  const { t } = useTranslation();
  const navigation = useWorkspaceEvidenceScope();
  const [groupBy, setGroupBy] = useState<ReportGroupBy>("run");
  const [range, setRange] = useState<ReportRangeKey>("all");
  const [exportState, setExportState] = useState<ReportExportState>("idle");

  // The run and seat come from the workspace scope rather than from a picker of this panel's own.
  // A reader arrives here from a trace or a record, and a second control that disagreed with the
  // one they used to get here would be two answers to the question of what they are looking at.
  const scope = useMemo<ReportScopeSelection>(() => {
    const { runId, seatId } = navigation.correlation;
    return {
      from: reportRangeStart(range, new Date()),
      groupBy,
      runIds: runId ? ([runId] as EvidenceRunId[]) : [],
      seatIds: seatId ? ([seatId] as EvidenceSeatId[]) : [],
    };
  }, [groupBy, navigation.correlation, range]);

  const evidence = service ?? defaultAgentService;
  const { isRefreshing, reasonCode, report, state } = useSessionRunReport({
    isVisible,
    scope,
    service: evidence,
    sessionId: sessionId as EvidenceSessionId | null,
  });

  // Deliberately not a mutation with cached state: an export is an action whose result is a
  // sentence, and caching it would leave last week's "exported" beside today's report.
  const onExport = useCallback(async () => {
    if (sessionId === null) return;
    setExportState("pending");
    try {
      const result = await evidence.exportSessionRunReport({
        sessionId: sessionId as EvidenceSessionId,
        runIds: [...scope.runIds],
        seatIds: [...scope.seatIds],
        from: scope.from,
        to: scope.to,
        groupBy: scope.groupBy,
      });
      setExportState(result.status);
    } catch {
      // The reason is not shown: an export failure reaches here as an untyped rejection, and its
      // text is untranslated. What a reader needs is that no file was written.
      setExportState("failed");
    }
  }, [evidence, scope, sessionId]);

  if (!sessionId) return <WorkspaceState kind="unavailable" />;

  return (
    <div className="flex h-full min-h-0 flex-col gap-3 overflow-hidden">
      <ReportScopeControls
        exportState={exportState}
        isRefreshing={isRefreshing}
        onClearCorrelation={() => navigation.clearScope(["runId", "seatId"])}
        onExport={() => void onExport()}
        onGroupByChange={setGroupBy}
        onRangeChange={setRange}
        range={range}
        scope={scope}
      />
      {report ? (
        <ReportSections
          onOpenEvidence={(section) => navigation.navigate(reportSectionTarget(report, section))}
          report={report}
        />
      ) : state === "loading" ? (
        <WorkspaceState kind="loading" message={t("sessionTabs.report.loading")} />
      ) : (
        <WorkspaceState
          kind="unavailable"
          // The refusal code, not a message: the backend answers in stable codes and the locale
          // file owns the sentence a reader sees.
          message={t(`evidence.reason.${reasonCode ?? "evidence_unavailable"}`, {
            defaultValue: t("evidence.reason.evidence_unavailable"),
          })}
        />
      )}
    </div>
  );
}

function ReportSections({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: (section: ReportSectionId) => void;
  report: Parameters<typeof reportSectionTarget>[0];
}) {
  return (
    <div className="grid gap-3 overflow-y-auto pr-1">
      <ReportOverviewSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportUsageSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportLatencySection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportAgentsSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportToolsSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportCommandsSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportChangesSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportVerificationSection onOpenEvidence={onOpenEvidence} report={report} />
      <ReportFailuresSection onOpenEvidence={onOpenEvidence} report={report} />
    </div>
  );
}
