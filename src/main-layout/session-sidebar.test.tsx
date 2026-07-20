import { renderToString } from "react-dom/server";
import "../i18n";
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import { SessionSidebar } from "./session-sidebar";
import { filterSearchResultsByAgent, filterSessionsByAgent, pruneSelectionToVisible } from "./session-sidebar-model";

function session(agentId: string): Session {
  return {
    id: `session-${agentId}`,
    title: `${agentId} work`,
    agentId,
    interactionMode: "cli",
    lifecycleState: "idle",
    folder: null,
    projectPath: null,
    worktreePath: null,
    worktreeName: null,
    worktreeBranch: null,
    remoteWorkspace: null,
    runtimeSessionId: null,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-07-18T00:00:00.000Z",
    updatedAt: "2026-07-18T00:00:00.000Z",
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
    expect(html).toContain("批量管理");
    expect(html).toContain("列表");
    expect(html).toContain("分类");
  });

});

describe("session sidebar filtering", () => {
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
});
