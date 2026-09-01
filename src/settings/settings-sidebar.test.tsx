// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import "../i18n";
import { SettingsSidebar } from "./settings-sidebar";

describe("SettingsSidebar", () => {
  it("keeps a selected entry inside the sidebar instead of widening the column past it", () => {
    render(<SettingsSidebar activePageId="agent-configurations" onSelectPage={vi.fn()} />);

    const navigation = screen.getByRole("navigation");
    // An auto-sized grid column takes its minimum from content, so a nowrap label longer than the
    // sidebar used to widen the column past the scroll container and clip the selected entry's
    // rounded highlight on both edges. The floor is what prevents that.
    expect(navigation.className).toContain("grid-cols-[minmax(0,1fr)]");

    const active = screen
      .getAllByRole("button")
      .find((button) => button.getAttribute("class")?.includes("nav-active-soft"));
    expect(active).toBeTruthy();
    const label = active?.querySelector("span.truncate");
    expect(label).toBeTruthy();
    // The full name has to survive truncation for pointer hover and assistive technology.
    expect(label?.getAttribute("title")).toBe(label?.textContent);
  });

  it("shows exactly one bounded status dot on the entry it belongs to (task 12.16)", () => {
    render(
      <SettingsSidebar
        activePageId="agent-configurations"
        onSelectPage={vi.fn()}
        pageStatuses={{ mcp: { kind: "error", labelKey: "cliParameters.error.status" } }}
      />,
    );

    const mcpButton = screen.getByRole("button", { name: /MCP/ });
    expect(mcpButton.querySelector(".bg-danger")).toBeTruthy();
    // Only the flagged entry gets a dot -- every other rendered page has nothing true to report.
    const otherButtons = screen.getAllByRole("button").filter((button) => button !== mcpButton);
    expect(otherButtons.every((button) => !button.querySelector(".bg-danger, .bg-blocked, .bg-attention, .bg-warning, .bg-information"))).toBe(true);
  });
});
