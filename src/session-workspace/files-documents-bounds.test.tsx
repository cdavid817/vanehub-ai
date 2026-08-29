/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { DirectoryEntry, DirectoryListing, FileContent } from "../types/session-workspace";
import type {
  WorkspaceContentMatch,
  WorkspaceContentSearchResult,
  WorkspacePathMatch,
  WorkspacePathSearchResult,
} from "../types/session-workspace-inspection";
import { ContentSearchPanel } from "./content-search-panel";
import { FilesTab } from "./files-tab";
import { QuickOpenDialog } from "./quick-open-dialog";
import { sessionWorkspaceLimits } from "./session-workspace-limits";

/**
 * What these surfaces say when they hit a wall.
 *
 * Every read in this workspace is bounded, and each bound has the same failure mode: the panel shows
 * a full-looking list and says nothing, so a reader concludes the workspace holds exactly what is on
 * screen. That conclusion is wrong in a way nothing on the page contradicts — which is worse than an
 * error, because an error at least stops them.
 *
 * The distinction each case here defends is between "this is everything" and "this is as much as
 * was looked at". They are separate facts, they have separate remediations, and a surface that
 * renders them identically has thrown one of them away.
 */

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

function entries(count: number, prefix = "file"): DirectoryEntry[] {
  return Array.from({ length: count }, (_, index) => ({
    name: `${prefix}-${index}.rs`,
    path: `${prefix}-${index}.rs`,
    kind: "file" as const,
    size: 10,
  }));
}

function listing(overrides: Partial<DirectoryListing> = {}): DirectoryListing {
  return { context: CONTEXT, items: [], nextCursor: null, path: "", truncated: false, ...overrides };
}

function pathMatch(path: string): WorkspacePathMatch {
  return { kind: "file", name: path.split("/").pop() ?? path, path };
}

function contentMatch(index: number): WorkspaceContentMatch {
  return {
    column: 1,
    line: index + 1,
    path: `src/file-${index}.rs`,
    snippet: `let value = ${index};`,
    snippetTruncated: false,
  };
}

let listDirectory: ReturnType<typeof vi.spyOn>;
let readFile: ReturnType<typeof vi.spyOn>;
let searchPaths: ReturnType<typeof vi.spyOn>;
let searchContent: ReturnType<typeof vi.spyOn>;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  listDirectory = vi.spyOn(agentService, "listSessionDirectory");
  readFile = vi.spyOn(agentService, "readSessionFile");
  searchPaths = vi.spyOn(agentService, "searchWorkspacePaths");
  searchContent = vi.spyOn(agentService, "searchWorkspaceContent");
  vi.spyOn(agentService, "getFileEvidenceLinks").mockResolvedValue({
    commandIds: [],
    observations: 0,
    runIds: [],
    truncated: false,
  });
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(
    new Error("not asked in this suite"),
  );
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("a directory at its entry bound", () => {
  it("shows every entry it was given and still says there are more", async () => {
    listDirectory.mockResolvedValue(
      listing({ items: entries(sessionWorkspaceLimits.directoryEntries), truncated: true }),
    );

    renderWithAppProviders(<FilesTab sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("file-0.rs")).toBeTruthy());
    // The last row, not just the first: a bound that silently dropped the tail would leave a list
    // that looks complete and is short, which is the failure this whole file is about.
    expect(screen.getByText(`file-${sessionWorkspaceLimits.directoryEntries - 1}.rs`)).toBeTruthy();
    expect(screen.getByRole("status").textContent).toBeTruthy();
  });

  it("says nothing extra when the folder simply ends", async () => {
    listDirectory.mockResolvedValue({
      ...listing({ items: entries(sessionWorkspaceLimits.directoryEntries) }),
      truncated: false,
    });

    renderWithAppProviders(<FilesTab sessionId="session-1" />);

    await waitFor(() => expect(screen.getByText("file-0.rs")).toBeTruthy());
    // A full page is not the same fact as a cut one. Warning on both would train a reader to read
    // past the warning, at which point it stops working for the case that needs it.
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("reports a nested directory being cut, not only the root", async () => {
    listDirectory.mockImplementation((_session: string, path: string) =>
      Promise.resolve(
        path === ""
          ? listing({ items: [{ kind: "directory", name: "src", path: "src", size: null }] })
          : listing({ items: entries(3, "nested"), path: "src", truncated: true }),
      ),
    );

    renderWithAppProviders(<FilesTab sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("src")).toBeTruthy());
    fireEvent.click(screen.getByText("src"));

    // The tree is one surface made of one listing per open folder. A notice derived from the root
    // alone would go quiet for exactly the folders a reader opened because they were interested.
    await waitFor(() => expect(screen.getByRole("status")).toBeTruthy());
  });
});

