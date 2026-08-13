import { afterEach, describe, expect, it, vi } from "vitest";
import type { PlanRunDetail } from "../types/plan";
import { subscribePlanRunPolling } from "./plan-run-polling";

function run(status: PlanRunDetail["status"], updatedAt: string): PlanRunDetail {
  return { id: "run-1", status, updatedAt, completedTasks: status === "running" ? 0 : 1 } as PlanRunDetail;
}

describe("subscribePlanRunPolling", () => {
  afterEach(() => vi.useRealTimers());

  it("emits changed bounded projections and stops cleanly", async () => {
    vi.useFakeTimers();
    const load = vi.fn<() => Promise<PlanRunDetail>>()
      .mockResolvedValueOnce(run("running", "first"))
      .mockResolvedValueOnce(run("running", "first"))
      .mockResolvedValue(run("awaiting_acceptance", "second"));
    const handler = vi.fn();
    const unsubscribe = subscribePlanRunPolling(load, handler, 100);
    await vi.advanceTimersByTimeAsync(200);
    expect(handler).toHaveBeenCalledOnce();
    expect(handler.mock.calls[0]?.[0].status).toBe("awaiting_acceptance");
    unsubscribe();
    await vi.advanceTimersByTimeAsync(200);
    expect(load).toHaveBeenCalledTimes(3);
  });
});
