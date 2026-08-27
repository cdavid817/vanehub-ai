/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview, ReviewDiffFile } from "../types/code-review";
import { ReviewCenter } from "./review-center";

/**
 * What the Review header says is left, and the two records that change it.
 *
 * Every one of these is a claim the panel cannot make for itself. Whether a file counts as read
 * depends on a mark in a store the review does not carry and on whether that mark still matches
 * the file; whether a hunk is accepted depends on a decision keyed by a fingerprint. Both come
 * back from the backend on every read, and both are re-read after a write rather than assumed —
 * so what these cases check is that the panel renders the answer instead of guessing it.
 */

const FINGERPRINT = "snapshot-a";

function review(overrides: Partial<CodeReview> = {}): CodeReview {
  return {
    comments: [],
    createdAt: "2026-08-27T00:00:00Z",
    decision: "pending",
    files: [
      { changeType: "modified", path: "src/main.rs", viewed: false },
      { changeType: "modified", path: "src/other.rs", viewed: false },
    ],
    findings: [],
    fingerprint: FINGERPRINT,
    hunkDecisions: [],
    id: "review-1",
    sessionId: "session-1",
    status: "active",
    summary: { changedFiles: 2, unresolvedComments: 0, unresolvedFindings: 0, viewedFiles: 0 },
    updatedAt: "2026-08-27T00:00:00Z",
    workspaceId: "workspace-1",
    ...overrides,
  };
}

function diff(): ReviewDiffFile {
  return {
    acceptedBytes: 0,
    binary: false,
    changeType: "modified",
    hunks: [
      {
        contextFingerprints: [],
        fingerprint: "hunk-1",
        header: "@@ -1,1 +1,1 @@",
        lines: [{ content: "fn main() { work(); }", kind: "addition" }],
        newLines: 1,
        newStart: 1,
        oldLines: 1,
        oldStart: 1,
      },
    ],
    oversized: false,
    path: "src/main.rs",
    truncated: false,
  };
}

let openReview: ReturnType<typeof vi.spyOn>;
let getReview: ReturnType<typeof vi.spyOn>;
let setViewed: ReturnType<typeof vi.spyOn>;
let setHunkDecision: ReturnType<typeof vi.spyOn>;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  openReview = vi.spyOn(agentService, "openCodeReview").mockResolvedValue(review());
  getReview = vi.spyOn(agentService, "getCodeReview").mockResolvedValue(review());
  vi.spyOn(agentService, "loadCodeReviewFile").mockResolvedValue(diff());
  setViewed = vi.spyOn(agentService, "setCodeReviewFileViewed");
  setHunkDecision = vi.spyOn(agentService, "setCodeReviewHunkDecision");
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function open() {
  renderWithAppProviders(<ReviewCenter sessionId="session-1" />);
  await waitFor(() => expect(screen.getByText("fn main() { work(); }")).toBeTruthy());
}

