import { describe, expect, it } from "vitest";
import {
  EMPTY_FILTERS,
  EXECUTION_RECORD_VIEWS,
  hasActiveFilters,
  isLegacyView,
  queryFilters,
  SELECTABLE_FIDELITIES,
  SELECTABLE_STATUSES,
  toggleSelection,
} from "./execution-record-view";

describe("execution record views", () => {
  it("offers the native kinds and legacy as separate views", () => {
    expect([...EXECUTION_RECORD_VIEWS]).toEqual([
      "all",
      "commands",
      "tools",
      "delegations",
      "verification",
      "legacy",
    ]);
    expect(isLegacyView("legacy")).toBe(true);
    expect(isLegacyView("all")).toBe(false);
  });

  it("lets the view own the kinds so the two can never contradict", () => {
    // A Commands view showing tools, or showing nothing because the view and a kind filter
    // disagreed, are both states a reader cannot explain from what is on screen.
    expect(queryFilters("commands", EMPTY_FILTERS).kinds).toEqual(["command"]);
    expect(queryFilters("tools", EMPTY_FILTERS).kinds).toEqual(["tool"]);
    expect(queryFilters("delegations", EMPTY_FILTERS).kinds).toEqual(["delegation"]);
    expect(queryFilters("verification", EMPTY_FILTERS).kinds).toEqual(["verification"]);
    expect(queryFilters("all", EMPTY_FILTERS).kinds).toEqual([
      "command",
      "tool",
      "delegation",
      "verification",
    ]);
  });

  it("asks the record query for nothing while showing legacy activity", () => {
    // The legacy corpus is projected from loaded messages; a kind filter here would be asking the
    // journal for rows it was never given.
    expect(queryFilters("legacy", EMPTY_FILTERS).kinds).toEqual([]);
  });

  it("drops a blank search rather than sending it as a filter", () => {
    expect(queryFilters("all", { ...EMPTY_FILTERS, search: "   " }).search).toBeUndefined();
    expect(queryFilters("all", { ...EMPTY_FILTERS, search: " npm " }).search).toBe("npm");
  });

  it("keeps a selection in its declared order so the query key is stable", () => {
    // Two readers who chose the same statuses in a different order must produce one cache entry,
    // not two answers to the same question.
    const clicked = toggleSelection(
      toggleSelection([], "failed", SELECTABLE_STATUSES),
      "running",
      SELECTABLE_STATUSES,
    );
    expect(clicked).toEqual(["running", "failed"]);
    expect(toggleSelection(clicked, "running", SELECTABLE_STATUSES)).toEqual(["failed"]);
  });

  it("knows when something is narrowing the list", () => {
    // An empty list has to explain itself differently when a filter is on, so this is what the
    // difference between "nothing happened" and "nothing matched" is decided from.
    expect(hasActiveFilters(EMPTY_FILTERS)).toBe(false);
    expect(hasActiveFilters({ ...EMPTY_FILTERS, search: "  " })).toBe(false);
    expect(hasActiveFilters({ ...EMPTY_FILTERS, search: "npm" })).toBe(true);
    expect(hasActiveFilters({ ...EMPTY_FILTERS, statuses: ["failed"] })).toBe(true);
    expect(hasActiveFilters({ ...EMPTY_FILTERS, fidelities: ["opaque"] })).toBe(true);
  });

  it("offers every status and fidelity the evidence vocabulary defines", () => {
    expect([...SELECTABLE_STATUSES].sort()).toEqual([
      "cancelled",
      "failed",
      "incomplete",
      "queued",
      "running",
      "succeeded",
    ]);
    expect([...SELECTABLE_FIDELITIES].sort()).toEqual([
      "inferred",
      "native",
      "opaque",
      "proxied",
    ]);
  });
});
