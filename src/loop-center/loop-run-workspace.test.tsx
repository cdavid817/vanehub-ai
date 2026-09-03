// @vitest-environment jsdom

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { afterEach, describe, expect, it, vi } from "vitest";
import "../i18n";
import { agentService } from "../services/runtime-agent-client";
import { loopEvidenceFixture, loopIterationFixture, loopRunFixture } from "../test/loop-fixtures";
import { LoopRunControls } from "./loop-run-controls";
import { LoopTimeline } from "./loop-timeline";

describe("Loop run workspace", () => {
  afterEach(() => vi.restoreAllMocks());

  it("shows comparison summaries and keeps raw evidence collapsed", () => {
    const previous = loopIterationFixture({ sequence: 1, evidence: [
      loopEvidenceFixture({ id: "old-check", kind: "verification", commandId: "lint", status: "failed", details: null }),
      loopEvidenceFixture({ id: "old-change", details: { changedFiles: 1, additions: 2, deletions: 0 } }),
    ] });
    const current = loopIterationFixture({ id: "iteration-2", sequence: 2, evidence: [
      loopEvidenceFixture({ id: "new-check", kind: "verification", commandId: "tests", status: "failed", details: null }),
      // A distinct summary, not the shared fixture default: the "new-check" evidence above also
      // uses the default and is a verification check, which (unlike this one) renders unconditionally
      // once the row is expanded -- reusing the same text would make this assertion ambiguous.
      loopEvidenceFixture({ id: "new-change", summary: "Raw worker evidence, only in the full dump.", details: { changedFiles: 3, additions: 8, deletions: 1 } }),
    ] });
    renderWithClient(<LoopTimeline run={loopRunFixture("awaiting-acceptance", { currentIteration: 2, iterations: [previous, current] })} />);
    expect(screen.getByText("已解决的失败检查：lint")).toBeTruthy();
    expect(screen.getByText("新增失败检查：tests")).toBeTruthy();
    // The current iteration's row auto-expands (it's the latest), but its raw evidence dump is a
    // second, nested disclosure that still starts closed -- same "collapsed by default" claim the
    // test name makes, now checked against the new nested toggle instead of a former `<details
    // details>` structural query. Scoped to this iteration's own row: `selectCurrentLoopActivity`
    // independently surfaces this same evidence's summary in the run header above, so an unscoped
    // query would find that unrelated match too.
    const currentIterationRow = screen.getByText("第 2 次迭代").closest("li");
    if (!currentIterationRow) throw new Error("iteration row not found");
    expect(within(currentIterationRow).queryByText(/Raw worker evidence/)).toBeNull();
  });

  it("shows exhausted continuation and recovery/no-progress guidance", () => {
    const exhausted = loopRunFixture("awaiting-acceptance", { currentIteration: 3 });
    exhausted.definitionSnapshot.limits.maxIterations = 3;
    const first = renderWithClient(<LoopTimeline run={exhausted} />);
    expect(screen.getByText("已达到最大迭代次数，请接受或拒绝此结果。")).toBeTruthy();
    first.unmount();

    const recovery = renderWithClient(<LoopTimeline run={loopRunFixture("paused", { terminalReason: "recovery-required" })} />);
    expect(screen.getByText(/请先检查已保留的证据/)).toBeTruthy();
    recovery.unmount();
    renderWithClient(<LoopTimeline run={loopRunFixture("failed", { terminalReason: "no-progress" })} />);
    expect(screen.getByText("无有效进展")).toBeTruthy();
  });

  it("recovers the mutation controls after an action failure", async () => {
    vi.spyOn(agentService, "pauseLoop").mockRejectedValue(new Error("pause failed"));
    renderWithClient(<LoopRunControls run={loopRunFixture("running")} />);
    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "暂停" }));
    await user.click(screen.getByRole("button", { name: "确认" }));
    await waitFor(() => expect(screen.getByText("pause failed")).toBeTruthy());
    expect((screen.getByRole("button", { name: "暂停" }) as HTMLButtonElement).disabled).toBe(false);
  });

  // 21.12 "large iteration timeline" budget: `generateLoopRuns` (testing/fixtures/loop-run-fixtures.ts)
  // ties its own iteration count to `definitionSnapshot.limits.maxIterations` (realistically 2-8,
  // task 0.9's own scale), which never exercises a run with a genuinely large timeline -- built
  // directly here instead, the same way this file's own other cases already hand-build iteration
  // arrays via `loopIterationFixture`, just at a much larger count.
  it("renders every row once for a run with 60 iterations, and only the latest auto-expands (21.12)", () => {
    const iterations = Array.from({ length: 60 }, (_unused, index) =>
      loopIterationFixture({ id: `iteration-${index + 1}`, sequence: index + 1, status: "succeeded" }));
    const run = loopRunFixture("awaiting-acceptance", { currentIteration: 60, iterations });

    const start = performance.now();
    renderWithClient(<LoopTimeline run={run} />);
    const elapsedMs = performance.now() - start;
    console.info(`LoopTimeline 60-iteration render: ${elapsedMs.toFixed(1)}ms`);

    // One row per iteration, no cap and no duplication -- unlike Goal Center's relationship
    // sections (15.10/20.17), Loop Center's own timeline was never meant to hide iterations behind
    // a "show more" control (every iteration is real, load-bearing acceptance history, not a
    // relationship list), so the real budget here is that the *default-collapsed* row stays cheap,
    // not that the list gets capped. Scoped to `ol > li` specifically (not a bare role query):
    // `LoopAcceptancePanel` (rendered above the timeline while `awaiting-acceptance`, as here) has
    // its own real `<ul>` lists for acceptance criteria/checks, and `PhaseStepper` renders its own
    // 5-step `<ol aria-label=...>` above the timeline -- the iteration list is the one `<ol>` with
    // no `aria-label` of its own, confirmed by reading `loop-timeline.tsx`/`phase-stepper.tsx` directly.
    expect(document.querySelectorAll("ol:not([aria-label]) > li")).toHaveLength(60);
    // 17.10's own compact-row rebuild is what makes 60 rows cheap: only the run's own latest
    // iteration mounts its full detail section by default (`LoopTimeline`'s own
    // `open={index === run.iterations.length - 1}`) -- every other row stays a single collapsed
    // button, so this proves that budget actually holds at a scale far beyond the 1-2 iteration
    // fixtures this file's other cases use.
    expect(screen.getAllByRole("button", { expanded: true })).toHaveLength(1);
  });

  // 21.12 "action-update budget": pausing/accepting/rejecting a Loop must only update the relevant
  // state, not re-fetch or re-derive the whole iteration list. `LoopRunControls.execute`
  // (loop-run-controls.tsx) already patches the React Query cache directly via
  // `queryClient.setQueryData`/`setQueriesData` on a successful mutation -- confirmed by reading it
  // -- rather than invalidating and refetching, and `applyLoopRunUpdate` (hooks/loop-query.ts,
  // already covered by loop-query.test.ts's own "updates a loaded run without dropping surrounding
  // history") is what makes that patch targeted rather than a wholesale replace. This proves the
  // "no re-fetch" half holds for real at a scale (60 iterations) this file's other cases never
  // reach, and that the large iteration list already mounted survives the action untouched.
  it("pausing a running Loop with 60 iterations calls pauseLoop once and never re-fetches the run or run list (21.12)", async () => {
    const iterations = Array.from({ length: 60 }, (_unused, index) =>
      loopIterationFixture({ id: `iteration-${index + 1}`, sequence: index + 1, status: "succeeded" }));
    const running = loopRunFixture("running", { iterations });
    const pauseLoop = vi.spyOn(agentService, "pauseLoop").mockResolvedValue({ ...running, pauseRequested: true });
    const getLoopRun = vi.spyOn(agentService, "getLoopRun");
    const listLoopRuns = vi.spyOn(agentService, "listLoopRuns");

    renderWithClient(<LoopTimeline run={running} />);
    expect(document.querySelectorAll("ol:not([aria-label]) > li")).toHaveLength(60);

    const user = userEvent.setup();
    await user.click(screen.getByRole("button", { name: "暂停" }));
    await user.click(screen.getByRole("button", { name: "确认" }));

    await waitFor(() => expect(pauseLoop).toHaveBeenCalledTimes(1));
    expect(pauseLoop).toHaveBeenCalledWith("run-1");
    // The only network call this action makes is the mutation itself -- no follow-up read of the
    // run or the run list, regardless of how many iterations that run carries.
    expect(getLoopRun).not.toHaveBeenCalled();
    expect(listLoopRuns).not.toHaveBeenCalled();
    // The already-mounted large iteration list is untouched by the action (still 60 rows, not
    // cleared or shrunk as a side effect of the mutation's own pending/success state changes).
    expect(document.querySelectorAll("ol:not([aria-label]) > li")).toHaveLength(60);
  });
});

function renderWithClient(node: React.ReactNode) {
  const client = new QueryClient({ defaultOptions: { mutations: { retry: false }, queries: { retry: false } } });
  return render(<QueryClientProvider client={client}>{node}</QueryClientProvider>);
}
