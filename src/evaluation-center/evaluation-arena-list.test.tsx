// @vitest-environment jsdom

import { fireEvent, render, screen, within } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { i18n } from "../i18n";
import type { EvaluationArena, EvaluationAttempt, EvaluationOutcome, EvaluationTask } from "../types/evaluation";
import { EvaluationArenaList } from "./evaluation-arena-list";

function attempt(outcome: EvaluationOutcome, agentId: string, arenaId: string, id: string): EvaluationAttempt {
  return {
    id, arenaId, canonicalRunId: `${id}-run`, taskId: "fix-null-auth-token", taskVersion: 1,
    agent: { agentId, providerId: agentId, modelId: null, interactionMode: "cli", configurationFingerprint: "fp" },
    outcome, checks: [], metrics: [], contextEvidenceManifestId: null, artifactIds: [], timeline: [],
  };
}

function arena(id: string, attempts: EvaluationAttempt[]): EvaluationArena {
  return { id, operationId: `${id}-op`, taskId: "fix-null-auth-token", taskVersion: 1, rankingVersion: "v1", attempts };
}

const TASK: EvaluationTask = {
  id: "fix-null-auth-token", version: 1, category: "bugfix",
  prompt: "Fix null authentication token handling.", timeoutSeconds: 120, verifierProfiles: ["npm-test"],
};

beforeEach(async () => {
  await i18n.changeLanguage("zh-CN");
});

