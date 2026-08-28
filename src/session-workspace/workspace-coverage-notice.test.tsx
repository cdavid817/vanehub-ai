/** @vitest-environment jsdom */
import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import "../i18n";
import {
  WorkspaceCoverageNotice,
  type WorkspaceCoverageReason,
} from "./workspace-coverage-notice";

const REASONS: WorkspaceCoverageReason[] = [
  "directory-page",
  "document-walk",
  "git-status-bound",
  "git-diff-bound",
];

describe("WorkspaceCoverageNotice", () => {
  it("says something different for each reason", () => {
    const messages = REASONS.map((reason) => {
      const { container, unmount } = render(<WorkspaceCoverageNotice reason={reason} />);
      const text = container.textContent ?? "";
      unmount();
      return text;
    });

    // Four surfaces, four causes, four next actions. The marker this replaces said the same thing
    // for all of them, which was true everywhere and useful nowhere.
    expect(new Set(messages).size).toBe(REASONS.length);
    expect(messages.every((message) => message.trim().length > 0)).toBe(true);
  });

  it("names the machine when the workspace is on another one", () => {
    render(<WorkspaceCoverageNotice provider="ssh" reason="document-walk" />);

    // "The walk stopped early" on a host across a network is a different problem from the same
    // words about a local disk, and the remediation differs with it.
    expect(screen.getByRole("status").textContent).toMatch(/remote host|远程主机/);
  });

  it("adds nothing for a workspace on this machine", () => {
    render(<WorkspaceCoverageNotice provider="local" reason="document-walk" />);

    // Local is what a reader assumes when nothing says otherwise, so saying it would be noise on
    // every notice the common case produces.
    expect(screen.getByRole("status").textContent).not.toMatch(/remote host|远程主机/);
  });

  it("stays provider-neutral before capabilities are known", () => {
    render(<WorkspaceCoverageNotice reason="git-diff-bound" />);

    // Naming a machine before the answer arrives would be guessing, and the guess a reader would
    // act on is the wrong one exactly when the workspace is remote.
    expect(screen.getByRole("status").textContent).not.toMatch(/remote host|远程主机|demo|演示/);
  });

  it("leaves no panel reaching for the marker this replaced", () => {
    const directory = dirname(fileURLToPath(import.meta.url));
    const offenders = readdirSync(directory)
      .filter((name) => /\.tsx?$/.test(name) && !name.includes(".test."))
      .filter((name) => readFileSync(join(directory, name), "utf8").includes("PartialNotice"));

    // Removed rather than deprecated. Left in place it would stay the easier import, which is how
    // one message came to serve four different facts in the first place.
    expect(offenders).toEqual([]);
  });
});
