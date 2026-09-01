import { describe, expect, it } from "vitest";
import { pickPageStatus } from "./settings-page-status";

describe("pickPageStatus", () => {
  it("returns null when nothing is true", () => {
    expect(pickPageStatus([null, undefined, null])).toBeNull();
  });

  it("passes through the single true condition", () => {
    expect(pickPageStatus([null, { kind: "unsaved", labelKey: "x" }, null])).toEqual({
      kind: "unsaved",
      labelKey: "x",
    });
  });

  it("prefers error over every other condition", () => {
    const winner = pickPageStatus([
      { kind: "update-available", labelKey: "u" },
      { kind: "unsaved", labelKey: "s" },
      { kind: "error", labelKey: "e" },
      { kind: "restart-required", labelKey: "r" },
    ]);
    expect(winner?.kind).toBe("error");
  });

  it("prefers dependency-unavailable over unsaved, restart-required, and update-available", () => {
    const winner = pickPageStatus([
      { kind: "update-available", labelKey: "u" },
      { kind: "restart-required", labelKey: "r" },
      { kind: "dependency-unavailable", labelKey: "d" },
      { kind: "unsaved", labelKey: "s" },
    ]);
    expect(winner?.kind).toBe("dependency-unavailable");
  });

  it("prefers unsaved over restart-required and update-available", () => {
    const winner = pickPageStatus([
      { kind: "update-available", labelKey: "u" },
      { kind: "restart-required", labelKey: "r" },
      { kind: "unsaved", labelKey: "s" },
    ]);
    expect(winner?.kind).toBe("unsaved");
  });

  it("prefers restart-required over update-available", () => {
    const winner = pickPageStatus([
      { kind: "update-available", labelKey: "u" },
      { kind: "restart-required", labelKey: "r" },
    ]);
    expect(winner?.kind).toBe("restart-required");
  });

  it("falls back to update-available alone", () => {
    expect(pickPageStatus([{ kind: "update-available", labelKey: "u" }])?.kind).toBe("update-available");
  });
});
