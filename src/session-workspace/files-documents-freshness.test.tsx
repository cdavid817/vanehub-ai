/** @vitest-environment jsdom */
import { act, fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { DirectoryEntry, DirectoryListing, FileContent } from "../types/session-workspace";
import type { WorkspaceInvalidationNotice } from "../types/session-workspace-inspection";
import { DocumentsTab } from "./documents-tab";
import { FilesTab } from "./files-tab";
import { useWorkspaceInvalidation } from "./use-workspace-invalidation";

/**
 * What the panels do when the workspace changes underneath the reader.
 *
 * Two failure modes, opposite and equally silent. Refresh too little and a panel keeps rendering a
 * file that was deleted twenty minutes ago, with the same confidence it renders a current one.
 * Refresh too much and every agent write collapses the tree, drops the reader's place, and re-reads
 * folders nobody has open.
 *
 * The unit under test is the tab composed with the subscription, which is how the tab host runs it.
 * Testing the hook alone would prove notices reach the query cache and not that anything on screen
 * changed because of it; testing the tab alone would prove the panel renders whatever it is given.
 */

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

function listing(items: DirectoryEntry[], path = ""): DirectoryListing {
  return { context: CONTEXT, coverage: { state: "complete" }, items, nextCursor: null, path, truncated: false };
}

function directory(name: string): DirectoryEntry {
  return { kind: "directory", name, path: name, size: null };
}

function file(name: string, path = name): DirectoryEntry {
  return { kind: "file", name, path, size: 12 };
}

function textFile(path: string, content: string): FileContent {
  return {
    content,
    encoding: "utf-8",
    name: path.split("/").pop() ?? path,
    newline: "lf",
    path,
    size: content.length,
    status: "text",
  };
}

function notice(overrides: Partial<WorkspaceInvalidationNotice>): WorkspaceInvalidationNotice {
  return {
    occurredAt: "2026-08-27T00:00:00.000Z",
    scope: "path",
    sequence: 1,
    sessionId: "session-1",
    source: "watch",
    ...overrides,
  };
}

/** The tab as the tab host runs it: mounted alongside the one subscription that feeds every panel. */
function WatchedFiles({ sessionId }: { sessionId: string }) {
  useWorkspaceInvalidation(sessionId);
  return <FilesTab sessionId={sessionId} />;
}

let listDirectory: ReturnType<typeof vi.spyOn>;
let readFile: ReturnType<typeof vi.spyOn>;
let publish: (notice: WorkspaceInvalidationNotice) => void;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  const listeners = new Set<(notice: WorkspaceInvalidationNotice) => void>();
  publish = (value) => {
    for (const listener of [...listeners]) listener(value);
  };
  listDirectory = vi.spyOn(agentService, "listSessionDirectory");
  readFile = vi.spyOn(agentService, "readSessionFile");
  vi.spyOn(agentService, "subscribeWorkspaceInvalidation").mockImplementation(
    async (handler: (notice: WorkspaceInvalidationNotice) => void) => {
      listeners.add(handler);
      return () => listeners.delete(handler);
    },
  );
  vi.spyOn(agentService, "getFileEvidenceLinks").mockResolvedValue({
    commandIds: [],
    observations: 0,
    runIds: [],
    truncated: false,
  });
});

afterEach(() => {
  vi.restoreAllMocks();
});

describe("a change notice reaches what it implicates", () => {
  it("re-reads the directory a changed file lives in", async () => {
    listDirectory.mockImplementation((_session: string, path: string) =>
      Promise.resolve(
        path === ""
          ? listing([directory("src"), directory("docs")])
          : listing([file(`${path}/one.rs`.split("/").pop() ?? "", `${path}/one.rs`)], path),
      ),
    );

    renderWithAppProviders(<WatchedFiles sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("src")).toBeTruthy());
    fireEvent.click(screen.getByText("src"));
    fireEvent.click(screen.getByText("docs"));
    await waitFor(() => expect(listDirectory).toHaveBeenCalledTimes(3));

    listDirectory.mockClear();
    await act(async () => {
      publish(notice({ change: "modified", relativePath: "src/one.rs" }));
    });

    await waitFor(() => {
      const refreshed = listDirectory.mock.calls.map((call: unknown[]) => call[1]);
      expect(refreshed).toContain("src");
      // `docs` saw nothing happen. Re-reading it would be a read per open folder per agent write,
      // and on a remote workspace that is the difference between a tree and a network stall.
      expect(refreshed).not.toContain("docs");
    });
  });

  it("re-reads everything open when a notice admits observation was lost", async () => {
    listDirectory.mockImplementation((_session: string, path: string) =>
      Promise.resolve(path === "" ? listing([directory("src"), directory("docs")]) : listing([], path)),
    );

    renderWithAppProviders(<WatchedFiles sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("src")).toBeTruthy());
    fireEvent.click(screen.getByText("src"));
    fireEvent.click(screen.getByText("docs"));
    await waitFor(() => expect(listDirectory).toHaveBeenCalledTimes(3));

    listDirectory.mockClear();
    await act(async () => {
      publish(notice({ relativePath: undefined, scope: "workspace" }));
    });

    // The broad notice is the one case where breadth is the honest answer: nothing knows what
    // changed, so narrowing would be a guess presented as knowledge.
    await waitFor(() => {
      const refreshed = listDirectory.mock.calls.map((call: unknown[]) => call[1]);
      expect(refreshed).toContain("");
      expect(refreshed).toContain("src");
      expect(refreshed).toContain("docs");
    });
  });
});

