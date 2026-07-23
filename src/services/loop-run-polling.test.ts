import { afterEach, describe, expect, it, vi } from "vitest";
import type { LoopRun } from "../types/loop";
import { subscribeLoopRunPolling } from "./loop-run-polling";

function run(updatedAt: string): LoopRun {
  return { id: "run-1", updatedAt } as LoopRun;
}

describe("subscribeLoopRunPolling", () => {
  afterEach(() => vi.useRealTimers());

  it("emits only changed snapshots and stops after unsubscribe", async () => {
    vi.useFakeTimers();
    const loadRun = vi
      .fn<() => Promise<LoopRun>>()
      .mockResolvedValueOnce(run("2026-07-22T10:00:00Z"))
      .mockResolvedValueOnce(run("2026-07-22T10:00:00Z"))
      .mockResolvedValue(run("2026-07-22T10:00:01Z"));
    const handler = vi.fn();

    const unsubscribe = subscribeLoopRunPolling(loadRun, handler, 100);
    await vi.advanceTimersByTimeAsync(200);

    expect(handler).toHaveBeenCalledOnce();
    expect(handler).toHaveBeenCalledWith({ kind: "run-updated", run: run("2026-07-22T10:00:01Z") });

    unsubscribe();
    await vi.advanceTimersByTimeAsync(200);
    expect(loadRun).toHaveBeenCalledTimes(3);
  });

  it("continues polling after a transient read failure", async () => {
    vi.useFakeTimers();
    const loadRun = vi
      .fn<() => Promise<LoopRun>>()
      .mockRejectedValueOnce(new Error("offline"))
      .mockResolvedValueOnce(run("2026-07-22T10:00:00Z"))
      .mockResolvedValue(run("2026-07-22T10:00:01Z"));
    const handler = vi.fn();

    const unsubscribe = subscribeLoopRunPolling(loadRun, handler, 100);
    await vi.advanceTimersByTimeAsync(200);

    expect(handler).toHaveBeenCalledOnce();
    unsubscribe();
  });
});
