import { QueryClient, QueryObserver } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { LoopRun } from "../types/loop";
import { applyLoopRunUpdate, loopQueryKeys, preserveLoopRuns } from "./loop-query";

const first = { id: "run-1", status: "running" } as LoopRun;
const second = { id: "run-2", status: "succeeded" } as LoopRun;

describe("Loop query model", () => {
  it("uses stable hierarchical keys", () => {
    expect(loopQueryKeys.definitions).toEqual(["loops", "definitions"]);
    expect(loopQueryKeys.runs("definition-1")).toEqual(["loops", "runs", "definition-1"]);
    expect(loopQueryKeys.run("run-1")).toEqual(["loops", "run", "run-1"]);
  });

  it("retains loaded history while a new filtered history is pending", () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const observer = new QueryObserver<LoopRun[]>(client, {
      queryKey: loopQueryKeys.runs(),
      queryFn: async () => [first],
      initialData: [first],
      placeholderData: preserveLoopRuns,
    });
    const unsubscribe = observer.subscribe(() => undefined);

    observer.setOptions({
      queryKey: loopQueryKeys.runs("definition-2"),
      queryFn: () => new Promise<LoopRun[]>(() => undefined),
      placeholderData: preserveLoopRuns,
    });

    expect(observer.getCurrentResult().data).toEqual([first]);
    expect(observer.getCurrentResult().isPlaceholderData).toBe(true);
    unsubscribe();
    client.clear();
  });

  it("returns the same loaded history snapshot during refresh retention", () => {
    const history = [first, second];
    expect(preserveLoopRuns(history)).toBe(history);
    expect(preserveLoopRuns(undefined)).toBeUndefined();
  });

  it("updates a loaded run without dropping surrounding history", () => {
    const updated = { ...first, status: "paused" as const };
    expect(applyLoopRunUpdate([first, second], updated)).toEqual([updated, second]);
    const unrelated = [second];
    expect(applyLoopRunUpdate(unrelated, updated)).toBe(unrelated);
    expect(applyLoopRunUpdate(undefined, updated)).toBeUndefined();
  });
});
