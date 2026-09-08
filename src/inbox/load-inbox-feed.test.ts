// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import { resetWebMissionControlRunsForTest, seedWebMissionControlRunsForTest } from "../services/web-agent-client";
import { resetWebSystemActivityForTest, seedWebSystemActivityEventForTest } from "../services/web-system-activity-state";
import { countInboxBadge } from "./inbox-feed";
import { attentionRunCap, loadInboxFeed, sessionActivitySignature } from "./load-inbox-feed";

beforeEach(() => {
  resetWebMissionControlRunsForTest();
  resetWebSystemActivityForTest();
});
afterEach(() => vi.restoreAllMocks());

describe("loadInboxFeed", () => {
  it("fetches activity per session and reuses a session whose list entry is unchanged", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "warning");
    seedWebSystemActivityEventForTest("global", "global", "breaker_opened", "critical");
    const timeline = vi.spyOn(agentService, "querySystemActivityTimeline");
    const readState = vi.spyOn(agentService, "getSystemActivityReadState");

    const first = await loadInboxFeed();
    expect(first.failed).toBe(false);
    expect(first.feed.activity).toHaveLength(2);
    expect(first.feed.unreadTotal).toBe(2);
    expect(timeline).toHaveBeenCalledTimes(2);
    expect(readState).toHaveBeenCalledTimes(2);

    // Nothing changed: only the two list calls are paid.
    const second = await loadInboxFeed(first.feed);
    expect(second.feed.activity).toEqual(first.feed.activity);
    expect(timeline).toHaveBeenCalledTimes(2);

    // One session changed: only that session is read again.
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    const third = await loadInboxFeed(second.feed);
    expect(third.feed.activity).toHaveLength(3);
    expect(timeline).toHaveBeenCalledTimes(3);
    expect(timeline.mock.calls[2][0].sessionId).toContain("workspace-w");

    // A forced refresh revalidates every session.
    await loadInboxFeed(third.feed, { force: true });
    expect(timeline).toHaveBeenCalledTimes(5);
  });

  it("asks the service for warning and critical entries so newer informational events cannot hide them", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    for (let index = 0; index < 60; index += 1) seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    const timeline = vi.spyOn(agentService, "querySystemActivityTimeline");

    const { feed } = await loadInboxFeed();
    expect(timeline.mock.calls[0][0].severities).toEqual(["warning", "critical"]);
    expect(feed.unreadTotal).toBe(61);
    expect(feed.activity.map((item) => item.entry.envelope.severity)).toEqual(["critical"]);
  });

  it("retries a session whose timeline failed instead of caching the gap as success", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    const timeline = vi.spyOn(agentService, "querySystemActivityTimeline").mockRejectedValueOnce(new Error("busy"));

    const first = await loadInboxFeed();
    expect(first.failed).toBe(true);
    expect(first.feed.activity).toEqual([]);

    const second = await loadInboxFeed(first.feed);
    expect(timeline).toHaveBeenCalledTimes(2);
    expect(second.failed).toBe(false);
    expect(second.feed.activity).toHaveLength(1);
  });

  it("keeps a session's previous entries when a later read fails and reports the partial failure", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    const first = await loadInboxFeed();
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "warning");
    vi.spyOn(agentService, "querySystemActivityTimeline").mockRejectedValueOnce(new Error("busy"));

    const second = await loadInboxFeed(first.feed);
    expect(second.failed).toBe(true);
    expect(second.feed.activity).toEqual(first.feed.activity);
    expect(second.feed.unreadTotal).toBe(2);
  });

  it("follows the attention pages so every attention run counts toward the badge", async () => {
    seedWebMissionControlRunsForTest(100);
    const overview = vi.spyOn(agentService, "getMissionControlOverview");
    const { feed } = await loadInboxFeed();
    const attention = feed.overview?.attention.items ?? [];
    // 100 runs cycle through seven states, five of which carry attention.
    expect(attention.length).toBeGreaterThan(20);
    expect(attention.length).toBeLessThanOrEqual(attentionRunCap);
    expect(new Set(attention.map((run) => run.runId)).size).toBe(attention.length);
    expect(overview.mock.calls.length).toBeGreaterThan(1);
    expect(countInboxBadge(feed)).toBe(attention.length);
    // Running and recent still come from the first page only.
    expect(feed.overview?.active.items.length).toBeLessThanOrEqual(20);
  });

  it("keeps the previous overview when the overview read fails and reports failure", async () => {
    const first = await loadInboxFeed();
    vi.spyOn(agentService, "getMissionControlOverview").mockRejectedValue(new Error("down"));
    const { feed, failed } = await loadInboxFeed(first.feed);
    expect(failed).toBe(true);
    expect(feed.overview).toEqual(first.feed.overview);
  });

  it("derives a session's signature from its last sequence and unread count", () => {
    const session = { sessionId: "s", lastSequence: 3, unreadCount: 2, visible: true } as Parameters<typeof sessionActivitySignature>[0];
    expect(sessionActivitySignature(session)).toBe("3:2");
    expect(sessionActivitySignature({ ...session, unreadCount: 1 })).toBe("3:1");
  });
});
