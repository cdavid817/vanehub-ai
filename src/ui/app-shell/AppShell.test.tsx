// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { AppShell } from "./AppShell";

describe("AppShell", () => {
  it("renders the top bar, activity rail, and route outlet in their own regions", () => {
    render(
      <AppShell activityRail={<nav>Activity rail</nav>} topBar={<header>Top bar</header>}>
        <p>Route content</p>
      </AppShell>,
    );
    expect(screen.getByText("Top bar")).toBeTruthy();
    expect(screen.getByText("Activity rail")).toBeTruthy();
    expect(screen.getByText("Route content")).toBeTruthy();
  });
});
