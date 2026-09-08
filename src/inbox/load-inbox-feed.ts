import { agentService } from "../services/runtime-agent-client";
import type { SystemActivitySession } from "../services/system-activity-service";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import { emptyInboxFeed, type InboxActivityCacheEntry, type InboxActivityItem, type InboxFeed } from "./inbox-feed";

/** Attention pages are followed until this many runs are known; the badge caps at 99+ anyway. */
export const attentionPageLimit = 20;
export const attentionRunCap = 100;
/** Per-session detail reads that may be in flight at once. */
const detailConcurrency = 4;

export interface LoadInboxFeedOptions {
  /** Refetch every session's activity even when its signature is unchanged. */
  force?: boolean;
}

/**
 * "Read through" the way the timeline computes it: an explicit mark-unread point wins over the
 * highest read sequence, and a session whose read state cannot be fetched falls back to the
 * count the session list already carries.
 */
async function effectiveReadSequence(session: SystemActivitySession): Promise<number> {
  try {
    const state = await agentService.getSystemActivityReadState(session.sessionId);
    return state.markUnreadSequence === null
      ? state.highestReadSequence
      : Math.min(state.highestReadSequence, state.markUnreadSequence - 1);
  } catch {
    return session.lastSequence - session.unreadCount;
  }
}

/**
 * Unread warning and critical entries for one session. The severity filter is applied by the
 * service so a burst of newer informational events cannot push a still-unread critical one out
 * of the page.
 */
async function loadUnreadActivity(session: SystemActivitySession): Promise<InboxActivityItem[]> {
  const [readThrough, page] = await Promise.all([
    effectiveReadSequence(session),
    agentService.querySystemActivityTimeline({ sessionId: session.sessionId, severities: ["warning", "critical"], pageSize: 50 }),
  ]);
  if (page.kind !== "page") return [];
  return page.entries
    .filter((entry) => entry.sequence > readThrough)
    .map((entry) => ({ sessionId: session.sessionId, entry }));
}

/** What the session list already tells us about one session's unread activity. */
export function sessionActivitySignature(session: SystemActivitySession): string {
  return `${session.lastSequence}:${session.unreadCount}`;
}

async function mapWithConcurrency<T, R>(items: T[], limit: number, task: (item: T) => Promise<R>): Promise<PromiseSettledResult<R>[]> {
  const results: PromiseSettledResult<R>[] = new Array(items.length);
  let next = 0;
  const worker = async () => {
    while (next < items.length) {
      const index = next;
      next += 1;
      try {
        results[index] = { status: "fulfilled", value: await task(items[index]) };
      } catch (reason) {
        results[index] = { status: "rejected", reason };
      }
    }
  };
  await Promise.all(Array.from({ length: Math.min(limit, items.length) }, worker));
  return results;
}

/**
 * Per-session activity: a session whose signature matches its cache entry is reused, one that
 * changed, never loaded, or failed last time is read again, and one whose read fails now keeps
 * whatever it had and is reported as a partial failure so the next poll retries it.
 */
async function loadActivity(
  sessions: SystemActivitySession[],
  previous: Record<string, InboxActivityCacheEntry>,
  force: boolean,
): Promise<{ cache: Record<string, InboxActivityCacheEntry>; failed: boolean }> {
  const cache: Record<string, InboxActivityCacheEntry> = {};
  const stale: SystemActivitySession[] = [];
  for (const session of sessions) {
    const signature = sessionActivitySignature(session);
    const cached = previous[session.sessionId];
    if (session.unreadCount === 0) cache[session.sessionId] = { signature, items: [] };
    else if (!force && cached && cached.signature === signature) cache[session.sessionId] = cached;
    else stale.push(session);
  }
  let failed = false;
  const results = await mapWithConcurrency(stale, detailConcurrency, loadUnreadActivity);
  stale.forEach((session, index) => {
    const result = results[index];
    if (result.status === "fulfilled") cache[session.sessionId] = { signature: sessionActivitySignature(session), items: result.value };
    else {
      failed = true;
      // Kept without a matching signature so the next poll tries again instead of trusting it.
      const stalePrevious = previous[session.sessionId];
      if (stalePrevious) cache[session.sessionId] = { signature: "", items: stalePrevious.items };
    }
  });
  return { cache, failed };
}

/** Follows the attention pages so the badge counts every attention run, not just the first page. */
async function loadOverview(): Promise<MissionControlOverview> {
  const first = await agentService.getMissionControlOverview({ limit: attentionPageLimit, sort: "attention" });
  const attention: MissionControlRunSummary[] = [...first.attention.items];
  let cursor = first.attention.nextCursor;
  while (cursor && attention.length < attentionRunCap) {
    const page = await agentService.getMissionControlOverview({ cursor, limit: attentionPageLimit, sort: "attention" });
    attention.push(...page.attention.items);
    cursor = page.attention.items.length > 0 ? page.attention.nextCursor : null;
  }
  return { ...first, attention: { items: attention, nextCursor: cursor } };
}

/**
 * The two data sources Inbox composes, fetched through the same service methods Mission Control
 * and System Activity already use. A failure on one side leaves the other side's data intact
 * rather than blanking the whole surface.
 */
export async function loadInboxFeed(previous: InboxFeed = emptyInboxFeed, options: LoadInboxFeedOptions = {}): Promise<{ feed: InboxFeed; failed: boolean }> {
  const [overviewResult, sessionsResult] = await Promise.allSettled([loadOverview(), agentService.listSystemActivitySessions()]);
  const feed: InboxFeed = { ...emptyInboxFeed, activityCache: {} };
  let failed = false;
  if (overviewResult.status === "fulfilled") feed.overview = overviewResult.value;
  else {
    failed = true;
    feed.overview = previous.overview;
  }
  if (sessionsResult.status === "fulfilled") {
    const sessions = sessionsResult.value.filter((session) => session.visible);
    feed.unreadTotal = sessions.reduce((total, session) => total + session.unreadCount, 0);
    const activity = await loadActivity(sessions, previous.activityCache, options.force ?? false);
    feed.activityCache = activity.cache;
    feed.activity = sessions.flatMap((session) => activity.cache[session.sessionId]?.items ?? []);
    failed = failed || activity.failed;
  } else {
    failed = true;
    feed.unreadTotal = previous.unreadTotal;
    feed.activityCache = previous.activityCache;
    feed.activity = previous.activity;
  }
  return { feed, failed };
}
