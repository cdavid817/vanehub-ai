import { describe, expect, it } from "vitest";
import type { WorkItem } from "../types/work-board";
import {
  batchIneligibleReason, isBatchEligible, partitionBatchSelection, pruneBatchSelection,
} from "./work-board-batch";

const item = (overrides: Partial<WorkItem> = {}): WorkItem => ({
  id: "work-1", title: "Ship it", description: "", stage: "inbox", priority: "none",
  rank: 1, projectPath: null, dueAt: null, archived: false,
  createdAt: "2026-01-01", updatedAt: "2026-01-01", sources: [],
  ...overrides,
});

describe("batchIneligibleReason / isBatchEligible", () => {
  it("is eligible for archive when active", () => {
    expect(isBatchEligible(item(), { kind: "archive" })).toBe(true);
    expect(batchIneligibleReason(item(), { kind: "archive" })).toBeNull();
  });

  it("is ineligible for archive when already archived", () => {
    expect(isBatchEligible(item({ archived: true }), { kind: "archive" })).toBe(false);
    expect(batchIneligibleReason(item({ archived: true }), { kind: "archive" })).toBe("archived");
  });

  it("is eligible to move to a different stage", () => {
    const action = { kind: "move" as const, stage: "done" as const };
    expect(isBatchEligible(item({ stage: "inbox" }), action)).toBe(true);
  });

  it("is ineligible to move to its own current stage", () => {
    const action = { kind: "move" as const, stage: "inbox" as const };
    expect(isBatchEligible(item({ stage: "inbox" }), action)).toBe(false);
    expect(batchIneligibleReason(item({ stage: "inbox" }), action)).toBe("sameStage");
  });

  it("is ineligible to move an archived item even to a different stage", () => {
    const action = { kind: "move" as const, stage: "done" as const };
    expect(isBatchEligible(item({ archived: true, stage: "inbox" }), action)).toBe(false);
    // Archived takes priority over "different stage" -- matches the single-card UI, which hides
    // the stage menu for an archived card entirely rather than offering a stage it cannot use.
    expect(batchIneligibleReason(item({ archived: true, stage: "inbox" }), action)).toBe("archived");
  });
});

describe("partitionBatchSelection", () => {
  const a = item({ id: "a", stage: "inbox" });
  const b = item({ id: "b", stage: "inbox", archived: true });
  const c = item({ id: "c", stage: "done" });
  const items = [a, b, c];

  it("splits the selected subset into eligible and ineligible for the given action", () => {
    const selected = new Set(["a", "b", "c"]);
    const partition = partitionBatchSelection(items, selected, { kind: "archive" });
    expect(partition.eligible.map((entry) => entry.id)).toEqual(["a", "c"]);
    expect(partition.ineligible.map((entry) => entry.id)).toEqual(["b"]);
  });

  it("only ever considers selected items, not the full item list", () => {
    const selected = new Set(["a"]);
    const partition = partitionBatchSelection(items, selected, { kind: "archive" });
    expect(partition.eligible.map((entry) => entry.id)).toEqual(["a"]);
    expect(partition.ineligible).toHaveLength(0);
  });

  it("recomputes eligibility per move target without mutating anything", () => {
    const selected = new Set(["a", "c"]);
    const moveToDone = partitionBatchSelection(items, selected, { kind: "move", stage: "done" });
    expect(moveToDone.eligible.map((entry) => entry.id)).toEqual(["a"]);
    expect(moveToDone.ineligible.map((entry) => entry.id)).toEqual(["c"]);

    const moveToInbox = partitionBatchSelection(items, selected, { kind: "move", stage: "inbox" });
    expect(moveToInbox.eligible.map((entry) => entry.id)).toEqual(["c"]);
    expect(moveToInbox.ineligible.map((entry) => entry.id)).toEqual(["a"]);
  });
});

describe("pruneBatchSelection", () => {
  it("drops ids no longer present in the visible item set", () => {
    const selected = new Set(["a", "b", "c"]);
    const visible = [item({ id: "a" }), item({ id: "c" })];
    const pruned = pruneBatchSelection(selected, visible);
    expect([...pruned]).toEqual(["a", "c"]);
  });

  it("returns the exact same Set reference when nothing needs pruning", () => {
    const selected = new Set(["a"]);
    const visible = [item({ id: "a" }), item({ id: "b" })];
    expect(pruneBatchSelection(selected, visible)).toBe(selected);
  });
});
