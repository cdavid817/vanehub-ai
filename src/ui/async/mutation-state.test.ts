// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { useMutationRegistry } from "./mutation-state";

describe("useMutationRegistry", () => {
  it("tracks pending state independently per target key", () => {
    const { result } = renderHook(() => useMutationRegistry());
    act(() => result.current.begin("row-1"));
    act(() => result.current.begin("row-2", "op-42"));
    expect(result.current.get("row-1")).toEqual({ targetKey: "row-1", operationId: undefined, pending: true });
    expect(result.current.get("row-2")).toEqual({ targetKey: "row-2", operationId: "op-42", pending: true });
  });

  it("clears a target from the registry on success rather than leaving a stale entry", () => {
    const { result } = renderHook(() => useMutationRegistry());
    act(() => result.current.begin("row-1"));
    act(() => result.current.succeed("row-1"));
    expect(result.current.get("row-1")).toBeUndefined();
    expect(result.current.registry.size).toBe(0);
  });

  it("preserves the operation id when a pending mutation transitions to failed", () => {
    const { result } = renderHook(() => useMutationRegistry());
    act(() => result.current.begin("row-1", "op-7"));
    act(() => result.current.fail("row-1", { kind: "error", message: "Update failed.", retryable: true }));
    expect(result.current.get("row-1")).toEqual({
      targetKey: "row-1",
      operationId: "op-7",
      pending: false,
      error: { kind: "error", message: "Update failed.", retryable: true },
    });
  });

  it("does not disturb other targets when one target's mutation fails", () => {
    const { result } = renderHook(() => useMutationRegistry());
    act(() => result.current.begin("row-1"));
    act(() => result.current.begin("row-2"));
    act(() => result.current.fail("row-1", { kind: "error", message: "boom", retryable: false }));
    expect(result.current.get("row-2")).toEqual({ targetKey: "row-2", operationId: undefined, pending: true });
  });

  it("clears an explicit target on dismiss", () => {
    const { result } = renderHook(() => useMutationRegistry());
    act(() => result.current.fail("row-1", { kind: "error", message: "boom", retryable: false }));
    act(() => result.current.clear("row-1"));
    expect(result.current.get("row-1")).toBeUndefined();
  });
});
