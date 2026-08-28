import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { clampSessionSidebarWidth } from "./main-layout";

const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");
const layout = readFileSync(new URL("./main-layout.tsx", import.meta.url), "utf8");

describe("workspace column separation", () => {
  it("reserves a real gap on the conversation column instead of covering sidebar content", () => {
    // The resize handle used to sit at `right: -5px`, which put it — and anything else the
    // sidebar overflowed — under the conversation column's opaque background.
    expect(styles).toContain("--session-conversation-gap: 12px;");
    expect(styles).toMatch(/\.ucd-conversation-shell \{[^}]*margin-left: var\(--session-conversation-gap, 12px\);/);
    expect(styles).toMatch(/\.ucd-session-sidebar-resize \{[^}]*right: 0;/);
    expect(styles).not.toMatch(/\.ucd-session-sidebar-resize \{[^}]*right: -/);
    expect(layout).toContain("ucd-conversation-shell");
  });

  it("drops the conversation gap when the sidebar is collapsed", () => {
    expect(styles).toMatch(
      /\.ucd-workspace-grid\[data-session-collapsed="true"\] \.ucd-conversation-shell \{[^}]*margin-left: 0;/,
    );
  });

  it("raises the sidebar shell above the conversation column", () => {
    // Both live in the same stacking context and the conversation column comes later in the DOM,
    // so without this an expanded sidebar menu is painted over rather than clipped.
    expect(styles).toMatch(/\.ucd-session-sidebar-shell \{[^}]*z-index: 10;/);
    expect(layout).toContain("ucd-session-sidebar-shell");
  });

  it("floors the sidebar wide enough to hold a row's trailing content beside the gutter", () => {
    // The external gap means the entire minimum width remains available to the sidebar itself.
    expect(clampSessionSidebarWidth(0)).toBe(232);
    expect(styles).toContain("minmax(232px, min(var(--session-sidebar-width, 232px), 42vw))");
  });
});
