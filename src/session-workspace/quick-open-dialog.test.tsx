/** @vitest-environment jsdom */
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type {
  WorkspacePathMatch,
  WorkspacePathSearchResult,
} from "../types/session-workspace-inspection";
import { QuickOpenDialog } from "./quick-open-dialog";

function match(path: string, kind: "file" | "directory" = "file"): WorkspacePathMatch {
  return { name: path.split("/").pop() ?? path, path, kind };
}

function result(overrides: Partial<WorkspacePathSearchResult> = {}): WorkspacePathSearchResult {
  return { coverage: { state: "complete" }, matches: [], ...overrides };
}

let search: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  search = vi.spyOn(agentService, "searchWorkspacePaths");
});

afterEach(() => {
  vi.restoreAllMocks();
});

/**
 * Waits for a row and drains the effects the render queued behind it.
 *
 * The dialog sends the highlight back to the top whenever a result set arrives, in an effect
 * queued by the same commit that renders the rows. React runs passive effects after the commit, so
 * a case that starts pressing keys the moment it sees a row gets its keystrokes undone by an
 * effect from before it pressed them. Under an unloaded run the effect always won the race and
 * these read as deterministic; under a full suite they do not.
 */
async function shown(text: string) {
  await waitFor(() => expect(screen.getByText(text)).toBeTruthy());
  await act(async () => {});
}

function open(onSelect = vi.fn()) {
  const rendered = renderWithAppProviders(
    <QuickOpenDialog isOpen onClose={vi.fn()} onSelect={onSelect} sessionId="session-1" />,
  );
  return { ...rendered, onSelect };
}

describe("QuickOpenDialog", () => {
  it("lists what the search returned", async () => {
    search.mockResolvedValue(result({ matches: [match("src/main.rs"), match("src", "directory")] }));
    open();

    await waitFor(() => expect(screen.getByText("src/main.rs")).toBeTruthy());
    expect(screen.getByText("src")).toBeTruthy();
  });

  it("moves through results with the arrow keys and opens with Enter", async () => {
    search.mockResolvedValue(result({ matches: [match("a.rs"), match("b.rs"), match("c.rs")] }));
    const { onSelect } = open();
    await shown("c.rs");

    const input = screen.getByRole("textbox");
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "Enter" });

    // Down twice, up once, so the second row. Keyboard-first is the only thing that makes this
    // faster than the tree.
    expect(onSelect).toHaveBeenCalledWith(match("b.rs"));
  });

  it("does not walk past either end of the list", async () => {
    search.mockResolvedValue(result({ matches: [match("a.rs"), match("b.rs")] }));
    const { onSelect } = open();
    await shown("b.rs");

    const input = screen.getByRole("textbox");
    fireEvent.keyDown(input, { key: "ArrowUp" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "ArrowDown" });
    fireEvent.keyDown(input, { key: "Enter" });

    // Clamped rather than wrapped. A list that jumped back to the top on the last row would open a
    // file the reader was not looking at.
    expect(onSelect).toHaveBeenCalledWith(match("b.rs"));
  });

  it("keeps focus in the input so a reader can keep typing", async () => {
    search.mockResolvedValue(result({ matches: [match("a.rs")] }));
    open();
    await shown("a.rs");

    const input = screen.getByRole("textbox");
    fireEvent.keyDown(input, { key: "ArrowDown" });

    // Moving the highlight must not move focus: the interaction is typing, and a result list that
    // took focus would end it.
    expect(document.activeElement).toBe(input);
  });

  it("renders the newest answer even when an older one arrives after it", async () => {
    const slow = result({ matches: [match("stale.rs")] });
    const fast = result({ matches: [match("fresh.rs")] });
    let releaseSlow: (value: WorkspacePathSearchResult) => void = () => {};
    search
      .mockImplementationOnce(
        () =>
          new Promise<WorkspacePathSearchResult>((resolve) => {
            releaseSlow = resolve;
          }),
      )
      .mockResolvedValue(fast);

    open();
    // The first request has to be in flight before the second is issued, or the debounce simply
    // cancels it and the case never exercises the overtaking it exists to check.
    await waitFor(() => expect(search).toHaveBeenCalledTimes(1));

    const input = screen.getByRole("textbox");
    fireEvent.change(input, { target: { value: "fr" } });
    await waitFor(() => expect(screen.getByText("fresh.rs")).toBeTruthy());

    releaseSlow(slow);

    // The abandoned keystroke's answer is dropped. This is what cancellation can mean over a
    // one-round-trip command: not that the work stops, but that nobody looks at it.
    await waitFor(() => expect(screen.queryByText("stale.rs")).toBeNull());
    expect(screen.getByText("fresh.rs")).toBeTruthy();
  });

  it("says when part of the workspace was never searched", async () => {
    search.mockResolvedValue(
      result({
        coverage: { state: "partial", reasonCode: "workspace_search_scan_limit" },
        matches: [match("a.rs")],
      }),
    );
    open();

    // Distinct from "more matches follow": paging to the end of the list does not resolve this,
    // and a reader who thought it did would conclude a file is not there.
    await waitFor(() =>
      expect(screen.getByText(/not searched|未被搜索/)).toBeTruthy(),
    );
  });

  it("offers another page only when one exists", async () => {
    search.mockResolvedValue(result({ matches: [match("a.rs")], nextCursor: "cursor-1" }));
    open();
    await waitFor(() => expect(screen.getByText("a.rs")).toBeTruthy());

    const more = screen.getByRole("button", { name: /load more|加载更多/i });
    search.mockResolvedValue(result({ matches: [match("b.rs")] }));
    fireEvent.click(more);

    // Appended, not replaced: a reader who asked for more expects the list to grow rather than to
    // jump to a page they have to scroll back from.
    await waitFor(() => expect(screen.getByText("b.rs")).toBeTruthy());
    expect(screen.getByText("a.rs")).toBeTruthy();
  });
});
