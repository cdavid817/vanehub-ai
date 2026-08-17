import { afterEach, describe, expect, it, vi } from "vitest";
import { createMissionControlCoalescer } from "./event-coalescer";

afterEach(() => vi.useRealTimers());

describe("Mission Control event coalescer", () => {
  it("batches noisy updates by Run and rejects stale versions", () => {
    vi.useFakeTimers(); const flush = vi.fn(); const coalescer = createMissionControlCoalescer(flush, 100);
    for (let version = 1; version <= 100; version += 1) coalescer.push({ runId: "run-1", version, kind: "usage" });
    coalescer.push({ runId: "run-1", version: 50, kind: "progress" });
    vi.advanceTimersByTime(100);
    expect(flush).toHaveBeenCalledOnce(); expect(flush.mock.calls[0][0]).toEqual([{ runId: "run-1", version: 100, kind: "usage" }]);
  });

  it("flushes attention and terminal updates immediately", () => {
    vi.useFakeTimers(); const flush = vi.fn(); const coalescer = createMissionControlCoalescer(flush);
    coalescer.push({ runId: "run-1", version: 1, kind: "progress" });
    coalescer.push({ runId: "run-1", version: 2, kind: "terminal" });
    expect(flush).toHaveBeenCalledWith([{ runId: "run-1", version: 2, kind: "terminal" }]);
    coalescer.dispose(); vi.runAllTimers(); expect(flush).toHaveBeenCalledOnce();
  });
});
