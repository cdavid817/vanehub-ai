/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { FileContent, SessionDocument } from "../types/session-workspace";
import { DocumentsTab } from "./documents-tab";

const CONTEXT = { availability: "available" as const, rootName: "project", reason: null };

const README = "# Title\n\nprose\n\n## Section\n\nmore\n";

function document(path: string): SessionDocument {
  return { name: path.split("/").pop() ?? path, path, kind: "markdown" };
}

function content(path: string, text: string): FileContent {
  return {
    path,
    name: path.split("/").pop() ?? path,
    status: "text",
    size: text.length,
    content: text,
    encoding: "utf-8",
    newline: "lf",
  };
}

let listDocuments: ReturnType<typeof vi.spyOn>;
let readFile: ReturnType<typeof vi.spyOn>;

beforeEach(() => {
  vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockResolvedValue({
    provider: "local",
    listFiles: { available: true },
    readTextFiles: { available: true },
    searchFiles: { available: true },
    gitStatus: { available: true },
    gitDiff: { available: true },
    watchMode: "polling",
  });
  listDocuments = vi.spyOn(agentService, "listSessionDocuments").mockResolvedValue({
    context: CONTEXT,
    coverage: { state: "complete" },
    items: [document("README.md"), document("docs/design.md")],
    truncated: false,
    nextCursor: null,
  });
  readFile = vi.spyOn(agentService, "readSessionFile").mockImplementation(async (_id, path) =>
    content(path, README),
  );
});

afterEach(() => {
  vi.restoreAllMocks();
});

function render() {
  return renderWithAppProviders(<DocumentsTab sessionId="session-1" />);
}

describe("DocumentsTab", () => {
  it("derives an outline from the loaded document", async () => {
    render();

    // Twice: once as an outline entry and once as the rendered heading. The ambiguity is the
    // feature — an outline that named a heading the document does not contain would be worse.
    await waitFor(() => expect(screen.getAllByText("Section")).toHaveLength(2));
    // From the content that was already read, not a second request: the outline cannot disagree
    // with the document it describes.
    expect(readFile).toHaveBeenCalledTimes(1);
  });

  it("filters the list by path", async () => {
    render();
    await waitFor(() => expect(screen.getByText("docs/design.md")).toBeTruthy());

    fireEvent.change(screen.getByRole("textbox", { name: /Filter documents|筛选文档/ }), {
      target: { value: "design" },
    });

    expect(screen.queryByText("README.md")).toBeNull();
    expect(screen.getByText("docs/design.md")).toBeTruthy();
  });

  it("says so rather than showing an empty list when nothing matches", async () => {
    render();
    await waitFor(() => expect(screen.getByText("docs/design.md")).toBeTruthy());

    fireEvent.change(screen.getByRole("textbox", { name: /Filter documents|筛选文档/ }), {
      target: { value: "nothing-like-this" },
    });

    expect(screen.getByText(/No documents match|没有匹配的文档/)).toBeTruthy();
  });

  it("remembers the documents this session opened", async () => {
    render();
    await waitFor(() => expect(screen.getByText("docs/design.md")).toBeTruthy());

    fireEvent.click(screen.getByText("docs/design.md"));

    // A Recent heading appears only once something has been opened: a permanently empty section is
    // one a reader learns to skip.
    await waitFor(() => expect(screen.getByText(/^Recent$|^最近$/)).toBeTruthy());
    expect(screen.getAllByText("docs/design.md").length).toBeGreaterThan(1);
  });

  it("switches between rendered and source views", async () => {
    render();
    await waitFor(() => expect(screen.getAllByText("Section")).toHaveLength(2));

    fireEvent.click(screen.getByRole("button", { name: /^Source$|^源码$/ }));

    // Source is the file preview, so it brings line numbers with it — the same ones any other file
    // gets, rather than a lesser renderer built for this one panel.
    await waitFor(() => expect(screen.getByTestId("preview-line-1")).toBeTruthy());
  });

  it("keeps the list usable when a document cannot be read", async () => {
    render();
    await waitFor(() => expect(screen.getByText("docs/design.md")).toBeTruthy());

    readFile.mockRejectedValue(new Error("gone"));
    fireEvent.click(screen.getByText("docs/design.md"));

    // A document that could not be read must not take away the ability to pick a different one.
    await waitFor(() => expect(screen.getByText("README.md")).toBeTruthy());
    expect(listDocuments).toHaveBeenCalled();
  });

  it("shows a hostile document as text rather than running it", async () => {
    const hostile = [
      "# Title",
      "",
      "<script>window.__owned = true</script>",
      "",
      '<img src=x onerror="window.__owned = true">',
    ].join("\n");
    readFile.mockResolvedValue(content("README.md", hostile));

    const { container } = render();

    // Asserted through the panel rather than only through the renderer: the question is what a
    // reader sees in the Documents tab, and a renderer that is safe in isolation proves nothing
    // about a panel that stopped using it.
    await waitFor(() => expect(screen.getAllByText("Title").length).toBeGreaterThan(0));
    expect(container.querySelector("script")).toBeNull();
    expect(container.querySelector("img[onerror]")).toBeNull();
    expect((window as unknown as { __owned?: boolean }).__owned).toBeUndefined();
  });

  it("offers Changes only for a document Git reports as changed", async () => {
    const onOpenChanges = vi.fn();
    vi.spyOn(agentService, "getSessionGitStatus").mockResolvedValue({
      context: CONTEXT,
      isGit: true,
      branch: "main",
      items: [
        { path: "README.md", previousPath: null, index: "unmodified", worktree: "modified" },
      ],
      truncated: false,
      nextCursor: null,
    });
    renderWithAppProviders(
      <DocumentsTab onOpenChanges={onOpenChanges} sessionId="session-1" />,
    );

    const action = await screen.findByRole("button", { name: /Open in Changes|在 Changes 中打开/ });
    fireEvent.click(action);

    expect(onOpenChanges).toHaveBeenCalledWith("README.md");
  });

  it("withholds Changes for a document Git does not list", async () => {
    vi.spyOn(agentService, "getSessionGitStatus").mockResolvedValue({
      context: CONTEXT,
      isGit: true,
      branch: "main",
      items: [],
      truncated: false,
      nextCursor: null,
    });
    renderWithAppProviders(<DocumentsTab onOpenChanges={vi.fn()} sessionId="session-1" />);
    await waitFor(() => expect(screen.getAllByText("Section").length).toBeGreaterThan(0));

    // An action that always appeared would open Changes on a file it does not list, which reads as
    // Changes being broken rather than as the document being unmodified.
    expect(
      screen.queryByRole("button", { name: /Open in Changes|在 Changes 中打开/ }),
    ).toBeNull();
  });

  it("reports a provider that cannot read rather than an empty workspace", async () => {
    vi.spyOn(agentService, "getWorkspaceInspectionCapabilities").mockResolvedValue({
      provider: "ssh",
      targetLabel: "build-host",
      listFiles: { available: true },
      readTextFiles: { available: false, reasonCode: "remote_helper_unavailable" },
      searchFiles: { available: true },
      gitStatus: { available: true },
      gitDiff: { available: true },
      watchMode: "polling",
    });
    render();

    // An empty list is indistinguishable from a workspace that genuinely has no documents, which
    // is the wrong conclusion to hand somebody about a host that simply could not answer.
    await waitFor(() => expect(screen.getByText(/build-host/)).toBeTruthy());
  });
});
