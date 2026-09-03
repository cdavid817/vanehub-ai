// @vitest-environment jsdom

import { beforeEach, describe, expect, it } from "vitest";
import { isOverWipLimit, readWorkBoardWipLimits, writeWorkBoardWipLimits, type WorkBoardWipLimits } from "./work-board-wip-limits";

const STORAGE_KEY = "vanehub.work-board.wip-limits.v1";

beforeEach(() => {
  localStorage.clear();
});

describe("isOverWipLimit", () => {
  it("is false when no limit is configured", () => {
    expect(isOverWipLimit(999, undefined)).toBe(false);
  });

  it("is false at or under the limit", () => {
    expect(isOverWipLimit(5, 5)).toBe(false);
    expect(isOverWipLimit(4, 5)).toBe(false);
  });

  it("is true only once the count exceeds the limit", () => {
    expect(isOverWipLimit(6, 5)).toBe(true);
  });

  it("treats a limit of 0 as no limit, not zero capacity", () => {
    expect(isOverWipLimit(1, 0)).toBe(false);
  });
});

describe("readWorkBoardWipLimits / writeWorkBoardWipLimits", () => {
  it("round-trips a saved set of per-stage limits", () => {
    const limits: WorkBoardWipLimits = { inbox: 5, review: 2 };
    writeWorkBoardWipLimits(limits);
    expect(readWorkBoardWipLimits()).toEqual(limits);
  });

  it("returns an empty object when nothing is stored", () => {
    expect(readWorkBoardWipLimits()).toEqual({});
  });

  it("fails closed to an empty object on a version mismatch, per the versioned-storage precedent in work-board-saved-views.ts", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 999, limits: { inbox: 5 } }));
    expect(readWorkBoardWipLimits()).toEqual({});
  });

  it("fails closed to an empty object on malformed JSON rather than throwing", () => {
    localStorage.setItem(STORAGE_KEY, "{not json");
    expect(readWorkBoardWipLimits()).toEqual({});
  });

  it("rejects a stored limit for a stage that does not exist", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, limits: { notAStage: 5 } }));
    expect(readWorkBoardWipLimits()).toEqual({});
  });

  it("rejects a non-numeric or non-positive stored limit", () => {
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, limits: { inbox: "five" } }));
    expect(readWorkBoardWipLimits()).toEqual({});
    localStorage.setItem(STORAGE_KEY, JSON.stringify({ version: 1, limits: { inbox: 0 } }));
    expect(readWorkBoardWipLimits()).toEqual({});
  });
});
