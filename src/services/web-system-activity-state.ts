import type {
  ActivityScopeKind,
  SystemActivityEnvelope,
  SystemActivityPreferences,
  SystemActivityReadState,
  SystemActivityRebuild,
  SystemActivitySession,
  SystemActivityTimelineEntry,
} from "./system-activity-service";

/**
 * In-memory Web/mock projection state with explicit provenance. It mirrors the desktop contract —
 * lazy sessions, append-only items, read cursors, preferences, rebuild and export shapes — but
 * resets on reload and never claims durable native background processing.
 */
interface WebActivityState {
  sessions: Map<string, SystemActivitySession>;
  entries: Map<string, SystemActivityTimelineEntry[]>;
  readStates: Map<string, SystemActivityReadState>;
  preferences: Map<string, SystemActivityPreferences>;
  rebuilds: Map<string, SystemActivityRebuild & { validated: boolean }>;
  counter: number;
}

export const webSystemActivityState: WebActivityState = {
  sessions: new Map(),
  entries: new Map(),
  readStates: new Map(),
  preferences: new Map(),
  rebuilds: new Map(),
  counter: 0,
};

export function resetWebSystemActivityForTest(): void {
  webSystemActivityState.sessions.clear();
  webSystemActivityState.entries.clear();
  webSystemActivityState.readStates.clear();
  webSystemActivityState.preferences.clear();
  webSystemActivityState.rebuilds.clear();
  webSystemActivityState.counter = 0;
}

export function scopeKey(scopeKind: ActivityScopeKind, canonicalScopeId: string): string {
  return `${scopeKind}:${canonicalScopeId}`;
}

export function sessionIdFor(scopeKind: ActivityScopeKind, canonicalScopeId: string): string {
  return `system-activity-v1-web-${scopeKind}-${canonicalScopeId}`;
}

/** Simulates one committed evolution outcome projecting into the timeline. Test/demo entry. */
export function seedWebSystemActivityEventForTest(
  scopeKind: ActivityScopeKind,
  canonicalScopeId: string,
  eventCode: string,
  severity: SystemActivityEnvelope["severity"] = "info",
): SystemActivitySession {
  const state = webSystemActivityState;
  state.counter += 1;
  const sessionId = sessionIdFor(scopeKind, canonicalScopeId);
  const existing = state.sessions.get(sessionId);
  const sequence = (existing?.lastSequence ?? 0) + 1;
  const envelope: SystemActivityEnvelope = {
    schemaVersion: 1,
    eventId: `web-event-${state.counter}`,
    eventCode,
    sourceDomain: "orchestration",
    sourceId: `web-source-${state.counter}`,
    sourceRevision: "1",
    sourceSequence: state.counter,
    scopeKind,
    canonicalScopeId,
    occurredAtMs: state.counter,
    committedAtMs: state.counter,
    severity,
    status: "succeeded",
    attentionKind: severity === "error" || severity === "critical" ? "breaker" : "none",
    safeActorKind: "system",
    safeIdentities: [],
    metrics: {},
    reasonCodes: [],
    navigation: null,
    supersedesEventId: null,
    payload: null,
    projectionPolicyVersion: 1,
    contentHash: `sha256:web-${state.counter}`,
  };
  const session: SystemActivitySession = {
    sessionId,
    kind: "system-activity",
    scopeKind,
    canonicalScopeId,
    safeDisplayIdentity: canonicalScopeId,
    activeGenerationId: existing?.activeGenerationId ?? `web-generation-${sessionId}`,
    lastSequence: sequence,
    unreadCount: (existing?.unreadCount ?? 0) + 1,
    attentionKind: envelope.attentionKind,
    firstActivityAtMs: existing?.firstActivityAtMs ?? state.counter,
    lastActivityAtMs: state.counter,
    visible: true,
    mockProvenance: "web_simulation",
  };
  state.sessions.set(sessionId, session);
  const entries = state.entries.get(sessionId) ?? [];
  entries.push({ sequence, envelope, detailUnavailableReason: null });
  state.entries.set(sessionId, entries);
  return session;
}

export function requireReadState(sessionId: string): SystemActivityReadState {
  const state = webSystemActivityState;
  const session = state.sessions.get(sessionId);
  if (!session) throw new Error("system-activity-invalid-input");
  const existing = state.readStates.get(sessionId);
  if (existing) return existing;
  const created: SystemActivityReadState = {
    sessionId,
    userId: "local",
    highestReadSequence: 0,
    markUnreadSequence: null,
    unreadCount: session.unreadCount,
    attentionKind: session.attentionKind,
    revision: 1,
  };
  state.readStates.set(sessionId, created);
  return created;
}

export function refreshUnread(sessionId: string): SystemActivityReadState {
  const state = webSystemActivityState;
  const session = state.sessions.get(sessionId);
  const read = state.readStates.get(sessionId);
  if (!session || !read) throw new Error("system-activity-invalid-input");
  const effectiveRead =
    read.markUnreadSequence === null
      ? read.highestReadSequence
      : Math.min(read.highestReadSequence, read.markUnreadSequence - 1);
  read.unreadCount = Math.max(0, session.lastSequence - effectiveRead);
  session.unreadCount = read.unreadCount;
  return { ...read };
}
