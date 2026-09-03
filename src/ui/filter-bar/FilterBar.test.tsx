// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { FilterBar, type FilterDefinition } from "./FilterBar";

const STATUS_FILTER: FilterDefinition = {
  id: "status",
  label: "Status",
  formatValue: (value) => (value === "blocked" ? "Blocked" : String(value)),
};

describe("FilterBar", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders a chip per active filter using its definition's label and formatted value", () => {
    render(
      <FilterBar
        active={[{ definitionId: "status", value: "blocked" }]}
        definitions={[STATUS_FILTER]}
        onClearAll={vi.fn()}
        onClearOne={vi.fn()}
        resultCount={4}
      />,
    );
    expect(screen.getByText("Status: Blocked")).toBeTruthy();
  });

  it("clears a single filter via its chip", () => {
    const onClearOne = vi.fn();
    render(
      <FilterBar
        active={[{ definitionId: "status", value: "blocked" }]}
        definitions={[STATUS_FILTER]}
        onClearAll={vi.fn()}
        onClearOne={onClearOne}
        resultCount={4}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Clear Status" }));
    expect(onClearOne).toHaveBeenCalledWith("status");
  });

  it("only offers Clear all when at least one filter is active", () => {
    const withoutFilters = render(<FilterBar active={[]} definitions={[STATUS_FILTER]} onClearAll={vi.fn()} onClearOne={vi.fn()} resultCount={10} />);
    expect(withoutFilters.queryByRole("button", { name: "Clear all filters" })).toBeNull();
    withoutFilters.unmount();

    const onClearAll = vi.fn();
    render(
      <FilterBar
        active={[{ definitionId: "status", value: "blocked" }]}
        definitions={[STATUS_FILTER]}
        onClearAll={onClearAll}
        onClearOne={vi.fn()}
        resultCount={4}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Clear all filters" }));
    expect(onClearAll).toHaveBeenCalledOnce();
  });

  it("shows a plain count with no total, and an 'of total' count when filtered", () => {
    const plain = render(<FilterBar active={[]} definitions={[]} onClearAll={vi.fn()} onClearOne={vi.fn()} resultCount={7} />);
    expect(plain.getByText("7 results")).toBeTruthy();
    plain.unmount();

    render(<FilterBar active={[]} definitions={[]} onClearAll={vi.fn()} onClearOne={vi.fn()} resultCount={3} totalCount={12} />);
    expect(screen.getByText("3 of 12 results")).toBeTruthy();
  });

  it("does not render a chip for an active filter whose definition is missing", () => {
    render(
      <FilterBar
        active={[{ definitionId: "unknown-filter", value: "x" }]}
        definitions={[STATUS_FILTER]}
        onClearAll={vi.fn()}
        onClearOne={vi.fn()}
        resultCount={4}
      />,
    );
    expect(screen.queryByText(/unknown-filter/)).toBeNull();
  });

  it("gives both the per-chip clear and Clear all a visible focus ring, matching every other interactive src/ui/ control", () => {
    render(
      <FilterBar
        active={[{ definitionId: "status", value: "blocked" }]}
        definitions={[STATUS_FILTER]}
        onClearAll={vi.fn()}
        onClearOne={vi.fn()}
        resultCount={4}
      />,
    );
    expect(screen.getByRole("button", { name: "Clear Status" }).className).toContain("ucd-focus-ring");
    expect(screen.getByRole("button", { name: "Clear all filters" }).className).toContain("ucd-focus-ring");
  });
});
