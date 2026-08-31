import { describe, expect, it } from "vitest";
import type { Session, SessionCategory, SessionSearchResult } from "../types/agent";
import {
  filterSearchResultsByAgent,
  filterSessionsByAgent,
  filterSessionsByDate,
  filterSessionsByProject,
  filterSessionsBySource,
  filterSessionsByStatus,
  getSessionProjectGroupKey,
  getSessionProjectGroupLabel,
  groupSessionsByAttentionTier,
  groupSessionsByCategory,
  groupSessionsByProject,
  pruneSelectionToVisible,
  sessionAttentionTier,
  sessionProjectFilterOptions,
  sortSessionsByAttention,
} from "./session-sidebar-model";

const NOW = Date.parse("2026-08-31T00:00:00Z");

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "s1",
    personalizationMode: "standard",
    title: "Untitled",
    agentId: "claude-code",
    interactionMode: "cli",
    lifecycleState: "idle",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-30T00:00:00Z",
    updatedAt: "2026-08-30T00:00:00Z",
    ...overrides,
  };
}

describe("sessionAttentionTier", () => {
  it("ranks a session needing review above every other signal", () => {
    expect(sessionAttentionTier(session({ recoveryStatus: "action_required", lifecycleState: "running", pinned: true }), NOW)).toBe("needs-attention");
  });

  it("ranks a running session above pinned and recency", () => {
    expect(sessionAttentionTier(session({ lifecycleState: "running", pinned: true }), NOW)).toBe("running");
  });

  it("treats starting the same as running", () => {
    expect(sessionAttentionTier(session({ lifecycleState: "starting" }), NOW)).toBe("running");
  });

  it("ranks pinned above recency once nothing needs attention or is running", () => {
    expect(sessionAttentionTier(session({ pinned: true, updatedAt: "2020-01-01T00:00:00Z" }), NOW)).toBe("pinned");
  });

  it("treats an update inside the recent window as recent", () => {
    expect(sessionAttentionTier(session({ updatedAt: new Date(NOW - 1000).toISOString() }), NOW)).toBe("recent");
  });

  it("treats the recent-window boundary itself as still recent", () => {
    const sevenDaysMs = 7 * 24 * 60 * 60 * 1000;
    expect(sessionAttentionTier(session({ updatedAt: new Date(NOW - sevenDaysMs).toISOString() }), NOW)).toBe("recent");
  });

  it("treats an update just past the recent window as remaining", () => {
    const justOverSevenDaysMs = 7 * 24 * 60 * 60 * 1000 + 1;
    expect(sessionAttentionTier(session({ updatedAt: new Date(NOW - justOverSevenDaysMs).toISOString() }), NOW)).toBe("remaining");
  });
});

describe("sortSessionsByAttention", () => {
  it("orders sessions by tier, then by most-recently-updated within a tier", () => {
    const remaining = session({ id: "remaining", updatedAt: "2020-01-01T00:00:00Z" });
    const recentOld = session({ id: "recent-old", updatedAt: new Date(NOW - 2000).toISOString() });
    const recentNew = session({ id: "recent-new", updatedAt: new Date(NOW - 1000).toISOString() });
    const pinned = session({ id: "pinned", pinned: true, updatedAt: "2020-01-01T00:00:00Z" });
    const running = session({ id: "running", lifecycleState: "running", updatedAt: "2020-01-01T00:00:00Z" });
    const needsAttention = session({ id: "needs-attention", recoveryStatus: "action_required", updatedAt: "2020-01-01T00:00:00Z" });

    const sorted = sortSessionsByAttention([remaining, recentOld, pinned, running, needsAttention, recentNew], NOW);

    expect(sorted.map((entry) => entry.id)).toEqual(["needs-attention", "running", "pinned", "recent-new", "recent-old", "remaining"]);
  });

  it("does not mutate the input array", () => {
    const input = [session({ id: "a", updatedAt: "2020-01-01T00:00:00Z" }), session({ id: "b", pinned: true })];
    const inputOrderBefore = input.map((entry) => entry.id);
    sortSessionsByAttention(input, NOW);
    expect(input.map((entry) => entry.id)).toEqual(inputOrderBefore);
  });
});

