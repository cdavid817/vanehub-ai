import { useTranslation } from "react-i18next";
import type { SessionRunReport } from "../types/session-workspace-evidence";
import { reportSectionCoverage, type ReportSectionId } from "./report-evidence-links";
import { ReportEmptyRow, ReportMetric, ReportSection } from "./report-section";

type OpenEvidence = (section: ReportSectionId) => void;

/** A count, or an em dash when nothing measured it. Shared by every row below. */
function Figure({ value }: { value: number | undefined }) {
  const { i18n } = useTranslation();
  if (value === undefined) return <span className="text-muted-foreground">—</span>;
  return <span>{new Intl.NumberFormat(i18n.language).format(value)}</span>;
}

/**
 * One row per agent, including a delegated child.
 *
 * A child agent did its own work and failed or succeeded on its own; folding it into its parent
 * would leave a report unable to answer which agent the failures came from, which is usually the
 * question somebody opened the report to ask.
 */
export function ReportAgentsSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "agents")}
      onOpenEvidence={() => onOpenEvidence("agents")}
      title={t("sessionTabs.report.section.agents")}
    >
      {report.agents.length === 0 ? (
        <ReportEmptyRow message={t("sessionTabs.report.noAgents")} />
      ) : (
        <table className="w-full text-sm">
          <thead className="text-xs text-muted-foreground">
            <tr>
              <th className="py-1 text-left font-medium">{t("sessionTabs.report.agent")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.runs")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.failed")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.durationMs")}</th>
            </tr>
          </thead>
          <tbody>
            {report.agents.map((row, index) => (
              <tr className="border-t border-border" key={`${row.agentId ?? ""}:${row.seatId ?? ""}:${index}`}>
                <td className="truncate py-1 font-mono text-xs">
                  {/* A run recorded under no agent still happened; naming it keeps the rows adding
                      up to the overview's total. */}
                  {row.agentId ?? t("sessionTabs.report.unattributedAgent")}
                  {row.seatId ? <span className="text-muted-foreground"> · {row.seatId}</span> : null}
                </td>
                <td className="py-1 text-right"><Figure value={row.runCount} /></td>
                <td className="py-1 text-right"><Figure value={row.failedCount} /></td>
                <td className="py-1 text-right"><Figure value={row.durationMs} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </ReportSection>
  );
}

export function ReportToolsSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "tools")}
      onOpenEvidence={() => onOpenEvidence("tools")}
      title={t("sessionTabs.report.section.tools")}
    >
      {report.tools.length === 0 ? (
        <ReportEmptyRow message={t("sessionTabs.report.noTools")} />
      ) : (
        <table className="w-full text-sm">
          <thead className="text-xs text-muted-foreground">
            <tr>
              <th className="py-1 text-left font-medium">{t("sessionTabs.report.tool")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.invocations")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.failed")}</th>
              <th className="py-1 text-right font-medium">{t("sessionTabs.report.durationMs")}</th>
            </tr>
          </thead>
          <tbody>
            {report.tools.map((row) => (
              <tr className="border-t border-border" key={row.toolName}>
                <td className="truncate py-1 font-mono text-xs">{row.toolName}</td>
                <td className="py-1 text-right"><Figure value={row.invocations} /></td>
                <td className="py-1 text-right"><Figure value={row.failures} /></td>
                <td className="py-1 text-right"><Figure value={row.durationMs} /></td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </ReportSection>
  );
}

export function ReportCommandsSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { commands } = report;
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "commands")}
      onOpenEvidence={() => onOpenEvidence("commands")}
      title={t("sessionTabs.report.section.commands")}
    >
      <div className="grid gap-2 sm:grid-cols-4">
        <ReportMetric label={t("sessionTabs.report.commandsTotal")} value={commands.total} />
        <ReportMetric label={t("sessionTabs.report.failed")} value={commands.failed} />
        <ReportMetric label={t("sessionTabs.report.commandsRunning")} value={commands.running} />
        {/* Absent while anything is still running: the session is not over, and a total that
            excluded the open command would read as though it were. */}
        <ReportMetric label={t("sessionTabs.report.durationMs")} value={commands.durationMs} />
      </div>
    </ReportSection>
  );
}

/**
 * Failures under stable codes, never messages.
 *
 * A report is quoted, and a message quoted out of one is producer text in a document nobody
 * redacted. The code is also what makes the rows groupable, which is the only reason this is a
 * table rather than a list of incidents.
 */
export function ReportFailuresSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "failures")}
      onOpenEvidence={() => onOpenEvidence("failures")}
      title={t("sessionTabs.report.section.failures")}
    >
      {report.failures.rows.length === 0 ? (
        <ReportEmptyRow message={t("sessionTabs.report.noFailures")} />
      ) : (
        <ul className="grid gap-1">
          {report.failures.rows.map((row) => (
            <li
              className="flex items-center justify-between gap-3 rounded border border-border bg-background px-2 py-1 text-sm"
              key={row.reasonCode}
            >
              <span className="truncate font-mono text-xs">{row.reasonCode}</span>
              <strong><Figure value={row.count} /></strong>
            </li>
          ))}
        </ul>
      )}
    </ReportSection>
  );
}
