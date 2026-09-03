// @vitest-environment jsdom

import { screen, waitFor } from "@testing-library/react";
import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { GitDiffHunk, GitStatusEntry } from "../types/session-workspace";
import { ChangesTab } from "./changes-tab";

const entries: GitStatusEntry[] = [
  { path: "src/main.ts", previousPath: null, index: "unmodified", worktree: "modified" },
  { path: "README.md", previousPath: null, index: "added", worktree: "unmodified" },
];

const sampleHunk: GitDiffHunk = {
  header: "@@ -1,3 +1,4 @@",
  oldStart: 1,
  oldLines: 3,
  newStart: 1,
  newLines: 4,
  lines: [
    { kind: "context", content: "line 1", oldLineNumber: 1, newLineNumber: 1 },
    { kind: "addition", content: "new line", oldLineNumber: null, newLineNumber: 2 },
    { kind: "context", content: "line 2", oldLineNumber: 2, newLineNumber: 3 },
  ],
};

const { mockAgentService } = vi.hoisted(() => ({
  mockAgentService: {
    getSessionGitStatus: vi.fn(),
    getSessionGitDiff: vi.fn(),
  },
}));

vi.mock("../services/runtime-agent-client", () => ({
  agentService: mockAgentService,
}));

function makeStatus(overrides?: { items?: GitStatusEntry[]; isGit?: boolean; truncated?: boolean }) {
  return Promise.resolve({
    context: { availability: "available" as const, rootName: "project", reason: null },
    isGit: overrides?.isGit ?? true,
    branch: "main",
    items: overrides?.items ?? entries,
    truncated: overrides?.truncated ?? false,
    nextCursor: null,
  });
}

function makeDiff(overrides?: { files?: typeof sampleDiffFiles; truncated?: boolean }) {
  return Promise.resolve({
    context: { availability: "available" as const, rootName: "project", reason: null },
    source: "working" as const,
    files: overrides?.files ?? sampleDiffFiles,
    truncated: overrides?.truncated ?? false,
  });
}

const sampleDiffFiles = [
  { oldPath: "src/main.ts", newPath: "src/main.ts", binary: false, oversized: false, hunks: [sampleHunk] },
];

describe("ChangesTab", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
  });

  beforeEach(() => {
    vi.clearAllMocks();
    mockAgentService.getSessionGitDiff.mockReturnValue(makeDiff());
  });

  it("renders the branch name and file status list", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus());

    renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    await waitFor(() => {
      // Branch name is shown above the file list
      expect(screen.getByText("main")).toBeTruthy();
      // File entries appear in the list (also in diff header since first is auto-selected)
      expect(screen.getAllByText("src/main.ts").length).toBeGreaterThanOrEqual(1);
      expect(screen.getByText("README.md")).toBeTruthy();
      // Status codes rendered in monospace spans
      const codes = document.querySelectorAll(".font-mono");
      expect(codes[0].textContent).toBe(" M");
      expect(codes[1].textContent).toBe("A ");
    });
  });

  it("shows a not-git message when the workspace is not a git repo", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus({ isGit: false, items: [] }));

    renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(screen.getByText(/not a git/i)).toBeTruthy();
    });
  });

  it("shows a clean message when there are no changes", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus({ items: [] }));

    renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(screen.getByText(/no visible changes/i)).toBeTruthy();
    });
  });

  it("auto-selects the first file and loads its diff", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus());

    renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(mockAgentService.getSessionGitDiff).toHaveBeenCalledWith(
        "session-1",
        "src/main.ts",
        "working",
      );
    });

    // Diff content should be rendered (the hunk header)
    await waitFor(() => {
      expect(screen.getByText("@@ -1,3 +1,4 @@")).toBeTruthy();
    });
  });

  it("fetches the diff when a different file is clicked", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus());
    mockAgentService.getSessionGitDiff.mockClear();

    const { user } = renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    await waitFor(() => {
      expect(screen.getByText("README.md")).toBeTruthy();
    });

    // Clear the initial auto-select diff call
    mockAgentService.getSessionGitDiff.mockClear();

    await user.click(screen.getByText("README.md"));

    await waitFor(() => {
      expect(mockAgentService.getSessionGitDiff).toHaveBeenCalledWith(
        "session-1",
        "README.md",
        "working",
      );
    });
  });

  it("says the diff was cut rather than that results are partial", async () => {
    mockAgentService.getSessionGitStatus.mockReturnValue(makeStatus());
    mockAgentService.getSessionGitDiff.mockReturnValue(makeDiff({ truncated: true }));

    renderWithAppProviders(<ChangesTab sessionId="session-1" />);

    // The message names this cause specifically. A generic "results are partial" was true of four
    // different surfaces and told a reader nothing about which one, or what to do next — and a
    // truncated diff in particular is the one way a change can be wrong without looking wrong.
    await waitFor(() => {
      expect(screen.getByText(/cut at its size limit|在大小上限处被截断/)).toBeTruthy();
    });
  });
});
