import { QueryClient, QueryObserver } from "@tanstack/react-query";
import { describe, expect, it } from "vitest";
import type { LoopDefinition, LoopRun } from "../types/loop";
import {
  applyLoopDefinitionUpdate,
  applyLoopRunUpdate,
  insertLoopDefinition,
  loopQueryKeys,
  preserveLoopRuns,
  removeLoopDefinition,
} from "./loop-query";

const first = { id: "run-1", status: "running" } as LoopRun;
const second = { id: "run-2", status: "succeeded" } as LoopRun;
const definitionA = { id: "definition-1", name: "A" } as LoopDefinition;
const definitionB = { id: "definition-2", name: "B" } as LoopDefinition;

describe("Loop query model", () => {
  it("uses stable hierarchical keys", () => {
    expect(loopQueryKeys.definitions).toEqual(["loops", "definitions"]);
    expect(loopQueryKeys.projects).toEqual(["loops", "projects"]);
    expect(loopQueryKeys.branches("D:/project")).toEqual(["loops", "branches", "D:/project"]);
    expect(loopQueryKeys.readiness("definition-1")).toEqual(["loops", "readiness", "definition-1"]);
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

  // Task 17.14: these three back `use-loop-mutations.ts`'s cache patches, replacing a whole
  // -collection `invalidateQueries` refetch (which would swap every row's object identity) with an
  // update targeted at just the row each mutation actually touched.
  it("updates a loaded definition without dropping surrounding rows", () => {
    const updated = { ...definitionA, name: "A renamed" };
    expect(applyLoopDefinitionUpdate([definitionA, definitionB], updated)).toEqual([updated, definitionB]);
    const unrelated = [definitionB];
    expect(applyLoopDefinitionUpdate(unrelated, updated)).toBe(unrelated);
    expect(applyLoopDefinitionUpdate(undefined, updated)).toBeUndefined();
  });

  it("inserts a newly created definition at the front without disturbing other rows", () => {
    const created = { id: "definition-3", name: "C" } as LoopDefinition;
    expect(insertLoopDefinition([definitionA, definitionB], created)).toEqual([created, definitionA, definitionB]);
    expect(insertLoopDefinition(undefined, created)).toBeUndefined();
  });

  it("removes a deleted definition without disturbing other rows", () => {
    expect(removeLoopDefinition([definitionA, definitionB], definitionA.id)).toEqual([definitionB]);
    const unrelated = [definitionB];
    expect(removeLoopDefinition(unrelated, "missing-id")).toEqual([definitionB]);
    expect(removeLoopDefinition(undefined, definitionA.id)).toBeUndefined();
  });
});
