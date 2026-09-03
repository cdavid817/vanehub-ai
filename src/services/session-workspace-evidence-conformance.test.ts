import { describe, expect, it } from "vitest";
import { createFixtureEvidenceTransport } from "../contracts/fixtures/session-workspace-evidence-transport";
import {
  evidenceRecordIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { ExecutionEvidenceNotice } from "../types/session-workspace-evidence";
import { EVIDENCE_PAGE_LIMITS } from "../types/session-workspace-evidence";
import { EvidenceUnavailableError, unavailableEvidenceTransport } from "./native-evidence-transport";
import type { SessionWorkspaceEvidenceService } from "./session-workspace-evidence-service";
import { createTauriSessionWorkspaceEvidenceClient } from "./tauri-session-workspace-evidence-client";
import { createWebSessionWorkspaceEvidenceClient } from "./web-session-workspace-evidence-client";

const sessionId = evidenceSessionIdSchema.parse("session-1");

/**
 * One suite, two runtimes.
 *
 * The Tauri client runs against the fixture transport rather than a live command, which is the
 * whole point of the seam: the behaviour under test is the client's, and it can be settled before
 * anything is registered. Task 3.15 re-runs these same cases against the native evidence commands
 * and 10.8 against the native report, so activation is checked against the shape the frontend
 * already committed to instead of against cases written afterwards.
 */
interface ConformanceRuntime {
  service: SessionWorkspaceEvidenceService;
  /** Each runtime is driven through its own channel, so the suite never reaches past the seam. */
  publish: (notice: {
    kind: ExecutionEvidenceNotice["kind"];
    sessionId: ExecutionEvidenceNotice["sessionId"];
    sequence: number;
  }) => void;
}

const runtimes: { name: string; create: () => ConformanceRuntime }[] = [
  {
    name: "Tauri fixture transport",
    create: () => {
      const transport = createFixtureEvidenceTransport();
      return {
        service: createTauriSessionWorkspaceEvidenceClient(transport),
        // The native event API delivers an unparsed payload; the client validates it.
        publish: (notice) => transport.publish({ ...notice, occurredAt: "2026-08-22T10:00:00.000Z" }),
      };
    },
  },
  {
    name: "Web/mock",
    create: () => {
      const service = createWebSessionWorkspaceEvidenceClient();
      return { service, publish: (notice) => service.emitSimulatedNotice(notice) };
    },
  },
];

describe.each(runtimes)("evidence service conformance: $name", ({ create }) => {
  it("returns a summary whose coverage state is explicit", async () => {
    const summary = await create().service.getWorkspaceEvidenceSummary({ sessionId });
    expect(summary.sessionId).toBe(sessionId);
    expect(["complete", "indexing", "partial", "unavailable"]).toContain(summary.coverage.state);
    expect(summary.shells.live).toBeGreaterThanOrEqual(0);
  });

  it("returns a bounded record page with coverage", async () => {
    const page = await create().service.listExecutionRecords({ scope: { sessionId } });
    expect(page.items.length).toBeLessThanOrEqual(EVIDENCE_PAGE_LIMITS.default);
    expect(page.coverage.reasonCodes).toBeInstanceOf(Array);
    expect(typeof page.coverage.truncated).toBe("boolean");
  });

  it("parses every record kind it returns into the discriminated union", async () => {
    const page = await create().service.listExecutionRecords({ scope: { sessionId } });
    for (const record of page.items) {
      expect(["command", "tool", "delegation", "verification", "legacy"]).toContain(record.kind);
      expect(record.sessionId).toBe(sessionId);
      if (record.kind === "command") expect(record.outputAvailability).toBeDefined();
      if (record.kind === "legacy") expect(record.source).toBe("message-history");
    }
  });

  // Both runtimes must express "we saw it finish but never saw it start" the same way.
  it("keeps a completion-only record startless while preserving its terminal state", async () => {
    const page = await create().service.listExecutionRecords({ scope: { sessionId } });
    const startless = page.items.filter((record) => record.startedAt === undefined);
    expect(startless.length).toBeGreaterThan(0);
    for (const record of startless) {
      // The outcome is still whatever was observed; a missing start says nothing about it.
      expect(["succeeded", "failed", "cancelled", "incomplete"]).toContain(record.status);
      expect(record.endedAt).toBeDefined();
      expect(record.coverage.reasonCodes).toContain("evidence_start_not_observed");
    }
  });

  it("exposes a record detail with related counts and safe attributes only", async () => {
    const { service } = create();
    const page = await service.listExecutionRecords({ scope: { sessionId } });
    const detail = await service.getExecutionRecord({ sessionId, recordId: page.items[0].id });
    expect(detail.relatedCounts.logs).toBeGreaterThanOrEqual(0);
    expect(Object.values(detail.safeAttributes).every((value) => typeof value === "string")).toBe(true);
  });

  it("returns a report that never claims a monetary cost", async () => {
    const report = await create().service.getSessionRunReport({ sessionId });
    expect(report.usage.costAvailable).toBe(false);
    expect(report.coverage.sections.usage.state).toBeDefined();
  });

  it("returns a report whose every section declares its own coverage", async () => {
    const report = await create().service.getSessionRunReport({ sessionId });
    // Per-section rather than per-report: a report is useful while one source is still indexing,
    // and one overall state would either hide that or discard the sections that are fine.
    for (const section of Object.values(report.coverage.sections)) {
      expect(["complete", "indexing", "partial", "unavailable"]).toContain(section.state);
      expect(section.reasonCodes).toBeInstanceOf(Array);
    }
  });

  it("answers an export with a status a reader can act on", async () => {
    const result = await create().service.exportSessionRunReport({
      sessionId,
      destinationDirectory: "D:/exports",
    });
    // Three states, and only one of them means a file exists. A runtime with nowhere to write
    // says `simulated` rather than reporting a path nobody could open.
    expect(["exported", "cancelled", "simulated"]).toContain(result.status);
    if (result.status !== "exported") expect(result.path).toBeNull();
  });

  it("returns failure rows keyed by codes rather than by messages", async () => {
    const report = await create().service.getSessionRunReport({ sessionId });
    for (const row of report.failures.rows) {
      // A report is quoted, and a message quoted out of one is producer text nobody redacted.
      expect(row.reasonCode).not.toContain(" ");
      expect(row.count).toBeGreaterThanOrEqual(0);
    }
  });

  it("de-duplicates replayed notices and reports a gap once", async () => {
    const { service, publish } = create();
    const seen: ExecutionEvidenceNotice[] = [];
    const unsubscribe = await service.subscribeExecutionEvidence(
      { sessionId, fromSequence: 4 },
      (notice) => seen.push(notice),
    );
    publish({ kind: "record-appended", sessionId, sequence: 3 });
    publish({ kind: "record-appended", sessionId, sequence: 5 });
    publish({ kind: "record-appended", sessionId, sequence: 5 });
    publish({ kind: "record-appended", sessionId, sequence: 8 });
    unsubscribe();

    // 3 is a replay below the resume point, the second 5 is a duplicate, and the jump to 8 hides
    // two notices the bounded queue dropped.
    expect(seen.map((notice) => `${notice.kind}:${notice.sequence}`)).toEqual([
      "record-appended:5",
      "coverage-gap:8",
      "record-appended:8",
    ]);
    expect(seen[1].droppedCount).toBe(2);
  });

  it("ignores a notice for another session", async () => {
    const { service, publish } = create();
    const seen: ExecutionEvidenceNotice[] = [];
    const unsubscribe = await service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    publish({
      kind: "record-appended",
      sessionId: evidenceSessionIdSchema.parse("session-2"),
      sequence: 1,
    });
    unsubscribe();
    expect(seen).toHaveLength(0);
  });

  it("stops delivering after unsubscribe and tolerates a second unsubscribe", async () => {
    const { service, publish } = create();
    const seen: ExecutionEvidenceNotice[] = [];
    const unsubscribe = await service.subscribeExecutionEvidence({ sessionId }, (n) => seen.push(n));
    unsubscribe();
    expect(() => unsubscribe()).not.toThrow();
    publish({ kind: "record-appended", sessionId, sequence: 1 });
    expect(seen).toHaveLength(0);
  });
});

describe("evidence transport bindings", () => {
  it("bounds a page request before it leaves the client", async () => {
    const transport = createFixtureEvidenceTransport();
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    await service.listExecutionRecords({ scope: { sessionId }, limit: 9000 });

    // The request is bounded before it leaves the client, not after the backend answers.
    expect(transport.requests[0].command).toBe("list_execution_records");
    expect(transport.requests[0].payload).toMatchObject({ limit: EVIDENCE_PAGE_LIMITS.maximum });
  });

  it("propagates the full scope so a seat-scoped query cannot silently widen", async () => {
    const transport = createFixtureEvidenceTransport();
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    await service.listExecutionRecords({
      scope: { sessionId, seatId: evidenceSeatIdSchema.parse("seat-a") },
    });
    expect(transport.requests[0].payload).toMatchObject({ scope: { seatId: "seat-a" } });
  });

  // No longer the application's binding — the evidence reads now reach registered commands — but
  // the fallback still has to refuse in a shape a panel can localize rather than throw a string.
  it("refuses every evidence read with a stable reason code when no runtime is bound", async () => {
    const service = createTauriSessionWorkspaceEvidenceClient(unavailableEvidenceTransport);
    const calls = [
      () => service.getWorkspaceEvidenceSummary({ sessionId }),
      () => service.listExecutionRecords({ scope: { sessionId } }),
      () => service.getExecutionRecord({ sessionId, recordId: evidenceRecordIdSchema.parse("record-1") }),
      () => service.getSessionRunReport({ sessionId }),
    ];
    for (const call of calls) {
      await expect(call()).rejects.toBeInstanceOf(EvidenceUnavailableError);
      await expect(call()).rejects.toMatchObject({ reasonCode: "evidence_unavailable" });
    }
  });

  // A panel that subscribes on mount should render its empty state, not an error boundary.
  it("lets a subscription resolve to a no-op when no runtime is bound", async () => {
    const service = createTauriSessionWorkspaceEvidenceClient(unavailableEvidenceTransport);
    const unsubscribe = await service.subscribeExecutionEvidence({ sessionId }, () => undefined);
    expect(() => unsubscribe()).not.toThrow();
  });

  it("drops a malformed notice without tearing down the subscription", async () => {
    const transport = createFixtureEvidenceTransport();
    const service = createTauriSessionWorkspaceEvidenceClient(transport);
    const seen: ExecutionEvidenceNotice[] = [];
    await service.subscribeExecutionEvidence({ sessionId }, (notice) => seen.push(notice));
    transport.publish({ kind: "not-a-kind", sequence: 1, sessionId: "session-1" });
    transport.publish({
      kind: "record-appended",
      sequence: 1,
      sessionId: "session-1",
      occurredAt: "2026-08-22T10:00:00.000Z",
    });
    expect(seen.map((notice) => notice.sequence)).toEqual([1]);
  });
});
