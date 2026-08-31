import { folderNameFromPath, normalizeDisplayPath } from "../lib/session-path";
import type { Session, SessionCategory, SessionLifecycleState, SessionSearchResult } from "../types/agent";

export type SessionAgentFilter = "all" | "claude-code" | "opencode" | "codex-cli" | "gemini-cli" | "antigravity-cli";
export type SessionPresentationMode = "list" | "category" | "project";
export type SessionSourceMode = "active" | "archived";

export interface SessionProjectGroup {
  id: string;
  label: string;
  path: string | null;
  sessions: Session[];
}

export interface SessionCategoryGroup {
  id: string | null;
  label: string;
  sessions: Session[];
}

const ungroupedProjectKey = "project:none";

/**
 * 7.3's spec'd six-tier order is needs-input, pending-verification-or-approval, running, pinned,
 * recent, remaining — but `Session` (types/agent.ts) has no per-session field for either of the
 * first two: those are live `TurnStatusEvent` concepts (services/turn-status.ts) delivered only
 * through a session's own active chat subscription, not present in the `Session[]` array this
 * sidebar receives for every session at once, and `agentService` has no bulk/aggregate query for
 * them either (confirmed by search) — unlike Mission Control's runs, which the backend itself
 * pre-buckets into an attention list. `recoveryStatus === "action_required"` is the one field
 * that IS present on every `Session` and genuinely means "this needs your review" (it drives the
 * existing `recovery.action_required.*` UI) — combining both spec'd tiers into it is an honest,
 * available-data-shaped approximation, not a claim that the two are distinguished.
 */
export type SessionAttentionTier = "needs-attention" | "running" | "pinned" | "recent" | "remaining";

/** Half of a typical work week — long enough that a session opened yesterday still reads as
 *  "recent" after a day off, short enough that a session from last quarter does not. */
const RECENT_WINDOW_MS = 7 * 24 * 60 * 60 * 1000;

const ATTENTION_TIER_ORDER: Record<SessionAttentionTier, number> = {
  "needs-attention": 0,
  running: 1,
  pinned: 2,
  recent: 3,
  remaining: 4,
};

export function sessionAttentionTier(session: Session, now: number): SessionAttentionTier {
  if (session.recoveryStatus === "action_required") return "needs-attention";
  if (session.lifecycleState === "running" || session.lifecycleState === "starting") return "running";
  if (session.pinned) return "pinned";
  const ageMs = now - Date.parse(session.updatedAt);
  return Number.isFinite(ageMs) && ageMs <= RECENT_WINDOW_MS ? "recent" : "remaining";
}

/** Stable within a tier: most-recently-updated first, ties broken by the incoming array order
 *  (`Array.prototype.sort` is spec-guaranteed stable since ES2019). */
export function sortSessionsByAttention(sessions: Session[], now: number): Session[] {
  return [...sessions].sort((a, b) => {
    const tierDiff = ATTENTION_TIER_ORDER[sessionAttentionTier(a, now)] - ATTENTION_TIER_ORDER[sessionAttentionTier(b, now)];
    if (tierDiff !== 0) return tierDiff;
    return Date.parse(b.updatedAt) - Date.parse(a.updatedAt);
  });
}

export interface SessionAttentionGroup {
  tier: SessionAttentionTier;
  sessions: Session[];
}

/** Consecutive same-tier runs collapsed into one group, in the sorted order — never one entry per
 *  tier unconditionally, so an empty tier (e.g. no session currently needs attention) contributes
 *  no empty section for a caller to have to filter back out. */
export function groupSessionsByAttentionTier(sessions: Session[], now: number): SessionAttentionGroup[] {
  const sorted = sortSessionsByAttention(sessions, now);
  const groups: SessionAttentionGroup[] = [];
  for (const session of sorted) {
    const tier = sessionAttentionTier(session, now);
    const last = groups.at(-1);
    if (last?.tier === tier) last.sessions.push(session);
    else groups.push({ tier, sessions: [session] });
  }
  return groups;
}

export function groupSessionsByCategory(sessions: Session[], categories: SessionCategory[], uncategorizedLabel: string): SessionCategoryGroup[] {
  return [
    ...categories.map((category) => ({ id: category.id, label: category.name, sessions: sessions.filter((session) => session.categoryId === category.id) })),
    { id: null, label: uncategorizedLabel, sessions: sessions.filter((session) => !session.categoryId) },
  ];
}

export const sessionAgentFilters: SessionAgentFilter[] = ["all", "claude-code", "opencode", "codex-cli", "gemini-cli", "antigravity-cli"];

export function filterSessionsByAgent(sessions: Session[], agentFilter: SessionAgentFilter): Session[] {
  if (agentFilter === "all") return sessions;
  return sessions.filter((session) =>
    session.agentId === agentFilter || session.seats?.some((seat) => seat.leftAt == null && seat.agentId === agentFilter),
  );
}

