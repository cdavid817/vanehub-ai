import { describe, expect, it, vi } from "vitest";
import { retainAsyncCleanup } from "./async-cleanup";

describe("retainAsyncCleanup", () => {
  it("runs a cleanup that resolves after its owner was disposed", () => {
    const cleanup = vi.fn();

    expect(retainAsyncCleanup(true, cleanup)).toBeNull();
    expect(cleanup).toHaveBeenCalledOnce();
  });

  it("returns a cleanup while its owner is active", () => {
    const cleanup = vi.fn();

    expect(retainAsyncCleanup(false, cleanup)).toBe(cleanup);
    expect(cleanup).not.toHaveBeenCalled();
  });
});
