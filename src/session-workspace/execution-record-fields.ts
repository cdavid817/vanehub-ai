import type { ExecutionRecord } from "../types/session-workspace-evidence";

/**
 * One displayable field.
 *
 * `absent` is a value, not a missing one. A row that rendered a dash for "no exit code" and a dash
 * for "exit code zero" would make a command that was killed indistinguishable from one that
 * succeeded, so the two cases are different shapes here and the view has to handle both.
 */
export type RecordField =
  | { kind: "absent"; reason: RecordFieldAbsence }
  | { kind: "text"; value: string }
  | { kind: "i18n"; key: string; values?: Record<string, string | number> };

/** Why a field has no value. Each maps to its own sentence, never to a shared dash. */
export type RecordFieldAbsence =
  | "not-observed"
  | "not-applicable"
  | "unavailable"
  | "redacted"
  | "merged";

const NOT_OBSERVED: RecordField = { kind: "absent", reason: "not-observed" };
const NOT_APPLICABLE: RecordField = { kind: "absent", reason: "not-applicable" };

/**
 * When the work started, or nothing.
 *
 * Never derived. `endedAt` minus `durationMs` looks like arithmetic and reads like an observation,
 * and a reader has no way to tell which one they are looking at.
 */
export function startedAtField(record: ExecutionRecord): RecordField {
  return record.startedAt === undefined ? NOT_OBSERVED : { kind: "text", value: record.startedAt };
}

export function endedAtField(record: ExecutionRecord): RecordField {
  return record.endedAt === undefined ? NOT_OBSERVED : { kind: "text", value: record.endedAt };
}

/**
 * How long it took, or nothing.
 *
 * Only the producer's own measurement. Subtracting two timestamps the record happens to carry
 * would report the distance between two observations as the duration of the work between them.
 */
export function durationField(record: ExecutionRecord): RecordField {
  return record.durationMs === undefined
    ? NOT_OBSERVED
    : { kind: "i18n", key: "executionRecords.field.durationMs", values: { ms: record.durationMs } };
}

/**
 * The exit status of a command, or why there is none.
 *
 * A missing exit code is never shown as `0`. Zero is success, and a command the runtime lost track
 * of is not a command that succeeded.
 */
export function exitField(record: ExecutionRecord): RecordField {
  if (record.kind !== "command") return NOT_APPLICABLE;
  if (record.signal !== undefined) {
    return { kind: "i18n", key: "executionRecords.field.signal", values: { signal: record.signal } };
  }
  return record.exitCode === undefined
    ? NOT_OBSERVED
    : { kind: "i18n", key: "executionRecords.field.exitCode", values: { code: record.exitCode } };
}

/** The command line the producer already redacted. Never reconstructed from anything else. */
export function commandDisplayField(record: ExecutionRecord): RecordField {
  if (record.kind !== "command") return NOT_APPLICABLE;
  return record.redactedDisplay === undefined || record.redactedDisplay.length === 0
    ? { kind: "absent", reason: "redacted" }
    : { kind: "text", value: record.redactedDisplay };
}

export function cwdField(record: ExecutionRecord): RecordField {
  if (record.kind !== "command") return NOT_APPLICABLE;
  return record.cwdDisplay === undefined ? NOT_OBSERVED : { kind: "text", value: record.cwdDisplay };
}

/**
 * What the runtime could observe about the output.
 *
 * A PTY hands back one merged stream, so a row that offered stdout and stderr separately would be
 * describing a capture nobody made. `merged` says so in its own words.
 */
export function outputField(record: ExecutionRecord): RecordField {
  if (record.kind !== "command") return NOT_APPLICABLE;
  switch (record.outputAvailability) {
    case "merged":
      return { kind: "absent", reason: "merged" };
    case "unavailable":
      return { kind: "absent", reason: "unavailable" };
    case "redacted":
      return { kind: "absent", reason: "redacted" };
    case "separate":
      return { kind: "i18n", key: "executionRecords.field.outputSeparate" };
  }
}

/** The label a row leads with, from the record's own structured fields. */
export function recordLabel(record: ExecutionRecord): RecordField {
  switch (record.kind) {
    case "command":
      return commandDisplayField(record);
    case "tool":
      return { kind: "text", value: record.toolName };
    case "verification":
      return { kind: "text", value: record.verificationName };
    case "legacy":
      return { kind: "text", value: record.label };
    case "delegation":
      return record.childAgentId === undefined
        ? { kind: "i18n", key: "executionRecords.field.delegationUnnamed" }
        : { kind: "text", value: record.childAgentId };
  }
}

/**
 * Verification counts, which exist only when the producer reported them.
 *
 * A verification that ran without saying how many checks passed is not a verification where zero
 * passed, and rendering it as `0 / 0` would report a green run as an empty one.
 */
export function verificationCountsField(record: ExecutionRecord): RecordField {
  if (record.kind !== "verification") return NOT_APPLICABLE;
  if (record.passedCount === undefined && record.failedCount === undefined) return NOT_OBSERVED;
  return {
    kind: "i18n",
    key: "executionRecords.field.verificationCounts",
    values: { failed: record.failedCount ?? 0, passed: record.passedCount ?? 0 },
  };
}

/**
 * Fidelity, exactly as the producer reported it.
 *
 * Never upgraded. An `inferred` row promoted to `native` because it happens to carry a run id
 * would claim the runtime watched work it only heard about.
 */
export function fidelityKey(record: ExecutionRecord): string {
  return `executionRecords.fidelity.${record.fidelity}`;
}

export function statusKey(record: ExecutionRecord): string {
  return `executionRecords.status.${record.status}`;
}

export function absenceKey(reason: RecordFieldAbsence): string {
  return `executionRecords.absent.${reason}`;
}
