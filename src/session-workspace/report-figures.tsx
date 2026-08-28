import { useTranslation } from "react-i18next";
import type { SessionRunReport } from "../types/session-workspace-evidence";
import { reportSectionCoverage, type ReportSectionId } from "./report-evidence-links";
import { ReportMetric, ReportSection } from "./report-section";

type OpenEvidence = (section: ReportSectionId) => void;

export function ReportOverviewSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { overview } = report;
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "overview")}
      onOpenEvidence={() => onOpenEvidence("overview")}
      title={t("sessionTabs.report.section.overview")}
    >
      <div className="grid gap-2 sm:grid-cols-3 lg:grid-cols-6">
        <ReportMetric label={t("sessionTabs.report.runs")} value={overview.runCount} />
        <ReportMetric label={t("sessionTabs.report.succeeded")} value={overview.succeeded} />
        <ReportMetric label={t("sessionTabs.report.failed")} value={overview.failed} />
        <ReportMetric label={t("sessionTabs.report.cancelled")} value={overview.cancelled} />
        {/* A retry is not a run. Counting it as one would report more work than happened. */}
        <ReportMetric label={t("sessionTabs.report.retries")} value={overview.retries} />
        <ReportMetric label={t("sessionTabs.report.durationMs")} value={overview.durationMs} />
      </div>
    </ReportSection>
  );
}

/**
 * Three qualities, three rows, never a sum.
 *
 * Adding a reported figure to a derived one produces a number in no unit at all, and adding either
 * to an estimate turns the estimate into a measurement. The layout is what stops a reader doing it
 * for themselves: the estimate is counted in characters and labelled as such.
 */
export function ReportUsageSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { usage } = report;
  return (
    <ReportSection
      // The section map, not `usage.coverage`. The wire contract carries both and the backend keeps
      // them equal; reading one of them everywhere is what stops a future divergence from showing
      // up as one section disagreeing with the eight beside it.
      coverage={reportSectionCoverage(report, "usage")}
      onOpenEvidence={() => onOpenEvidence("usage")}
      title={t("sessionTabs.report.section.usage")}
    >
      <div className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
        <ReportMetric label={t("sessionTabs.report.reportedInputTokens")} value={usage.reportedInputTokens} />
        <ReportMetric label={t("sessionTabs.report.reportedOutputTokens")} value={usage.reportedOutputTokens} />
        <ReportMetric label={t("sessionTabs.report.reportedDerivedTokens")} value={usage.reportedDerivedTokens} />
        <ReportMetric label={t("sessionTabs.report.estimatedCharacters")} value={usage.estimatedCharacters} />
        <ReportMetric label={t("sessionTabs.report.responses")} value={usage.responseCount} />
        <ReportMetric label={t("sessionTabs.report.internalResponses")} value={usage.internalPurposeResponseCount} />
      </div>
      {/* Stated rather than omitted: a usage panel with no cost reads as a cost of zero unless it
          says why there is none. */}
      <p className="mt-2 text-xs text-muted-foreground">{t("sessionTabs.report.costUnavailable")}</p>
    </ReportSection>
  );
}

export function ReportLatencySection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { latency } = report;
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "latency")}
      onOpenEvidence={() => onOpenEvidence("latency")}
      title={t("sessionTabs.report.section.latency")}
    >
      <div className="grid gap-2 sm:grid-cols-3">
        <ReportMetric label={t("sessionTabs.report.p50Ms")} value={latency.p50Ms} />
        <ReportMetric label={t("sessionTabs.report.p95Ms")} value={latency.p95Ms} />
        <ReportMetric label={t("sessionTabs.report.slowestMs")} value={latency.slowestRecordDurationMs} />
      </div>
    </ReportSection>
  );
}

export function ReportChangesSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { changes } = report;
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "changes")}
      onOpenEvidence={() => onOpenEvidence("changes")}
      title={t("sessionTabs.report.section.changes")}
    >
      <div className="grid gap-2 sm:grid-cols-3">
        <ReportMetric label={t("sessionTabs.report.changedFiles")} value={changes.changedFiles} />
        {/* Absent in this build. The em dash is the point: zero would claim every changed file had
            been reviewed. */}
        <ReportMetric label={t("sessionTabs.report.unviewedFiles")} value={changes.unviewedFiles} />
        <ReportMetric label={t("sessionTabs.report.unresolvedFindings")} value={changes.unresolvedFindings} />
      </div>
    </ReportSection>
  );
}

export function ReportVerificationSection({
  onOpenEvidence,
  report,
}: {
  onOpenEvidence: OpenEvidence;
  report: SessionRunReport;
}) {
  const { t } = useTranslation();
  const { verification } = report;
  return (
    <ReportSection
      coverage={reportSectionCoverage(report, "verification")}
      onOpenEvidence={() => onOpenEvidence("verification")}
      title={t("sessionTabs.report.section.verification")}
    >
      {/* Failures beside what passed. Failures alone make every run look broken. */}
      <div className="grid gap-2 sm:grid-cols-3">
        <ReportMetric label={t("sessionTabs.report.testsPassed")} value={verification.passed} />
        <ReportMetric label={t("sessionTabs.report.testsFailed")} value={verification.failed} />
        <ReportMetric label={t("sessionTabs.report.testsSkipped")} value={verification.skipped} />
      </div>
    </ReportSection>
  );
}
