import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import { clampSessionSidebarWidth } from "./main-layout";

const styles = readFileSync(new URL("../styles.css", import.meta.url), "utf8");
const layout = readFileSync(new URL("./main-layout.tsx", import.meta.url), "utf8");

describe("workspace column separation", () => {
  it("reserves the gutter inside the sidebar column instead of hanging off its edge", () => {
    // The resize handle used to sit at `right: -5px`, which put it — and anything else the
    // sidebar overflowed — under the conversation column's opaque background.
    expect(styles).toContain("--session-sidebar-gutter: 10px;");
    expect(styles).toContain("padding-right: var(--session-sidebar-gutter, 10px);");
    expect(styles).toMatch(/\.ucd-session-sidebar-resize \{[^}]*right: 0;/);
    expect(styles).not.toMatch(/\.ucd-session-sidebar-resize \{[^}]*right: -/);
  });

  it("drops the gutter when the sidebar is collapsed", () => {
    // A border-box element cannot shrink below its own padding, so leaving the gutter on kept a
    // collapsed sidebar 11px wide inside a zero-width column — the desktop smoke caught it.
    expect(styles).toMatch(
      /\.ucd-workspace-grid\[data-session-collapsed="true"\] \.ucd-session-sidebar-shell \{[^}]*padding-right: 0;/,
    );
  });

  it("raises the sidebar shell above the conversation column", () => {
    // Both live in the same stacking context and the conversation column comes later in the DOM,
    // so without this an expanded sidebar menu is painted over rather than clipped.
    expect(styles).toMatch(/\.ucd-session-sidebar-shell \{[^}]*z-index: 10;/);
    expect(layout).toContain("ucd-session-sidebar-shell");
  });

  it("floors the sidebar wide enough to hold a row's trailing content beside the gutter", () => {
    // The floor grew with the gutter: the column now pays for the separation, so the content
    // width a session row gets is unchanged.
    expect(clampSessionSidebarWidth(0)).toBe(232);
    expect(styles).toContain("minmax(232px, min(var(--session-sidebar-width, 232px), 42vw))");
  });
});
