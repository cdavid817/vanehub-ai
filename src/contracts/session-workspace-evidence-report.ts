import { z } from "zod";
import {
  evidenceAgentIdSchema,
  evidenceRunIdSchema,
  evidenceSeatIdSchema,
  evidenceSessionIdSchema,
} from "./session-workspace-evidence-ids";
import {
  evidenceCoverageStateSchema,
  queryCoverageSchema,
  workspaceEvidenceTargetSchema,
} from "./session-workspace-evidence-core";

export const reportGroupBySchema = z.enum(["run", "agent", "seat", "model", "tool"]);

const reportSectionCoverageSchema = z.object({
  state: evidenceCoverageStateSchema,
  reasonCodes: z.array(z.string()),
});

export const reportCoverageSchema = z.object({
  overall: evidenceCoverageStateSchema,
  sections: z.object({
    overview: reportSectionCoverageSchema,
    usage: reportSectionCoverageSchema,
    latency: reportSectionCoverageSchema,
    agents: reportSectionCoverageSchema,
    tools: reportSectionCoverageSchema,
    commands: reportSectionCoverageSchema,
    changes: reportSectionCoverageSchema,
    verification: reportSectionCoverageSchema,
    failures: reportSectionCoverageSchema,
  }),
});

/**
 * `costAvailable` is pinned to `false` rather than made optional. This change introduces no
 * provider pricing catalogue, so a backend that started sending a cost would be sending something
 * no versioned pricing observation backs; the literal makes that a parse failure instead of a
 * number the Report tab would happily display.
 */
const sessionUsageReportSchema = z.object({
  reportedInputTokens: z.number().int().nonnegative().optional(),
  reportedOutputTokens: z.number().int().nonnegative().optional(),
  reportedDerivedTokens: z.number().int().nonnegative().optional(),
  estimatedCharacters: z.number().int().nonnegative().optional(),
  responseCount: z.number().int().nonnegative(),
  internalPurposeResponseCount: z.number().int().nonnegative(),
  coverage: reportSectionCoverageSchema,
  costAvailable: z.literal(false),
});

export const sessionRunReportSchema = z.object({
  scope: z.object({
    sessionId: evidenceSessionIdSchema,
    runIds: z.array(evidenceRunIdSchema),
    seatIds: z.array(evidenceSeatIdSchema),
    from: z.string().optional(),
    to: z.string().optional(),
    groupBy: reportGroupBySchema,
  }),
  generatedAt: z.string(),
  coverage: reportCoverageSchema,
  overview: z.object({
    runCount: z.number().int().nonnegative(),
    durationMs: z.number().int().nonnegative().optional(),
    succeeded: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
    cancelled: z.number().int().nonnegative(),
    retries: z.number().int().nonnegative(),
  }),
  usage: sessionUsageReportSchema,
  latency: z.object({
    p50Ms: z.number().int().nonnegative().optional(),
    p95Ms: z.number().int().nonnegative().optional(),
    slowestRecordDurationMs: z.number().int().nonnegative().optional(),
  }),
  agents: z.array(z.object({
    agentId: evidenceAgentIdSchema.optional(),
    seatId: evidenceSeatIdSchema.optional(),
    runCount: z.number().int().nonnegative(),
    failedCount: z.number().int().nonnegative(),
    durationMs: z.number().int().nonnegative().optional(),
  })),
  tools: z.array(z.object({
    toolName: z.string(),
    invocations: z.number().int().nonnegative(),
    failures: z.number().int().nonnegative(),
    durationMs: z.number().int().nonnegative().optional(),
  })),
  commands: z.object({
    total: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
    running: z.number().int().nonnegative(),
    durationMs: z.number().int().nonnegative().optional(),
  }),
  changes: z.object({
    changedFiles: z.number().int().nonnegative(),
    // Optional: nothing records per-file review progress, and a required zero here would assert
    // that every changed file had been reviewed.
    unviewedFiles: z.number().int().nonnegative().optional(),
    unresolvedFindings: z.number().int().nonnegative(),
  }),
  verification: z.object({
    passed: z.number().int().nonnegative(),
    failed: z.number().int().nonnegative(),
    skipped: z.number().int().nonnegative(),
  }),
  failures: z.object({
    rows: z.array(z.object({
      reasonCode: z.string(),
      count: z.number().int().nonnegative(),
      target: workspaceEvidenceTargetSchema.optional(),
    })),
  }),
  evidenceLinks: z.array(workspaceEvidenceTargetSchema),
  sourceCoverage: queryCoverageSchema,
});
