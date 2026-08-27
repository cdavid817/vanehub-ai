/** @vitest-environment jsdom */
import { fireEvent, screen, waitFor } from "@testing-library/react";
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import { agentService } from "../services/runtime-agent-client";
import type { CodeReview, ReviewDiffFile } from "../types/code-review";
import { ReviewCenter } from "./review-center";

/**
 * Two copy actions that are not the same action.
 *
 * Copying the displayed lines gives a reviewer what is on their screen: no file header, no hunk
 * header, and truncated exactly where the panel truncated it. It is for quoting into a message.
 * Copying the standard patch gives them something `git apply` accepts. The two look almost alike
 * on a button row and are wrong for each other's purpose, so what each writes to the clipboard is
 * worth holding rather than assuming.
 */

const FINGERPRINT = "snapshot-a";

function review(): CodeReview {
  return {
    comments: [],
    createdAt: "2026-08-27T00:00:00Z",
    decision: "pending",
    files: [{ path: "src/main.rs", changeType: "modified", viewed: false }],
    findings: [],
    fingerprint: FINGERPRINT,
    hunkDecisions: [],
    id: "review-1",
    sessionId: "session-1",
    status: "active",
    summary: { changedFiles: 1, unresolvedComments: 0, unresolvedFindings: 0, viewedFiles: 0 },
    updatedAt: "2026-08-27T00:00:00Z",
    workspaceId: "workspace-1",
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
        lines: [
          { content: "fn main() {}", kind: "deletion" },
          { content: "fn main() { work(); }", kind: "addition" },
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
  };
}

let written: string[];
let getPatch: ReturnType<typeof vi.spyOn>;

beforeAll(async () => {
  await activateAppLanguage("en");
});

beforeEach(() => {
  written = [];
  vi.spyOn(agentService, "openCodeReview").mockResolvedValue(review());
  vi.spyOn(agentService, "loadCodeReviewFile").mockResolvedValue(diff());
  getPatch = vi.spyOn(agentService, "getCodeReviewPatch");
});

afterEach(() => {
  vi.restoreAllMocks();
});

async function open() {
  renderWithAppProviders(<ReviewCenter sessionId="session-1" />);
  // After the render, not before it. `renderWithAppProviders` calls `userEvent.setup()`, which
  // installs its own clipboard stub on `navigator` — a mock placed in `beforeEach` is silently
  // replaced by it, and the only symptom is that nothing was ever written.
  Object.defineProperty(navigator, "clipboard", {
    configurable: true,
    value: {
      writeText: vi.fn(async (text: string) => {
        written.push(text);
      }),
    },
  });
  // Waits for the diff, not just for the review. The buttons render as soon as the review loads,
  // so a wait on them lets a click land while `diff` is still null — which copies an empty string
  // and reads as the action being broken. The explicit timeout is for the same reason the wait
  // exists: two queries deep, and Testing Library's one-second default is a number about its own
  // convenience rather than about this load.
  await waitFor(() => expect(screen.getByText("+fn main() { work(); }")).toBeTruthy(), {
    timeout: 5_000,
  });
}

describe("the Review Center's two copy actions", () => {
  it("copies the displayed lines without asking the backend for anything", async () => {
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Copy displayed lines" }));

    await waitFor(() => expect(written).toHaveLength(1));
    // What is on screen and nothing else. Headers here would be a quote nobody can paste into a
    // sentence — but the signs stay, because they are on screen and because a pasted run of
    // unmarked lines reads as if every one of them were added.
    expect(written[0]).toBe("-fn main() {}\n+fn main() { work(); }");
    expect(getPatch).not.toHaveBeenCalled();
  });

  it("asks the backend for the standard patch against the snapshot on screen", async () => {
    getPatch.mockResolvedValue({
      fingerprint: "patch-1",
      hunks: 1,
      patch: "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n",
      path: "src/main.rs",
      snapshot: FINGERPRINT,
    });
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Copy standard patch" }));

    await waitFor(() => expect(written).toHaveLength(1));
    // Rendered by the side that can see the repository, witnessed to the diff the reviewer is
    // reading. Assembling it here from the hunks on screen would produce something that reads like
    // a patch and that Git refuses.
    expect(getPatch).toHaveBeenCalledWith({
      expectedSnapshot: FINGERPRINT,
      path: "src/main.rs",
      sessionId: "session-1",
    });
    expect(written[0]).toContain("diff --git");
    await waitFor(() =>
      expect(screen.getByText("Standard patch copied.")).toBeTruthy(),
    );
  });

  it.each([
    ["stale_witness", "This diff changed while you were reading it. Reload and copy again."],
    ["patch_unavailable_binary", "A binary file has no patch to copy."],
    ["patch_too_large", "This change is too large to copy as a patch."],
    ["something else entirely", "The standard patch could not be produced."],
  ])("says what happened when the patch is refused with %s", async (code, message) => {
    getPatch.mockRejectedValue(new Error(code));
    await open();

    fireEvent.click(screen.getByRole("button", { name: "Copy standard patch" }));

    // Four refusals, four sentences. One shared "could not copy" would be true of all of them and
    // would leave a reviewer with nothing to do about any of them.
    await waitFor(() => expect(screen.getByText(message)).toBeTruthy());
    expect(written).toHaveLength(0);
  });
});
