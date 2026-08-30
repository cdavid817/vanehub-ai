// @vitest-environment jsdom

import { useRef } from "react";
import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Toolbar } from "./Toolbar";

function ToolbarWithSearch() {
  const inputRef = useRef<HTMLInputElement>(null);
  return (
    <Toolbar
      search={<input aria-label="Search sessions" ref={inputRef} type="text" />}
      searchInputRef={inputRef}
    />
  );
}

describe("Toolbar", () => {
  it("renders the search, filter trigger, sort/view controls, and batch-mode slot", () => {
    render(
      <Toolbar
        activeFilters={<span>1 filter active</span>}
        batchModeSlot={<button type="button">Select</button>}
        filterTriggerLabel="Filter"
        onFilterTrigger={vi.fn()}
        search={<input aria-label="Search" type="text" />}
        sortControl={<span>Sort: Newest</span>}
        viewControl={<span>View: List</span>}
      />,
    );
    expect(screen.getByRole("textbox", { name: "Search" })).toBeTruthy();
    expect(screen.getByRole("button", { name: "Filter" })).toBeTruthy();
    expect(screen.getByText("1 filter active")).toBeTruthy();
    expect(screen.getByText("Sort: Newest")).toBeTruthy();
    expect(screen.getByText("View: List")).toBeTruthy();
    expect(screen.getByRole("button", { name: "Select" })).toBeTruthy();
  });

  it("calls onFilterTrigger when the filter button is activated", () => {
    const onFilterTrigger = vi.fn();
    render(<Toolbar filterTriggerLabel="Filter" onFilterTrigger={onFilterTrigger} />);
    fireEvent.click(screen.getByRole("button", { name: "Filter" }));
    expect(onFilterTrigger).toHaveBeenCalledOnce();
  });

  it("focuses the search input on the `/` shortcut", () => {
    render(<ToolbarWithSearch />);
    const input = screen.getByRole("textbox", { name: "Search sessions" });
    expect(document.activeElement).not.toBe(input);
    fireEvent.keyDown(document, { key: "/" });
    expect(document.activeElement).toBe(input);
  });

  it("does not hijack `/` while the user is already typing in an editable field", () => {
    render(
      <>
        <textarea aria-label="Composer" />
        <ToolbarWithSearch />
      </>,
    );
    const textarea = screen.getByRole("textbox", { name: "Composer" });
    textarea.focus();
    fireEvent.keyDown(textarea, { key: "/" });
    expect(document.activeElement).toBe(textarea);
  });
});
