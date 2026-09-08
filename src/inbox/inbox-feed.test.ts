// @vitest-environment jsdom

import { afterEach, describe, expect, it } from "vitest";
import type { SystemActivityTimelineEntry } from "../services/system-activity-service";
import type { MissionControlOverview, MissionControlRunSummary } from "../types/mission-control";
import {
  classifyInboxFeed,
  countInboxBadge,
  inboxActivityRunId,
  readInboxViewMode,
  rememberInboxViewMode,
  type InboxActivityItem,
} from "./inbox-feed";

afterEach(() => localStorage.clear());

function run(runId: string, state: MissionControlRunSummary["state"] = "running"): MissionControlRunSummary {
  return {
    runId, version: 1, ownerType: "agent", ownerId: "owner", agentId: "opencode", title: runId, state,
    createdAt: "2026-09-07T00:00:00.000Z", updatedAt: "2026-09-07T00:00:00.000Z", endedAt: null,
    projectId: null, workspace: null, phase: null, attention: null, reasonCode: null,
    verification: "unavailable", tokens: null, cost: null, actions: ["open"], navigation: null, runner: null,
  };
}

function activity(eventId: string, severity: SystemActivityTimelineEntry["envelope"]["severity"], runId: string | null = null): InboxActivityItem {
  return {
    sessionId: "system-activity-v1-web-workspace-w",
    entry: {
      sequence: Number(eventId.replace(/\D/g, "")),
      detailUnavailableReason: null,
      envelope: {
        schemaVersion: 1, eventId, eventCode: "run_failed", sourceDomain: "orchestration", sourceId: eventId,
        sourceRevision: "1", sourceSequence: 1, scopeKind: "workspace", canonicalScopeId: "w",
        occurredAtMs: 1, committedAtMs: 1, severity, status: "failed", attentionKind: "none",
        safeActorKind: "system", safeIdentities: [], metrics: {}, reasonCodes: [],
        navigation: runId ? { kind: "run", stableId: runId } : null, supersedesEventId: null,
        payload: null, projectionPolicyVersion: 1, contentHash: eventId,
      },
    },
  };
}

function overview(attention: MissionControlRunSummary[], active: MissionControlRunSummary[] = [], recent: MissionControlRunSummary[] = []): MissionControlOverview {
  return {
    counts: { running: 0, waitingApproval: 0, waitingUser: 0, retrying: 0, blocked: 0, failed: 0, completedRecently: 0 },
    attention: { items: attention, nextCursor: null },
    active: { items: active, nextCursor: null },
    recent: { items: recent, nextCursor: null },
  };
}

describe("inbox feed", () => {
  it("places attention runs and unread warning or critical activity under Needs attention", () => {
    const sections = classifyInboxFeed({
      overview: overview([run("a", "waiting_approval")], [run("b")], [run("c", "completed")]),
      activity: [activity("e1", "warning"), activity("e2", "info"), activity("e3", "critical"), activity("e4", "error")],
      unreadTotal: 4,
      activitySignature: null,
    });
    expect(sections.attention.runs.map((item) => item.runId)).toEqual(["a"]);
    expect(sections.attention.activity.map((item) => item.entry.envelope.eventId)).toEqual(["e1", "e3"]);
    expect(sections.running.map((item) => item.runId)).toEqual(["b"]);
    expect(sections.recent.map((item) => item.runId)).toEqual(["c"]);
  });

  it("renders empty sections without an overview", () => {
    const sections = classifyInboxFeed({ overview: null, activity: [], unreadTotal: 0, activitySignature: null });
    expect(sections.attention).toEqual({ runs: [], activity: [] });
    expect(sections.running).toEqual([]);
    expect(sections.recent).toEqual([]);
  });

  it("counts a run once when Mission Control and System Activity both report it", () => {
    const feed = {
      overview: overview([run("a", "waiting_approval"), run("b", "failed")]),
      activity: [activity("e1", "error", "a"), activity("e2", "warning", "zzz"), activity("e3", "info")],
      unreadTotal: 3,
      activitySignature: null,
    };
    // Two attention runs + three unread items, minus the one item that is about run "a".
    expect(countInboxBadge(feed)).toBe(4);
    expect(countInboxBadge({ ...feed, activity: [], unreadTotal: 0 })).toBe(2);
    expect(countInboxBadge({ overview: null, activity: [], unreadTotal: 5, activitySignature: null })).toBe(5);
  });

  it("identifies the run an entry is about only through run navigation", () => {
    expect(inboxActivityRunId(activity("e1", "info", "run-1").entry)).toBe("run-1");
    expect(inboxActivityRunId(activity("e1", "info").entry)).toBeNull();
  });

  it("persists the list or board preference and defaults to list", () => {
    expect(readInboxViewMode()).toBe("list");
    rememberInboxViewMode("board");
    expect(readInboxViewMode()).toBe("board");
    expect(localStorage.getItem("vanehub.inbox.view-mode.v1")).toBe("board");
    rememberInboxViewMode("list");
    expect(readInboxViewMode()).toBe("list");
  });
});
