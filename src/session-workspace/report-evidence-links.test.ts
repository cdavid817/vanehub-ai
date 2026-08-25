import { describe, expect, it } from "vitest";
import type {
  EvidenceRunId,
  EvidenceSeatId,
  EvidenceSessionId,
} from "../types/session-workspace-evidence";
import {
  REPORT_SECTIONS,
  reportSectionCoverage,
  reportSectionTarget,
} from "./report-evidence-links";
import { emptySessionRunReport } from "./report-test-fixtures";

const SESSION = "session-1" as EvidenceSessionId;

describe("report evidence links", () => {
  it("gives every section somewhere to go", () => {
    const report = emptySessionRunReport(SESSION);
    // A section with no destination is a section whose number a reader cannot check, which is the
    // one thing a report of counts must not have.
    for (const section of REPORT_SECTIONS) {
      const target = reportSectionTarget(report, section);
      expect(target.scope.sessionId).toBe(SESSION);
      expect(target.tab.length).toBeGreaterThan(0);
    }
  });

  it("sends failures to the logs and changes to the diff", () => {
    const report = emptySessionRunReport(SESSION);
    expect(reportSectionTarget(report, "failures").tab).toBe("logs");
    expect(reportSectionTarget(report, "changes").tab).toBe("changes");
    expect(reportSectionTarget(report, "latency").tab).toBe("traces");
    expect(reportSectionTarget(report, "tools").tab).toBe("terminal");
  });

  it("carries a single run or seat into the destination scope", () => {
    const report = emptySessionRunReport(SESSION);
    report.scope.runIds = ["run-1" as EvidenceRunId];
    report.scope.seatIds = ["seat-a" as EvidenceSeatId];

    const target = reportSectionTarget(report, "overview");

    expect(target.scope.runId).toBe("run-1");
    expect(target.scope.seatId).toBe("seat-a");
  });

  it("stays at the session when several runs were in scope", () => {
    const report = emptySessionRunReport(SESSION);
    report.scope.runIds = ["run-1", "run-2"] as EvidenceRunId[];

    const target = reportSectionTarget(report, "overview");

    // Several runs cannot be expressed as one scope. Picking one would land the reader on a panel
    // whose contents do not add up to the number they clicked.
    expect(target.scope.runId).toBeUndefined();
  });

  it("reads a missing section's coverage as unavailable rather than complete", () => {
    const report = emptySessionRunReport(SESSION);
    // A backend that did not report on a section has not vouched for it. Treating the gap as
    // completeness is the exact failure the coverage map exists to prevent.
    delete (report.coverage.sections as Record<string, unknown>).tools;

    expect(reportSectionCoverage(report, "tools").state).toBe("unavailable");
  });
});
