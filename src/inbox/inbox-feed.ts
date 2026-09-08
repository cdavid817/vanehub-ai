import type { SystemActivityTimelineEntry } from "../services/system-activity-service";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";

export type InboxViewMode = "list" | "board";

const viewModeStorageKey = "vanehub.inbox.view-mode.v1";

/** One unread System Activity entry together with the system session it belongs to. */
export interface InboxActivityItem {
  sessionId: string;
  entry: SystemActivityTimelineEntry;
}

/** What one session's unread activity was fetched for, so an unchanged session is not refetched. */
export interface InboxActivityCacheEntry {
  signature: string;
  items: InboxActivityItem[];
}

export interface InboxFeed {
  /** Mission Control overview with every fetched attention page merged into `attention.items`. */
  overview: MissionControlOverview | null;
  /** Unread warning/critical entries across visible system sessions, newest first per session. */
  activity: InboxActivityItem[];
  /** Per-session cache behind `activity`; a session that failed to load has no entry. */
  activityCache: Record<string, InboxActivityCacheEntry>;
  /** Sum of per-session unread counts across all severities. */
  unreadTotal: number;
}

export interface InboxSections {
  attention: { runs: MissionControlRunSummary[]; activity: InboxActivityItem[] };
  running: MissionControlRunSummary[];
  recent: MissionControlRunSummary[];
}

export const emptyInboxFeed: InboxFeed = { overview: null, activity: [], activityCache: {}, unreadTotal: 0 };

/** The run an activity entry is about, when the envelope names one; used to count it once. */
export function inboxActivityRunId(entry: SystemActivityTimelineEntry): string | null {
  const navigation = entry.envelope.navigation;
  return navigation?.kind === "run" ? navigation.stableId : null;
}

export function classifyInboxFeed(feed: InboxFeed): InboxSections {
  const overview = feed.overview;
  return {
    attention: {
      runs: overview?.attention.items ?? [],
      activity: feed.activity.filter(({ entry }) => entry.envelope.severity === "warning" || entry.envelope.severity === "critical"),
    },
    running: overview?.active.items ?? [],
    recent: overview?.recent.items ?? [],
  };
}

/**
 * Attention runs plus unread activity, counting a run once even when both surfaces report it.
 * The unread total is used rather than the fetched entries so a session with more unread items
 * than one page still contributes all of them; only fetched entries can be matched to runs.
 */
export function countInboxBadge(feed: InboxFeed): number {
  const attentionRunIds = new Set((feed.overview?.attention.items ?? []).map((run) => run.runId));
  const overlap = feed.activity.filter(({ entry }) => {
    const runId = inboxActivityRunId(entry);
    return runId !== null && attentionRunIds.has(runId);
  }).length;
  return attentionRunIds.size + Math.max(0, feed.unreadTotal - overlap);
}

export function readInboxViewMode(): InboxViewMode {
  if (typeof localStorage === "undefined") return "list";
  return localStorage.getItem(viewModeStorageKey) === "board" ? "board" : "list";
}

export function rememberInboxViewMode(mode: InboxViewMode): void {
  if (typeof localStorage !== "undefined") localStorage.setItem(viewModeStorageKey, mode);
}
