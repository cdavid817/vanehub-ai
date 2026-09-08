import { agentService } from "../services/runtime-agent-client";
import type { SystemActivitySession } from "../services/system-activity-service";
import { emptyInboxFeed, type InboxActivityItem, type InboxFeed } from "./inbox-feed";

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

async function loadUnreadActivity(session: SystemActivitySession): Promise<InboxActivityItem[]> {
  const [readThrough, page] = await Promise.all([
    effectiveReadSequence(session),
    agentService.querySystemActivityTimeline({ sessionId: session.sessionId, pageSize: 50 }),
  ]);
  if (page.kind !== "page") return [];
  return page.entries
    .filter((entry) => entry.sequence > readThrough)
    .map((entry) => ({ sessionId: session.sessionId, entry }));
}

/**
 * What the session list already tells us about unread activity. While it is unchanged, the
 * per-session read-state and timeline calls that produced the last activity list would return
 * the same rows, so a poll can reuse them instead of paying two requests per session again.
 */
export function inboxActivitySignature(sessions: SystemActivitySession[]): string {
  return sessions.map((session) => `${session.sessionId}:${session.lastSequence}:${session.unreadCount}`).join("|");
}

/**
 * The two data sources Inbox composes, fetched through the same service methods Mission Control
 * and System Activity already use. A failure on one side leaves the other side's data intact
 * rather than blanking the whole surface.
 */
export async function loadInboxFeed(previous: InboxFeed = emptyInboxFeed): Promise<{ feed: InboxFeed; failed: boolean }> {
  const [overviewResult, sessionsResult] = await Promise.allSettled([
    agentService.getMissionControlOverview({ limit: 20, sort: "attention" }),
    agentService.listSystemActivitySessions(),
  ]);
  const feed: InboxFeed = { ...emptyInboxFeed };
  let failed = false;
  if (overviewResult.status === "fulfilled") feed.overview = overviewResult.value;
  else failed = true;
  if (sessionsResult.status === "fulfilled") {
    const sessions = sessionsResult.value.filter((session) => session.visible);
    feed.unreadTotal = sessions.reduce((total, session) => total + session.unreadCount, 0);
    feed.activitySignature = inboxActivitySignature(sessions);
    if (feed.activitySignature === previous.activitySignature) {
      feed.activity = previous.activity;
    } else {
      const pages = await Promise.allSettled(sessions.filter((session) => session.unreadCount > 0).map(loadUnreadActivity));
      feed.activity = pages.flatMap((page) => page.status === "fulfilled" ? page.value : []);
    }
  } else {
    failed = true;
  }
  return { feed, failed };
}
