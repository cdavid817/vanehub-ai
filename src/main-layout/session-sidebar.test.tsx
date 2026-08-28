import { renderToString } from "react-dom/server";
import "../i18n";
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import { clampSessionSidebarWidth } from "./main-layout";
import { SessionSidebar } from "./session-sidebar";
import { filterSearchResultsByAgent, filterSessionsByAgent, groupSessionsByProject, pruneSelectionToVisible } from "./session-sidebar-model";

function session(agentId: string): Session {
  return {
    id: `session-${agentId}`,
    title: `${agentId} work`,
    agentId,
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "idle",
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
    createdAt: "2026-07-18T00:00:00.000Z",
    updatedAt: "2026-07-18T00:00:00.000Z",
  };
}

function multiAgentSession(): Session {
  return {
    ...session("codex-cli"),
    id: "session-multi-agent",
    title: "shared review",
    seats: [
      { seatId: "seat-codex", agentId: "codex-cli", roleId: "architect", leftAt: null },
      { seatId: "seat-gemini", agentId: "gemini-cli", roleId: "reviewer", leftAt: null },
    ],
  };
}

describe("SessionSidebar CLI icons", () => {
  it("renders stable CLI identity from session agent ids", () => {
    const html = renderToString(
      <SessionSidebar
        activeSessionId="session-codex-cli"
        agentsAvailable
        archivedSessions={[session("future-agent")]}
        categories={[]}
        onAssignCategory={vi.fn()}
        onBatchDelete={vi.fn()}
        onContextMenu={vi.fn()}
        onNew={vi.fn()}
        onSearchChange={vi.fn()}
        onSelect={vi.fn()}
        searchQuery=""
        searchResults={[]}
        sessions={[session("claude-code"), session("codex-cli"), session("gemini-cli"), session("opencode")]}
      />,
    );

    expect(html).toContain("Claude Code");
    expect(html).toContain("Codex CLI");
    expect(html).toContain("Gemini CLI");
    expect(html).toContain("OpenCode");
    expect(html).toContain("ucd-agent-codex");
    expect(html).toContain("ucd-agent-claude");
    expect(html).toContain("更多操作");
    expect(html).toContain("列表");
    expect(html).toContain("分类");
    expect(html).toContain("项目");
    expect(html).toContain('data-testid="session-sidebar"');
    expect(html).not.toContain('class="ucd-panel flex h-full');
  });

  it("labels only multi-Agent session cards", () => {
    const html = renderToString(
      <SessionSidebar
        activeSessionId="session-multi-agent"
        agentsAvailable
        archivedSessions={[]}
        categories={[]}
        onAssignCategory={vi.fn()}
        onBatchDelete={vi.fn()}
        onContextMenu={vi.fn()}
        onNew={vi.fn()}
        onSearchChange={vi.fn()}
        onSelect={vi.fn()}
        searchQuery=""
        searchResults={[]}
        sessions={[session("claude-code"), multiAgentSession()]}
      />,
    );

    expect(html).toContain('data-testid="multi-agent-session-badge"');
    expect(html.match(/>多 Agent</g)).toHaveLength(1);
    expect(html).toContain("ucd-agent-codex");
    expect(html).not.toContain('data-role-icon="architect"');
  });
});

describe("session sidebar filtering and grouping", () => {
  it("filters sessions by stable managed agent id", () => {
    const sessions = [session("claude-code"), session("codex-cli"), session("gemini-cli")];

    expect(filterSessionsByAgent(sessions, "all")).toHaveLength(3);
    expect(filterSessionsByAgent(sessions, "codex-cli").map((item) => item.id)).toEqual(["session-codex-cli"]);
  });

  it("filters search results by stable agent id and archived source", () => {
    const active = session("codex-cli");
    const archived = { ...session("codex-cli"), id: "archived-codex", archived: true };
    const other = session("claude-code");
    const results = [active, archived, other].map((item) => ({ session: item, matches: [{ kind: "title" as const, excerpt: item.title }] }));

    expect(filterSearchResultsByAgent(results, "codex-cli", "active").map((result) => result.session.id)).toEqual(["session-codex-cli"]);
    expect(filterSearchResultsByAgent(results, "codex-cli", "archived").map((result) => result.session.id)).toEqual(["archived-codex"]);
  });

  it("prunes batch selection to visible session ids", () => {
    const selected = new Set(["session-claude-code", "hidden"]);
    const pruned = pruneSelectionToVisible(selected, [session("claude-code")]);

    expect([...pruned]).toEqual(["session-claude-code"]);
  });

  it("groups sessions by project metadata while preserving in-group order", () => {
    const worktree = { ...session("codex-cli"), id: "worktree", worktreePath: "D:\\code\\demo-feature", worktreeName: "feature" };
    const projectFirst = { ...session("claude-code"), id: "project-1", projectPath: "D:\\code\\demo" };
    const projectSecond = { ...session("gemini-cli"), id: "project-2", projectPath: "D:\\code\\demo" };
    const remote = { ...session("opencode"), id: "remote", remoteWorkspace: { host: "devbox", user: null, path: "/srv/app", displayName: "devbox:app", uri: "ssh://devbox/srv/app" } };
    const ungrouped = { ...session("future-agent"), id: "ungrouped" };

    const groups = groupSessionsByProject([worktree, projectFirst, projectSecond, remote, ungrouped], "No Project");

    expect(groups.map((group) => [group.label, group.sessions.map((item) => item.id)])).toEqual([
      ["feature", ["worktree"]],
      ["demo", ["project-1", "project-2"]],
      ["devbox:app", ["remote"]],
      ["No Project", ["ungrouped"]],
    ]);
  });

  it("clamps the resizable session sidebar width", () => {
    expect(clampSessionSidebarWidth(120)).toBe(232);
    expect(clampSessionSidebarWidth(300.4)).toBe(300);
    expect(clampSessionSidebarWidth(900)).toBe(420);
  });
});