describe("EvaluationArenaList", () => {
  it("renders exactly one row per arena, not one per attempt", () => {
    const mixedArena = arena("arena-mixed", [
      attempt("succeeded", "claude-code", "arena-mixed", "attempt-1"),
      attempt("task_failed", "codex-cli", "arena-mixed", "attempt-2"),
      attempt("succeeded", "opencode", "arena-mixed", "attempt-3"),
    ]);
    const soloArena = arena("arena-solo", [attempt("succeeded", "claude-code", "arena-solo", "attempt-4")]);
    render(<EvaluationArenaList arenas={[mixedArena, soloArena]} tasks={[]} />);
    const rows = screen.getAllByTestId("evaluation-arena-row");
    expect(rows).toHaveLength(2);
    expect(rows.map((row) => row.getAttribute("data-arena-id"))).toEqual(["arena-mixed", "arena-solo"]);
  });

  it("derives hasFailures for a terminal arena with a mixed outcome, and shows the outcome tally", () => {
    const mixedArena = arena("arena-mixed", [
      attempt("succeeded", "claude-code", "arena-mixed", "attempt-1"),
      attempt("succeeded", "codex-cli", "arena-mixed", "attempt-2"),
      attempt("task_failed", "opencode", "arena-mixed", "attempt-3"),
    ]);
    render(<EvaluationArenaList arenas={[mixedArena]} tasks={[]} />);
    const row = screen.getByTestId("evaluation-arena-row");
    expect(row.getAttribute("data-arena-state")).toBe("hasFailures");
    expect(within(row).getByText("状态: 存在失败")).toBeTruthy();
    expect(within(row).getByText("2 · 通过")).toBeTruthy();
    expect(within(row).getByText("1 · 任务失败")).toBeTruthy();
  });

  it("derives running for an arena still in progress, even though it also holds a settled attempt", () => {
    const inProgress = arena("arena-progress", [
      attempt("succeeded", "claude-code", "arena-progress", "attempt-1"),
      attempt("running", "codex-cli", "arena-progress", "attempt-2"),
    ]);
    render(<EvaluationArenaList arenas={[inProgress]} tasks={[]} />);
    const row = screen.getByTestId("evaluation-arena-row");
    expect(row.getAttribute("data-arena-state")).toBe("running");
    expect(within(row).getByText("状态: 运行中")).toBeTruthy();
  });

  it("de-duplicates the Agent set by agentId when an arena holds two attempts against the same Agent", () => {
    const rerun = arena("arena-rerun", [
      attempt("task_failed", "claude-code", "arena-rerun", "attempt-1"),
      attempt("succeeded", "claude-code", "arena-rerun", "attempt-2"),
    ]);
    render(<EvaluationArenaList arenas={[rerun]} tasks={[]} />);
    const row = screen.getByTestId("evaluation-arena-row");
    // A naive (non-de-duplicated) implementation would render "claude-code, claude-code".
    expect(within(row).getByText("claude-code")).toBeTruthy();
  });

  it("shows the regression-state and updated-time fields as honestly unavailable, not fabricated", () => {
    render(<EvaluationArenaList arenas={[arena("arena-a", [attempt("succeeded", "claude-code", "arena-a", "attempt-1")])]} tasks={[]} />);
    const row = screen.getByTestId("evaluation-arena-row");
    expect(within(row).getByTestId("evaluation-arena-regression").textContent).toBe("不可用");
    expect(within(row).getByTestId("evaluation-arena-updated").textContent).toBe("不可用");
  });

  it("cross-references the task catalog for a human-readable prompt, and omits it when no task matches", () => {
    const known = arena("arena-known", [attempt("succeeded", "claude-code", "arena-known", "attempt-1")]);
    const unknown = arena("arena-unknown", [attempt("succeeded", "claude-code", "arena-unknown", "attempt-2")]);
    unknown.taskId = "no-such-task";
    render(<EvaluationArenaList arenas={[known, unknown]} tasks={[TASK]} />);
    const rows = screen.getAllByTestId("evaluation-arena-row");
    expect(within(rows[0]).getByText("Fix null authentication token handling.")).toBeTruthy();
    expect(within(rows[1]).queryByText("Fix null authentication token handling.")).toBeNull();
  });

  it("shows the shared empty-result message and zero rows when there are no arenas yet", () => {
    render(<EvaluationArenaList arenas={[]} tasks={[]} />);
    expect(screen.queryByTestId("evaluation-arena-row")).toBeNull();
    expect(screen.getByText("运行基准以比较结果。")).toBeTruthy();
  });

  /**
   * 20.16: `taskId`/`agentId` are catalog/provider keys, not translated UI text, rendered next to
   * this row's own fixed-direction " vN" suffix and ", " separators -- both now wrapped in `<bdi>`
   * (the standard HTML bidi-isolation element) so a real id containing a strong-RTL character
   * cannot read that surrounding chrome out of order. Real, DOM-structural proof: a fixture id
   * containing an actual RTL character, asserting the isolation boundary wraps exactly that text --
   * not a claim about how it paints, which jsdom cannot render.
   */
  describe("bidi isolation (20.16)", () => {
    it("wraps a task id containing an RTL character in its own bdi boundary", () => {
      const rtlTaskId = "משימה-לדוגמה";
      const rtlArena = arena("arena-rtl", [attempt("succeeded", "claude-code", "arena-rtl", "attempt-rtl")]);
      rtlArena.taskId = rtlTaskId;
      render(<EvaluationArenaList arenas={[rtlArena]} tasks={[]} />);
      const isolated = screen.getByText(rtlTaskId, { selector: "bdi" });
      expect(isolated.textContent).toBe(rtlTaskId);
    });

    it("wraps an agent id containing an RTL character in its own bdi boundary", () => {
      const rtlAgentId = "עוזר-לדוגמה";
      const rtlArena = arena("arena-rtl-agent", [attempt("succeeded", rtlAgentId, "arena-rtl-agent", "attempt-rtl-agent")]);
      render(<EvaluationArenaList arenas={[rtlArena]} tasks={[]} />);
      const isolated = screen.getByText(rtlAgentId, { selector: "bdi" });
      expect(isolated.textContent).toBe(rtlAgentId);
    });

    it("still separates two agent ids with a plain comma-space when de-duplicated and joined", () => {
      const twoAgents = arena("arena-two", [
        attempt("succeeded", "claude-code", "arena-two", "attempt-1"),
        attempt("succeeded", "codex-cli", "arena-two", "attempt-2"),
      ]);
      render(<EvaluationArenaList arenas={[twoAgents]} tasks={[]} />);
      const row = screen.getByTestId("evaluation-arena-row");
      // Both ids still render as their own real text, and the dd's own concatenated text still
      // reads as a normal comma-separated list -- switching from `.join(", ")` to per-id `<bdi>`
      // wrapping did not silently drop the separator.
      expect(within(row).getByText("claude-code", { selector: "bdi" })).toBeTruthy();
      expect(within(row).getByText("codex-cli", { selector: "bdi" })).toBeTruthy();
      expect(row.querySelector("dd")?.textContent).toBe("claude-code, codex-cli");
    });
  });

  // 18.6: the "load more" control is optional-props-driven so every pre-existing caller/test above
  // (none of which pass `hasMore`) keeps rendering with no pagination control at all.
  describe("pagination (18.6)", () => {
    const oneArena = [arena("arena-a", [attempt("succeeded", "claude-code", "arena-a", "attempt-1")])];

    it("renders no load-more control when there is no further page", () => {
      render(<EvaluationArenaList arenas={oneArena} tasks={[]} />);
      expect(screen.queryByTestId("evaluation-arena-load-more")).toBeNull();
    });

    it("renders an enabled load-more control that calls back on click when a further page exists", () => {
      const onLoadMore = vi.fn();
      render(<EvaluationArenaList arenas={oneArena} hasMore onLoadMore={onLoadMore} tasks={[]} />);
      const button = screen.getByTestId("evaluation-arena-load-more") as HTMLButtonElement;
      expect(button.disabled).toBe(false);
      expect(button.textContent).toBe("加载更多");
      fireEvent.click(button);
      expect(onLoadMore).toHaveBeenCalledTimes(1);
    });

    it("disables the load-more control and shows a distinct label while a page fetch is in flight", () => {
      render(<EvaluationArenaList arenas={oneArena} hasMore loadingMore tasks={[]} />);
      const button = screen.getByTestId("evaluation-arena-load-more") as HTMLButtonElement;
      expect(button.disabled).toBe(true);
      expect(button.textContent).toBe("加载中...");
    });
  });
});
