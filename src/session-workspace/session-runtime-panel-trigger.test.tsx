// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { I18nextProvider } from "react-i18next";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage, i18n } from "../i18n";
import type { Session } from "../types/agent";
import { SessionWorkspaceRegionsHost } from "./session-tabs";

vi.mock("./agent-terminal-tab", () => ({
  AgentTerminalTab: () => <div data-testid="agent-terminal" />,
}));
vi.mock("../components/lazy-feature", () => ({
  LazyFeature: () => <div data-testid="lazy-session-page" />,
}));
vi.mock("./folder-opener-control", () => ({
  FolderOpenerControl: () => <div data-testid="folder-opener" />,
}));

const session: Session = {
  id: "session-1",
  title: "Runtime panel trigger",
  agentId: "opencode",
  interactionMode: "cli",
  personalizationMode: "standard",
  lifecycleState: "running",
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
  runtimeSessionId: "session-1-runtime",
  categoryId: null,
  pinned: false,
  archived: false,
  createdAt: "2026-08-02T00:00:00.000Z",
  updatedAt: "2026-08-02T00:00:00.000Z",
};

function mount() {
  const queryClient = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  return render(
    <I18nextProvider i18n={i18n}>
      <QueryClientProvider client={queryClient}>
        <SessionWorkspaceRegionsHost
          activeSession={session}
          messages={[]}
          messagesPartial={false}
          onOpenSettings={() => undefined}
          sessionActivationKey={0}
        />
      </QueryClientProvider>
    </I18nextProvider>,
  );
}

// design.md Decision 7's "Open runtime evidence" scenario lists "panel tab" among the ways a
// reader opens a runtime surface — with the nine-tab bar gone, this button is that entry point for
// a reader who was not sent there by a slash command, badge, or evidence link.
describe("Runtime Panel open trigger", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("offers a trigger to open the Runtime Panel when it is closed", () => {
    mount();
    expect(screen.getByRole("button", { name: "Runtime Panel" })).toBeTruthy();
    expect(screen.queryByRole("tablist", { name: "Runtime Panel" })).toBeNull();
  });

  it("opens the panel to its last surface and hides the trigger once open", async () => {
    const user = userEvent.setup();
    mount();

    await user.click(screen.getByRole("button", { name: "Runtime Panel" }));

    expect(await screen.findByRole("tablist", { name: "Runtime Panel" })).toBeTruthy();
    expect(screen.getByRole("tab", { name: "Terminal History" }).getAttribute("aria-selected")).toBe("true");
    expect(screen.queryByRole("button", { name: "Runtime Panel" })).toBeNull();
  });

  it("brings the trigger back once the panel is closed", async () => {
    const user = userEvent.setup();
    mount();
    await user.click(screen.getByRole("button", { name: "Runtime Panel" }));
    await screen.findByRole("tablist", { name: "Runtime Panel" });

    await user.click(screen.getByRole("button", { name: "Close" }));

    expect(await screen.findByRole("button", { name: "Runtime Panel" })).toBeTruthy();
    expect(screen.queryByRole("tablist", { name: "Runtime Panel" })).toBeNull();
  });
});
