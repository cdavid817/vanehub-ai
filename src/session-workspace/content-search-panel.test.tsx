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
  return { generation: 1, coverage: { state: "complete" }, matches: [], ...overrides };
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
  fireEvent.change(screen.getByRole("combobox"), { target: { value } });
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

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Enter" });

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

    fireEvent.keyDown(screen.getByRole("combobox"), { key: "Escape" });

    // Closing alone would leave a full workspace scan running for a reader who has already stopped
    // looking at it.
    expect(cancel).toHaveBeenCalled();
    expect(onClose).toHaveBeenCalled();
  });

  it("keeps one search id for the panel so a keystroke supersedes rather than races", async () => {
    // Still running when the next keystroke arrives, which is the whole point: an answered search
    // needs no stopping.
    search.mockImplementation(() => new Promise<WorkspaceContentSearchResult>(() => {}));
    open();
    await type("need");
    await waitFor(() => expect(search).toHaveBeenCalledTimes(1));

    await type("needle");
    await waitFor(() => expect(search).toHaveBeenCalledTimes(2));

    // One id, reused. Registering under an id already in flight is what stops the previous scan, and
    // it happens under the registry's own lock — so there is no window where two scans are running
    // and neither has been told to stop. A fresh id per keystroke would make every scan look
    // independent, and the only thing ending the old one would be a cancel racing the new request.
    const first = search.mock.calls[0]?.[0] as { searchId: string };
    const second = search.mock.calls[1]?.[0] as { searchId: string };
    expect(first.searchId).toBeTruthy();
    expect(second.searchId).toBe(first.searchId);
  });

  it("drops an answer older than the one already on screen", async () => {
    // Two scans in flight and the older one returns second. Nothing in arrival order says which
    // query an answer was for, so a panel that took the last response would replace a fresh result
    // with a stale one and leave nothing on screen to say it had.
    search.mockResolvedValueOnce(result({ generation: 7, matches: [hit({ path: "fresh.rs" })] }));
    open();
    await type("needle");
    await waitFor(() => expect(screen.getByText(/fresh\.rs/)).toBeTruthy());

    search.mockResolvedValueOnce(result({ generation: 3, matches: [hit({ path: "stale.rs" })] }));
    await type("needles");
    await waitFor(() => expect(search).toHaveBeenCalledTimes(2));

    expect(screen.queryByText(/stale\.rs/)).toBeNull();
    expect(screen.getByText(/fresh\.rs/)).toBeTruthy();
  });

  it("stops the search when the panel goes away rather than when the reader closes it", async () => {
    // Unmounted, not closed. Escape has a handler; a route change, a session switch, or a parent
    // re-render does not — and a scan reading every file in a workspace has nobody left waiting on
    // it either way.
    search.mockImplementation(() => new Promise<WorkspaceContentSearchResult>(() => {}));
    const { unmount } = open();
    await type("needle");
    await waitFor(() => expect(search).toHaveBeenCalled());
    cancel.mockClear();

    unmount();

    expect(cancel).toHaveBeenCalledTimes(1);
  });

  it("says the host was too busy rather than that nothing matched", async () => {
    search.mockResolvedValue(
      result({ coverage: { state: "unavailable", reasonCode: "inspection_busy" } }),
    );
    open();
    await type("needle");

    // A refused admission is the one stop a reader can act on directly: wait and ask again. Folding
    // it into "no matches" turns a queue into a fact about the workspace.
    await waitFor(() => expect(screen.getByText(/搜索过多|Too many searches/)).toBeTruthy());
  });

  it("words every budget stop rather than putting its code on screen", async () => {
    // Each of these is a different thing to have run out of, and a reader deciding whether to narrow
    // the query or the folder needs to know which. The failure this guards against is not a missing
    // sentence — it is `byte_budget_exhausted` rendered verbatim, which the key lookup would do if a
    // code were ever added on one side only.
    const budgetStops = [
      "directory_budget_exhausted",
      "entry_budget_exhausted",
      "file_budget_exhausted",
      "byte_budget_exhausted",
      "metadata_budget_exhausted",
      "candidate_budget_exhausted",
      "result_budget_exhausted",
      "depth_budget_exhausted",
      "deadline_exceeded",
      "unreadable_entries",
    ];
    const worded = new Set<string>();

    for (const reasonCode of budgetStops) {
      search.mockResolvedValue(result({ coverage: { state: "partial", reasonCode } }));
      const { unmount } = open();
      await type("needle");
      const notice = await screen.findByRole("status");
      const sentence = notice.parentElement?.textContent ?? "";

      expect(sentence).not.toContain(reasonCode);
      expect(sentence.length).toBeGreaterThan(0);
      worded.add(sentence);
      unmount();
    }

    // Distinct, not merely present. Ten stops sharing one sentence would pass every assertion above
    // and still leave the reader unable to tell which limit they hit.
    expect(worded.size).toBe(budgetStops.length);
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
