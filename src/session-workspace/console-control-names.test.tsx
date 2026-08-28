/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import { DiffView } from "./diff-view";
import { FilesToolbar } from "./files-toolbar";
import { LogEntryArticle } from "./log-entry-article";
import { LogsToolbar } from "./logs-toolbar";
import { ReviewCenter } from "./review-center";
import { ReviewProgress } from "./review-progress";
import type { CodeReview, ReviewDiffFile } from "../types/code-review";
import type { GitDiffFile, GitDiffLine } from "../types/session-workspace";

/**
 * Every control says what it does, and no control says it only in colour.
 *
 * These two failures are the same failure seen twice. An icon with no name is a control that
 * exists for everyone who can see it and for nobody else; a red border with no words is a warning
 * that exists for everyone who can distinguish red and for nobody else. Both look complete in a
 * screenshot, which is how both survive review.
 *
 * The names are checked by rendering rather than by reading the source. Testing Library computes
 * the accessible name the way a browser does — from the label, the title, the text, and the
 * `sr-only` span, in that order — and a source scan would have to reimplement that and would get
 * it wrong in exactly the cases that matter.
 */

beforeAll(async () => {
  await activateAppLanguage("en");
});

// Two cases stub the review service. Without this the stub survives into the colour cases, which
// render no review at all and would pass while quietly holding a mock nobody asked for.
afterEach(() => {
  vi.restoreAllMocks();
});

function everyButtonIsNamed() {
  return screen.getAllByRole("button").filter((button) => {
    // The computed name, as an assistive technology would receive it.
    const name = (
      button.getAttribute("aria-label") ??
      button.getAttribute("title") ??
      button.textContent ??
      ""
    ).trim();
    return name.length === 0;
  });
}

describe("icon-only controls", () => {
  it("names every control in the Logs toolbar", () => {
    renderWithAppProviders(
      <LogsToolbar
        following
        levels={["error"]}
        onExport={vi.fn()}
        onJumpToLatest={vi.fn()}
        onLocate={vi.fn()}
        onSearchDraftChange={vi.fn()}
        onSubmitSearch={vi.fn()}
        onTimestampDraftChange={vi.fn()}
        onToggleLevel={vi.fn()}
        onTogglePause={vi.fn()}
        paused={false}
        pendingCount={0}
        searchDraft=""
        seeking={false}
        timestampDraft=""
      />,
    );

    // Pause, jump-to-latest, and locate are glyphs. A glyph with no name is a control that exists
    // for everyone who can see it and for nobody else.
    expect(everyButtonIsNamed()).toEqual([]);
  });

  it("names every control in the Files toolbar", () => {
    renderWithAppProviders(
      <FilesToolbar
        isRemote={false}
        onContentSearch={vi.fn()}
        onQuickOpen={vi.fn()}
        onShellOpened={vi.fn()}
        selectedPath="src/main.rs"
        sessionId="session-1"
      />,
    );

    expect(everyButtonIsNamed()).toEqual([]);
  });

  it("names the Review progress toggle", () => {
    const review = {
      comments: [],
      createdAt: "2026-08-27T00:00:00Z",
      decision: "pending",
      files: [{ changeType: "modified", path: "src/main.rs", viewed: false }],
      findings: [],
      fingerprint: "snapshot-a",
      hunkDecisions: [],
      id: "review-1",
      sessionId: "session-1",
      status: "active",
      summary: { changedFiles: 1, unresolvedComments: 0, unresolvedFindings: 0, viewedFiles: 0 },
      updatedAt: "2026-08-27T00:00:00Z",
      workspaceId: "workspace-1",
    } satisfies CodeReview;

    renderWithAppProviders(
      <ReviewProgress
        onToggleViewed={vi.fn()}
        review={review}
        selectedPath="src/main.rs"
        viewed={false}
      />,
    );

    expect(everyButtonIsNamed()).toEqual([]);
  });
});

/**
 * The colour half of the same rule, checked by rendering rather than by reading.
 *
 * A source scan cannot decide this one. The line that gets it right — `entry.level === "error" &&
 * "text-destructive"` on the span that renders the word `error` — and the line that got it wrong
 * are written identically; what separates them is whether the element the colour lands on has any
 * text of its own, which is a fact about the rendered tree. Scanning source here produced one true
 * finding and one false accusation against the module that was already correct.
 */

function line(kind: GitDiffLine["kind"], content: string, at: number): GitDiffLine {
  return {
    content,
    kind,
    newLineNumber: kind === "deletion" ? null : at,
    oldLineNumber: kind === "addition" ? null : at,
  };
}

