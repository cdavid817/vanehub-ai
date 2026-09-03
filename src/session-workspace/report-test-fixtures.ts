import type {
  EvidenceSessionId,
  ReportSectionCoverage,
  SessionRunReport,
} from "../types/session-workspace-evidence";
import { REPORT_SECTIONS } from "./report-evidence-links";

/**
 * A complete report with nothing in it, as the starting point for a test's own overrides.
 *
 * Written once here because a report has fourteen fields and nine coverage entries, and a test that
 * spelled them out inline would spend more lines constructing the subject than asserting about it —
 * with the fields it happened to care about drowned among the ones it did not.
 *
 * Every section is `complete`, which is the honest default for a fixture: a session where nothing
 * happened is a real state, and it is the one a test asserting "zero" wants. A test that needs a
 * degraded section overrides that section, which reads as the deviation it is.
 */
export function emptySessionRunReport(sessionId: string): SessionRunReport {
  const complete: ReportSectionCoverage = { state: "complete", reasonCodes: [] };
  const sections = Object.fromEntries(
    REPORT_SECTIONS.map((section) => [section, complete]),
  ) as SessionRunReport["coverage"]["sections"];

  return {
    scope: {
      sessionId: sessionId as EvidenceSessionId,
      runIds: [],
      seatIds: [],
      groupBy: "run",
    },
    generatedAt: "2026-08-25T10:00:00.000Z",
    coverage: { overall: "complete", sections },
    overview: { runCount: 0, succeeded: 0, failed: 0, cancelled: 0, retries: 0 },
    usage: {
      responseCount: 0,
      internalPurposeResponseCount: 0,
      coverage: complete,
      costAvailable: false,
    },
    latency: {},
    agents: [],
    tools: [],
    commands: { total: 0, failed: 0, running: 0 },
    changes: { changedFiles: 0, unresolvedFindings: 0 },
    verification: { passed: 0, failed: 0, skipped: 0 },
    failures: { rows: [] },
    evidenceLinks: [],
    sourceCoverage: { state: "complete", reasonCodes: [], truncated: false },
  };
}