describe("a path search at its result bound", () => {
  function openQuickOpen() {
    return renderWithAppProviders(
      <QuickOpenDialog isOpen onClose={vi.fn()} onSelect={vi.fn()} sessionId="session-1" />,
    );
  }

  it("offers the next page when the search minted a cursor", async () => {
    const matches = Array.from({ length: 50 }, (_, index) => pathMatch(`src/file-${index}.rs`));
    searchPaths.mockResolvedValue({
      coverage: { state: "complete" },
      matches,
      nextCursor: "cursor-2",
    } satisfies WorkspacePathSearchResult);

    openQuickOpen();

    await waitFor(() => expect(screen.getByText("src/file-49.rs")).toBeTruthy());
    expect(screen.getByRole("button", { name: "Load more" })).toBeTruthy();
  });

  it("offers no next page for a full result set that happens to end there", async () => {
    const matches = Array.from({ length: 50 }, (_, index) => pathMatch(`src/file-${index}.rs`));
    searchPaths.mockResolvedValue({ coverage: { state: "complete" }, matches });

    openQuickOpen();

    await waitFor(() => expect(screen.getByText("src/file-49.rs")).toBeTruthy());
    // A page that is full and a page that has a successor are different claims, and only the search
    // knows which one this is. Inferring "more" from the count would offer a page that is not there.
    expect(screen.queryByRole("button", { name: "Load more" })).toBeNull();
  });

  it("keeps a cursor and a coverage gap as two separate sentences", async () => {
    searchPaths.mockResolvedValue({
      coverage: { state: "partial", reasonCode: "scan_limit" },
      matches: [pathMatch("src/main.rs")],
      nextCursor: "cursor-2",
    });

    openQuickOpen();

    await waitFor(() => expect(screen.getByText("src/main.rs")).toBeTruthy());
    // Paging resolves the cursor and can never resolve the gap. A reader who clicks Load more until
    // it disappears has reached the end of what was searched, not the end of the workspace.
    expect(screen.getByRole("button", { name: "Load more" })).toBeTruthy();
    expect(screen.getByText("Part of this workspace was not searched.")).toBeTruthy();
  });
});

describe("a content search at its match bound", () => {
  it("renders every match it was handed and says the walk was cut", async () => {
    const matches = Array.from({ length: 200 }, (_, index) => contentMatch(index));
    searchContent.mockResolvedValue({
      generation: 1,
      coverage: { state: "partial", reasonCode: "result_budget_exhausted" },
      matches,
    } satisfies WorkspaceContentSearchResult);

    renderWithAppProviders(
      <ContentSearchPanel isOpen onClose={vi.fn()} onSelect={vi.fn()} sessionId="session-1" />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "value" } });

    await waitFor(() => expect(screen.getAllByRole("option")).toHaveLength(200), { timeout: 4000 });
    // The reason as well as the state. "Part of this workspace was not searched" tells a reader
    // there is more; only the reason tells them whether narrowing the query would find it.
    expect(
      screen.getByText(
        "Part of this workspace was not searched. Stopped at the maximum number of results.",
      ),
    ).toBeTruthy();
  });

  it("distinguishes a search that was cut from one that found nothing", async () => {
    searchContent.mockResolvedValue({ coverage: { state: "complete" }, matches: [] });

    renderWithAppProviders(
      <ContentSearchPanel isOpen onClose={vi.fn()} onSelect={vi.fn()} sessionId="session-1" />,
    );
    fireEvent.change(screen.getByRole("combobox"), { target: { value: "value" } });

    await waitFor(() => expect(screen.getByText("No matching lines.")).toBeTruthy(), {
      timeout: 4000,
    });
    // "Nothing matched" is a conclusion a reader can act on. It is only available when the whole
    // workspace was examined, so it must never appear beside a coverage gap.
    expect(screen.queryByText("Part of this workspace was not searched.")).toBeNull();
  });
});

describe("a file at the preview bound", () => {
  function file(overrides: Partial<FileContent>): FileContent {
    return {
      content: null,
      name: "big.log",
      path: "big.log",
      size: sessionWorkspaceLimits.fileBytes + 1,
      status: "oversized",
      ...overrides,
    };
  }

  async function selectTheFile(content: FileContent) {
    listDirectory.mockResolvedValue(
      listing({
        items: [{ kind: "file", name: content.name, path: content.path, size: content.size }],
      }),
    );
    readFile.mockResolvedValue(content);

    renderWithAppProviders(<FilesTab sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText(content.name)).toBeTruthy());
    fireEvent.click(screen.getByText(content.name));
  }

  it("names the bound rather than failing to read", async () => {
    await selectTheFile(file({}));

    await waitFor(() =>
      expect(screen.getByText("This file exceeds the 1 MiB preview limit.")).toBeTruthy(),
    );
    // No find box, no line numbers: offering them over content that was never decoded would be an
    // interface for searching nothing.
    expect(screen.queryByRole("textbox", { name: "Find in file" })).toBeNull();
  });

  it("keeps oversized and binary as different answers", async () => {
    await selectTheFile(file({ name: "logo.png", path: "logo.png", size: 900, status: "binary" }));

    await waitFor(() =>
      expect(screen.getByText("Binary files cannot be previewed as text.")).toBeTruthy(),
    );
    // Both are "not previewable" and the remedies differ — one is opened elsewhere, the other is
    // read in pieces — so one shared message would answer neither.
    expect(screen.queryByText("This file exceeds the 1 MiB preview limit.")).toBeNull();
  });

  it("renders a long file that is inside the bound", async () => {
    const lineCount = 4000;
    const content = Array.from({ length: lineCount }, (_, index) => `let v${index} = ${index};`)
      .join("\n");

    await selectTheFile(
      file({ content, name: "long.rs", path: "long.rs", size: content.length, status: "text" }),
    );

    await waitFor(() => expect(screen.getByText("4,000 lines")).toBeTruthy(), { timeout: 20_000 });
    // Every line, not a prefix. The preview has no virtualization, so "it renders" is a claim worth
    // holding: the day it becomes untrue, this is where it shows up rather than in a reader's
    // session.
    //
    // Read off the row rather than matched as text: highlighting splits a line across spans by
    // design, so a text query for the whole line finds nothing even when the line is right there.
    const lastRow = screen.getByTestId(`preview-line-${lineCount}`);
    expect(lastRow.textContent).toContain(`let v${lineCount - 1} = ${lineCount - 1};`);
  }, 30_000);
});
