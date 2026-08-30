// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { QualityDestination } from "./quality-destination";

vi.mock("../components/lazy-feature", () => ({
  LazyFeature: () => <div data-testid="lazy-feature" />,
}));

describe("QualityDestination", () => {
  it("renders a single lazy-loaded panel and no secondary navigation", () => {
    render(<QualityDestination />);
    expect(screen.getByTestId("lazy-feature")).toBeTruthy();
    expect(screen.queryByRole("tablist")).toBeNull();
    expect(screen.queryByRole("tab")).toBeNull();
  });
});
