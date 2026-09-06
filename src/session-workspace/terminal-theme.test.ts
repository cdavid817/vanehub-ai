// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { createTerminalTheme } from "./terminal-theme";

function stubComputedStyle(values: Map<Element, Record<string, string>>) {
  return vi.spyOn(window, "getComputedStyle").mockImplementation((element) => ({
    getPropertyValue: (name: string) => values.get(element as Element)?.[name] ?? "",
  }) as unknown as CSSStyleDeclaration);
}

describe("createTerminalTheme", () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it("reads the palette from the element it is given, not from the document root", () => {
    const host = document.createElement("div");
    stubComputedStyle(new Map<Element, Record<string, string>>([
      [document.documentElement, { "--terminal-background": "#0d1117", "--terminal-foreground": "#c9d1d9" }],
      [host, { "--terminal-background": "#ffffff", "--terminal-foreground": "#1f2328", "--terminal-scrollbar-slider": "#8c959f66" }],
    ]));

    const scoped = createTerminalTheme(host);
    expect(scoped.background).toBe("#ffffff");
    expect(scoped.foreground).toBe("#1f2328");
    expect(scoped.cursorAccent).toBe("#ffffff");
    expect(scoped.scrollbarSliderBackground).toBe("#8c959f66");

    // Existing callers that pass nothing keep the root palette.
    const root = createTerminalTheme();
    expect(root.background).toBe("#0d1117");
    expect(root.foreground).toBe("#c9d1d9");
  });

  it("falls back to the dark defaults when no variable resolves, never to a transparent value", () => {
    stubComputedStyle(new Map<Element, Record<string, string>>());
    const theme = createTerminalTheme(document.createElement("div"));
    expect(theme.background).toBe("#0d1117");
    expect(theme.selectionForeground).toBe("#f0f6fc");
    expect(theme.brightWhite).toBe("#f0f6fc");
    expect(Object.values(theme)).not.toContain("rgba(0, 0, 0, 0)");
  });
});
