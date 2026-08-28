// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { Session } from "../types/agent";
import { SessionTabs } from "./session-tabs";

vi.mock("./agent-terminal-tab", () => ({
  AgentTerminalTab: ({ active }: { active: boolean }) => (
    <div data-active={String(active)} data-testid="agent-terminal" />
  ),
}));

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: () => <div data-testid="lazy-session-page" />,
}));

vi.mock("./folder-opener-control", () => ({
  FolderOpenerControl: () => <div data-testid="folder-opener" />,
}));

function session(id: string): Session {
  return {
    id,
    title: id,
    agentId: "opencode",
    interactionMode: "cli",
    personalizationMode: "standard", lifecycleState: "running",
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
    runtimeSessionId: `${id}-runtime`,
    categoryId: null,
    pinned: false,
    archived: false,
    createdAt: "2026-08-02T00:00:00.000Z",
    updatedAt: "2026-08-02T00:00:00.000Z",
  };
}

// SessionTabs' descendants read react-query context, so the provider tree has to be reused across
// rerenders (not `renderWithAppProviders`, whose wrapper element isn't exported) — otherwise React
// sees the root element type change on rerender and remounts SessionTabs from scratch, silently
// defeating the point of "rerender with new props onto the same instance".
function mount(ui: ReactElement) {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  const wrap = (element: ReactElement) => (
    <I18nextProvider i18n={i18n}><QueryClientProvider client={queryClient}>{element}</QueryClientProvider></I18nextProvider>
  );
  const rendered = render(wrap(ui));
  return { rerenderTabs: (next: ReactElement) => rendered.rerender(wrap(next)) };
}

function expectActiveTab(name: string) {
  expect(screen.getByRole("tab", { name }).getAttribute("aria-selected")).toBe("true");
}

describe("SessionTabs slash-tab request reset on session switch", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("lands on chat when the caller clears the request in the same update as the session switch", () => {
    const { rerenderTabs } = mount(
      <SessionTabs
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedTab={null}
        requestedTabNonce={0}
        sessionActivationKey={1}
      />,
    );
    expectActiveTab("Workspace");

    // Simulate `/logs`.
    rerenderTabs(
      <SessionTabs
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedTab="logs"
        requestedTabNonce={1}
        sessionActivationKey={1}
      />,
    );
    expectActiveTab("Logs");

    // MainLayout's fix clears the request in the same render that changes the active session, so
    // SessionTabs never observes a stale, truthy requestedTab paired with a new sessionId. This
    // asserts the contract the fix relies on; it does not exercise MainLayout's own wiring.
    rerenderTabs(
      <SessionTabs
        activeSession={session("session-b")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedTab={null}
        requestedTabNonce={0}
        sessionActivationKey={1}
      />,
    );
    expectActiveTab("Workspace");
  });

  it("documents why the clear must happen: an un-cleared request survives a session switch", () => {
    const { rerenderTabs } = mount(
      <SessionTabs
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedTab="logs"
        requestedTabNonce={1}
        sessionActivationKey={1}
      />,
    );
    expectActiveTab("Logs");

    // A caller that fails to clear the request (pre-fix MainLayout) leaves requestedTab truthy
    // across the switch. SessionTabs' reset-to-chat effect and its tab-request effect are both
    // keyed on sessionId and run in declaration order, so the tab-request effect re-fires and wins.
    rerenderTabs(
      <SessionTabs
        activeSession={session("session-b")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedTab="logs"
        requestedTabNonce={1}
        sessionActivationKey={1}
      />,
    );
    expectActiveTab("Logs");
  });
});
