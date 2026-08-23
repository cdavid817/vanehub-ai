import { describe, expect, it } from "vitest";
import {
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
} from "./session-workspace-evidence-ids";
import type {
  ExecutionRecord,
  ExecutionRecordQuery,
  QueryCoverage,
  ToolExecutionRecord,
} from "../types/session-workspace-evidence";
import { matchesExecutionRecordQuery } from "./execution-record-filter";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const otherSessionId = evidenceSessionIdSchema.parse("session-b");
const seatId = evidenceSeatIdSchema.parse("seat-1");
const runId = evidenceRunIdSchema.parse("run-1");
const coverage: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

function tool(overrides: Partial<ToolExecutionRecord> = {}): ToolExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse("tool:call-1"),
    kind: "tool",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage,
    toolName: "read_file",
    source: "native",
    ...overrides,
  };
}

function query(overrides: Partial<ExecutionRecordQuery> = {}): ExecutionRecordQuery {
  return { scope: { sessionId }, ...overrides };
}

function matches(record: ExecutionRecord, input: ExecutionRecordQuery): boolean {
  return matchesExecutionRecordQuery(record, input);
}

describe("execution record filter contract", () => {
  it("keeps one session's records out of another's answer", () => {
    expect(matches(tool(), query())).toBe(true);
    expect(matches(tool(), query({ scope: { sessionId: otherSessionId } }))).toBe(false);
  });

  it("does not attribute an uncorrelated record to a concrete filter", () => {
    // An absent seat is not a match for a chosen seat; treating it as one is how a panel shows
    // another participant's work under the selected one.
    expect(matches(tool(), query({ scope: { sessionId, seatId } }))).toBe(false);
    expect(matches(tool({ seatId }), query({ scope: { sessionId, seatId } }))).toBe(true);
    expect(matches(tool(), query({ scope: { sessionId, runId } }))).toBe(false);
  });

  it("narrows by kind, status, and fidelity", () => {
    expect(matches(tool(), query({ filters: { kinds: ["tool"] } }))).toBe(true);
    expect(matches(tool(), query({ filters: { kinds: ["command"] } }))).toBe(false);
    expect(matches(tool(), query({ filters: { statuses: ["failed"] } }))).toBe(false);
    expect(matches(tool(), query({ filters: { fidelities: ["native"] } }))).toBe(true);
    expect(matches(tool(), query({ filters: { fidelities: ["opaque"] } }))).toBe(false);
  });

  it("searches the redacted display text and nothing else", () => {
    expect(matches(tool(), query({ filters: { search: "read" } }))).toBe(true);
    expect(matches(tool(), query({ filters: { search: "READ_FILE" } }))).toBe(true);
    // The id is not searchable: a search box that matched identifiers would be a way to probe for
    // them, and identifiers are not what a reader is looking for here.
    expect(matches(tool(), query({ filters: { search: "call-1" } }))).toBe(false);
  });

  it("treats a wildcard the reader typed as a literal", () => {
    // The desktop side escapes these into its LIKE pattern; both runtimes have to agree, or one
    // of them answers a question the other refuses.
    expect(matches(tool(), query({ filters: { search: "rea_" } }))).toBe(false);
    expect(matches(tool(), query({ filters: { search: "read%file" } }))).toBe(false);
    expect(matches(tool(), query({ filters: { search: "%" } }))).toBe(false);
  });

  it("treats a blank search as no filter rather than one that matches nothing", () => {
    expect(matches(tool(), query({ filters: { search: "   " } }))).toBe(true);
    expect(matches(tool(), query({ filters: {} }))).toBe(true);
  });

  it("never matches a record that has no searchable text on a non-empty term", () => {
    const delegation: ExecutionRecord = {
      id: evidenceRecordIdSchema.parse("delegation:a:b"),
      kind: "delegation",
      sessionId,
      status: "running",
      fidelity: "native",
      coverage,
    };
    // Matching everything because there was nothing to compare against would make a delegation
    // appear under every search term.
    expect(matches(delegation, query({ filters: { search: "anything" } }))).toBe(false);
  });
});
