import { describe, expect, it } from "vitest";
import {
  evidenceCommandIdSchema,
  evidenceRecordIdSchema,
  evidenceSessionIdSchema,
} from "../contracts/session-workspace-evidence-ids";
import type {
  CommandExecutionRecord,
  ExecutionRecord,
  QueryCoverage,
  VerificationExecutionRecord,
} from "../types/session-workspace-evidence";
import {
  commandDisplayField,
  durationField,
  endedAtField,
  exitField,
  fidelityKey,
  outputField,
  recordLabel,
  startedAtField,
  verificationCountsField,
} from "./execution-record-fields";

const sessionId = evidenceSessionIdSchema.parse("session-a");
const coverage: QueryCoverage = { state: "complete", reasonCodes: [], truncated: false };

function command(overrides: Partial<CommandExecutionRecord> = {}): CommandExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse("command:cmd-1"),
    kind: "command",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage,
    commandId: evidenceCommandIdSchema.parse("cmd-1"),
    runtimeKind: "local-shell",
    outputAvailability: "merged",
    outputTruncated: false,
    ...overrides,
  };
}

function verification(
  overrides: Partial<VerificationExecutionRecord> = {},
): VerificationExecutionRecord {
  return {
    id: evidenceRecordIdSchema.parse("verification:v-1"),
    kind: "verification",
    sessionId,
    status: "succeeded",
    fidelity: "native",
    coverage,
    verificationName: "cargo test",
    outcome: "passed",
    ...overrides,
  };
}

describe("execution record fields", () => {
  it("says a start was not observed rather than deriving one", () => {
    // `endedAt` minus `durationMs` looks like arithmetic and reads like an observation, and a
    // reader has no way to tell which one they are looking at.
    const record = command({ endedAt: "2026-08-23T10:00:12.000Z", durationMs: 12_000 });
    expect(startedAtField(record)).toEqual({ kind: "absent", reason: "not-observed" });
    expect(endedAtField(record).kind).toBe("text");
  });

  it("reports only the producer's own duration", () => {
    expect(durationField(command())).toEqual({ kind: "absent", reason: "not-observed" });
    expect(durationField(command({ durationMs: 12_400 }))).toEqual({
      kind: "i18n",
      key: "executionRecords.field.durationMs",
      values: { ms: 12_400 },
    });
  });

  it("never renders a missing exit code as zero", () => {
    // Zero is success. A command the runtime lost track of is not a command that succeeded.
    expect(exitField(command())).toEqual({ kind: "absent", reason: "not-observed" });
    expect(exitField(command({ exitCode: 0 }))).toEqual({
      kind: "i18n",
      key: "executionRecords.field.exitCode",
      values: { code: 0 },
    });
    expect(exitField(command({ signal: "SIGKILL" }))).toEqual({
      kind: "i18n",
      key: "executionRecords.field.signal",
      values: { signal: "SIGKILL" },
    });
  });

  it("does not offer stdout and stderr for a merged terminal stream", () => {
    // A PTY hands back one stream. Offering two would describe a capture nobody made.
    expect(outputField(command({ outputAvailability: "merged" }))).toEqual({
      kind: "absent",
      reason: "merged",
    });
    expect(outputField(command({ outputAvailability: "unavailable" })).kind).toBe("absent");
    expect(outputField(command({ outputAvailability: "redacted" }))).toEqual({
      kind: "absent",
      reason: "redacted",
    });
    expect(outputField(command({ outputAvailability: "separate" })).kind).toBe("i18n");
  });

  it("shows only the display the producer already redacted", () => {
    expect(commandDisplayField(command())).toEqual({ kind: "absent", reason: "redacted" });
    expect(commandDisplayField(command({ redactedDisplay: "npm test" }))).toEqual({
      kind: "text",
      value: "npm test",
    });
  });

  it("marks a field not applicable rather than not observed on the wrong kind", () => {
    // "Not observed" says the runtime failed to see something. A tool has no exit code to see.
    expect(exitField(verification())).toEqual({ kind: "absent", reason: "not-applicable" });
    expect(outputField(verification())).toEqual({ kind: "absent", reason: "not-applicable" });
  });

  it("does not report an unreported verification as zero checks", () => {
    expect(verificationCountsField(verification())).toEqual({
      kind: "absent",
      reason: "not-observed",
    });
    expect(verificationCountsField(verification({ passedCount: 138, failedCount: 2 }))).toEqual({
      kind: "i18n",
      key: "executionRecords.field.verificationCounts",
      values: { failed: 2, passed: 138 },
    });
  });

  it("labels each kind from its own structured field", () => {
    expect(recordLabel(command({ redactedDisplay: "npm test" }))).toEqual({
      kind: "text",
      value: "npm test",
    });
    expect(recordLabel(verification())).toEqual({ kind: "text", value: "cargo test" });
    const delegation: ExecutionRecord = {
      id: evidenceRecordIdSchema.parse("delegation:a:b"),
      kind: "delegation",
      sessionId,
      status: "running",
      fidelity: "proxied",
      coverage,
    };
    expect(recordLabel(delegation)).toEqual({
      kind: "i18n",
      key: "executionRecords.field.delegationUnnamed",
    });
  });

  it("reports fidelity exactly as the producer set it", () => {
    // Never upgraded: an inferred row promoted to native would claim the runtime watched work it
    // only heard about.
    for (const fidelity of ["native", "proxied", "inferred", "opaque"] as const) {
      expect(fidelityKey(command({ fidelity }))).toBe(`executionRecords.fidelity.${fidelity}`);
    }
  });
});
