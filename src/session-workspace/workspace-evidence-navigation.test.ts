import { describe, expect, it } from "vitest";
import {
  evidenceCommandIdSchema,
  evidenceOperationIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type { WorkspaceEvidenceScope } from "../types/session-workspace-evidence";
import {
  consumedScopeFields,
  EVIDENCE_SCOPE_FIELDS,
  TAB_SCOPE_FIELDS,
  tabConsumesScope,
  unsupportedScopeFields,
  withDependentScopeFields,
} from "./workspace-evidence-navigation";

/**
 * Every field, deliberately. `satisfies Required<...>` is the guard: a field added to the scope
 * without being added here stops this file compiling, which is the only way to notice a field the
 * reducer's copies would otherwise drop in silence.
 */
const fullScope = {
  sessionId: evidenceSessionIdSchema.parse("session-1"),
  seatId: evidenceSeatIdSchema.parse("seat-1"),
  runId: evidenceRunIdSchema.parse("run-1"),
  traceId: evidenceTraceIdSchema.parse("trace-1"),
  spanId: evidenceSpanIdSchema.parse("span-1"),
  operationId: evidenceOperationIdSchema.parse("operation-1"),
  commandId: evidenceCommandIdSchema.parse("command-1"),
  relativePath: "src/main.rs",
  hunkFingerprint: "hunk-1",
  occurredAt: "2026-08-23T10:00:00.000Z",
} satisfies Required<WorkspaceEvidenceScope>;

const fullCorrelation = {
  seatId: fullScope.seatId,
  runId: fullScope.runId,
  traceId: fullScope.traceId,
  spanId: fullScope.spanId,
  operationId: fullScope.operationId,
  commandId: fullScope.commandId,
  relativePath: fullScope.relativePath,
  hunkFingerprint: fullScope.hunkFingerprint,
  occurredAt: fullScope.occurredAt,
};

describe("workspace evidence navigation", () => {
  it("lists every correlation field exactly once", () => {
    const declared = Object.keys(fullScope).filter((key) => key !== "sessionId");
    expect([...EVIDENCE_SCOPE_FIELDS].sort()).toEqual(declared.sort());
    expect(new Set(EVIDENCE_SCOPE_FIELDS).size).toBe(EVIDENCE_SCOPE_FIELDS.length);
  });

  it("never treats the owning session as a filter", () => {
    // A chip for `sessionId` would offer to clear the thing that decides whose evidence is shown.
    expect(EVIDENCE_SCOPE_FIELDS).not.toContain("sessionId");
  });

  it("reports the fields a destination applies and the ones it will ignore", () => {
    expect(consumedScopeFields("files", fullCorrelation)).toEqual(["relativePath"]);
    expect(unsupportedScopeFields("files", fullCorrelation)).toEqual([
      "seatId",
      "runId",
      "traceId",
      "spanId",
      "operationId",
      "commandId",
      "hunkFingerprint",
      "occurredAt",
    ]);
  });

  it("reports nothing for a field that is absent rather than unsupported", () => {
    expect(consumedScopeFields("traces", { runId: fullScope.runId })).toEqual(["runId"]);
    expect(unsupportedScopeFields("traces", { runId: fullScope.runId })).toEqual([]);
  });

  it("clears what a cleared field owns", () => {
    // A span whose trace is gone matches by identifier alone, which is how a filter starts
    // resolving against a different trace's span.
    expect([...withDependentScopeFields(["traceId"])].sort()).toEqual(["spanId", "traceId"]);
    expect([...withDependentScopeFields(["relativePath"])].sort()).toEqual([
      "hunkFingerprint",
      "relativePath",
    ]);
    expect([...withDependentScopeFields(["runId"])].sort()).toEqual([
      "commandId",
      "operationId",
      "runId",
      "spanId",
      "traceId",
    ]);
  });

  it("gives every evidence destination a declared field set", () => {
    for (const [tab, fields] of Object.entries(TAB_SCOPE_FIELDS)) {
      expect(fields.length, `${tab} declares no consumed fields`).toBeGreaterThan(0);
      for (const field of fields) expect(EVIDENCE_SCOPE_FIELDS).toContain(field);
    }
    expect(tabConsumesScope("shell")).toBe(true);
  });
});
