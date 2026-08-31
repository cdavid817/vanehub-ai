import { z } from "zod";
import {
  evidenceCommandIdSchema,
  evidenceCursorSchema,
  evidenceOperationIdSchema,
  evidenceRecordIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
  evidenceSpanIdSchema,
  evidenceTraceIdSchema,
} from "./session-workspace-evidence-ids";

export const evidenceFidelitySchema = z.enum(["native", "proxied", "inferred", "opaque"]);

export const evidenceStatusSchema = z.enum([
  "queued",
  "running",
  "succeeded",
  "failed",
  "cancelled",
  "incomplete",
]);

export const evidenceCoverageStateSchema = z.enum([
  "complete",
  "indexing",
  "partial",
  "unavailable",
]);

export const queryCoverageSchema = z.object({
  state: evidenceCoverageStateSchema,
  reasonCodes: z.array(z.string()),
  oldestAvailableAt: z.string().optional(),
  newestAvailableAt: z.string().optional(),
  indexedThroughAt: z.string().optional(),
  droppedCount: z.number().int().nonnegative().optional(),
  truncated: z.boolean(),
});

/**
 * Objects are parsed non-strictly, which is deliberate. A backend that adds a field must not break
 * a frontend that has not learned about it yet; unknown keys are dropped rather than rejected.
 * Discriminated unions are the exception — see `session-workspace-evidence-records.ts`.
 */
export function cursorPageSchema<Item extends z.ZodTypeAny>(item: Item) {
  return z.object({
    items: z.array(item),
    nextCursor: evidenceCursorSchema.optional(),
    coverage: queryCoverageSchema,
  });
}

export const workspaceEvidenceScopeSchema = z.object({
  sessionId: evidenceSessionIdSchema,
  seatId: evidenceSeatIdSchema.optional(),
  runId: evidenceRunIdSchema.optional(),
  traceId: evidenceTraceIdSchema.optional(),
  spanId: evidenceSpanIdSchema.optional(),
  operationId: evidenceOperationIdSchema.optional(),
  commandId: evidenceCommandIdSchema.optional(),
  relativePath: z.string().optional(),
  hunkFingerprint: z.string().optional(),
  occurredAt: z.string().optional(),
});

export const workspaceEvidenceTabIdSchema = z.enum([
  "terminal-history",
  "shell",
  "logs",
  "traces",
  "changes",
  "files",
  "report",
]);

export const workspaceEvidenceTargetSchema = z.object({
  tab: workspaceEvidenceTabIdSchema,
  scope: workspaceEvidenceScopeSchema,
  focus: z.enum(["row", "detail", "filter", "timestamp"]).optional(),
});

export const workspaceEvidenceSummarySchema = z.object({
  sessionId: evidenceSessionIdSchema,
  generatedAt: z.string(),
  coverage: queryCoverageSchema,
  runState: z.object({
    status: evidenceStatusSchema,
    runId: evidenceRunIdSchema.optional(),
    startedAt: z.string().optional(),
  }),
  changes: z.object({
    changedFiles: z.number().int().nonnegative(),
    unviewedFiles: z.number().int().nonnegative(),
  }),
  executionRecords: z.object({
    running: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
  }),
  shells: z.object({ live: z.number().int().nonnegative() }),
  logs: z.object({ newErrors: z.number().int().nonnegative() }),
  traces: z.object({
    running: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
  }),
  verification: z.object({
    passed: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
  }),
  usage: z.object({
    reportedTokens: z.number().int().nonnegative().optional(),
    coverage: evidenceCoverageStateSchema,
  }),
});

export const evidenceSubscriptionBootstrapSchema = z.object({
  sessionId: evidenceSessionIdSchema,
  watermarkSequence: z.number().int().nonnegative(),
  coverage: queryCoverageSchema,
});

export const executionEvidenceNoticeSchema = z.object({
  kind: z.enum(["record-appended", "record-updated", "summary-changed", "coverage-gap"]),
  sequence: z.number().int().nonnegative(),
  sessionId: evidenceSessionIdSchema,
  occurredAt: z.string(),
  recordId: evidenceRecordIdSchema.optional(),
  runId: evidenceRunIdSchema.optional(),
  traceId: evidenceTraceIdSchema.optional(),
  spanId: evidenceSpanIdSchema.optional(),
  operationId: evidenceOperationIdSchema.optional(),
  commandId: evidenceCommandIdSchema.optional(),
  seatId: evidenceSeatIdSchema.optional(),
  droppedCount: z.number().int().nonnegative().optional(),
});
