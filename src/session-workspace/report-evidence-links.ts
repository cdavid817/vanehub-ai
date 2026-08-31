import type {
  ReportSectionCoverage,
  SessionRunReport,
  WorkspaceEvidenceTarget,
} from "../types/session-workspace-evidence";

/**
 * The report's sections, in the order they are read.
 *
 * The same nine names the backend's coverage map uses. Naming them once here is what lets a section
 * and its coverage be looked up together; two lists would eventually disagree, and the failure — a
 * section rendering another section's coverage — looks like correct output.
 */
export const REPORT_SECTIONS = [
  "overview",
  "usage",
  "latency",
  "agents",
  "tools",
  "commands",
  "changes",
  "verification",
  "failures",
] as const;

export type ReportSectionId = (typeof REPORT_SECTIONS)[number];

/**
 * Where each section sends a reader who wants the records behind it.
 *
 * Chosen by what actually answers the section's question rather than by what is nearest. Failures
 * go to the logs because a reason code is a thing the logs can be filtered by; latency and the
 * per-agent rows go to the traces because a duration is a span; tool and command counts go to the
 * terminal history, which is the record of the calls themselves.
 *
 * `usage` has no better home than the terminal history: nothing in the console shows per-response
 * accounting on its own, so the honest destination is the list of responses it was measured from.
 */
const SECTION_TABS: Record<ReportSectionId, WorkspaceEvidenceTarget["tab"]> = {
  overview: "traces",
  usage: "terminal-history",
  latency: "traces",
  agents: "traces",
  tools: "terminal-history",
  commands: "terminal-history",
  changes: "changes",
  verification: "terminal-history",
  failures: "logs",
};

/**
 * The jump for one section, scoped to exactly what the report was scoped to.
 *
 * Built from the report's own scope rather than from the row that happens to be under the cursor:
 * a report section summarises everything in scope, so a link that narrowed further would land on a
 * filtered panel whose contents do not add up to the number the reader just clicked.
 */
export function reportSectionTarget(
  report: SessionRunReport,
  section: ReportSectionId,
): WorkspaceEvidenceTarget {
  const { runIds, seatIds, sessionId } = report.scope;
  return {
    tab: SECTION_TABS[section],
    focus: "filter",
    scope: {
      sessionId,
      // One run or one seat narrows; several cannot be expressed as a single scope, so the link
      // stays at the session and the panel shows everything the report covered rather than an
      // arbitrary one of them.
      ...(runIds.length === 1 ? { runId: runIds[0] } : {}),
      ...(seatIds.length === 1 ? { seatId: seatIds[0] } : {}),
    },
  };
}

/**
 * A section's coverage, defaulting to `unavailable` rather than to `complete`.
 *
 * A section this build knows about but the backend did not report on has not been vouched for, and
 * treating the gap as completeness is the exact failure the coverage map exists to prevent.
 */
export function reportSectionCoverage(
  report: SessionRunReport,
  section: ReportSectionId,
): ReportSectionCoverage {
  return report.coverage.sections[section] ?? { state: "unavailable", reasonCodes: [] };
}
