// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { Session } from "../types/agent";
import { SessionWorkspaceRegionsHost } from "./session-tabs";

vi.mock("./agent-terminal-tab", () => ({
  AgentTerminalTab: ({ isVisible }: { isVisible: boolean }) => (
    <div data-active={String(isVisible)} data-testid="agent-terminal" />
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

// The workspace's descendants read react-query context, so the provider tree has to be reused
// across rerenders (not `renderWithAppProviders`, whose wrapper element isn't exported) —
// otherwise React sees the root element type change on rerender and remounts the host from
// scratch, silently defeating the point of "rerender with new props onto the same instance".
function mount(ui: ReactElement) {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  const wrap = (element: ReactElement) => (
    <I18nextProvider i18n={i18n}><QueryClientProvider client={queryClient}>{element}</QueryClientProvider></I18nextProvider>
  );
  const rendered = render(wrap(ui));
  return { rerenderTabs: (next: ReactElement) => rendered.rerender(wrap(next)) };
}

// The Runtime Panel is behind a `LazyFeature` boundary (its shared `RuntimePanel` primitive and
// icon dependencies are dead weight for a reader who never opens it) — the first render that
// activates a runtime surface needs a tick to resolve, unlike the primary tab bar, which is always
// synchronous. `findByRole` waits for either.
async function expectActiveTab(name: string) {
  expect((await screen.findByRole("tab", { name })).getAttribute("aria-selected")).toBe("true");
}

describe("session workspace requested-surface reset on session switch", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("lands on Work when the caller clears the request in the same update as the session switch", async () => {
    const { rerenderTabs } = mount(
      <SessionWorkspaceRegionsHost
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedSurface={null}
        requestedSurfaceNonce={0}
        sessionActivationKey={1}
      />,
    );
    await expectActiveTab("Work");

    // Simulate `/logs` — Logs is a Runtime Panel surface, so the request also opens the panel.
    rerenderTabs(
      <SessionWorkspaceRegionsHost
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedSurface="logs"
        requestedSurfaceNonce={1}
        sessionActivationKey={1}
      />,
    );
    await expectActiveTab("Logs");

    // MainLayout's fix clears the request in the same render that changes the active session, so
    // the host never observes a stale, truthy requestedSurface paired with a new sessionId. This
    // asserts the contract the fix relies on; it does not exercise MainLayout's own wiring.
    rerenderTabs(
      <SessionWorkspaceRegionsHost
        activeSession={session("session-b")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedSurface={null}
        requestedSurfaceNonce={0}
        sessionActivationKey={1}
      />,
    );
    await expectActiveTab("Work");
    // The Runtime Panel closed with the session reset, so Logs is no longer even in the tree.
    expect(screen.queryByRole("tab", { name: "Logs" })).toBeNull();
  });

  it("documents why the clear must happen: an un-cleared request survives a session switch", async () => {
    const { rerenderTabs } = mount(
      <SessionWorkspaceRegionsHost
        activeSession={session("session-a")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedSurface="logs"
        requestedSurfaceNonce={1}
        sessionActivationKey={1}
      />,
    );
    await expectActiveTab("Logs");

    // A caller that fails to clear the request (pre-fix MainLayout) leaves requestedSurface truthy
    // across the switch. The reset to Work happens during the scope provider's render and the
    // surface-request effect runs afterwards, keyed on sessionId, so the request re-fires and wins.
    rerenderTabs(
      <SessionWorkspaceRegionsHost
        activeSession={session("session-b")}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        requestedSurface="logs"
        requestedSurfaceNonce={1}
        sessionActivationKey={1}
      />,
    );
    await expectActiveTab("Logs");
  });
});
