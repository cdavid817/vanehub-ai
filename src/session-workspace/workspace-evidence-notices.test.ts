import { describe, expect, it } from "vitest";
import {
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { ExecutionEvidenceNotice } from "../types/session-workspace-evidence";
import {
  createEvidenceInvalidationBuffer,
  MAX_TRACKED_NOTICE_RECORDS,
  noticeQueryFamilies,
  EVIDENCE_NOTICE_WINDOW_MS,
} from "./workspace-evidence-notices";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const otherSessionId = evidenceSessionIdSchema.parse("session-b");

function notice(
  kind: ExecutionEvidenceNotice["kind"],
  sequence: number,
  overrides: Partial<ExecutionEvidenceNotice> = {},
): ExecutionEvidenceNotice {
  return {
    kind,
    sequence,
    sessionId,
    occurredAt: "2026-08-23T10:00:00.000Z",
    ...overrides,
  };
}

describe("evidence notice invalidation", () => {
  it("keeps the coalescing window fixed and testable", () => {
    // Fixed rather than adaptive: a test that had to guess the window would either sleep for the
    // worst case or assert nothing.
    expect(EVIDENCE_NOTICE_WINDOW_MS).toBe(250);
    expect(MAX_TRACKED_NOTICE_RECORDS).toBe(32);
  });

  it("maps each notice kind to a finite set of families", () => {
    expect(noticeQueryFamilies(notice("record-appended", 1))).toEqual(["records", "summary"]);
    expect(noticeQueryFamilies(notice("record-updated", 2))).toEqual([
      "records",
      "record-detail",
      "summary",
    ]);
    expect(noticeQueryFamilies(notice("summary-changed", 3))).toEqual(["summary"]);
    expect(noticeQueryFamilies(notice("coverage-gap", 4))).toEqual([
      "records",
      "record-detail",
      "summary",
      "report",
    ]);
  });

  it("folds a burst into one invalidation", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    for (let index = 1; index <= 5; index += 1) {
      expect(buffer.accept(notice("record-appended", index))).toBe(true);
    }

    const invalidation = buffer.drain();

    // Five appends are one refetch of the same page, not five.
    expect(invalidation).toEqual({ broad: false, families: ["records", "summary"], recordIds: [] });
    expect(buffer.drain()).toBeNull();
  });

  it("refuses a notice belonging to another session", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);

    expect(buffer.accept(notice("record-appended", 1, { sessionId: otherSessionId }))).toBe(false);

    // Refetching the right keys for the wrong reason would hide that the subscription is pointed
    // at the wrong place.
    expect(buffer.pending()).toBe(false);
    expect(buffer.drain()).toBeNull();
  });

  it("names the records whose detail moved", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    const recordId = evidenceRecordIdSchema.parse("record-1");
    buffer.accept(notice("record-updated", 1, { recordId }));

    expect(buffer.drain()).toEqual({
      broad: false,
      families: ["records", "record-detail", "summary"],
      recordIds: [recordId],
    });
  });

  it("widens to the whole session when a gap says rows were dropped", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    buffer.accept(notice("record-updated", 1, { recordId: evidenceRecordIdSchema.parse("r-1") }));
    buffer.accept(notice("coverage-gap", 5, { droppedCount: 3 }));

    const invalidation = buffer.drain();

    // A gap says rows were dropped without saying which, so the named list is no longer a list of
    // everything that changed — keeping it would claim knowledge the subscription lost.
    expect(invalidation?.broad).toBe(true);
    expect(invalidation?.recordIds).toEqual([]);
  });

  it("stops naming records rather than tracking an unbounded set", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    for (let index = 0; index <= MAX_TRACKED_NOTICE_RECORDS; index += 1) {
      buffer.accept(
        notice("record-updated", index + 1, {
          recordId: evidenceRecordIdSchema.parse(`record-${index}`),
        }),
      );
    }

    const invalidation = buffer.drain();

    // Naming a truncated subset would read as "these and only these changed".
    expect(invalidation?.broad).toBe(true);
    expect(invalidation?.recordIds).toEqual([]);
  });

  it("carries identifiers and nothing a reader could see", () => {
    const buffer = createEvidenceInvalidationBuffer(sessionId);
    buffer.accept(notice("record-updated", 1, { recordId: evidenceRecordIdSchema.parse("r-1") }));

    const invalidation = buffer.drain();

    // This is the one path from the event channel into React. Anything textual retained here
    // would be content that redaction can no longer be applied to.
    expect(Object.keys(invalidation ?? {}).sort()).toEqual(["broad", "families", "recordIds"]);
  });
});
