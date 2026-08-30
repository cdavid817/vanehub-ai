import type {
  ActivityScopeKind,
  SystemActivityDashboardSummary,
  SystemActivityDigest,
  SystemActivityExportRecord,
  SystemActivityExportRequest,
  SystemActivityExportFormat,
  SystemActivityHealth,
  SystemActivityNotificationOpenResult,
  SystemActivityPreferences,
  SystemActivityPreferenceUpdateResult,
  SystemActivityReadState,
  SystemActivityRebuild,
  SystemActivityRebuildStep,
  SystemActivitySession,
  SystemActivityTimelineQuery,
  SystemActivityTimelineResult,
} from "../types/system-activity";

export type * from "../types/system-activity";

export interface SystemActivityService {
  listSystemActivitySessions(): Promise<SystemActivitySession[]>;
  querySystemActivityTimeline(
    query: SystemActivityTimelineQuery,
  ): Promise<SystemActivityTimelineResult>;
  getSystemActivityReadState(sessionId: string): Promise<SystemActivityReadState>;
  advanceSystemActivityReadCursor(
    sessionId: string,
    throughSequence: number,
    expectedRevision: number,
  ): Promise<SystemActivityReadState>;
  markSystemActivityUnread(
    sessionId: string,
    fromSequence: number,
    expectedRevision: number,
  ): Promise<SystemActivityReadState>;
  getSystemActivityPreferences(
    scopeKind: ActivityScopeKind,
    canonicalScopeId: string,
  ): Promise<SystemActivityPreferences | null>;
  updateSystemActivityPreferences(
    preferences: SystemActivityPreferences,
  ): Promise<SystemActivityPreferenceUpdateResult>;
  getSystemActivityDashboard(
    scopeKind: ActivityScopeKind,
    canonicalScopeId: string,
  ): Promise<SystemActivityDashboardSummary[]>;
  getSystemActivityHealth(): Promise<SystemActivityHealth>;
  openSystemActivityNotification(
    requestId: string,
    visibleSequence: number,
  ): Promise<SystemActivityNotificationOpenResult>;
  dismissSystemActivityNotification(requestId: string): Promise<void>;
  claimSystemActivityDigests(): Promise<SystemActivityDigest[]>;
  beginSystemActivityRebuild(
    scopeKind: ActivityScopeKind,
    canonicalScopeId: string,
    itemBudget: number,
  ): Promise<SystemActivityRebuild>;
  advanceSystemActivityRebuild(
    rebuildId: string,
    batchLimit: number,
  ): Promise<SystemActivityRebuildStep>;
  validateSystemActivityRebuild(rebuildId: string): Promise<SystemActivityRebuildStep>;
  activateSystemActivityRebuild(rebuildId: string): Promise<SystemActivityRebuildStep>;
  cancelSystemActivityRebuild(rebuildId: string): Promise<void>;
  chooseSystemActivityExportTarget(format: SystemActivityExportFormat): Promise<string | null>;
  exportSystemActivity(request: SystemActivityExportRequest): Promise<SystemActivityExportRecord>;
}

declare module "./agent-service" {
  interface AgentService {
    listSystemActivitySessions: SystemActivityService["listSystemActivitySessions"];
    querySystemActivityTimeline: SystemActivityService["querySystemActivityTimeline"];
    getSystemActivityReadState: SystemActivityService["getSystemActivityReadState"];
    advanceSystemActivityReadCursor: SystemActivityService["advanceSystemActivityReadCursor"];
    markSystemActivityUnread: SystemActivityService["markSystemActivityUnread"];
    getSystemActivityPreferences: SystemActivityService["getSystemActivityPreferences"];
    updateSystemActivityPreferences: SystemActivityService["updateSystemActivityPreferences"];
    getSystemActivityDashboard: SystemActivityService["getSystemActivityDashboard"];
    getSystemActivityHealth: SystemActivityService["getSystemActivityHealth"];
    openSystemActivityNotification: SystemActivityService["openSystemActivityNotification"];
    dismissSystemActivityNotification: SystemActivityService["dismissSystemActivityNotification"];
    claimSystemActivityDigests: SystemActivityService["claimSystemActivityDigests"];
    beginSystemActivityRebuild: SystemActivityService["beginSystemActivityRebuild"];
    advanceSystemActivityRebuild: SystemActivityService["advanceSystemActivityRebuild"];
    validateSystemActivityRebuild: SystemActivityService["validateSystemActivityRebuild"];
    activateSystemActivityRebuild: SystemActivityService["activateSystemActivityRebuild"];
    cancelSystemActivityRebuild: SystemActivityService["cancelSystemActivityRebuild"];
    chooseSystemActivityExportTarget: SystemActivityService["chooseSystemActivityExportTarget"];
    exportSystemActivity: SystemActivityService["exportSystemActivity"];
  }
}
