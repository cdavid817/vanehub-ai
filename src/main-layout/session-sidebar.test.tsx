import { renderToString } from "react-dom/server";
import "../i18n";
import { describe, expect, it, vi } from "vitest";
import type { Session } from "../types/agent";
import { SessionSidebar } from "./session-sidebar";

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
  });
});
