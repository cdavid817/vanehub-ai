// @vitest-environment jsdom

import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import type { Session } from "../types/agent";
import type { FolderOpenerAvailability, FolderOpenerPreferences } from "../types/folder-opener";

const service = vi.hoisted(() => ({
  listFolderOpeners: vi.fn(),
  getFolderOpenerPreferences: vi.fn(),
  subscribeFolderOpenerEvents: vi.fn(),
  openSessionFolder: vi.fn(),
  saveFolderOpenerPreferences: vi.fn(),
}));

vi.mock("../services/runtime-agent-client", () => ({ agentService: service }));
import { FolderOpenerControl } from "./folder-opener-control";

function makeSession(overrides: Partial<Session> = {}): Session {
  return {
    id: "session", title: "Session", agentId: "claude-code", interactionMode: "cli", personalizationMode: "standard",
    lifecycleState: "running", recoveryStatus: "clean", recoveryRevision: 0, stateRevision: 0, historyRevision: 0,
    activeExecutionRunId: null, folder: null, projectPath: "D:/repo", worktreePath: "D:/repo-worktree",
    worktreeName: "feature", worktreeBranch: "feature/x", remoteWorkspace: null, remoteSshConnectionId: null,
    remoteSshConnectionRevision: null, runtimeSessionId: null, categoryId: null, pinned: false, archived: false,
    createdAt: "now", updatedAt: "now", ...overrides,
  };
}

function opener(id: FolderOpenerAvailability["id"]): FolderOpenerAvailability {
  return { id, category: "editor", status: "available", executablePath: "C:/bin", version: "1.0", edition: null, detectionSource: "path", iconKey: id, reason: null };
}

const preferences: FolderOpenerPreferences = {
  configuredDefaultOpenerId: "vscode",
  effectiveDefaultOpenerId: "vscode",
  enabledOpenerIds: ["vscode", "file-explorer"],
  fallbackActive: false,
};

describe("FolderOpenerControl keyboard navigation", () => {
  beforeAll(async () => activateAppLanguage("en"));

  async function renderControl() {
    service.listFolderOpeners.mockResolvedValue([opener("vscode"), opener("file-explorer")]);
    service.getFolderOpenerPreferences.mockResolvedValue(preferences);
    service.subscribeFolderOpenerEvents.mockResolvedValue(() => undefined);
    render(<FolderOpenerControl onOpenSettings={vi.fn()} session={makeSession()} />);
    await waitFor(() => expect(screen.getByRole("button", { name: "Open folder with Visual Studio Code" })).toBeTruthy());
  }

  it("focuses the first item when opened", async () => {
    await renderControl();
    fireEvent.click(screen.getByRole("button", { name: "Choose workspace opener" }));
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Visual Studio Code" }));
  });

  // 20.7 regression: this popup's roving index and its on-open focus used to live in two separate
  // effects racing on the open transition -- reopening after arrowing to a later item left focus
  // stuck on that stale item instead of resetting to the first one.
  it("resets to the first item on a second open, even after navigating to the last item", async () => {
    await renderControl();
    const menuButton = screen.getByRole("button", { name: "Choose workspace opener" });
    fireEvent.click(menuButton);
    fireEvent.keyDown(screen.getByRole("menuitem", { name: "Visual Studio Code" }), { key: "End" });
    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Manage workspace openers" }));

    // Escape closes it (this popup's own real close path), then reopen.
    fireEvent.keyDown(screen.getByRole("menu"), { key: "Escape" });
    fireEvent.click(menuButton);

    expect(document.activeElement).toBe(screen.getByRole("menuitem", { name: "Visual Studio Code" }));
  });
});
