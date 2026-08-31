import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { renderToStaticMarkup } from "react-dom/server";
import { readFileSync } from "node:fs";
import ReactMarkdown from "react-markdown";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../i18n";
import { managedCliAgentIds, type Session } from "../types/agent";
import type { ChatMessage } from "../types/chat";
import { agentTerminalInputClassName } from "./agent-terminal-tab";
import { SessionTabBar } from "./session-tab-bar";
import { SessionConversationHeader } from "./session-conversation-header";
import { SessionWorkspaceRegionsHost } from "./session-tabs";
import { toolUseCount } from "./terminal-tab";

const message: ChatMessage = {
  id: "message-1",
  sessionId: "session-1",
  role: "assistant",
  content: "done",
  status: "completed",
  toolUse: [{ id: "tool-1", name: "read_file", input: { path: "README.md" }, status: "completed" }],
  tokenUsage: { input: 12, output: 8 },
  createdAt: "2026-07-17T00:00:00.000Z",
  updatedAt: "2026-07-17T00:00:01.000Z",
  sessionSequence: 1,
  executionRunId: null,
};

const cleanRecovery = {
  recoveryStatus: "clean" as const,
  recoveryRevision: 0,
  stateRevision: 0,
  historyRevision: 0,
  activeExecutionRunId: null,
};

