import { invoke } from "@tauri-apps/api/core";
import type {
  SystemActivityService,
  SystemActivitySession,
  SystemActivityDashboardSummary,
  SystemActivityDigest,
} from "./system-activity-service";

interface SessionListPayload {
  sessions: SystemActivitySession[];
}

interface DashboardPayload {
  summaries: SystemActivityDashboardSummary[];
}

interface DigestPayload {
  digests: SystemActivityDigest[];
}

export const tauriSystemActivityClient: SystemActivityService = {
  async listSystemActivitySessions() {
    const payload = await invoke<SessionListPayload>("list_system_activity_sessions");
    return payload.sessions;
  },
  querySystemActivityTimeline(query) {
    return invoke("query_system_activity_timeline", { input: query });
  },
  getSystemActivityReadState(sessionId) {
    return invoke("get_system_activity_read_state", { sessionId });
  },
  advanceSystemActivityReadCursor(sessionId, throughSequence, expectedRevision) {
    return invoke("advance_system_activity_read_cursor", {
      sessionId,
      throughSequence,
      expectedRevision,
    });
  },
  markSystemActivityUnread(sessionId, fromSequence, expectedRevision) {
    return invoke("mark_system_activity_unread", { sessionId, fromSequence, expectedRevision });
  },
  getSystemActivityPreferences(scopeKind, canonicalScopeId) {
    return invoke("get_system_activity_preferences", { scopeKind, canonicalScopeId });
  },
  updateSystemActivityPreferences(preferences) {
    return invoke("update_system_activity_preferences", { input: preferences });
  },
  async getSystemActivityDashboard(scopeKind, canonicalScopeId) {
    const payload = await invoke<DashboardPayload>("get_system_activity_dashboard", {
      scopeKind,
      canonicalScopeId,
    });
    return payload.summaries;
  },
  getSystemActivityHealth() {
    return invoke("get_system_activity_health");
  },
  openSystemActivityNotification(requestId, visibleSequence) {
    return invoke("open_system_activity_notification", { requestId, visibleSequence });
  },
  async dismissSystemActivityNotification(requestId) {
    await invoke("dismiss_system_activity_notification", { requestId });
  },
  async claimSystemActivityDigests() {
    const payload = await invoke<DigestPayload>("claim_system_activity_digests");
    return payload.digests;
  },
  beginSystemActivityRebuild(scopeKind, canonicalScopeId, itemBudget) {
    return invoke("begin_system_activity_rebuild", { scopeKind, canonicalScopeId, itemBudget });
  },
  advanceSystemActivityRebuild(rebuildId, batchLimit) {
    return invoke("advance_system_activity_rebuild", { rebuildId, batchLimit });
  },
  validateSystemActivityRebuild(rebuildId) {
    return invoke("validate_system_activity_rebuild", { rebuildId });
  },
  activateSystemActivityRebuild(rebuildId) {
    return invoke("activate_system_activity_rebuild", { rebuildId });
  },
  async cancelSystemActivityRebuild(rebuildId) {
    await invoke("cancel_system_activity_rebuild", { rebuildId });
  },
  exportSystemActivity(request) {
    return invoke("export_system_activity", { input: request });
  },
};
