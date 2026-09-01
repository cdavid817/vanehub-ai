// @vitest-environment jsdom

import { act, renderHook } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { useSettingsAnchorHighlight } from "./use-settings-anchor-highlight";

describe("useSettingsAnchorHighlight", () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    document.body.innerHTML = "";
    vi.useRealTimers();
  });

  it("scrolls to, focuses, and highlights an element already present, then calls onConsumed", () => {
    const target = document.createElement("div");
    target.id = "field-anchor";
    target.tabIndex = -1;
    target.scrollIntoView = vi.fn();
    target.focus = vi.fn();
    document.body.appendChild(target);
    const onConsumed = vi.fn();

    renderHook(() => useSettingsAnchorHighlight("field-anchor", onConsumed));

    expect(target.scrollIntoView).toHaveBeenCalledWith({ behavior: "smooth", block: "center" });
    expect(target.focus).toHaveBeenCalledWith({ preventScroll: true });
    expect(target.classList.contains("ring-2")).toBe(true);
    expect(onConsumed).toHaveBeenCalledTimes(1);

    act(() => vi.advanceTimersByTime(2000));
    expect(target.classList.contains("ring-2")).toBe(false);
  });

  it("polls for a not-yet-rendered element (a still-loading lazy page) and finds it once it appears", () => {
    const onConsumed = vi.fn();
    renderHook(() => useSettingsAnchorHighlight("late-anchor", onConsumed));
    expect(onConsumed).not.toHaveBeenCalled();

    act(() => vi.advanceTimersByTime(300));
    const target = document.createElement("div");
    target.id = "late-anchor";
    target.scrollIntoView = vi.fn();
    target.focus = vi.fn();
    document.body.appendChild(target);

    act(() => vi.advanceTimersByTime(100));
    expect(target.scrollIntoView).toHaveBeenCalled();
    expect(onConsumed).toHaveBeenCalledTimes(1);
  });

  it("gives up and still calls onConsumed once the bounded poll window elapses", () => {
    const onConsumed = vi.fn();
    renderHook(() => useSettingsAnchorHighlight("never-appears", onConsumed));

    act(() => vi.advanceTimersByTime(50 * 100 + 50));
    expect(onConsumed).toHaveBeenCalledTimes(1);
  });

  it("does nothing when anchorId is null", () => {
    const onConsumed = vi.fn();
    renderHook(() => useSettingsAnchorHighlight(null, onConsumed));
    act(() => vi.advanceTimersByTime(10_000));
    expect(onConsumed).not.toHaveBeenCalled();
  });

  it("stops polling once unmounted, so a later-appearing element is never touched", () => {
    const onConsumed = vi.fn();
    const { unmount } = renderHook(() => useSettingsAnchorHighlight("after-unmount", onConsumed));
    unmount();

    const target = document.createElement("div");
    target.id = "after-unmount";
    target.scrollIntoView = vi.fn();
    document.body.appendChild(target);
    act(() => vi.advanceTimersByTime(10_000));

    expect(target.scrollIntoView).not.toHaveBeenCalled();
    expect(onConsumed).not.toHaveBeenCalled();
  });
});
