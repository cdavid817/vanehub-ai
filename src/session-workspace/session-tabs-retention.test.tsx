// @vitest-environment jsdom

import { screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { Session } from "../types/agent";
import { SessionTabs } from "./session-tabs";

vi.mock("./agent-terminal-tab", () => ({
  AgentTerminalTab: ({ active }: { active: boolean }) => (
    <div data-active={String(active)} data-testid="retained-agent-terminal" />
  ),
}));

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: () => <div data-testid="lazy-session-page" />,
}));

vi.mock("./folder-opener-control", () => ({
  FolderOpenerControl: () => <div data-testid="folder-opener" />,
}));

const session: Session = {
  id: "retained-terminal-session",
  title: "Retained terminal",
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
  runtimeSessionId: "terminal-runtime",
  categoryId: null,
  pinned: false,
  archived: false,
  createdAt: "2026-08-02T00:00:00.000Z",
  updatedAt: "2026-08-02T00:00:00.000Z",
};

describe("session tab retention", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  it("keeps the agent terminal mounted and reactivates it after page switches", async () => {
    const { user } = renderWithAppProviders(
      <SessionTabs
        activeSession={session}
        messages={[]}
        messagesPartial={false}
        onOpenSettings={() => undefined}
        sessionActivationKey={1}
      />,
    );
    const terminal = screen.getByTestId("retained-agent-terminal");
    expect(terminal.dataset.active).toBe("true");

    await user.click(screen.getByRole("tab", { name: "Report" }));
    expect(screen.getByTestId("retained-agent-terminal")).toBe(terminal);
    expect(terminal.dataset.active).toBe("false");
    expect(terminal.closest("section")?.classList.contains("hidden")).toBe(true);

    await user.click(screen.getByRole("tab", { name: "Workspace" }));
    expect(screen.getByTestId("retained-agent-terminal")).toBe(terminal);
    expect(terminal.dataset.active).toBe("true");
    expect(terminal.closest("section")?.classList.contains("block")).toBe(true);
  });
});
