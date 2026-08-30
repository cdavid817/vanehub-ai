import type { SystemActivityService, SystemActivityRebuild } from "./system-activity-service";
import {
  ensureWebSystemActivitySeed,
  refreshUnread,
  requireReadState,
  scopeKey,
  sessionIdFor,
  webSystemActivityState as state,
} from "./web-system-activity-state";

export const webSystemActivityClient: SystemActivityService = {
  async listSystemActivitySessions() {
    ensureWebSystemActivitySeed();
    return [...state.sessions.values()].map((session) => ({ ...session }));
  },
  async querySystemActivityTimeline(query) {
    const session = state.sessions.get(query.sessionId);
    if (!session) throw new Error("system-activity-invalid-input");
    const severities = new Set(query.severities ?? []);
    const entries = (state.entries.get(query.sessionId) ?? []).filter((entry) => {
      if (severities.size > 0 && !severities.has(entry.envelope.severity)) return false;
      if (query.committedFromMs !== undefined && entry.envelope.committedAtMs < query.committedFromMs) return false;
      if (query.committedToMs !== undefined && entry.envelope.committedAtMs > query.committedToMs) return false;
      if (query.searchText && !entry.envelope.eventCode.includes(query.searchText)) return false;
      return true;
    });
    // Newest-first, matching the desktop adapter's ORDER BY sequence DESC — adapter parity is a
    // contract, and the read cursor's MAX-monotonic semantics assume both page the same way.
    entries.sort((left, right) => right.sequence - left.sequence);
    const start = query.cursor ? Number.parseInt(query.cursor, 10) : 0;
    if (Number.isNaN(start) || start < 0) throw new Error("system-activity-invalid-input");
    const pageSize = Math.min(query.pageSize ?? 50, 100);
    const page = entries.slice(start, start + pageSize);
    const nextIndex = start + pageSize;
    return {
      kind: "page",
      activeGenerationId: session.activeGenerationId,
      entries: page.map((entry) => ({ ...entry })),
      nextCursor: nextIndex < entries.length ? String(nextIndex) : null,
      complete: nextIndex >= entries.length,
    };
  },
  async getSystemActivityReadState(sessionId) {
    requireReadState(sessionId);
    return refreshUnread(sessionId);
  },
  async advanceSystemActivityReadCursor(sessionId, throughSequence, expectedRevision) {
    const read = requireReadState(sessionId);
    if (read.revision !== expectedRevision) throw new Error("system-activity-conflict");
    // The cursor is monotonic: reading an older page never un-reads newer activity.
    read.highestReadSequence = Math.max(read.highestReadSequence, throughSequence);
    if (read.markUnreadSequence !== null && read.markUnreadSequence <= throughSequence) {
      read.markUnreadSequence = null;
    }
    read.revision += 1;
    return refreshUnread(sessionId);
  },
  async markSystemActivityUnread(sessionId, fromSequence, expectedRevision) {
    const read = requireReadState(sessionId);
    if (read.revision !== expectedRevision) throw new Error("system-activity-conflict");
    read.markUnreadSequence = Math.max(1, fromSequence);
    read.revision += 1;
    return refreshUnread(sessionId);
  },
  async getSystemActivityPreferences(scopeKind, canonicalScopeId) {
    return state.preferences.get(scopeKey(scopeKind, canonicalScopeId)) ?? null;
  },
  async updateSystemActivityPreferences(preferences) {
    const key = scopeKey(preferences.scopeKind, preferences.canonicalScopeId);
    const current = state.preferences.get(key);
    if (current && current.revision !== preferences.revision) {
      return { outcome: "conflict", preferences: { ...current } };
    }
    const updated = { ...preferences, revision: preferences.revision + 1 };
    state.preferences.set(key, updated);
    return { outcome: "updated", preferences: { ...updated } };
  },
  async getSystemActivityDashboard(scopeKind, canonicalScopeId) {
    const sessionId = sessionIdFor(scopeKind, canonicalScopeId);
    const entries = state.entries.get(sessionId) ?? [];
    if (entries.length === 0) return [];
    return [
      {
        materializationKind: "current_runs",
        state: {
          totalEvents: entries.length,
          mockProvenance: "web_simulation",
        },
        lastEventId: entries[entries.length - 1].envelope.eventId,
        updatedAtMs: entries[entries.length - 1].envelope.committedAtMs,
      },
    ];
  },
  async getSystemActivityHealth() {
    return {
      leaseOwner: "web-simulation",
      domains: [],
      lastCompletedAtMs: state.counter > 0 ? state.counter : null,
      rebuilds: [...state.rebuilds.values()].map((rebuild) => ({ ...rebuild })),
    };
  },
  async openSystemActivityNotification(requestId, visibleSequence) {
    const eventId = requestId.replace("activity-notification:", "");
    for (const [sessionId, entries] of state.entries) {
      const entry = entries.find((candidate) => candidate.envelope.eventId === eventId);
      if (!entry) continue;
      if (visibleSequence < entry.sequence) return { kind: "pending" };
      const read = requireReadState(sessionId);
      read.highestReadSequence = Math.max(read.highestReadSequence, entry.sequence);
      read.revision += 1;
      return { kind: "opened", sessionId, sequence: entry.sequence, readState: refreshUnread(sessionId) };
    }
    return { kind: "pending" };
  },
  async dismissSystemActivityNotification() {
    // Dismissal is presentation-only: no session, item, or read state changes.
  },
  async claimSystemActivityDigests() {
    return [];
  },
  async beginSystemActivityRebuild(scopeKind, canonicalScopeId, itemBudget) {
    const sessionId = sessionIdFor(scopeKind, canonicalScopeId);
    if (!state.sessions.has(sessionId)) throw new Error("system-activity-invalid-input");
    state.counter += 1;
    const rebuild: SystemActivityRebuild & { validated: boolean } = {
      rebuildId: `web-rebuild-${state.counter}`,
      scopeKind,
      canonicalScopeId,
      shadowGenerationId: `web-shadow-${state.counter}`,
      sourceSnapshotHash: `sha256:web-snapshot-${state.counter}`,
      status: "running",
      processedItems: 0,
      itemBudget,
      revision: 1,
      validated: false,
    };
    state.rebuilds.set(rebuild.rebuildId, rebuild);
    return {
      rebuildId: rebuild.rebuildId,
      scopeKind: rebuild.scopeKind,
      canonicalScopeId: rebuild.canonicalScopeId,
      shadowGenerationId: rebuild.shadowGenerationId,
      sourceSnapshotHash: rebuild.sourceSnapshotHash,
      status: rebuild.status,
      processedItems: rebuild.processedItems,
      itemBudget: rebuild.itemBudget,
      revision: rebuild.revision,
    };
  },
  async advanceSystemActivityRebuild(rebuildId) {
    const rebuild = state.rebuilds.get(rebuildId);
    if (!rebuild || rebuild.status === "cancelled") throw new Error("system-activity-conflict");
    const total = state.entries.get(sessionIdFor(rebuild.scopeKind, rebuild.canonicalScopeId))?.length ?? 0;
    if (rebuild.processedItems >= total) {
      rebuild.status = "validating";
      return { step: "validating" };
    }
    rebuild.processedItems = Math.min(total, rebuild.processedItems + 10);
    return { step: "running", processedItems: rebuild.processedItems };
  },
  async validateSystemActivityRebuild(rebuildId) {
    const rebuild = state.rebuilds.get(rebuildId);
    if (!rebuild || rebuild.status !== "validating") throw new Error("system-activity-conflict");
    rebuild.status = "ready";
    rebuild.validated = true;
    return { step: "ready" };
  },
  async activateSystemActivityRebuild(rebuildId) {
    const rebuild = state.rebuilds.get(rebuildId);
    if (!rebuild || !rebuild.validated || rebuild.status !== "ready") {
      throw new Error("system-activity-conflict");
    }
    rebuild.status = "active";
    const session = state.sessions.get(sessionIdFor(rebuild.scopeKind, rebuild.canonicalScopeId));
    if (session) session.activeGenerationId = rebuild.shadowGenerationId;
    return { step: "active" };
  },
  async cancelSystemActivityRebuild(rebuildId) {
    const rebuild = state.rebuilds.get(rebuildId);
    if (!rebuild || rebuild.status === "active") throw new Error("system-activity-conflict");
    rebuild.status = "cancelled";
  },
  async exportSystemActivity(request) {
    const result = await webSystemActivityClient.querySystemActivityTimeline(request.query);
    if (result.kind !== "page") throw new Error("system-activity-conflict");
    const limited = result.entries.slice(0, request.itemLimit ?? 1000);
    const body = JSON.stringify(limited.map((entry) => entry.envelope.contentHash));
    return {
      exportId: request.exportId,
      sessionId: request.query.sessionId,
      generationId: result.activeGenerationId,
      format: request.format,
      itemCount: limited.length,
      byteCount: body.length,
      complete: limited.length === result.entries.length && result.complete,
      redactionVersion: "activity-redaction-v1",
      contentHash: `sha256:web-${body.length}-${limited.length}`,
      targetPath: request.targetPath,
      outsideAutomaticRetention: true,
    };
  },
};
