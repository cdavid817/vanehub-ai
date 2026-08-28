/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import { evidenceSessionIdSchema } from "../contracts/session-workspace-evidence-ids";
import type { WorkspaceEvidenceSummary } from "../types/session-workspace-evidence";
import { SessionEvidenceSummary } from "./session-evidence-summary";

/**
 * Seven lines that say what a session has actually done.
 *
 * The interesting cases are the ones where a number would be a lie. A summary that could not be
 * read and a session that has done nothing produce identical zeroes, and lead a reader to opposite
 * conclusions; a provider that reported no token total is not a provider that reported zero.
 */

const SESSION = "session-1";

function summary(overrides: Partial<WorkspaceEvidenceSummary> = {}): WorkspaceEvidenceSummary {
  return {
    changes: { changedFiles: 8, unviewedFiles: 4 },
    coverage: { reasonCodes: [], state: "complete", truncated: false },
    executionRecords: { failed: 0, running: 1 },
    generatedAt: "2026-08-27T00:00:00Z",
    logs: { newErrors: 3 },
    runState: { startedAt: "2026-08-27T06:32:00Z", status: "running" },
    sessionId: evidenceSessionIdSchema.parse(SESSION),
    shells: { live: 2 },
    traces: { failed: 0, running: 1 },
    usage: { coverage: "partial", reportedTokens: 112000 },
    verification: { failed: 2, passed: 138 },
    ...overrides,
  };
}

function capabilities(provider: "local" | "ssh" | "simulated", git: boolean) {
  return {
    gitDiff: { available: git },
    gitStatus: { available: git },
    listFiles: { available: true },
    provider,
    readTextFiles: { available: true },
    searchFiles: { available: true },
    watchMode: "polling" as const,
  };
}

let getSummary: ReturnType<typeof vi.spyOn>;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  getSummary = vi.spyOn(agentService, "getWorkspaceEvidenceSummary");
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockResolvedValue(
    capabilities("ssh", true),
  );
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("the Basic Info evidence summary", () => {
  it("reports each dimension from the one summary read", async () => {
    getSummary.mockResolvedValue(summary());
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    await waitFor(() => expect(screen.getByText("2 live")).toBeTruthy());
    expect(screen.getByText("8 files · 4 unviewed")).toBeTruthy();
    expect(screen.getByText("138 passed · 2 failed")).toBeTruthy();
    expect(screen.getByText("3 errors · 1 running")).toBeTruthy();
    expect(screen.getByText(/112,000 reported/)).toBeTruthy();
    expect(screen.getByText(/Running/)).toBeTruthy();
  });

  it("describes the workspace from what is already known rather than asking again", async () => {
    getSummary.mockResolvedValue(summary());
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    // Dirty is derived from the change count beside it. A second read for the same fact could
    // disagree with the row below it, and nothing on screen would say which was right.
    await waitFor(() => expect(screen.getByText("Remote SSH · Git · dirty")).toBeTruthy());
  });

  it("says a summary could not be read rather than rendering it as an empty session", async () => {
    getSummary.mockRejectedValue(new Error("evidence_persistence_unavailable"));
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    // Zeroes here would be a confident report that this session did nothing, which is the one
    // conclusion a reader must not draw from a read that failed.
    await waitFor(() =>
      expect(screen.getByText("This session's activity could not be read.")).toBeTruthy(),
    );
    expect(screen.queryByText(/passed/)).toBeNull();
  });

  it("distinguishes a provider that reported nothing from one that reported zero", async () => {
    getSummary.mockResolvedValue(
      summary({ usage: { coverage: "unavailable", reportedTokens: undefined } }),
    );
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    await waitFor(() => expect(screen.getByText("coverage unknown")).toBeTruthy());
    expect(screen.queryByText(/0 reported/)).toBeNull();
  });

  it("shows a run that has not started without inventing a moment for it", async () => {
    getSummary.mockResolvedValue(
      summary({ runState: { startedAt: undefined, status: "queued" } }),
    );
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    await waitFor(() => expect(screen.getByText("Queued")).toBeTruthy());
    expect(screen.queryByText(/since/)).toBeNull();
  });

  it.each([
    ["Runtime", "traces"],
    ["Workspace", "files"],
    ["Shells", "shell"],
    ["Changes", "changes"],
    ["Verification", "report"],
    ["Diagnostics", "logs"],
  ])("sends the %s row to the tab that owns it", async (label, tab) => {
    getSummary.mockResolvedValue(summary());
    const onNavigateToTab = vi.fn();
    renderWithAppProviders(
      <SessionEvidenceSummary
        onNavigateToTab={onNavigateToTab}
        onShowUsage={vi.fn()}
        sessionId={SESSION}
      />,
    );
    await waitFor(() => expect(screen.getByText("2 live")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: new RegExp(`^${label}`) }));

    expect(onNavigateToTab).toHaveBeenCalledWith(tab);
  });

  it("keeps usage in this panel rather than sending it to a workspace tab", async () => {
    getSummary.mockResolvedValue(summary());
    const onNavigateToTab = vi.fn();
    const onShowUsage = vi.fn();
    renderWithAppProviders(
      <SessionEvidenceSummary
        onNavigateToTab={onNavigateToTab}
        onShowUsage={onShowUsage}
        sessionId={SESSION}
      />,
    );
    await waitFor(() => expect(screen.getByText("2 live")).toBeTruthy());

    fireEvent.click(screen.getByRole("button", { name: /^Usage/ }));

    // Token usage is a pane here, not a tab over there. Routing it through the same callback would
    // make one destination mean two different kinds of place.
    expect(onShowUsage).toHaveBeenCalled();
    expect(onNavigateToTab).not.toHaveBeenCalled();
  });

  it("renders rows as plain text where nothing owns the tabs", async () => {
    getSummary.mockResolvedValue(summary());
    renderWithAppProviders(<SessionEvidenceSummary sessionId={SESSION} />);

    await waitFor(() => expect(screen.getByText("2 live")).toBeTruthy());
    // Not disabled buttons. A row nobody can follow is a fact with no destination, and a dead
    // control says the opposite.
    expect(screen.queryAllByRole("button")).toHaveLength(0);
  });

  it("names each row by its label and its value together", async () => {
    getSummary.mockResolvedValue(summary());
    renderWithAppProviders(
      <SessionEvidenceSummary onNavigateToTab={vi.fn()} onShowUsage={vi.fn()} sessionId={SESSION} />,
    );

    // What a screen reader announces is the sentence that is on screen, not the label alone —
    // "Changes" says nothing a reader can act on without the count beside it.
    await waitFor(() =>
      expect(screen.getByRole("button", { name: "Changes 8 files · 4 unviewed" })).toBeTruthy(),
    );
  });

  it("renders nothing at all without a session", () => {
    const { container } = renderWithAppProviders(<SessionEvidenceSummary sessionId={null} />);

    // Not an empty summary and not a placeholder: the panel above already says there is no
    // session, and repeating it is the noise this block was added beneath.
    expect(container.textContent).toBe("");
    expect(getSummary).not.toHaveBeenCalled();
  });
});
