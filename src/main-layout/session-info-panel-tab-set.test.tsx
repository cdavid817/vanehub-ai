/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { Session } from "../types/agent";
import { SessionInfoPanel } from "./session-info-panel";

/**
 * The tab set and the state semantics the evidence summary was added beneath.
 *
 * 14.1 put a new block inside Basic Info and 14.2 made its rows navigate. Both are additions to a
 * panel five other things already depend on, and the failure mode of an addition like that is
 * quiet: a tab that stops appearing for the sessions that used to show it, or a pane that loses
 * what a reader had typed because it started unmounting when inactive.
 *
 * So this holds the shape rather than the contents: which tabs exist for which session, that the
 * optional two appear only when they should, and that every pane stays mounted across a switch.
 */

function session(overrides: Partial<Session> = {}): Session {
  return {
    activeExecutionRunId: null,
    agentId: "codex-cli",
    archived: false,
    categoryId: null,
    createdAt: "2026-08-27T00:00:00.000Z",
    folder: "D:\\code\\vanehub-ai",
    historyRevision: 0,
    id: "session-1",
    interactionMode: "cli",
    lifecycleState: "running",
    pinned: false,
    projectPath: "D:\\code\\vanehub-ai",
    recoveryRevision: 0,
    recoveryStatus: "clean",
    remoteSshConnectionId: null,
    remoteSshConnectionRevision: null,
    remoteWorkspace: null,
    runtimeSessionId: null,
    stateRevision: 0,
    title: "CLI work",
    updatedAt: "2026-08-27T00:00:00.000Z",
    worktreeBranch: null,
    worktreeName: null,
    worktreePath: null,
    ...overrides,
  };
}

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  // The summary is not the subject here; it must simply not take the panel down with it.
  vi.spyOn(agentService, "getWorkspaceEvidenceSummary").mockRejectedValue(new Error("unavailable"));
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(
    new Error("unavailable"),
  );
  vi.spyOn(agentService, "getSessionChatConfig").mockResolvedValue({
    modelId: null,
    providerId: null,
  } as never);
});

afterEach(() => {
  vi.restoreAllMocks();
});

/**
 * The panel's own tabs, not every tab on screen.
 *
 * The Skill pane renders its own Effective/Global/Project tablist inside the panel, so an
 * unscoped role query returns seven tabs from two unrelated controls and any assertion about "the
 * tab set" is really about both.
 */
function tabNames() {
  const tablist = screen.getByRole("tablist", { name: "Info Panel" });
  return within(tablist).getAllByRole("tab").map((tab) => tab.textContent ?? "");
}

describe("the information panel's tab set", () => {
  it("keeps the four tabs every session has", () => {
    renderWithAppProviders(<SessionInfoPanel activeSession={session()} collapsed={false} />);

    expect(tabNames()).toEqual(["Basic Info", "Token Usage", "Skill", "IM"]);
  });

  it("adds Code Index only for a OnePiece session with a workspace", () => {
    renderWithAppProviders(
      <SessionInfoPanel
        activeSession={session({ agentId: "onepiece", worktreePath: "D:\\code\\wt" })}
        collapsed={false}
      />,
    );

    expect(tabNames()).toContain("Code Index");
  });

  it("withholds Code Index from a OnePiece session with no workspace", () => {
    renderWithAppProviders(
      <SessionInfoPanel
        activeSession={session({ agentId: "onepiece", folder: null, projectPath: null })}
        collapsed={false}
      />,
    );

    // A code index for a session with nothing to index is a tab that opens onto an explanation of
    // why it is empty, which is a worse answer than not offering it.
    expect(tabNames()).not.toContain("Code Index");
  });

  it("keeps every pane mounted across a tab switch", async () => {
    renderWithAppProviders(<SessionInfoPanel activeSession={session()} collapsed={false} />);
    await waitFor(() => expect(screen.getByTestId("info-pane-basic")).toBeTruthy());

    fireEvent.click(within(screen.getByRole("tablist", { name: "Info Panel" })).getByRole("tab", { name: "IM" }));

    // Hidden rather than unmounted. These panes hold local form state, and a reader who typed
    // something, checked another tab, and came back would otherwise find it gone.
    for (const pane of ["basic", "usage", "skills", "im"]) {
      expect(screen.getByTestId(`info-pane-${pane}`)).toBeTruthy();
    }
    expect(screen.getByTestId("info-pane-basic").className).toContain("hidden");
    expect(screen.getByTestId("info-pane-im").className).not.toContain("hidden");
  });

  it("keeps Basic Info readable when the evidence summary cannot be read", async () => {
    renderWithAppProviders(<SessionInfoPanel activeSession={session()} collapsed={false} />);

    // The block added in 14.1 sits beneath five fields that were there before it. A failed summary
    // must cost the reader the summary and nothing else.
    await waitFor(() =>
      expect(screen.getByText("This session's activity could not be read.")).toBeTruthy(),
    );
    expect(screen.getByText("CLI work")).toBeTruthy();
  });

  it("says there is no session rather than an empty summary", () => {
    renderWithAppProviders(<SessionInfoPanel activeSession={null} collapsed={false} />);

    expect(screen.getByText("No session selected")).toBeTruthy();
    expect(screen.queryByText("This session's activity could not be read.")).toBeNull();
  });
});
