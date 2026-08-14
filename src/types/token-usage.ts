export type UsagePurpose =
  | "assistant-initial"
  | "tool-continuation"
  | "context-compaction"
  | "memory-extraction"
  | "retry"
  | "terminal-interval";

export type UsageQuality = "reported" | "reported-derived" | "estimated";
export type UsageStatus = "running" | "succeeded" | "failed" | "cancelled";
export type UsageUnit = "tokens" | "characters";
export type UsageInteractionKind = "managed-cli" | "terminal-cli" | "native-api";
export type UsageMeasurementKind = "interval" | "cumulative-snapshot";
export type TokenOverlap = "subset" | "exclusive" | "unknown";

export interface TokenUsageFilters {
  sessionId?: string;
  agentId?: string;
  providerId?: string;
  modelId?: string;
  purpose?: UsagePurpose;
  quality?: UsageQuality;
  status?: UsageStatus;
}

export interface TokenUsageSummaryQuery extends TokenUsageFilters {
  messageId?: string;
  generationId?: string;
  rangeStart?: string;
  rangeEnd?: string;
  breakdownLimit?: number;
}

export interface TokenUsageDetailsQuery extends TokenUsageFilters {
  afterId?: string;
  limit?: number;
}

export interface TokenDimensions {
  input: number;
  output: number;
  cachedInput: number;
  cacheWriteInput: number;
  reasoningOutput: number;
  providerTotal: number | null;
}

export interface UsageMeasure {
  unit: UsageUnit;
  dimensions: TokenDimensions;
  headlineTotal: number | null;
  callCount: number;
  observationCount: number;
}

export interface UsageQualityTotals {
  reported: UsageMeasure;
  reportedDerived: UsageMeasure;
  estimated: UsageMeasure;
}

export interface UsageEntityCounts {
  calls: number;
  generations: number;
  sessions: number;
}

export interface UsageDailyPoint {
  localDate: string;
  totals: UsageQualityTotals;
  counts: UsageEntityCounts;
}

export type UsageBreakdownDimension =
  | "agent"
  | "provider"
  | "model"
  | "purpose"
  | "quality"
  | "status";

export interface UsageBreakdownEntry {
  key: string;
  totals: UsageQualityTotals;
  counts: UsageEntityCounts;
}

export interface UsageBreakdown {
  dimension: UsageBreakdownDimension;
  entries: UsageBreakdownEntry[];
}

export interface TokenUsageSummary {
  schemaVersion: 1;
  totals: UsageQualityTotals;
  userResponse: UsageQualityTotals;
  internal: UsageQualityTotals;
  counts: UsageEntityCounts;
  daily: UsageDailyPoint[];
  breakdowns: UsageBreakdown[];
  generatedAt: string;
}

export interface ModelInvocation {
  id: string;
  generationId: string | null;
  runId: string | null;
  operationId: string | null;
  sessionId: string;
  messageId: string | null;
  agentId: string;
  providerId: string | null;
  profileId: string | null;
  endpointId: string | null;
  modelId: string | null;
  interactionKind: UsageInteractionKind;
  purpose: UsagePurpose;
  requestSequence: number;
  attempt: number;
  status: UsageStatus;
  startedAt: string;
  completedAt: string | null;
}

export interface UsageObservation {
  id: string;
  invocationId: string;
  quality: UsageQuality;
  unit: UsageUnit;
  measurementKind: UsageMeasurementKind;
  dimensions: TokenDimensions;
  cacheOverlap: TokenOverlap;
  reasoningOverlap: TokenOverlap;
  normalizationVersion: string;
  source: string;
  sourceRevision: string | null;
  eventAt: string | null;
  observedAt: string;
}

export interface TokenUsageDetailsPage {
  schemaVersion: 1;
  invocations: ModelInvocation[];
  observations: UsageObservation[];
  nextCursor: string | null;
}
