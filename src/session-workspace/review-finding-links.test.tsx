/** @vitest-environment jsdom */
import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../i18n";
import { renderWithAppProviders } from "../test/render";
import type { ReviewFinding } from "../types/code-review";
import { ReviewFindings } from "./review-findings";

/**
 * A finding is a claim; these are how a reviewer checks it.
 *
 * Before this, a finding rendered as one line of text — a machine's assertion with nothing beside
 * it, which a reader either believes or ignores. The run that produced it and the code it is about
 * are both reachable from what the finding already carries, so the interesting question is not
 * whether the links render but whether they are offered exactly when they lead somewhere.
 */

beforeAll(async () => {
  await activateAppLanguage("en");
});

function finding(overrides: Partial<ReviewFinding> = {}): ReviewFinding {
  return {
    id: "finding-1",
    operationId: "operation-1",
    resolved: false,
    severity: "error",
    source: "tests",
    title: "A test failed",
    ...overrides,
  };
}

const ANCHOR = {
  contextFingerprint: "context-1",
  endLine: 4,
  filePath: "src/main.rs",
  hunkFingerprint: "hunk-1",
  side: "new" as const,
  startLine: 4,
  state: "current" as const,
};

describe("finding links", () => {
  it("offers the run that produced it", () => {
    const onShowOperation = vi.fn();
    renderWithAppProviders(
      <ReviewFindings
        findings={[finding()]}
        onShowCode={vi.fn()}
        onShowOperation={onShowOperation}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Show the run" }));

    // The operation and nothing else. The records tab resolves the run, the trace, and the span
    // from it; carrying those here would be three identifiers this side has to keep agreeing with
    // a store it cannot read.
    expect(onShowOperation).toHaveBeenCalledWith("operation-1");
  });

  it("offers the code only for a finding that names some", () => {
    renderWithAppProviders(
      <ReviewFindings
        findings={[finding(), finding({ anchor: ANCHOR, id: "finding-2" })]}
        onShowCode={vi.fn()}
        onShowOperation={vi.fn()}
      />,
    );

    // One of the two. A finding with no anchor is about the run, not about a line, and a link that
    // led nowhere would read as the application being broken when it was pressed.
    expect(screen.getAllByRole("button", { name: "Show the code" })).toHaveLength(1);
  });

  it("selects the anchored file inside the panel rather than navigating away", () => {
    const onShowCode = vi.fn();
    renderWithAppProviders(
      <ReviewFindings
        findings={[finding({ anchor: ANCHOR })]}
        onShowCode={onShowCode}
        onShowOperation={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Show the code" }));

    // The Review Center *is* the Changes tab, so navigating there would be a no-op that looks like
    // a broken link.
    expect(onShowCode).toHaveBeenCalledWith("src/main.rs");
  });

  it("withholds the run link where nothing owns the evidence scope", () => {
    renderWithAppProviders(<ReviewFindings findings={[finding()]} onShowCode={vi.fn()} />);

    // Rendered without the navigation, which is how every panel test in this directory mounts it.
    // A component that read the scope context directly would throw here instead.
    expect(screen.queryByRole("button", { name: "Show the run" })).toBeNull();
    expect(screen.getByText(/A test failed/)).toBeTruthy();
  });
});
