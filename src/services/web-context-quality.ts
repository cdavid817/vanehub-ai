import type {
  ContextQualityAssessment,
  ContextQualityHistoryPage,
  ContextQualityHistoryQuery,
  ContextQualityRangeDays,
  ContextQualitySummary,
  ContextQualitySummaryQuery,
} from "../types/context-quality";
import { contextQualityRangeDaysOptions } from "../types/context-quality";
import { readWebAppSettings } from "./web-settings-client";
import { ContextQualityServiceError } from "./context-quality-error";

export { ContextQualityServiceError } from "./context-quality-error";

const HARD_LIMIT = 10_000;
const DEFAULT_LIMIT = 25;
const MAX_LIMIT = 100;
const DAY_MS = 86_400_000;

function fixture(offsetDays: number, index: number): ContextQualityAssessment {
  const outcomes = ["compacted", "fallback", "bypassed", "failed"] as const;
  const outcome = outcomes[index % outcomes.length];
  const tokenMeasured = index % 3 !== 2;
  return {
    version: "onepiece-context-quality-assessment-v1",
    attemptId: `web-context-quality-${String(index + 1).padStart(2, "0")}`,
    sessionCorrelation: `web-session-${(index % 3) + 1}`,
    decisionSequence: index + 1,
    recordedAt: new Date(Date.now() - offsetDays * DAY_MS).toISOString(),
    outcome,
    path: outcome === "bypassed" ? null : index % 2 === 0 ? "optimizer" : "compatibility",
    reason: outcome === "bypassed" ? "user-preference-suppressed" : outcome === "fallback" ? "verification-failed" : outcome === "failed" ? "provider-failure" : null,
    triggerSource: index % 2 === 0 ? "token-aware" : "character-fallback",
    beforeCharacters: 12_000 + index * 1_000,
    afterCharacters: 7_000 + index * 500,
    savedCharacters: 5_000 + index * 500,
    beforeTokens: tokenMeasured ? 3_000 + index * 200 : null,
    afterTokens: tokenMeasured ? 1_800 + index * 100 : null,
    savedTokens: tokenMeasured ? 1_200 + index * 100 : null,
    measurementQuality: tokenMeasured ? (index % 2 === 0 ? "reported" : "estimated") : "characters-only",
    invariants: outcome === "compacted" ? { protocolComplete: true, protectedRetained: true, verbatimRetained: true, reinjectionComplete: true } : null,
    contextPolicyVersion: "onepiece-context-policy-v1",
    optimizerVersion: "onepiece-context-optimizer-v1",
    verifierVersion: "onepiece-context-verifier-v1",
  };
}

const createLedger = () => [1, 4, 10, 25, 45, 80].map(fixture);
let ledger = createLedger();

export function resetWebContextQualityForTest(): void {
  ledger = createLedger();
}

function rangeDays(value: number): ContextQualityRangeDays {
  if (!contextQualityRangeDaysOptions.some((candidate) => candidate === value)) {
    throw new ContextQualityServiceError("invalid-range", "Context quality range must be 7, 30, or 90 days.");
  }
  return value as ContextQualityRangeDays;
}

function retainedRecords(range: ContextQualityRangeDays): ContextQualityAssessment[] {
  const retention = readWebAppSettings().contextQualityRetentionDays;
  const retentionCutoff = Date.now() - retention * DAY_MS;
  ledger = ledger.filter((record) => Date.parse(record.recordedAt) >= retentionCutoff).slice(0, HARD_LIMIT);
  const cutoff = Date.now() - range * DAY_MS;
  return ledger.filter((record) => Date.parse(record.recordedAt) >= cutoff);
}

export function listWebContextQualityHistory(input: ContextQualityHistoryQuery): ContextQualityHistoryPage {
  const records = retainedRecords(rangeDays(input.rangeDays));
  const limit = input.limit ?? DEFAULT_LIMIT;
  if (!Number.isInteger(limit) || limit < 1 || limit > MAX_LIMIT) {
    throw new ContextQualityServiceError("invalid-cursor", "Context quality page size must be between 1 and 100.");
  }
  const start = input.cursor == null ? 0 : records.findIndex((record) => record.attemptId === input.cursor) + 1;
  if (input.cursor != null && start === 0) {
    throw new ContextQualityServiceError("invalid-cursor", "Context quality cursor is invalid.");
  }
  const items = records.slice(start, start + limit);
  return { items, nextCursor: start + limit < records.length ? items.at(-1)?.attemptId ?? null : null };
}

function counts(values: Array<string | null>): Record<string, number> {
  return values.reduce<Record<string, number>>((result, value) => {
    if (value != null) result[value] = (result[value] ?? 0) + 1;
    return result;
  }, {});
}

export function getWebContextQualitySummary(input: ContextQualitySummaryQuery): ContextQualitySummary {
  const range = rangeDays(input.rangeDays);
  const records = retainedRecords(range);
  const tokenMeasurementCount = records.filter((record) => record.savedTokens != null).length;
  const timestamps = records.map((record) => record.recordedAt);
  return {
    rangeDays: range,
    evaluated: records.length,
    savedCharacters: records.reduce((sum, record) => sum + record.savedCharacters, 0),
    savedTokens: records.reduce((sum, record) => sum + (record.savedTokens ?? 0), 0),
    tokenMeasurementCount,
    qualityCoverage: {
      measuredWithTokens: tokenMeasurementCount,
      charactersOnly: records.length - tokenMeasurementCount,
      tokenCoverageBasisPoints: records.length === 0 ? 0 : Math.floor(tokenMeasurementCount * 10_000 / records.length),
    },
    outcomes: counts(records.map((record) => record.outcome)),
    paths: counts(records.map((record) => record.path)),
    qualities: counts(records.map((record) => record.measurementQuality)),
    reasons: counts(records.map((record) => record.reason)),
    policyVersions: counts(records.map((record) => record.contextPolicyVersion)),
    earliestRecordedAt: timestamps.at(-1) ?? null,
    latestRecordedAt: timestamps.at(0) ?? null,
  };
}
