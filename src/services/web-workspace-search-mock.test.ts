import { afterEach, describe, expect, it } from "vitest";
import {
  cancelWebWorkspaceSearch,
  configureWebWorkspaceSearch,
  resetWebWorkspaceSearch,
  runWebWorkspaceSearch,
} from "./web-workspace-search-mock";

const files = {
  "a.ts": "const needle = 1;\nconst other = 2;",
  "b.ts": "// needle again\n",
  "c.ts": "nothing here\n",
};

/**
 * One turn of the scan.
 *
 * The scan yields before each file, so a single flush lets exactly one file through. That is what
 * makes "stopped part way" testable at all: a synchronous mock would finish before any cancel could
 * be issued, and every cancellation test would pass by never exercising the mechanism.
 */
function oneFile() {
  return Promise.resolve();
}

afterEach(() => {
  resetWebWorkspaceSearch();
});

describe("web workspace search", () => {
  it("reports complete and accounts what it read", async () => {
    const result = await runWebWorkspaceSearch({ query: "needle", searchId: "s" }, files);

    expect(result.coverage.state).toBe("complete");
    expect(result.coverage.reasonCode).toBeUndefined();
    expect(result.matches.map((match) => match.path)).toEqual(["a.ts", "b.ts"]);
    // Accounted rather than asserted as zero. A coverage that claimed no spend would be describing
    // a scan that never happened, which is the one thing a simulated adapter must not do.
    expect(result.coverage.budget?.filesOpened).toBe(3);
    expect(result.coverage.budget?.bytesRead).toBeGreaterThan(0);
  });

  it("stops where the cancel reached it and keeps what it had already found", async () => {
    const running = runWebWorkspaceSearch({ query: "needle", searchId: "s" }, files);
    await oneFile();
    expect(cancelWebWorkspaceSearch("s")).toBe(true);
    const result = await running;

    expect(result.coverage.state).toBe("partial");
    expect(result.coverage.reasonCode).toBe("cancelled");
    // Kept, not discarded. The reader asked it to stop, not to forget — and the matches it already
    // has are matches for the query they typed.
    expect(result.matches.map((match) => match.path)).toEqual(["a.ts"]);
    expect(result.coverage.budget?.filesOpened).toBe(1);
  });

  it("answers false for a cancel with nothing to stop", async () => {
    // Not an error. A caller cannot know whether their cancel beat the search's own completion, and
    // turning that ordinary race into a failure would put an error on screen for a keystroke that
    // worked exactly as intended.
    expect(cancelWebWorkspaceSearch("never-started")).toBe(false);
  });

  it("drops a superseded generation's matches and says why", async () => {
    const stale = runWebWorkspaceSearch({ query: "needle", searchId: "s" }, files);
    await oneFile();
    const fresh = runWebWorkspaceSearch({ query: "other", searchId: "s" }, files);

    const late = await stale;
    const current = await fresh;

    expect(late.coverage.reasonCode).toBe("superseded");
    // The matches it found are matches for a query the reader has already retyped. Handing them back
    // would invite the panel to render text nobody searched for.
    expect(late.matches).toEqual([]);
    expect(late.generation).toBeLessThan(current.generation);
    expect(current.coverage.state).toBe("complete");
  });

  it("reports the byte budget the same way the native adapter does", async () => {
    configureWebWorkspaceSearch({ maxBytes: 20 });

    const result = await runWebWorkspaceSearch({ query: "needle", searchId: "s" }, files);

    expect(result.coverage.state).toBe("partial");
    expect(result.coverage.reasonCode).toBe("byte_budget_exhausted");
    expect(result.coverage.budget?.bytesRead).toBeLessThanOrEqual(20);
  });

  it("reports the result budget rather than silently returning a short list", async () => {
    const result = await runWebWorkspaceSearch({ query: "needle", searchId: "s", limit: 1 }, files);

    expect(result.matches).toHaveLength(1);
    // A short list that claimed `complete` is the failure this exists to prevent: the reader would
    // read one match as the only match.
    expect(result.coverage.reasonCode).toBe("result_budget_exhausted");
  });

  it("refuses an admission it has no capacity for without claiming a scan", async () => {
    configureWebWorkspaceSearch({ maxConcurrent: 1 });
    const holding = runWebWorkspaceSearch({ query: "needle", searchId: "first" }, files);

    const refused = await runWebWorkspaceSearch({ query: "needle", searchId: "second" }, files);

    expect(refused.coverage.state).toBe("unavailable");
    expect(refused.coverage.reasonCode).toBe("inspection_busy");
    // `unavailable` rather than `partial`, and a budget of zero, because nothing was examined. A
    // partial would say "here is what we found so far" about a search that never started.
    expect(refused.matches).toEqual([]);
    expect(refused.coverage.budget?.filesOpened).toBe(0);
    await holding;
  });

  it("frees its capacity once a search finishes", async () => {
    configureWebWorkspaceSearch({ maxConcurrent: 1 });
    await runWebWorkspaceSearch({ query: "needle", searchId: "first" }, files);

    const second = await runWebWorkspaceSearch({ query: "needle", searchId: "second" }, files);

    // The refusal above has to be about capacity in use, not capacity permanently spent.
    expect(second.coverage.state).toBe("complete");
  });

  it("charges nothing for an empty query", async () => {
    const result = await runWebWorkspaceSearch({ query: "  ", searchId: "s" }, files);

    expect(result.matches).toEqual([]);
    expect(result.coverage.state).toBe("complete");
    // Reporting a spend here would be inventing work to explain an answer that cost none.
    expect(result.coverage.budget?.filesOpened).toBe(0);
  });
});
