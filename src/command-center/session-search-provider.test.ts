import { afterEach, describe, expect, it, vi } from "vitest";
import { agentService } from "../services/runtime-agent-client";
import type { Session, SessionSearchResult } from "../types/agent";
import { sessionSearchProvider } from "./session-search-provider";

afterEach(() => vi.restoreAllMocks());

function session(overrides: Partial<Session> = {}): Session {
  return {
    id: "session-1",
    personalizationMode: "standard",
    title: "Fix null auth token",
    agentId: "claude-code",
    interactionMode: "cli",
    lifecycleState: "running",
    recoveryStatus: "clean",
    recoveryRevision: 0,
    stateRevision: 0,
    historyRevision: 0,
    activeExecutionRunId: null,
    folder: null,
    projectPath: "D:\\code\\vanehub",
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
    createdAt: "2026-08-14T00:00:00.000Z",
    updatedAt: "2026-08-14T01:00:00.000Z",
    ...overrides,
  };
}

function searchRequest(overrides: Partial<{ query: string; limit: number }> = {}) {
  return { query: "auth", scopes: ["session" as const], limit: 20, signal: new AbortController().signal, ...overrides };
}

describe("sessionSearchProvider", () => {
  it("supports only the session scope", () => {
    expect(sessionSearchProvider.supports("session")).toBe(true);
    expect(sessionSearchProvider.supports("project")).toBe(false);
    expect(sessionSearchProvider.supports("run")).toBe(false);
  });

  it("maps a search result's title, subtitle, route, and status", async () => {
    const searchSessions = vi.spyOn(agentService, "searchSessions").mockResolvedValue([
      { session: session(), matches: [{ kind: "title", excerpt: "Fix null auth token" }] } satisfies SessionSearchResult,
    ]);

    const page = await sessionSearchProvider.search(searchRequest());

    expect(searchSessions).toHaveBeenCalledWith({ query: "auth", limit: 20 });
    expect(page.nextCursor).toBeNull();
    expect(page.items).toEqual([{
      key: "session-1",
      kind: "session",
      title: "Fix null auth token",
      subtitle: "D:\\code\\vanehub",
      status: "active",
      route: { destination: "sessions", sessionId: "session-1", creatingSession: false },
      updatedAt: "2026-08-14T01:00:00.000Z",
    }]);
  });

  it.each([
    ["idle", "neutral"],
    ["starting", "active"],
    ["running", "active"],
    ["failed", "error"],
    ["stopped", "neutral"],
  ] as const)("maps lifecycleState %s to status %s", async (lifecycleState, status) => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([
      { session: session({ lifecycleState }), matches: [] },
    ]);
    const page = await sessionSearchProvider.search(searchRequest());
    expect(page.items[0].status).toBe(status);
  });

  it("respects the requested limit even if the service returns more", async () => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([
      { session: session({ id: "session-1" }), matches: [] },
      { session: session({ id: "session-2" }), matches: [] },
      { session: session({ id: "session-3" }), matches: [] },
    ]);
    const page = await sessionSearchProvider.search(searchRequest({ limit: 2 }));
    expect(page.items).toHaveLength(2);
  });

  it("returns an empty page when nothing matches", async () => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([]);
    const page = await sessionSearchProvider.search(searchRequest());
    expect(page.items).toEqual([]);
  });

  it("never surfaces a message match's excerpt text anywhere in the result", async () => {
    vi.spyOn(agentService, "searchSessions").mockResolvedValue([
      {
        session: session(),
        matches: [{ kind: "message", excerpt: "SECRET_MESSAGE_CONTENT should never leak", messageId: "msg-1" }],
      },
    ]);
    const page = await sessionSearchProvider.search(searchRequest());
    expect(JSON.stringify(page)).not.toContain("SECRET_MESSAGE_CONTENT");
  });
});