describe("content that changed while it was being read", () => {
  it("keeps the old text on screen and says it is being re-read", async () => {
    listDirectory.mockResolvedValue(listing([file("main.rs")]));
    readFile.mockResolvedValue(textFile("main.rs", "fn main() {}"));

    renderWithAppProviders(<WatchedFiles sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("main.rs")).toBeTruthy());
    fireEvent.click(screen.getByText("main.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());

    // Never settles, so the panel stays in the state this test is about.
    readFile.mockImplementation(() => new Promise<FileContent>(() => {}));
    await act(async () => {
      publish(notice({ change: "modified", relativePath: "main.rs" }));
    });

    // Blanking here would take the file away from a reader at the exact moment an agent touched it,
    // which is the moment they most want to be looking at it. "Re-reading" rather than "loading":
    // what is below is probably still right, and the two sentences ask for different patience.
    await waitFor(() => expect(screen.getByText("Re-reading this file…")).toBeTruthy());
    expect(screen.getByTestId("preview-line-1").textContent).toContain("fn main() {}");
  });

  it("stops showing a file the refreshed listing no longer has", async () => {
    listDirectory.mockResolvedValue(listing([file("main.rs")]));
    readFile.mockResolvedValue(textFile("main.rs", "fn main() {}"));

    renderWithAppProviders(<WatchedFiles sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("main.rs")).toBeTruthy());
    fireEvent.click(screen.getByText("main.rs"));
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());

    listDirectory.mockResolvedValue(listing([]));
    await act(async () => {
      publish(notice({ change: "removed", relativePath: "main.rs" }));
    });

    // Driven by the refreshed listing rather than by the notice's own `removed`: the listing answers
    // for every cause, including a manual refresh and a deletion nobody published.
    await waitFor(() =>
      expect(screen.getByText("Select a text file to preview it.")).toBeTruthy(),
    );
  });
});

describe("a workspace on a host that cannot answer", () => {
  it("says why Documents is empty instead of showing an empty list", async () => {
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockResolvedValue({
      gitDiff: { available: false },
      gitStatus: { available: false },
      listFiles: { available: false, reasonCode: "remote_connection_unavailable" },
      provider: "ssh",
      readTextFiles: {
        available: false,
        reasonCode: "remote_connection_unavailable",
      },
      searchFiles: { available: false },
      targetLabel: "build-01",
      watchMode: "none",
    });
    vi.spyOn(agentService, "listSessionDocuments").mockResolvedValue({
      context: CONTEXT,
      coverage: { state: "complete" },
      items: [],
      nextCursor: null,
      truncated: false,
    });
    vi.spyOn(agentService, "getSessionGitStatus").mockResolvedValue({
      branch: null,
      context: CONTEXT,
      isGit: false,
      items: [],
      nextCursor: null,
      truncated: false,
    });

    renderWithAppProviders(<DocumentsTab sessionId="session-1" />);

    // An unreachable host and a workspace with no documents both produce an empty list, and only one
    // of them is something the reader can do anything about.
    await waitFor(() =>
      expect(screen.getByText("The remote host could not be reached.")).toBeTruthy(),
    );
    expect(screen.getByText("On build-01.")).toBeTruthy();
    expect(screen.getByText("The Shell for this session is still available.")).toBeTruthy();
  });

  it("leaves a reason on screen when the tree itself cannot be read", async () => {
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockRejectedValue(
      new Error("capabilities unavailable"),
    );
    listDirectory.mockRejectedValue(new Error("Connection closed by remote host"));

    renderWithAppProviders(<WatchedFiles sessionId="session-1" />);

    // An empty tree with no message reads as an empty workspace, which is the one conclusion a
    // reader must not reach from a host that simply stopped answering.
    await waitFor(() => {
      const text = document.body.textContent ?? "";
      expect(text.toLowerCase()).toContain("could not be completed");
    });
  });
});
