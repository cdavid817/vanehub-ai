export const contextQualityRangeDaysOptions = [7, 30, 90] as const;
export type ContextQualityRangeDays = (typeof contextQualityRangeDaysOptions)[number];

export type ContextQualityOutcome = "compacted" | "bypassed" | "fallback" | "failed";
export type ContextQualityPath = "optimizer" | "compatibility";
export type ContextQualityMeasurement =
  | "reported"
  | "reported-plus-estimated-delta"
  | "estimated"
  | "characters-only";
export type ContextQualityTriggerSource = "token-aware" | "character-fallback";
export type ContextQualityReason =
  | "request-suppressed"
  | "user-preference-suppressed"
  | "cooldown"
  | "circuit-open"
  | "invalid-plan"
  | "insufficient-reclaimable-context"
  | "reduction-failed"
  | "reinjection-unavailable"
  | "summary-failed"
  | "reconstruction-failed"
  | "verification-failed"
  | "provider-failure"
  | "persistence-failure";

export interface ContextQualityInvariants {
  protocolComplete: boolean;
  protectedRetained: boolean;
  verbatimRetained: boolean;
  reinjectionComplete: boolean;
}

export interface ContextQualityAssessment {
  version: string;
  attemptId: string;
  sessionCorrelation: string | null;
  decisionSequence: number;
  recordedAt: string;
  outcome: ContextQualityOutcome;
  path: ContextQualityPath | null;
  reason: ContextQualityReason | null;
  triggerSource: ContextQualityTriggerSource | null;
  beforeCharacters: number;
  afterCharacters: number;
  savedCharacters: number;
  beforeTokens: number | null;
  afterTokens: number | null;
  savedTokens: number | null;
  measurementQuality: ContextQualityMeasurement;
  invariants: ContextQualityInvariants | null;
  contextPolicyVersion: string;
  optimizerVersion: string;
  verifierVersion: string;
}

export interface ContextQualityHistoryQuery {
  rangeDays: ContextQualityRangeDays;
  cursor?: string | null;
  limit?: number;
}

export interface ContextQualityHistoryPage {
  items: ContextQualityAssessment[];
  nextCursor: string | null;
}

export type ContextQualityOutcomeDistribution = Partial<Record<ContextQualityOutcome, number>>;
export type ContextQualityPathDistribution = Partial<Record<ContextQualityPath, number>>;
export type ContextQualityMeasurementDistribution = Partial<
  Record<ContextQualityMeasurement, number>
>;
export type ContextQualityReasonDistribution = Partial<Record<ContextQualityReason, number>>;

export interface ContextQualityCoverage {
  measuredWithTokens: number;
  charactersOnly: number;
  tokenCoverageBasisPoints: number;
}

export interface ContextQualitySummary {
  rangeDays: ContextQualityRangeDays;
  evaluated: number;
  savedCharacters: number;
  savedTokens: number;
  tokenMeasurementCount: number;
  qualityCoverage: ContextQualityCoverage;
  outcomes: ContextQualityOutcomeDistribution;
  paths: ContextQualityPathDistribution;
  qualities: ContextQualityMeasurementDistribution;
  reasons: ContextQualityReasonDistribution;
  policyVersions: Record<string, number>;
  earliestRecordedAt: string | null;
  latestRecordedAt: string | null;
}

export interface ContextQualitySummaryQuery {
  rangeDays: ContextQualityRangeDays;
}

export type ContextQualityErrorCode = "invalid-range" | "invalid-cursor" | "unavailable";

export interface ContextQualitySafeError {
  code: ContextQualityErrorCode;
  message: string;
}