export function filterSearchResultsByAgent(results: SessionSearchResult[], agentFilter: SessionAgentFilter, sourceMode: SessionSourceMode): SessionSearchResult[] {
  return results.filter((result) => {
    const sourceMatches = sourceMode === "archived" ? result.session.archived : !result.session.archived;
    const agentMatches = agentFilter === "all" || result.session.agentId === agentFilter ||
      result.session.seats?.some((seat) => seat.leftAt == null && seat.agentId === agentFilter);
    return sourceMatches && agentMatches;
  });
}

export function pruneSelectionToVisible(selectedIds: Set<string>, visibleSessions: Session[]): Set<string> {
  const visibleIds = new Set(visibleSessions.map((session) => session.id));
  let changed = false;
  const next = new Set<string>();
  selectedIds.forEach((id) => {
    if (visibleIds.has(id)) next.add(id);
    else changed = true;
  });
  return changed ? next : selectedIds;
}

export function getSessionProjectGroupKey(session: Session): string {
  const path = session.worktreePath ?? session.remoteWorkspace?.uri ?? session.projectPath ?? session.folder;
  const normalized = path?.trim() ? normalizeDisplayPath(path.trim()) : null;
  return normalized ? `project:${normalized}` : ungroupedProjectKey;
}

export function getSessionProjectGroupLabel(session: Session, ungroupedLabel: string): string {
  if (session.worktreeName?.trim()) return session.worktreeName.trim();
  if (session.remoteWorkspace?.displayName.trim()) return session.remoteWorkspace.displayName.trim();
  const path = session.worktreePath ?? session.projectPath ?? session.folder;
  if (!path?.trim()) return ungroupedLabel;
  const normalized = normalizeDisplayPath(path.trim());
  return folderNameFromPath(normalized) || normalized;
}

export type SessionStatusFilter = "all" | SessionLifecycleState;
export const sessionStatusFilters: SessionStatusFilter[] = ["all", "idle", "starting", "running", "failed", "stopped"];

export function filterSessionsByStatus(sessions: Session[], statusFilter: SessionStatusFilter): Session[] {
  if (statusFilter === "all") return sessions;
  return sessions.filter((session) => session.lifecycleState === statusFilter);
}

export type SessionSourceFilter = "all" | "desktop" | "im";
export const sessionSourceFilters: SessionSourceFilter[] = ["all", "desktop", "im"];

/** A session predates `source` (optional field) as often as not — absent means the original,
 *  desktop-native kind every session was before IM integration existed, not "unknown." */
export function filterSessionsBySource(sessions: Session[], sourceFilter: SessionSourceFilter): Session[] {
  if (sourceFilter === "all") return sessions;
  return sessions.filter((session) => (session.source?.kind ?? "desktop") === sourceFilter);
}

export type SessionDateFilter = "all" | "today" | "week" | "month";
export const sessionDateFilters: SessionDateFilter[] = ["all", "today", "week", "month"];

const DATE_FILTER_WINDOW_MS: Record<Exclude<SessionDateFilter, "all">, number> = {
  today: 24 * 60 * 60 * 1000,
  week: 7 * 24 * 60 * 60 * 1000,
  month: 30 * 24 * 60 * 60 * 1000,
};

/** No dedicated date-range picker (7.5 only asks for "date filters," not a calendar widget) —
 *  three relative presets against `updatedAt`, the same field the attention sort already reads. */
export function filterSessionsByDate(sessions: Session[], dateFilter: SessionDateFilter, now: number): Session[] {
  if (dateFilter === "all") return sessions;
  const windowMs = DATE_FILTER_WINDOW_MS[dateFilter];
  return sessions.filter((session) => {
    const ageMs = now - Date.parse(session.updatedAt);
    return Number.isFinite(ageMs) && ageMs >= 0 && ageMs <= windowMs;
  });
}

export const ALL_PROJECTS_FILTER = "all";

/** Options are derived from whichever sessions are currently visible, not a static enum — unlike
 *  agent/status/source, "project" has no fixed vocabulary to declare up front. */
export function sessionProjectFilterOptions(sessions: Session[], ungroupedLabel: string): { value: string; label: string }[] {
  const seen = new Map<string, string>();
  sessions.forEach((session) => {
    const key = getSessionProjectGroupKey(session);
    if (!seen.has(key)) seen.set(key, getSessionProjectGroupLabel(session, ungroupedLabel));
  });
  return [...seen.entries()].map(([value, label]) => ({ value, label }));
}

export function filterSessionsByProject(sessions: Session[], projectFilter: string): Session[] {
  if (projectFilter === ALL_PROJECTS_FILTER) return sessions;
  return sessions.filter((session) => getSessionProjectGroupKey(session) === projectFilter);
}

export function groupSessionsByProject(sessions: Session[], ungroupedLabel: string): SessionProjectGroup[] {
  const groups: SessionProjectGroup[] = [];
  const byKey = new Map<string, SessionProjectGroup>();
  sessions.forEach((session) => {
    const id = getSessionProjectGroupKey(session);
    let group = byKey.get(id);
    if (!group) {
      const path = id === ungroupedProjectKey ? null : id.slice("project:".length);
      group = {
        id,
        label: getSessionProjectGroupLabel(session, ungroupedLabel),
        path,
        sessions: [],
      };
      byKey.set(id, group);
      groups.push(group);
    }
    group.sessions.push(session);
  });
  return groups;
}
