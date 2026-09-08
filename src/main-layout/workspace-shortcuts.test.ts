// @vitest-environment jsdom

import { renderHook } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import {
  formatActivityShortcut,
  isTextEntryTarget,
  matchWorkspaceShortcut,
  shortcutKeyDescriptor,
  useWorkspaceShortcuts,
} from "./workspace-shortcuts";

function keydown(target: EventTarget, init: KeyboardEventInit) {
  const event = new KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init });
  target.dispatchEvent(event);
  return event;
}

describe("workspace shortcuts", () => {
  it("spells the modifier the way the platform does", () => {
    expect(formatActivityShortcut("Mod+1", false)).toBe("Ctrl+1");
    expect(formatActivityShortcut("Mod+1", true)).toBe("⌘1");
    expect(shortcutKeyDescriptor("Mod+2", false)).toBe("Control+2");
    expect(shortcutKeyDescriptor("Mod+2", true)).toBe("Meta+2");
  });

  it("treats text inputs, textareas, selects, editable regions, and the composer as text entry", () => {
    const textarea = document.createElement("textarea");
    const text = document.createElement("input");
    const checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    const button = document.createElement("button");
    const composer = document.createElement("div");
    composer.setAttribute("data-composer", "");
    const inner = document.createElement("span");
    composer.append(inner);
    expect(isTextEntryTarget(textarea)).toBe(true);
    expect(isTextEntryTarget(text)).toBe(true);
    expect(isTextEntryTarget(document.createElement("select"))).toBe(true);
    expect(isTextEntryTarget(inner)).toBe(true);
    expect(isTextEntryTarget(checkbox)).toBe(false);
    expect(isTextEntryTarget(button)).toBe(false);
    expect(isTextEntryTarget(null)).toBe(false);
  });

  it("matches Mod+N with either modifier and rejects other chords", () => {
    const bindings = [{ shortcut: "Mod+1", onSelect: vi.fn() }, { shortcut: "Mod+2", onSelect: vi.fn() }];
    expect(matchWorkspaceShortcut(new KeyboardEvent("keydown", { key: "2", ctrlKey: true }), bindings)).toBe(bindings[1]);
    expect(matchWorkspaceShortcut(new KeyboardEvent("keydown", { key: "1", metaKey: true }), bindings)).toBe(bindings[0]);
    expect(matchWorkspaceShortcut(new KeyboardEvent("keydown", { key: "1" }), bindings)).toBeNull();
    expect(matchWorkspaceShortcut(new KeyboardEvent("keydown", { key: "1", ctrlKey: true, shiftKey: true }), bindings)).toBeNull();
    expect(matchWorkspaceShortcut(new KeyboardEvent("keydown", { key: "9", ctrlKey: true }), bindings)).toBeNull();
  });

  it("fires the bound entry from the shell but stays out of text entry", () => {
    const onSelect = vi.fn();
    const bindings = [{ shortcut: "Mod+3", onSelect }];
    const textarea = document.createElement("textarea");
    document.body.append(textarea);
    renderHook(() => useWorkspaceShortcuts(bindings));

    const handled = keydown(document.body, { key: "3", ctrlKey: true });
    expect(onSelect).toHaveBeenCalledOnce();
    expect(handled.defaultPrevented).toBe(true);

    keydown(textarea, { key: "3", ctrlKey: true });
    expect(onSelect).toHaveBeenCalledOnce();
    keydown(document.body, { key: "3", ctrlKey: true, repeat: true });
    expect(onSelect).toHaveBeenCalledOnce();
    textarea.remove();
  });
});