describe("session workspace components", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });
  it("renders all tab labels and the changes badge", () => {
    const html = renderToStaticMarkup(<SessionTabBar activeTab="work" badges={{ changes: { kind: "count", count: 1, tone: "neutral", atLeast: false } }} onActivate={() => undefined} onOpenSettings={() => undefined} session={null} />);
    expect(html).toContain("Work");
    expect(html).toContain("Changes");
    expect(html).toContain("Report");
    expect(html).toContain("1");
  });

  it("omits a badge for a known zero rather than rendering 0", () => {
    // A known zero is the absence of a badge. "Zero unviewed files" and "nobody has counted the
    // changed files" must not look the same, so the count case never carries a zero.
    const html = renderToStaticMarkup(<SessionTabBar activeTab="work" badges={{ changes: { kind: "none" } }} onActivate={() => undefined} onOpenSettings={() => undefined} session={null} />);
    expect(html).not.toContain('data-badge="changes-count"');
    expect(html).not.toContain('data-badge="changes-unknown"');
  });

  it("renders tool execution cards and report values", () => {
    expect(toolUseCount([message])).toBe(1);
    // Terminal History is no longer server-renderable on its own: it reads the workspace evidence
    // scope and needs a session id. The tool row it produces is asserted in terminal-tab.test.tsx,
    // which mounts it with both and switches to the legacy view.
    // The Report tab left this file with Terminal History and for the same reason: it now reads
    // the workspace evidence scope and queries the backend, so it cannot be rendered to static
    // markup on its own. Its sections are asserted in report-tab.test.tsx against a real report.
  });

  it("renders API chat instead of an Agent Terminal for OnePiece sessions", () => {
    const session: Session = {
      id: "session-onepiece",
      title: "OnePiece session",
      agentId: "onepiece",
      interactionMode: "api",
      personalizationMode: "standard", lifecycleState: "idle",
      recoveryStatus: "clean",
      recoveryRevision: 0,
      stateRevision: 0,
      historyRevision: 0,
      activeExecutionRunId: null,
      folder: "D:/project",
      projectPath: "D:/project",
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
      createdAt: "2026-08-02T00:00:00.000Z",
      updatedAt: "2026-08-02T00:00:00.000Z",
    };
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <SessionWorkspaceRegionsHost
          activeSession={session}
          apiComposer={<div>API composer</div>}
          messages={[message]}
          messagesPartial={false}
          onOpenSettings={() => undefined}
          sessionActivationKey={0}
        />
      </QueryClientProvider>,
    );

    expect(html).toContain("OnePiece session");
    expect(html).toContain('data-testid="session-conversation-header"');
    expect(html).not.toContain('data-testid="session-roster-chips"');
    expect(html).not.toMatch(/>onepiece<\/span>/);
    expect(html).toContain("API composer");
    expect(html).toContain('data-testid="contiguous-chat-workspace"');
    expect(html).toContain('data-testid="message-readable-measure"');
    expect(html).toContain('data-message-canvas="adaptive"');
    expect(html).not.toContain("max-w-5xl");
    expect(html).toContain('data-testid="attached-chat-composer"');
    expect(html).not.toContain("Agent CLI workspace");
  });

  it("renders one structured thread for a multi-Agent CLI session", () => {
    const session: Session = {
      ...cleanRecovery,
      id: "session-shared-cli", title: "Shared CLI session", agentId: "codex-cli", interactionMode: "cli",
      personalizationMode: "standard", lifecycleState: "idle", folder: "D:/project", projectPath: "D:/project", worktreePath: null,
      worktreeName: null, worktreeBranch: null, remoteWorkspace: null, remoteSshConnectionId: null,
      remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null, pinned: false, archived: false,
      createdAt: "2026-08-02T00:00:00.000Z", updatedAt: "2026-08-02T00:00:00.000Z",
      seats: [
        { seatId: "seat-1", agentId: "codex-cli", roleId: null, joinedAt: "2026-08-02T00:00:00.000Z", leftAt: null },
        { seatId: "seat-2", agentId: "claude-code", roleId: null, joinedAt: "2026-08-02T00:00:00.000Z", leftAt: null },
      ],
    };
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <SessionWorkspaceRegionsHost
          activeSession={session}
          apiComposer={<div>Shared composer</div>}
          messages={[]}
          messagesPartial={false}
          onOpenSettings={() => undefined}
          sessionActivationKey={0}
        />
      </QueryClientProvider>,
    );

    expect(html).toContain("Shared CLI session");
    expect(html).toContain("Shared composer");
    expect(html).not.toContain('data-testid="session-roster-chips"');
    expect(html).not.toMatch(/>codex-cli<\/span>|>claude-code<\/span>/);
    expect(html).not.toContain("Terminal input");
  });

  it("keeps every managed CLI identity out of the conversation header", () => {
    for (const agentId of managedCliAgentIds) {
      const session: Session = {
        ...cleanRecovery,
        id: `session-${agentId}`, title: `${agentId} session`, agentId, interactionMode: "cli",
        personalizationMode: "standard", lifecycleState: "idle", folder: null, projectPath: null, worktreePath: null, worktreeName: null,
        worktreeBranch: null, remoteWorkspace: null, remoteSshConnectionId: null,
        remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null, pinned: false,
        archived: false, createdAt: "2026-08-11T00:00:00.000Z", updatedAt: "2026-08-11T00:00:00.000Z",
        seats: [{ seatId: `seat-${agentId}`, agentId, roleId: "builtin-architect", joinedAt: "2026-08-11T00:00:00.000Z", leftAt: null }],
      };
      const html = renderToStaticMarkup(<SessionConversationHeader isStreaming={false} session={session} />);
      expect(html).not.toContain('data-testid="session-roster-chips"');
      expect(html).not.toMatch(new RegExp(`>架构师<\\/span>|>${agentId}<\\/span>`));
    }
  });

  it("collapses workspace tabs without changing the focused conversation panel", () => {
    const session: Session = {
      ...cleanRecovery,
      id: "session-focus", title: "Focused session", agentId: "onepiece", interactionMode: "api",
      personalizationMode: "standard", lifecycleState: "idle", folder: null, projectPath: null, worktreePath: null, worktreeName: null,
      worktreeBranch: null, remoteWorkspace: null, remoteSshConnectionId: null,
      remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null, pinned: false,
      archived: false, createdAt: "2026-08-02T00:00:00.000Z", updatedAt: "2026-08-02T00:00:00.000Z",
    };
    const html = renderToStaticMarkup(
      <QueryClientProvider client={new QueryClient()}>
        <SessionWorkspaceRegionsHost
          activeSession={session}
          apiComposer={<div>Focused composer</div>}
          focusMode
          messages={[]}
          messagesPartial={false}
          onOpenSettings={() => undefined}
          sessionActivationKey={0}
        />
      </QueryClientProvider>,
    );

    expect(html).toContain('data-focus-mode="true"');
    expect(html).toContain("Focused session");
    expect(html).toContain("Focused composer");
    expect(html).not.toContain('role="tablist"');
  });

  it("does not render raw Markdown HTML", () => {
    const html = renderToStaticMarkup(<ReactMarkdown skipHtml>{"# Safe\n<script>alert('x')</script>"}</ReactMarkdown>);
    expect(html).toContain("Safe");
    expect(html).not.toContain("<script");
  });

  it("keeps all managed CLI terminal output readable", () => {
    expect(agentTerminalInputClassName).toContain("ucd-agent-terminal-input");
    const source = readFileSync(new URL("./agent-terminal-tab.tsx", import.meta.url), "utf8");
    const primarySurfacesSource = readFileSync(new URL("./session-primary-surfaces.tsx", import.meta.url), "utf8");
    const themeSource = readFileSync(new URL("./terminal-theme.ts", import.meta.url), "utf8");
    const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");

    expect(managedCliAgentIds).toEqual([
      "claude-code",
      "codex-cli",
      "opencode",
      "antigravity-cli",
      "gemini-cli",
    ]);
    expect(primarySurfacesSource).toContain("<AgentTerminalTab isVisible={isVisible}");
    expect(source).toContain("requestAnimationFrame");
    expect(source).toContain("fitRef.current?.fit()");
    expect(source).toContain("resizeAgentTerminal");
    expect(source).toContain("}, [isVisible]);");
    expect(source).not.toContain("bg-zinc-950");
    // Full-screen TUIs paint 256-color/truecolor backgrounds that no per-class CSS
    // override can catch, so the terminal renders on an opaque dark canvas with a
    // complete ANSI palette instead of a transparent surface patched per ANSI class.
    expect(source).toContain("allowTransparency: false");
    expect(themeSource).toContain("selectionForeground");
    expect(themeSource).toContain("--terminal-background");
    expect(themeSource).toContain("brightWhite");
    expect(themeSource).not.toContain("rgba(0, 0, 0, 0)");
    expect(styles).toContain("--terminal-background");
    expect(styles).toContain("--terminal-ansi-blue");
    expect(styles).toContain("background: var(--terminal-background)");
    expect(styles).not.toContain(".xterm-bg-0");
    expect(styles).not.toContain(".xterm-fg-257");
  });

  it("keeps the ordinary shell readable for ANSI inverse output", () => {
    // The terminal is constructed by the surface, which is the component that owns one Shell's
    // xterm instance; `shell-tab.tsx` above it owns the list and the dialogs.
    const source = readFileSync(new URL("./shell-surface.tsx", import.meta.url), "utf8");
    const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");

    expect(source).toContain("allowTransparency: false");
    expect(source).toContain("createTerminalTheme()");
    expect(source).toContain("ucd-shell-terminal");
    // Inverse video swaps theme fg/bg; both come from the opaque terminal palette.
    expect(styles).toContain("background: var(--terminal-background)");
    expect(styles).not.toContain(".xterm-viewport");
    expect(styles).not.toContain(".xterm-bg-257");
  });

  it("reconnects stopped agent terminals only after an explicit session activation", () => {
    const source = readFileSync(new URL("./agent-terminal-tab.tsx", import.meta.url), "utf8");

    expect(source).toContain("sessionActivationKey");
    expect(source).toContain("state !== \"stopped\" && state !== \"failed\"");
    expect(source).toContain("setConnectNonce");
  });

  it("detaches the Agent terminal renderer without stopping accepted native work", () => {
    const source = readFileSync(new URL("./agent-terminal-tab.tsx", import.meta.url), "utf8");
    const cleanup = source.slice(source.indexOf("return () => {"), source.indexOf("}, [connectNonce"));

    expect(cleanup).toContain("unsubscribe?.()");
    expect(cleanup).toContain("terminal.dispose()");
    expect(cleanup).not.toContain("stopGeneration(");
    expect(cleanup).not.toContain("stopAgentTerminal(");
  });
});
