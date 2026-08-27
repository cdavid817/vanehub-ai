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
});
