/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type {
  WorkspaceContentMatch,
  WorkspaceContentSearchResult,
} from "../types/session-workspace-inspection";
import { ContentSearchPanel } from "./content-search-panel";

function hit(overrides: Partial<WorkspaceContentMatch> = {}): WorkspaceContentMatch {
  return {
    path: "src/main.rs",
    line: 12,
    column: 5,
    snippet: "let needle = 1;",
    snippetTruncated: false,
    ...overrides,
  };
}

function result(overrides: Partial<WorkspaceContentSearchResult> = {}): WorkspaceContentSearchResult {
  return { coverage: { state: "complete" }, matches: [], ...overrides };
}

let search: ReturnType<typeof vi.spyOn>;
let cancel: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  search = vi.spyOn(agentService, "searchWorkspaceContent");
  cancel = vi.spyOn(agentService, "cancelWorkspaceSearch").mockResolvedValue(true);
});

afterEach(() => {
  vi.restoreAllMocks();
});

function open(onSelect = vi.fn(), onClose = vi.fn()) {
  const rendered = renderWithAppProviders(
    <ContentSearchPanel isOpen onClose={onClose} onSelect={onSelect} sessionId="session-1" />,
  );
  return { ...rendered, onClose, onSelect };
}

async function type(value: string) {
  fireEvent.change(screen.getByRole("textbox"), { target: { value } });
}

describe("ContentSearchPanel", () => {
  it("shows each match as a position rather than just a file", async () => {
    search.mockResolvedValue(result({ matches: [hit()] }));
    open();
    await type("needle");

    // The position is the whole point: a result that named only the file would leave a reader
    // searching a second time, by eye, for the thing the search already found.
    await waitFor(() => expect(screen.getByText("src/main.rs:12:5")).toBeTruthy());
    expect(screen.getByText("let needle = 1;")).toBeTruthy();
  });

  it("hands the selected match's line to the caller", async () => {
    search.mockResolvedValue(result({ matches: [hit({ line: 42 })] }));
    const { onSelect } = open();
    await type("needle");
    await waitFor(() => expect(screen.getByText(/src\/main\.rs:42/)).toBeTruthy());

    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Enter" });

    expect(onSelect).toHaveBeenCalledWith(expect.objectContaining({ line: 42 }));
  });

  it("does not search an empty query", async () => {
    open();
    await type("   ");

    // An empty query would match every line of every file. Nothing to send, so nothing is sent.
    await new Promise((resolve) => setTimeout(resolve, 400));
    expect(search).not.toHaveBeenCalled();
  });

  it("cancels the running search when the reader presses Escape", async () => {
    // Never resolves, so the search is genuinely still in flight when Escape arrives. A search that
    // had already answered would have nothing to cancel, which is a different case entirely.
    search.mockImplementation(() => new Promise<WorkspaceContentSearchResult>(() => {}));
    const { onClose } = open();
    await type("needle");
    await waitFor(() => expect(search).toHaveBeenCalled());

    fireEvent.keyDown(screen.getByRole("textbox"), { key: "Escape" });

    // Closing alone would leave a full workspace scan running for a reader who has already stopped
    // looking at it.
    expect(cancel).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("cancels the previous search before starting the next", async () => {
    // Still running when the next keystroke arrives, which is the whole point: an answered search
    // needs no cancelling.
    search.mockImplementation(() => new Promise<WorkspaceContentSearchResult>(() => {}));
    open();
    await type("need");
    await waitFor(() => expect(search).toHaveBeenCalledTimes(1));
    const first = search.mock.calls[0]?.[0] as { searchId: string };

    await type("needle");
    await waitFor(() => expect(search).toHaveBeenCalledTimes(2));

    // By name, so the still-running scan actually stops. Dropping the answer would be enough for a
    // path search and is not enough here: this one reads every file in the workspace.
    expect(cancel).toHaveBeenCalledWith(first.searchId);
  });

  it("says when part of the workspace was not searched", async () => {
    search.mockResolvedValue(
      result({
        coverage: { state: "partial", reasonCode: "workspace_search_files_skipped" },
        matches: [hit()],
      }),
    );
    open();
    await type("needle");

    await waitFor(() => expect(screen.getByText(/not searched|未被搜索/)).toBeTruthy());
  });

  it("distinguishes an unavailable search from one that matched nothing", async () => {
    search.mockResolvedValue(
      result({ coverage: { state: "unavailable", reasonCode: "remote_ripgrep_missing" } }),
    );
    open();
    await type("needle");

    // "Nothing matched" is a claim about the workspace; "cannot search" is a claim about the host.
    // A reader acts on them completely differently.
    await waitFor(() =>
      expect(screen.getByText(/unavailable|不支持内容搜索/)).toBeTruthy(),
    );
  });
});