const REVIEW = {
  comments: [],
  createdAt: "2026-08-27T00:00:00Z",
  decision: "pending",
  files: [{ changeType: "modified", path: "src/main.rs", viewed: false }],
  findings: [],
  fingerprint: "snapshot-a",
  hunkDecisions: [],
  id: "review-1",
  sessionId: "session-1",
  status: "active",
  summary: { changedFiles: 1, unresolvedComments: 0, unresolvedFindings: 0, viewedFiles: 0 },
  updatedAt: "2026-08-27T00:00:00Z",
  workspaceId: "workspace-1",
} satisfies CodeReview;

const REVIEW_DIFF = {
  acceptedBytes: 0,
  binary: false,
  changeType: "modified",
  hunks: [
    {
      contextFingerprints: [],
      fingerprint: "hunk-1",
      header: "@@ -1,1 +1,1 @@",
      lines: [
        { content: "let value = 1;", kind: "deletion" },
        { content: "let value = 2;", kind: "addition" },
      ],
      newLines: 1,
      newStart: 1,
      oldLines: 1,
      oldStart: 1,
    },
  ],
  oversized: false,
  path: "src/main.rs",
  truncated: false,
} satisfies ReviewDiffFile;

const CHANGED_FILE = {
  binary: false,
  hunks: [
    {
      header: "@@ -1,2 +1,2 @@",
      lines: [line("deletion", "let value = 1;", 1), line("addition", "let value = 2;", 1)],
      newLines: 1,
      newStart: 1,
      oldLines: 1,
      oldStart: 1,
    },
  ],
  newPath: "src/main.rs",
  oldPath: "src/main.rs",
  oversized: false,
} satisfies GitDiffFile;

/** The rendered text of every row, with the leading sign kept. */
function rowsOf(container: HTMLElement): string[] {
  return [...container.querySelectorAll(".whitespace-pre")].map((row) => row.textContent ?? "");
}

describe("status is never only a colour", () => {
  it("marks an added line apart from a deleted one in the unified diff", () => {
    const { container } = renderWithAppProviders(<DiffView file={CHANGED_FILE} mode="unified" />);

    expect(rowsOf(container)).toEqual(["-let value = 1;", "+let value = 2;"]);
  });

  it("marks them apart in the split diff too", () => {
    const { container } = renderWithAppProviders(<DiffView file={CHANGED_FILE} mode="split" />);

    // The split view is where this failed. Its two columns rendered bare content, so a deletion on
    // the left and an addition on the right were the same string in two differently tinted boxes.
    const rows = rowsOf(container);
    expect(rows).toContain("-let value = 1;");
    expect(rows).toContain("+let value = 2;");
  });

  it("would notice if the signs came back off", () => {
    // Anti-vacuity. Both assertions above pass on any string that merely contains the content, so
    // this pins the thing that actually distinguishes the two rows: the first character.
    const { container } = renderWithAppProviders(<DiffView file={CHANGED_FILE} mode="split" />);

    const signs = rowsOf(container)
      .filter((row) => row.trim().length > 0)
      .map((row) => row[0]);
    expect(new Set(signs)).toEqual(new Set(["-", "+"]));
  });

  it.each(["unified", "split"] as const)("marks them apart in the review center's %s view", async (
    view,
  ) => {
    // The review center draws its own diff rather than reusing the one above, and it was the worse
    // case of the two: each line is a button, so the accessible name a reviewer hears was the line
    // number and the text with nothing to say which side of the change it was on.
    vi.spyOn(agentService, "openCodeReview").mockResolvedValue(REVIEW);
    vi.spyOn(agentService, "loadCodeReviewFile").mockResolvedValue(REVIEW_DIFF);

    renderWithAppProviders(<ReviewCenter sessionId="session-1" />);
    await waitFor(() => expect(screen.getByText("+let value = 2;")).toBeTruthy());
    if (view === "split") fireEvent.click(screen.getByRole("button", { name: "Split" }));

    // Read off the buttons, which is where the name a screen reader announces comes from.
    const named = screen
      .getAllByRole("button")
      .map((button) => button.textContent ?? "")
      .filter((text) => text.includes("let value ="));
    expect(named.some((text) => text.includes("-let value = 1;"))).toBe(true);
    expect(named.some((text) => text.includes("+let value = 2;"))).toBe(true);
  });

  it("says the level of a log entry in words, not only in red", () => {
    renderWithAppProviders(
      <LogEntryArticle
        entry={{
          category: "runtime",
          context: {},
          id: "log-1",
          level: "error",
          message: "the process exited",
          timestamp: "2026-08-27T00:00:00Z",
        }}
        focused={false}
        language="en"
        onFocused={vi.fn()}
        position={1}
        total={1}
      />,
    );

    // The colour lands on the span holding this word, which is the shape that makes a colour
    // redundant rather than load-bearing.
    expect(screen.getByText("error")).toBeTruthy();
  });
});
