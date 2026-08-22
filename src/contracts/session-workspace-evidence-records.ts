import { z } from "zod";
import {
  evidenceAgentIdSchema,
  evidenceCommandIdSchema,
  evidenceOperationIdSchema,
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceToolCallIdSchema,
  evidenceTraceIdSchema,
} from "./session-workspace-evidence-ids";
import {
  evidenceFidelitySchema,
  evidenceStatusSchema,
  queryCoverageSchema,
} from "./session-workspace-evidence-core";

const executionRecordBaseShape = {
  id: evidenceRecordIdSchema,
  sessionId: evidenceSessionIdSchema,
  runId: evidenceRunIdSchema.optional(),
  traceId: evidenceTraceIdSchema.optional(),
  spanId: evidenceSpanIdSchema.optional(),
  operationId: evidenceOperationIdSchema.optional(),
  agentId: evidenceAgentIdSchema.optional(),
  seatId: evidenceSeatIdSchema.optional(),
  startedAt: z.string(),
  endedAt: z.string().optional(),
  durationMs: z.number().int().nonnegative().optional(),
  status: evidenceStatusSchema,
  fidelity: evidenceFidelitySchema,
  coverage: queryCoverageSchema,
};

export const commandExecutionRecordSchema = z.object({
  ...executionRecordBaseShape,
  kind: z.literal("command"),
  commandId: evidenceCommandIdSchema,
  runtimeKind: z.enum(["local-shell", "remote-shell", "process", "unknown"]),
  redactedDisplay: z.string().max(2048).optional(),
  cwdDisplay: z.string().optional(),
  exitCode: z.number().int().optional(),
  signal: z.string().optional(),
  outputAvailability: z.enum(["merged", "separate", "unavailable", "redacted"]),
  outputTruncated: z.boolean(),
});

export const toolExecutionRecordSchema = z.object({
  ...executionRecordBaseShape,
  kind: z.literal("tool"),
  toolCallId: evidenceToolCallIdSchema.optional(),
  toolName: z.string(),
  source: z.enum(["native", "message-history"]),
});

export const delegationExecutionRecordSchema = z.object({
  ...executionRecordBaseShape,
  kind: z.literal("delegation"),
  parentAgentId: evidenceAgentIdSchema.optional(),
  childAgentId: evidenceAgentIdSchema.optional(),
  attempt: z.number().int().positive().optional(),
});

export const verificationExecutionRecordSchema = z.object({
  ...executionRecordBaseShape,
  kind: z.literal("verification"),
  verificationName: z.string(),
  outcome: z.enum(["passed", "failed", "skipped", "unknown"]),
  passedCount: z.number().int().nonnegative().optional(),
  failedCount: z.number().int().nonnegative().optional(),
});

export const legacyActivityRecordSchema = z.object({
  ...executionRecordBaseShape,
  kind: z.literal("legacy"),
  label: z.string(),
  source: z.literal("message-history"),
  messageId: z.string(),
});

/**
 * Fails closed on an unrecognised `kind`.
 *
 * A record whose kind this build does not know is not a record with missing fields — it is a shape
 * whose meaning is unknown, and rendering it as the nearest familiar kind would attribute fields
 * that may mean something else. The row is rejected so the caller reports reduced coverage rather
 * than a confidently wrong row.
 */
export const executionRecordSchema = z.discriminatedUnion("kind", [
  commandExecutionRecordSchema,
  toolExecutionRecordSchema,
  delegationExecutionRecordSchema,
  verificationExecutionRecordSchema,
  legacyActivityRecordSchema,
]);

export const executionRecordDetailSchema = z.object({
  record: executionRecordSchema,
  relatedCounts: z.object({
    logs: z.number().int().nonnegative(),
    commands: z.number().int().nonnegative(),
    files: z.number().int().nonnegative(),
    findings: z.number().int().nonnegative(),
    usageObservations: z.number().int().nonnegative(),
  }),
  safeAttributes: z.record(z.string(), z.string()),
  errorReasonCode: z.string().optional(),
});
