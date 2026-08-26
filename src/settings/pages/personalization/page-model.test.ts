import { describe, expect, it } from "vitest";
import {
  acceptsWrites,
  advanceMemoryCursor,
  applyHealth,
  describeQuery,
  initialPersonalizationPageState,
  selectScope,
  setMemoryFilter,
  setReconciling,
  withQuery,
} from "./page-model";
import { draftFromPolicy, editDraft, scopeKeyOf } from "./instruction-drafts";

const GLOBAL = { scopeKind: "global" } as const;
const AGENT = { scopeKind: "agent", agentId: "onepiece" } as const;

describe("personalization page model", () => {
  it("assumes nothing about health until something reports it", () => {
    // Assuming `ready` would let the page offer writes during a migration, which is the one state
    // the store must not take them in.
    expect(initialPersonalizationPageState.maintenance.health).toBe("not_started");
    expect(acceptsWrites(initialPersonalizationPageState.maintenance)).toBe(false);
  });

  it("accepts writes only once the store reports itself ready", () => {
    const ready = applyHealth(initialPersonalizationPageState, {
      state: "ready",
      memoryAvailable: true,
      pendingCandidates: 2,
    });

    expect(acceptsWrites(ready.maintenance)).toBe(true);
    expect(ready.maintenance.pendingCandidates).toBe(2);
  });

  it("refuses writes while a reconciliation the user started is running", () => {
    const ready = applyHealth(initialPersonalizationPageState, {
      state: "ready",
      memoryAvailable: true,
      pendingCandidates: 0,
    });

    // A query result cannot report this: the rebuild is in flight from this page.
    expect(acceptsWrites(setReconciling(ready, true).maintenance)).toBe(false);
    expect(acceptsWrites(setReconciling(setReconciling(ready, true), false).maintenance)).toBe(true);
  });

  it.each(["migrating", "repair_required", "busy", "failed"] as const)(
    "refuses writes while the store reports %s",
    (state) => {
      const health = applyHealth(initialPersonalizationPageState, {
        state,
        memoryAvailable: false,
        pendingCandidates: 0,
      });

      expect(acceptsWrites(health.maintenance)).toBe(false);
    },
  );

  it("keeps drafts when the user switches scope and comes back", () => {
    let state = {
      ...initialPersonalizationPageState,
      drafts: { [scopeKeyOf(GLOBAL)]: draftFromPolicy(GLOBAL, null) },
    };
    state = { ...state, drafts: editDraft(state.drafts, GLOBAL, { aboutUser: "typed" }) };

    state = selectScope(state, AGENT);
    state = selectScope(state, GLOBAL);

    expect(state.scope).toEqual(GLOBAL);
    expect(state.drafts[scopeKeyOf(GLOBAL)].values.aboutUser).toBe("typed");
  });

  it("starts the result set over when a filter changes", () => {
    let state = advanceMemoryCursor(initialPersonalizationPageState, "cursor-2");

    state = setMemoryFilter(state, { status: "archived" });

    // A cursor names a position in one filtered ordering; carrying it into another resumes from a
    // row that is no longer in the set, which reads as a page of missing results.
    expect(state.memoryQuery.cursor).toBeUndefined();
    expect(state.memoryQuery.status).toBe("archived");
  });

  it("keeps earlier filters when one of them changes", () => {
    let state = setMemoryFilter(initialPersonalizationPageState, { text: "npm" });

    state = setMemoryFilter(state, { memoryType: "project" });

    expect(state.memoryQuery).toEqual({ text: "npm", memoryType: "project", cursor: undefined });
  });

  it("clears the cursor when a page reports it has no next one", () => {
    const state = advanceMemoryCursor(
      advanceMemoryCursor(initialPersonalizationPageState, "cursor-2"),
      null,
    );

    expect(state.memoryQuery.cursor).toBeUndefined();
  });

  it("projects a cache entry rather than copying it", () => {
    expect(describeQuery({ isPending: true, error: null })).toEqual({ status: "loading", error: null });
    expect(describeQuery({ isPending: false, error: null })).toEqual({ status: "ready", error: null });
    expect(describeQuery({ isPending: false, error: new Error("personalization-not-found") })).toEqual({
      status: "error",
      error: "personalization-not-found",
    });
  });

  it("reports an error even while the query is still pending a retry", () => {
    // react-query keeps `isPending` true through retries; a page that checked pending first would
    // render a spinner over a failure the user could act on.
    expect(describeQuery({ isPending: true, error: new Error("boom") }).status).toBe("error");
  });

  it("updates one query slice without resetting the others", () => {
    const state = withQuery(initialPersonalizationPageState, {
      policies: { status: "ready", error: null },
    });

    expect(state.query.policies.status).toBe("ready");
    expect(state.query.memories.status).toBe("loading");
  });
});
