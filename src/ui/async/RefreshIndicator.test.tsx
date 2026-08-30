// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { RefreshIndicator } from "./RefreshIndicator";

describe("RefreshIndicator", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders nothing when neither refreshing nor stale", () => {
    const { container } = render(<RefreshIndicator refreshing={false} />);
    expect(container.firstChild).toBeNull();
  });

  it("announces an active refresh", () => {
    render(<RefreshIndicator refreshing />);
    expect(screen.getByRole("status").textContent).toContain("Refreshing");
  });

  it("announces stale data distinctly from an active refresh", () => {
    render(<RefreshIndicator refreshing={false} stale />);
    expect(screen.getByRole("status").textContent).toContain("Showing saved data");
  });

  it("prioritizes the refreshing message when both refreshing and stale are true", () => {
    render(<RefreshIndicator refreshing stale />);
    expect(screen.getByRole("status").textContent).toContain("Refreshing");
  });
});
