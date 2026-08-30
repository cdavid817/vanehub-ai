// @vitest-environment jsdom

import { renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import { useCommandCenterShortcut } from "./use-command-center-shortcut";

afterEach(() => document.body.innerHTML = "");

function pressKey(init: KeyboardEventInit) {
  document.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init }));
}

describe("useCommandCenterShortcut", () => {
  it("opens on Ctrl+K", () => {
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    pressKey({ ctrlKey: true, key: "k" });
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("opens on Cmd+K (metaKey)", () => {
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    pressKey({ metaKey: true, key: "k" });
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("is case-insensitive on the key itself", () => {
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    pressKey({ ctrlKey: true, key: "K" });
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("opens even while a text field would otherwise own the keystroke", () => {
    const input = document.createElement("input");
    document.body.appendChild(input);
    input.focus();
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    document.dispatchEvent(new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ctrlKey: true, key: "k" }));
    expect(onOpen).toHaveBeenCalledTimes(1);
  });

  it("does not fire for K alone, or for other Ctrl/Cmd combos", () => {
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    pressKey({ key: "k" });
    pressKey({ ctrlKey: true, key: "s" });
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("does not fire when Alt or Shift rides along, to avoid stealing a different app's chord", () => {
    const onOpen = vi.fn();
    renderHook(() => useCommandCenterShortcut(onOpen));
    pressKey({ ctrlKey: true, altKey: true, key: "k" });
    pressKey({ metaKey: true, shiftKey: true, key: "k" });
    expect(onOpen).not.toHaveBeenCalled();
  });

  it("removes its listener on unmount", () => {
    const onOpen = vi.fn();
    const { unmount } = renderHook(() => useCommandCenterShortcut(onOpen));
    unmount();
    pressKey({ ctrlKey: true, key: "k" });
    expect(onOpen).not.toHaveBeenCalled();
  });
});
