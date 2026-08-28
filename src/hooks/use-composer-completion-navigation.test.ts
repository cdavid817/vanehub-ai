// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useComposerCompletionNavigation } from "./use-composer-completion-navigation";

function keyEvent(key: string) {
  return {
    key,
    shiftKey: false,
    preventDefault: vi.fn(),
    nativeEvent: { isComposing: false },
  } as unknown as Parameters<
    ReturnType<typeof useComposerCompletionNavigation>["onKeyDown"]
  >[0];
}

describe("composer completion navigation", () => {
  it("moves through the list and accepts the option the keyboard is on", () => {
    const activate = vi.fn();
    const { result } = renderHook(() => useComposerCompletionNavigation(["a", "b", "c"]));

    act(() => {
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
    });
    expect(result.current.activeIndex).toBe(0);

    act(() => {
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
    });
    expect(result.current.activeIndex).toBe(1);

    act(() => {
      result.current.onKeyDown(keyEvent("Enter"), activate);
    });
    expect(activate).toHaveBeenCalledWith(1);
    expect(result.current.activeIndex).toBeNull();
  });

  /**
   * The list is rebuilt when the file search a mention token started comes back, and that can land
   * between the arrow key and Enter. Clearing the highlight there sent the message instead of
   * completing the mention, emptying the composer -- a keystroke doing something destructive
   * rather than what it was aimed at.
   */
  it("keeps the highlight when the list is rebuilt around it", () => {
    const activate = vi.fn();
    const { result, rerender } = renderHook(
      ({ identities }) => useComposerCompletionNavigation(identities),
      { initialProps: { identities: ["participant:architect", "participant:implementer"] } },
    );

    act(() => {
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
    });
    expect(result.current.activeIndex).toBe(1);

    // Late file results arrive; every participant is still offered.
    rerender({
      identities: [
        "participant:architect",
        "participant:implementer",
        "file:src/main.rs",
        "file:src/lib.rs",
      ],
    });

    expect(result.current.activeIndex).toBe(1);
    act(() => {
      result.current.onKeyDown(keyEvent("Enter"), activate);
    });
    expect(activate).toHaveBeenCalledWith(1);
  });

  it("drops the highlight only when its option is gone", () => {
    const activate = vi.fn();
    const { result, rerender } = renderHook(
      ({ identities }) => useComposerCompletionNavigation(identities),
      { initialProps: { identities: ["participant:architect", "participant:implementer"] } },
    );

    act(() => {
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
      result.current.onKeyDown(keyEvent("ArrowDown"), activate);
    });
    expect(result.current.activeIndex).toBe(1);

    rerender({ identities: ["participant:architect"] });

    expect(result.current.activeIndex).toBeNull();
    // With nothing highlighted, Enter is the composer's again rather than the list's.
    act(() => {
      expect(result.current.onKeyDown(keyEvent("Enter"), activate)).toBe(false);
    });
    expect(activate).not.toHaveBeenCalled();
  });

  it("reports no active option and handles nothing when the list is empty", () => {
    const activate = vi.fn();
    const { result } = renderHook(() => useComposerCompletionNavigation([]));

    act(() => {
      expect(result.current.onKeyDown(keyEvent("ArrowDown"), activate)).toBe(false);
    });
    expect(result.current.activeIndex).toBeNull();
    expect(result.current.activeOptionId).toBeUndefined();
  });
});
