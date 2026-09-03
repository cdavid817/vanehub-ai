// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it } from "vitest";
import "../i18n";
import { loopEvidenceFixture, loopIterationFixture } from "../test/loop-fixtures";
import { LoopIterationRow } from "./loop-iteration-row";

describe("LoopIterationRow", () => {
  it("shows the decision-relevant facts on the row itself, with full detail collapsed by default", async () => {
    const user = userEvent.setup();
    render(<LoopIterationRow iteration={loopIterationFixture()} open={false} previousIteration={null} />);

    // Verdict-first: number, outcome, status, verifier recommendation, duration, and evidence
    // count are all visible without expanding anything.
    expect(screen.getByText("第 1 次迭代")).toBeTruthy();
    expect(screen.getByText("Ready for acceptance.")).toBeTruthy();
    expect(screen.getByText("等待验收")).toBeTruthy();
    expect(screen.getByText("验证者：通过")).toBeTruthy();
    expect(screen.getByText("1:30")).toBeTruthy();
    expect(screen.getByText("1 条证据")).toBeTruthy();

    // Full detail (worker summary text, diff fingerprint) is not yet in the DOM at all.
    expect(screen.queryByText("Implemented the change.")).toBeNull();
    expect(screen.queryByText("diff-1")).toBeNull();

    await user.click(screen.getByRole("button", { expanded: false }));

    expect(screen.getByText("Implemented the change.")).toBeTruthy();
    expect(screen.getByText("diff-1")).toBeTruthy();
    expect(screen.getByText("2 个文件，+12 / -3")).toBeTruthy();
  });

  it("auto-expands when told it is the latest iteration, without requiring a click", () => {
    render(<LoopIterationRow iteration={loopIterationFixture()} open previousIteration={null} />);
    expect(screen.getByRole("button", { expanded: true })).toBeTruthy();
    expect(screen.getByText("Implemented the change.")).toBeTruthy();
  });

  it("collapses the detail again on a second click of the same toggle", async () => {
    const user = userEvent.setup();
    render(<LoopIterationRow iteration={loopIterationFixture()} open={false} previousIteration={null} />);
    await user.click(screen.getByRole("button", { expanded: false }));
    expect(screen.getByText("Implemented the change.")).toBeTruthy();
    await user.click(screen.getByRole("button", { expanded: true }));
    expect(screen.queryByText("Implemented the change.")).toBeNull();
  });

  it("keeps the raw per-evidence dump a second, nested disclosure that starts closed even once the row is expanded", async () => {
    const user = userEvent.setup();
    render(<LoopIterationRow iteration={loopIterationFixture()} open previousIteration={null} />);
    // The default fixture's one evidence item's own `.summary` text ("Worker completed.") is only
    // ever rendered inside the raw evidence dump -- no other section reads an evidence item's own
    // summary field, so its absence/presence is a direct signal of that nested disclosure's state.
    expect(screen.queryByText(/Worker completed\./)).toBeNull();
    const nestedToggle = screen.getByRole("button", { expanded: false, name: "证据时间线" });
    await user.click(nestedToggle);
    expect(screen.getByText(/Worker completed\./)).toBeTruthy();
  });

  it("shows a checks pass/fail tally on the row, and the full per-check breakdown once expanded", async () => {
    const user = userEvent.setup();
    const iteration = loopIterationFixture({
      evidence: [
        loopEvidenceFixture({ id: "check-pass", kind: "verification", commandId: "lint", status: "passed", details: null }),
        loopEvidenceFixture({ id: "check-fail", kind: "verification", commandId: "tests", status: "failed", details: null }),
      ],
    });
    render(<LoopIterationRow iteration={iteration} open={false} previousIteration={null} />);
    expect(screen.getByText("验证检查 1/2")).toBeTruthy();
    expect(screen.queryByText("lint")).toBeNull();

    await user.click(screen.getByRole("button", { expanded: false }));
    expect(screen.getByText("lint")).toBeTruthy();
    expect(screen.getByText("tests")).toBeTruthy();
  });

  it("renders resolved/new-failure comparisons against the previous iteration", () => {
    const previous = loopIterationFixture({ sequence: 1, evidence: [
      loopEvidenceFixture({ id: "old-check", kind: "verification", commandId: "lint", status: "failed", details: null }),
    ] });
    const current = loopIterationFixture({ id: "iteration-2", sequence: 2, evidence: [
      loopEvidenceFixture({ id: "new-check", kind: "verification", commandId: "tests", status: "failed", details: null }),
    ] });
    render(<LoopIterationRow iteration={current} open previousIteration={previous} />);
    expect(screen.getByText("已解决的失败检查：lint")).toBeTruthy();
    expect(screen.getByText("新增失败检查：tests")).toBeTruthy();
  });

  it("keeps verifier findings, continuation feedback, and recovery evidence available once expanded", () => {
    const iteration = loopIterationFixture({
      verifierFindings: ["Missing null check"],
      userFeedback: "Please also update the changelog.",
      evidence: [loopEvidenceFixture(), loopEvidenceFixture({ id: "recovery-1", kind: "recovery", summary: "Resumed after crash." })],
    });
    render(<LoopIterationRow iteration={iteration} open previousIteration={null} />);
    expect(screen.getByText("Missing null check")).toBeTruthy();
    expect(screen.getByText("Please also update the changelog.")).toBeTruthy();
    expect(screen.getByText("Resumed after crash.")).toBeTruthy();
  });
});
