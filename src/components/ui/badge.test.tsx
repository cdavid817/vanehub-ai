// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { Badge } from "./badge";

describe("Badge", () => {
  it("renders its children as visible text", () => {
    render(<Badge tone="success">Active</Badge>);
    expect(screen.getByText("Active")).toBeTruthy();
  });

  it("applies the matching semantic tone class for every non-default tone", () => {
    const tones = ["success", "warning", "danger"] as const;
    for (const tone of tones) {
      const { container, unmount } = render(<Badge tone={tone}>{tone}</Badge>);
      expect(container.querySelector(`.ucd-status-${tone}`)).not.toBeNull();
      unmount();
    }
  });

  it("defaults to the primary tone when none is given", () => {
    render(<Badge>Default</Badge>);
    expect(screen.getByText("Default").className).toContain("bg-primary");
  });

  it("merges a caller-supplied className alongside the tone classes", () => {
    render(<Badge className="ml-2" tone="muted">Muted</Badge>);
    expect(screen.getByText("Muted").className).toContain("ml-2");
  });
});
