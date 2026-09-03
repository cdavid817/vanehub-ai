import { describe, expect, it } from "vitest";
import {
  evidenceCursorSchema,
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import { workspaceEvidenceScopeSchema } from "../contracts/session-workspace-evidence-core";
import type { ExecutionRecordFilters } from "../types/session-workspace-evidence";
import { evidenceQueryKeys } from "./evidence-query-keys";

const sessionId = evidenceSessionIdSchema.parse("session-1");

function scope(extra: Record<string, string> = {}) {
  return workspaceEvidenceScopeSchema.parse({ sessionId: "session-1", ...extra });
}

describe("evidenceQueryKeys", () => {
  // The failure this guards is silent: a key that omits the seat returns another seat's cached
  // rows without erroring anywhere.
  it("separates two seats", () => {
    expect(evidenceQueryKeys.records(scope({ seatId: "seat-a" })))
      .not.toEqual(evidenceQueryKeys.records(scope({ seatId: "seat-b" })));
  });

  it("separates a seat-scoped query from an all-seats query", () => {
    expect(evidenceQueryKeys.records(scope({ seatId: "seat-a" })))
      .not.toEqual(evidenceQueryKeys.records(scope()));
  });

  it.each(["runId", "traceId", "spanId", "operationId", "commandId", "relativePath", "hunkFingerprint", "occurredAt"])(
    "separates queries that differ only by %s",
    (field) => {
      expect(evidenceQueryKeys.records(scope({ [field]: "left" })))
        .not.toEqual(evidenceQueryKeys.records(scope({ [field]: "right" })));
    },
  );

  // A page fetched with one continuation token is not the same result as a page fetched with
  // another; sharing an entry is how a keyset list duplicates or skips a row.
  it("separates pages by cursor", () => {
    const first = evidenceQueryKeys.records(scope(), undefined, undefined);
    const second = evidenceQueryKeys.records(scope(), undefined, evidenceCursorSchema.parse("v1.next"));
    expect(first).not.toEqual(second);
  });

  it("separates queries that differ only by filters", () => {
    const failed: ExecutionRecordFilters = { statuses: ["failed"] };
    const running: ExecutionRecordFilters = { statuses: ["running"] };
    expect(evidenceQueryKeys.records(scope(), failed))
      .not.toEqual(evidenceQueryKeys.records(scope(), running));
  });

  // Two equivalent filter sets must not produce two cache entries.
  it("normalizes filter order, blank search, and empty arrays", () => {
    expect(evidenceQueryKeys.records(scope(), { kinds: ["tool", "command"] }))
      .toEqual(evidenceQueryKeys.records(scope(), { kinds: ["command", "tool"] }));
    expect(evidenceQueryKeys.records(scope(), { search: "  " }))
      .toEqual(evidenceQueryKeys.records(scope(), undefined));
    expect(evidenceQueryKeys.records(scope(), { kinds: [] }))
      .toEqual(evidenceQueryKeys.records(scope(), undefined));
    expect(evidenceQueryKeys.records(scope(), { search: " npm test " }))
      .toEqual(evidenceQueryKeys.records(scope(), { search: "npm test" }));
  });

  it("keeps summary, records, detail, and report in distinct namespaces", () => {
    const keys = [
      evidenceQueryKeys.summary(sessionId),
      evidenceQueryKeys.records(scope()),
      evidenceQueryKeys.recordDetail(sessionId, evidenceRecordIdSchema.parse("record-1")),
      evidenceQueryKeys.report(sessionId, [], [], undefined, undefined, undefined),
    ].map((key) => JSON.stringify(key));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it("separates report scopes that differ only by run or group-by", () => {
    expect(evidenceQueryKeys.report(sessionId, ["run-1"], [], undefined, undefined, undefined))
      .not.toEqual(evidenceQueryKeys.report(sessionId, ["run-2"], [], undefined, undefined, undefined));
    expect(evidenceQueryKeys.report(sessionId, [], [], undefined, undefined, "agent"))
      .not.toEqual(evidenceQueryKeys.report(sessionId, [], [], undefined, undefined, "seat"));
  });

  it("orders report run and seat ids so an equivalent scope reuses its entry", () => {
    expect(evidenceQueryKeys.report(sessionId, ["run-2", "run-1"], [], undefined, undefined, undefined))
      .toEqual(evidenceQueryKeys.report(sessionId, ["run-1", "run-2"], [], undefined, undefined, undefined));
  });
});
