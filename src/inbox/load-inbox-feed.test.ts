// @vitest-environment jsdom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import { resetWebMissionControlRunsForTest } from "../services/web-agent-client";
import { resetWebSystemActivityForTest, seedWebSystemActivityEventForTest } from "../services/web-system-activity-state";
import { inboxActivitySignature, loadInboxFeed } from "./load-inbox-feed";

beforeEach(() => {
  resetWebMissionControlRunsForTest();
  resetWebSystemActivityForTest();
});
afterEach(() => vi.restoreAllMocks());

describe("loadInboxFeed", () => {
  it("fetches activity once per session-list signature and reuses it while the list is unchanged", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    seedWebSystemActivityEventForTest("global", "global", "skill_created");
    const timeline = vi.spyOn(agentService, "querySystemActivityTimeline");
    const readState = vi.spyOn(agentService, "getSystemActivityReadState");

    const first = await loadInboxFeed();
    expect(first.failed).toBe(false);
    expect(first.feed.activity).toHaveLength(2);
    expect(first.feed.unreadTotal).toBe(2);
    expect(timeline).toHaveBeenCalledTimes(2);
    expect(readState).toHaveBeenCalledTimes(2);

    // A poll that finds the same sessions, sequences, and unread counts pays only the two list calls.
    const second = await loadInboxFeed(first.feed);
    expect(second.feed.activity).toBe(first.feed.activity);
    expect(second.feed.activitySignature).toBe(first.feed.activitySignature);
    expect(timeline).toHaveBeenCalledTimes(2);
    expect(readState).toHaveBeenCalledTimes(2);

    // New activity changes the signature, so the entries are fetched again.
    seedWebSystemActivityEventForTest("workspace", "w", "breaker_opened", "critical");
    const third = await loadInboxFeed(second.feed);
    expect(third.feed.activity).toHaveLength(3);
    expect(third.feed.activitySignature).not.toBe(second.feed.activitySignature);
    expect(timeline).toHaveBeenCalledTimes(4);
  });

  it("derives the signature from session identity, last sequence, and unread count only", () => {
    const session = { sessionId: "s", lastSequence: 3, unreadCount: 2, visible: true } as Parameters<typeof inboxActivitySignature>[0][number];
    expect(inboxActivitySignature([session])).toBe("s:3:2");
    expect(inboxActivitySignature([{ ...session, unreadCount: 1 }])).toBe("s:3:1");
    expect(inboxActivitySignature([])).toBe("");
  });

  it("keeps the other side's data and reports failure when one source rejects", async () => {
    seedWebSystemActivityEventForTest("workspace", "w", "run_completed");
    vi.spyOn(agentService, "getMissionControlOverview").mockRejectedValue(new Error("down"));
    const { feed, failed } = await loadInboxFeed();
    expect(failed).toBe(true);
    expect(feed.overview).toBeNull();
    expect(feed.activity).toHaveLength(1);
    expect(feed.unreadTotal).toBe(1);
  });
});
