export type ActivityScopeKind = "global" | "workspace";
export type ActivitySeverity = "info" | "warning" | "error" | "critical";
export type ActivityDigestCadence = "off" | "hourly" | "daily";
export type SystemActivityExportFormat = "json" | "markdown";

/** A read-only Skill Evolution activity session; never an Agent session. */
export interface SystemActivitySession {
  sessionId: string;
  kind: "system-activity";
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  safeDisplayIdentity: string | null;
  activeGenerationId: string;
  lastSequence: number;
  unreadCount: number;
  attentionKind: string;
  firstActivityAtMs: number;
  lastActivityAtMs: number;
  visible: boolean;
  mockProvenance?: "web_simulation";
}

/** The canonical locale-neutral envelope; titles localize at render time from these codes. */
export interface SystemActivityEnvelope {
  schemaVersion: number;
  eventId: string;
  eventCode: string;
  sourceDomain: string;
  sourceId: string;
  sourceRevision: string;
  sourceSequence: number;
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  occurredAtMs: number;
  committedAtMs: number;
  severity: ActivitySeverity;
  status: string;
  attentionKind: string;
  safeActorKind: string;
  safeIdentities: Array<{ kind: string; value: string }>;
  metrics: Record<string, number>;
  reasonCodes: string[];
  navigation: { kind: string; stableId: string; childId?: string | null } | null;
  supersedesEventId: string | null;
  payload: Record<string, unknown> | null;
  projectionPolicyVersion: number;
  contentHash: string;
}

export interface SystemActivityTimelineEntry {
  sequence: number;
  envelope: SystemActivityEnvelope;
  detailUnavailableReason: string | null;
}

export type SystemActivityTimelineResult =
  | {
      kind: "page";
      activeGenerationId: string;
      entries: SystemActivityTimelineEntry[];
      nextCursor: string | null;
      complete: boolean;
    }
  | { kind: "staleGeneration"; requestedGenerationId: string; activeGenerationId: string };

export interface SystemActivityTimelineQuery {
  sessionId: string;
  committedFromMs?: number;
  committedToMs?: number;
  severities?: ActivitySeverity[];
  sourceDomains?: string[];
  statuses?: string[];
  skillId?: string;
  runId?: string;
  curatorStates?: string[];
  attentionKinds?: string[];
  searchText?: string;
  cursor?: string;
  pageSize?: number;
}

export interface SystemActivityReadState {
  sessionId: string;
  userId: string;
  highestReadSequence: number;
  markUnreadSequence: number | null;
  unreadCount: number;
  attentionKind: string;
  revision: number;
}

export interface SystemActivityPreferences {
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  visible: boolean;
  minimumTimelineSeverity: ActivitySeverity;
  notificationThreshold: ActivitySeverity;
  digestCadence: ActivityDigestCadence;
  readRetentionDays: number;
  detailRetentionDays: number;
  exportItemLimit: number;
  exportSizeLimitBytes: number;
  revision: number;
}

export interface SystemActivityPreferenceUpdateResult {
  outcome: "updated" | "conflict";
  preferences: SystemActivityPreferences;
}

export interface SystemActivityDashboardSummary {
  materializationKind: string;
  state: Record<string, unknown>;
  lastEventId: string | null;
  updatedAtMs: number;
}

export interface SystemActivityDomainHealth {
  sourceDomain: string;
  opaqueCursor: unknown | null;
  lastSequence: number;
  lastSourceHash: string | null;
  retentionFloor: unknown | null;
  pendingCount: number;
  oldestPendingAtMs: number | null;
  gap: string | null;
  failureCode: string | null;
  revision: number;
}

export interface SystemActivityRebuildHealth {
  rebuildId: string;
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  status: string;
  processedItems: number;
  itemBudget: number;
  updatedAtMs?: number;
}

export interface SystemActivityHealth {
  leaseOwner: string | null;
  domains: SystemActivityDomainHealth[];
  lastCompletedAtMs: number | null;
  rebuilds: SystemActivityRebuildHealth[];
}

export interface SystemActivityRebuild {
  rebuildId: string;
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  shadowGenerationId: string;
  sourceSnapshotHash: string;
  status: string;
  processedItems: number;
  itemBudget: number;
  revision: number;
}

export interface SystemActivityRebuildStep {
  step: "running" | "validating" | "ready" | "needsCatchUp" | "active";
  processedItems?: number;
}

export type SystemActivityNotificationOpenResult =
  | { kind: "pending" }
  | { kind: "opened"; sessionId: string; sequence: number; readState: SystemActivityReadState };

export interface SystemActivityDigest {
  scopeKind: ActivityScopeKind;
  canonicalScopeId: string;
  cadence: Exclude<ActivityDigestCadence, "off">;
  windowStartedAtMs: number;
  windowEndsAtMs: number;
  countsByEventCode: Record<string, number>;
  highestSeverity: ActivitySeverity;
}

export interface SystemActivityExportRequest {
  exportId: string;
  query: SystemActivityTimelineQuery;
  format: SystemActivityExportFormat;
  locale: string;
  localeLabels?: Record<string, string>;
  targetPath: string;
  itemLimit?: number;
  sizeLimitBytes?: number;
}

export interface SystemActivityExportRecord {
  exportId: string;
  sessionId: string;
  generationId: string;
  format: SystemActivityExportFormat;
  itemCount: number;
  byteCount: number;
  complete: boolean;
  redactionVersion: string;
  contentHash: string;
  targetPath?: string;
  /** Exported files leave the app's automatic retention; the UI must disclose this. */
  outsideAutomaticRetention?: boolean;
}
