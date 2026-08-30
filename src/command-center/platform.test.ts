// @vitest-environment jsdom

import { afterEach, describe, expect, it, vi } from "vitest";
import { commandCenterShortcutLabel, isMacPlatform } from "./platform";

afterEach(() => vi.unstubAllGlobals());

function stubUserAgent(userAgent: string) {
  vi.stubGlobal("navigator", { ...navigator, userAgent });
}

describe("platform detection", () => {
  it("detects macOS from the user agent", () => {
    stubUserAgent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7)");
    expect(isMacPlatform()).toBe(true);
    expect(commandCenterShortcutLabel()).toBe("⌘K");
  });

  it("treats every other platform as non-Mac", () => {
    stubUserAgent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)");
    expect(isMacPlatform()).toBe(false);
    expect(commandCenterShortcutLabel()).toBe("Ctrl+K");

    stubUserAgent("Mozilla/5.0 (X11; Linux x86_64)");
    expect(isMacPlatform()).toBe(false);
    expect(commandCenterShortcutLabel()).toBe("Ctrl+K");
  });
});
