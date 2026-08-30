// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { Star } from "lucide-react";
import { describe, expect, it, vi } from "vitest";
import { EmptyState, type EmptyStateVariant } from "./EmptyState";

describe("EmptyState", () => {
  it("renders caller-supplied title and description", () => {
    render(<EmptyState description="Create your first one to get started." title="No sessions yet" variant="first-run" />);
    expect(screen.getByText("No sessions yet")).toBeTruthy();
    expect(screen.getByText("Create your first one to get started.")).toBeTruthy();
  });

  it("renders an optional action", () => {
    const onClick = vi.fn();
    render(
      <EmptyState
        action={<button onClick={onClick} type="button">Create session</button>}
        title="No sessions yet"
        variant="first-run"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Create session" }));
    expect(onClick).toHaveBeenCalledOnce();
  });

  it("tags its root with the variant, and uses a distinct default icon per variant", () => {
    const variants: EmptyStateVariant[] = ["first-run", "no-data", "no-filter-match", "unsupported", "unavailable", "restricted"];
    const iconMarkup = new Set<string>();
    for (const variant of variants) {
      const { container, unmount } = render(<EmptyState title={variant} variant={variant} />);
      expect(container.querySelector(`[data-empty-state-variant="${variant}"]`)).not.toBeNull();
      iconMarkup.add(container.querySelector("svg")?.innerHTML ?? "");
      unmount();
    }
    expect(iconMarkup.size).toBe(variants.length);
  });

  it("accepts a caller override icon instead of the variant default", () => {
    const withDefault = render(<EmptyState title="Custom" variant="no-data" />);
    const defaultMarkup = withDefault.container.querySelector("svg")?.innerHTML;
    withDefault.unmount();

    render(<EmptyState icon={Star} title="Custom" variant="no-data" />);
    expect(document.querySelector("svg")?.innerHTML).not.toBe(defaultMarkup);
  });
});
