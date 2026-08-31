// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { Session } from "../types/agent";
import { SessionSidebar } from "./session-sidebar";

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
    createdAt: "2026-08-30T00:00:00.000Z",
    updatedAt: "2026-08-30T00:00:00.000Z",
    ...overrides,
  };
}

function baseProps(overrides: Partial<Parameters<typeof SessionSidebar>[0]> = {}) {
  return {
    activeSessionId: null,
    agentsAvailable: true,
    archivedSessions: [],
    categories: [],
    onAssignCategory: vi.fn(),
    onBatchDelete: vi.fn(),
    onContextMenu: vi.fn(),
    onNew: vi.fn(),
    onSearchChange: vi.fn(),
    onSelect: vi.fn(),
    searchQuery: "",
    searchResults: [],
    sessions: [],
    ...overrides,
  };
}

describe("SessionSidebar attention-first ordering (7.3)", () => {
  beforeEach(() => localStorage.clear());

  it("ranks a running, unpinned session ahead of an idle, pinned one", async () => {
    await activateAppLanguage("en");
    const pinnedIdle = session({ id: "pinned-idle", title: "Pinned idle", pinned: true });
    const runningUnpinned = session({ id: "running", title: "Running now", lifecycleState: "running" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [pinnedIdle, runningUnpinned] })} />);

    const cardTitles = screen.getAllByText(/Pinned idle|Running now/).map((node) => node.textContent);
    expect(cardTitles).toEqual(["Running now", "Pinned idle"]);
  });

  it("ranks a session needing review ahead of everything else", async () => {
    await activateAppLanguage("en");
    // Titled distinctly from the "Running" lifecycle label text every row already shows, so the
    // title query below can't accidentally match that label instead of (or as well as) the card.
    const running = session({ id: "running", title: "Running work", lifecycleState: "running" });
    const needsReview = session({ id: "needs-review", title: "Needs review", recoveryStatus: "action_required" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [running, needsReview] })} />);

    const cardTitles = screen.getAllByText(/^Running work$|^Needs review$/).map((node) => node.textContent);
    expect(cardTitles).toEqual(["Needs review", "Running work"]);
  });
});

describe("SessionSidebar row actions (7.14)", () => {
  beforeEach(() => localStorage.clear());

  it("opens the same context menu from the trailing action button as from a right-click", async () => {
    await activateAppLanguage("en");
    const onContextMenu = vi.fn();
    const target = session({ id: "s1", title: "Only session" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ onContextMenu, sessions: [target] })} />);

    fireEvent.click(screen.getByRole("button", { name: "Session actions" }));
    expect(onContextMenu).toHaveBeenCalledTimes(1);
    expect(onContextMenu.mock.calls[0][1]).toEqual(target);
  });
});

describe("SessionSidebar indicators (7.9)", () => {
  beforeEach(() => localStorage.clear());

  it("shows the needs-review indicator only for a session with action_required or quarantined recovery", async () => {
    await activateAppLanguage("en");
    const clean = session({ id: "clean", title: "Clean", recoveryStatus: "clean" });
    const needsReview = session({ id: "review", title: "Review", recoveryStatus: "action_required" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [clean, needsReview] })} />);

    expect(screen.getAllByTestId("session-needs-review-indicator")).toHaveLength(1);
  });

  it("shows the IM indicator only for a session sourced from IM", async () => {
    await activateAppLanguage("en");
    const desktop = session({ id: "desktop", title: "Desktop" });
    const im = session({ id: "im", title: "IM", source: { kind: "im", connector: null } });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [desktop, im] })} />);

    expect(screen.getAllByTestId("session-im-indicator")).toHaveLength(1);
  });

  it("shows the remote indicator only for a session with a remote workspace", async () => {
    await activateAppLanguage("en");
    const local = session({ id: "local", title: "Local" });
    const remote = session({
      id: "remote",
      title: "Remote",
      remoteWorkspace: { host: "devbox", user: null, path: "/srv/app", displayName: "devbox:app", uri: "ssh://devbox/srv/app" },
    });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [local, remote] })} />);

    expect(screen.getAllByTestId("session-remote-indicator")).toHaveLength(1);
  });

  it("shows a distinct agent marker for OnePiece and a neutral fallback for an unrecognized agent id", async () => {
    await activateAppLanguage("en");
    const onepiece = session({ id: "onepiece", title: "OnePiece session", agentId: "onepiece" });
    const unknown = session({ id: "unknown", title: "Unknown agent session", agentId: "future-agent" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [onepiece, unknown] })} />);

    expect(screen.getByTitle("OnePiece")).toBeTruthy();
    expect(screen.getByTitle("Agent")).toBeTruthy();
  });
});

describe("SessionSidebar filters (7.5)", () => {
  beforeEach(() => localStorage.clear());

  it("filters by status through the FilterPopover", async () => {
    await activateAppLanguage("en");
    const idle = session({ id: "idle", title: "Idle session", lifecycleState: "idle" });
    const running = session({ id: "running", title: "Running session", lifecycleState: "running" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [idle, running] })} />);

    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    fireEvent.change(screen.getByRole("combobox", { name: "Status" }), { target: { value: "running" } });

    expect(screen.getByText("Running session")).toBeTruthy();
    expect(screen.queryByText("Idle session")).toBeNull();
  });

  it("filters by source through the FilterPopover", async () => {
    await activateAppLanguage("en");
    const desktop = session({ id: "desktop", title: "Desktop session" });
    const im = session({ id: "im", title: "IM session", source: { kind: "im", connector: null } });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [desktop, im] })} />);

    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    fireEvent.change(screen.getByRole("combobox", { name: "Source" }), { target: { value: "im" } });

    expect(screen.getByText("IM session")).toBeTruthy();
    expect(screen.queryByText("Desktop session")).toBeNull();
  });

  it("filters by project through the FilterPopover, offering only projects present in the current sessions", async () => {
    await activateAppLanguage("en");
    const projectA = session({ id: "a", title: "Project A session", worktreePath: "/repo/a", worktreeName: "a" });
    const projectB = session({ id: "b", title: "Project B session", worktreePath: "/repo/b", worktreeName: "b" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [projectA, projectB] })} />);

    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    fireEvent.change(screen.getByRole("combobox", { name: "Project" }), { target: { value: "project:/repo/a" } });

    expect(screen.getByText("Project A session")).toBeTruthy();
    expect(screen.queryByText("Project B session")).toBeNull();
  });

  it("filters by a relative date window through the FilterPopover", async () => {
    await activateAppLanguage("en");
    const recent = session({ id: "recent", title: "Recent session", updatedAt: new Date().toISOString() });
    const old = session({ id: "old", title: "Old session", updatedAt: "2020-01-01T00:00:00.000Z" });
    renderWithAppProviders(<SessionSidebar {...baseProps({ sessions: [recent, old] })} />);

    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    fireEvent.change(screen.getByRole("combobox", { name: "Date" }), { target: { value: "today" } });

    expect(screen.getByText("Recent session")).toBeTruthy();
    expect(screen.queryByText("Old session")).toBeNull();
  });
});