describe("the Review header and the marks behind it", () => {
  it("shows how much of the review has been read", async () => {
    openReview.mockResolvedValue(
      review({
        files: [
          { changeType: "modified", path: "src/main.rs", viewed: true },
          { changeType: "modified", path: "src/other.rs", viewed: false },
        ],
        summary: { changedFiles: 2, unresolvedComments: 3, unresolvedFindings: 1, viewedFiles: 1 },
      }),
    );
    await open();

    expect(screen.getByRole("status").textContent).toContain("1 of 2 files read");
    expect(screen.getByText("3 open comments")).toBeTruthy();
    expect(screen.getByText("1 open finding")).toBeTruthy();
  });

  it("says nothing about counts that are zero", async () => {
    await open();

    // A review with nothing outstanding should read as having nothing outstanding. A row of zeroes
    // is three numbers a reader has to check before learning that.
    expect(screen.queryByText(/open comment/)).toBeNull();
    expect(screen.queryByText(/open finding/)).toBeNull();
  });

  it("marks the selected file read against the snapshot on screen", async () => {
    setViewed.mockResolvedValue({
      fileWitness: "witness-1",
      relativePath: "src/main.rs",
      reviewId: "review-1",
      simulated: false,
      viewed: true,
    });
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Mark as read" }));

    await waitFor(() =>
      expect(setViewed).toHaveBeenCalledWith({
        expectedSnapshotFingerprint: FINGERPRINT,
        relativePath: "src/main.rs",
        reviewId: "review-1",
        viewed: true,
      }),
    );
    // Re-read rather than patched locally: the header's counts are derived on the other side, and
    // a local edit would leave them agreeing with the click rather than with what was recorded.
    await waitFor(() => expect(getReview).toHaveBeenCalledWith("review-1"));
  });

  it("offers the opposite action for a file that is already read", async () => {
    openReview.mockResolvedValue(
      review({
        files: [
          { changeType: "modified", path: "src/main.rs", viewed: true },
          { changeType: "modified", path: "src/other.rs", viewed: false },
        ],
        summary: { changedFiles: 2, unresolvedComments: 0, unresolvedFindings: 0, viewedFiles: 1 },
      }),
    );
    await open();

    const toggle = screen.getByRole("button", { name: "Mark as unread" });
    expect(toggle.getAttribute("aria-pressed")).toBe("true");
  });

  it("renders the decision a hunk already holds rather than assuming it has none", async () => {
    openReview.mockResolvedValue(
      review({
        hunkDecisions: [
          { decision: "changes-requested", hunkFingerprint: "hunk-1", relativePath: "src/main.rs" },
        ],
      }),
    );
    await open();

    // Matched by fingerprint, so it survives an edit to a different hunk. A panel that only knew
    // what it had set this session would show every decision as undecided after a reload.
    expect(screen.getByText("Changes requested")).toBeTruthy();
    expect(
      screen.getByRole("button", { name: "Request changes on this hunk" }).getAttribute("aria-pressed"),
    ).toBe("true");
  });

  it("keeps accepting a hunk separate from accepting the review", async () => {
    setHunkDecision.mockResolvedValue({
      decision: "accepted",
      hunkFingerprint: "hunk-1",
      relativePath: "src/main.rs",
      reviewId: "review-1",
      simulated: false,
    });
    const setDecision = vi.spyOn(agentService, "setCodeReviewDecision");
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Accept hunk" }));

    await waitFor(() =>
      expect(setHunkDecision).toHaveBeenCalledWith({
        decision: "accepted",
        expectedSnapshotFingerprint: FINGERPRINT,
        hunkFingerprint: "hunk-1",
        relativePath: "src/main.rs",
        reviewId: "review-1",
      }),
    );
    // The defect this whole group exists to remove: accepting one block of a diff used to mark the
    // entire review accepted.
    expect(setDecision).not.toHaveBeenCalled();
  });

  it("clears a decision by pressing the control that already holds it", async () => {
    openReview.mockResolvedValue(
      review({
        hunkDecisions: [
          { decision: "accepted", hunkFingerprint: "hunk-1", relativePath: "src/main.rs" },
        ],
      }),
    );
    setHunkDecision.mockResolvedValue({
      decision: "pending",
      hunkFingerprint: "hunk-1",
      relativePath: "src/main.rs",
      reviewId: "review-1",
      simulated: false,
    });
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Accept hunk" }));

    // Undoing is the same control, not a third one. A reviewer who accepted by mistake has nowhere
    // else to go.
    await waitFor(() =>
      expect(setHunkDecision).toHaveBeenCalledWith(
        expect.objectContaining({ decision: "pending" }),
      ),
    );
  });

  it("tells the reviewer to reload when the review moved under them", async () => {
    setViewed.mockRejectedValue(new Error("stale_witness"));
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Mark as read" }));

    await waitFor(() =>
      expect(
        screen.getByText("This review changed while you were reading it. Reload and try again."),
      ).toBeTruthy(),
    );
  });
});