describe("groupSessionsByAttentionTier", () => {
  it("collapses consecutive same-tier sessions into one group and omits empty tiers", () => {
    const groups = groupSessionsByAttentionTier([
      session({ id: "running-1", lifecycleState: "running" }),
      session({ id: "running-2", lifecycleState: "running", updatedAt: "2020-01-01T00:00:00Z" }),
      session({ id: "remaining-1", updatedAt: "2020-01-01T00:00:00Z" }),
    ], NOW);

    expect(groups.map((group) => group.tier)).toEqual(["running", "remaining"]);
    expect(groups[0].sessions.map((entry) => entry.id)).toEqual(["running-1", "running-2"]);
    expect(groups[1].sessions.map((entry) => entry.id)).toEqual(["remaining-1"]);
  });

  it("returns no groups for an empty session list", () => {
    expect(groupSessionsByAttentionTier([], NOW)).toEqual([]);
  });
});

describe("groupSessionsByCategory", () => {
  const categories: SessionCategory[] = [
    { id: "cat-1", name: "Work", sortOrder: 0, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
    { id: "cat-2", name: "Empty", sortOrder: 1, createdAt: "2026-01-01T00:00:00Z", updatedAt: "2026-01-01T00:00:00Z" },
  ];

  it("buckets sessions by categoryId, keeps an empty category visible, and groups the rest as uncategorized", () => {
    const groups = groupSessionsByCategory([
      session({ id: "a", categoryId: "cat-1" }),
      session({ id: "b", categoryId: null }),
    ], categories, "Uncategorized");

    expect(groups).toEqual([
      { id: "cat-1", label: "Work", sessions: [expect.objectContaining({ id: "a" })] },
      { id: "cat-2", label: "Empty", sessions: [] },
      { id: null, label: "Uncategorized", sessions: [expect.objectContaining({ id: "b" })] },
    ]);
  });
});

describe("filterSessionsByStatus", () => {
  it("returns every session for \"all\" and matches exact lifecycle state otherwise", () => {
    const running = session({ id: "running", lifecycleState: "running" });
    const idle = session({ id: "idle", lifecycleState: "idle" });
    expect(filterSessionsByStatus([running, idle], "all")).toEqual([running, idle]);
    expect(filterSessionsByStatus([running, idle], "running")).toEqual([running]);
  });
});

describe("filterSessionsBySource", () => {
  it("treats an absent source as desktop, matching a session that predates IM integration", () => {
    const noSource = session({ id: "no-source" });
    const imSource = session({ id: "im", source: { kind: "im", connector: null } });
    expect(filterSessionsBySource([noSource, imSource], "desktop")).toEqual([noSource]);
    expect(filterSessionsBySource([noSource, imSource], "im")).toEqual([imSource]);
    expect(filterSessionsBySource([noSource, imSource], "all")).toEqual([noSource, imSource]);
  });
});

describe("filterSessionsByDate", () => {
  it("keeps only sessions updated within the chosen relative window", () => {
    const today = session({ id: "today", updatedAt: new Date(NOW - 1000).toISOString() });
    const lastWeek = session({ id: "last-week", updatedAt: new Date(NOW - 5 * 24 * 60 * 60 * 1000).toISOString() });
    const lastQuarter = session({ id: "last-quarter", updatedAt: "2020-01-01T00:00:00Z" });

    expect(filterSessionsByDate([today, lastWeek, lastQuarter], "today", NOW)).toEqual([today]);
    expect(filterSessionsByDate([today, lastWeek, lastQuarter], "week", NOW)).toEqual([today, lastWeek]);
    expect(filterSessionsByDate([today, lastWeek, lastQuarter], "all", NOW)).toEqual([today, lastWeek, lastQuarter]);
  });
});

describe("sessionProjectFilterOptions / filterSessionsByProject", () => {
  it("derives one option per distinct project key present, then filters to just that project", () => {
    const a1 = session({ id: "a1", worktreePath: "/repo/a", worktreeName: "a" });
    const a2 = session({ id: "a2", worktreePath: "/repo/a", worktreeName: "a" });
    const b1 = session({ id: "b1", worktreePath: "/repo/b", worktreeName: "b" });

    const options = sessionProjectFilterOptions([a1, a2, b1], "Ungrouped");
    expect(options).toEqual([{ value: "project:/repo/a", label: "a" }, { value: "project:/repo/b", label: "b" }]);

    expect(filterSessionsByProject([a1, a2, b1], "project:/repo/a")).toEqual([a1, a2]);
    expect(filterSessionsByProject([a1, a2, b1], "all")).toEqual([a1, a2, b1]);
  });
});

describe("filterSessionsByAgent (pre-existing, previously untested)", () => {
  it("returns every session when the filter is \"all\"", () => {
    const sessions = [session({ agentId: "claude-code" }), session({ agentId: "codex-cli" })];
    expect(filterSessionsByAgent(sessions, "all")).toEqual(sessions);
  });

  it("matches by the session's own agentId", () => {
    const match = session({ agentId: "codex-cli" });
    expect(filterSessionsByAgent([match, session({ agentId: "claude-code" })], "codex-cli")).toEqual([match]);
  });

  it("also matches through an active (non-departed) seat's agentId", () => {
    const match = session({ agentId: "claude-code", seats: [{ agentId: "codex-cli", leftAt: null }] as Session["seats"] });
    expect(filterSessionsByAgent([match], "codex-cli")).toEqual([match]);
  });

  it("does not match through a seat the participant has already left", () => {
    const noMatch = session({ agentId: "claude-code", seats: [{ agentId: "codex-cli", leftAt: "2026-01-01T00:00:00Z" }] as Session["seats"] });
    expect(filterSessionsByAgent([noMatch], "codex-cli")).toEqual([]);
  });
});

describe("filterSearchResultsByAgent (pre-existing, previously untested)", () => {
  function result(overrides: Partial<Session> = {}): SessionSearchResult {
    return { session: session(overrides), matches: [] };
  }

  it("keeps only results matching both the source mode and the agent filter", () => {
    const activeMatch = result({ agentId: "claude-code", archived: false });
    const archivedMatch = result({ agentId: "claude-code", archived: true });
    const wrongAgent = result({ agentId: "codex-cli", archived: false });

    expect(filterSearchResultsByAgent([activeMatch, archivedMatch, wrongAgent], "claude-code", "active")).toEqual([activeMatch]);
    expect(filterSearchResultsByAgent([activeMatch, archivedMatch, wrongAgent], "claude-code", "archived")).toEqual([archivedMatch]);
  });
});

describe("pruneSelectionToVisible (pre-existing, previously untested)", () => {
  it("drops ids no longer present in the visible set and returns the same reference when nothing changed", () => {
    const visible = [session({ id: "a" }), session({ id: "b" })];
    const pruned = pruneSelectionToVisible(new Set(["a", "c"]), visible);
    expect(pruned).toEqual(new Set(["a"]));

    const unchanged = new Set(["a", "b"]);
    expect(pruneSelectionToVisible(unchanged, visible)).toBe(unchanged);
  });
});

describe("groupSessionsByProject (pre-existing, previously untested)", () => {
  it("groups by worktree path and prefers the worktree name as the label", () => {
    const groups = groupSessionsByProject([
      session({ id: "a", worktreePath: "/repo/main", worktreeName: "main" }),
      session({ id: "b", worktreePath: "/repo/main", worktreeName: "main" }),
      session({ id: "c", worktreePath: null, projectPath: null, folder: null }),
    ], "Ungrouped");

    expect(groups).toHaveLength(2);
    expect(groups[0]).toMatchObject({ label: "main", path: "/repo/main" });
    expect(groups[0].sessions).toHaveLength(2);
    expect(groups[1]).toMatchObject({ label: "Ungrouped", path: null });
  });
});

describe("getSessionProjectGroupKey / getSessionProjectGroupLabel (pre-existing, previously untested)", () => {
  it("falls back through worktreePath, remoteWorkspace, projectPath, then folder for the key", () => {
    expect(getSessionProjectGroupKey(session({ folder: "/a/b" }))).toBe("project:/a/b");
    expect(getSessionProjectGroupKey(session())).toBe("project:none");
  });

  it("falls back to the ungrouped label when no path or name is available", () => {
    expect(getSessionProjectGroupLabel(session(), "Ungrouped")).toBe("Ungrouped");
  });
});
