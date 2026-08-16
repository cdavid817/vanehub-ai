export type ContextSourceOutcome = "ready" | "warming" | "unavailable" | "timed_out" | "failed" | "cancelled";

export interface ContextEvidenceSummary {
  id: string;
  sourceKind: string;
  sourceRef: string;
  startLine: number | null;
  endLine: number | null;
  symbol: string | null;
  tokenEstimate: number;
  reasonCodes: string[];
}

export interface ContextEvidenceManifest {
  sessionId: string;
  turnId: string;
  generationId: string;
  policyVersion: string;
  evidenceBudget: number;
  occupiedTokens: number;
  selected: ContextEvidenceSummary[];
  rejected: Array<{ id: string; reasonCode: string }>;
  sourceOutcomes: Record<string, ContextSourceOutcome>;
  duplicateTokensSaved: number;
  collectionLatencyBucket: string;
  rankingLatencyBucket: string;
  compactionTriggered: boolean;
  runtime: "desktop" | "web-mock";
}

export interface ContextEvidenceManifestPage {
  items: ContextEvidenceManifest[];
  nextCursor: string | null;
}

export interface ContextEvidenceManifestQuery {
  sessionId?: string;
  cursor: string | null;
  limit: number;
}
