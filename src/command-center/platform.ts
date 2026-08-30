/**
 * No existing platform-detection utility anywhere in this codebase (confirmed by search) — the one
 * near-precedent, `region-capture/capture-protocol-url.ts`, checks `navigator.userAgent` for an
 * unrelated, narrower purpose. `navigator.userAgentData.platform` is not universally available
 * (not in Safari/Firefox as of this writing), so `userAgent` substring matching is the only check
 * that works everywhere this app actually runs.
 */
export function isMacPlatform(): boolean {
  return typeof navigator !== "undefined" && navigator.userAgent.includes("Mac");
}

/** "⌘K" on macOS, "Ctrl+K" elsewhere — the one shortcut hint 6.2 needs to render correctly. */
export function commandCenterShortcutLabel(): string {
  return isMacPlatform() ? "⌘K" : "Ctrl+K";
}
