// @vitest-environment jsdom

import { fireEvent, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { renderWithAppProviders } from "../../test/render";
import { FilterPopover, type FilterField } from "./FilterPopover";

// Typed as the bare `FilterField` (i.e. `FilterField<string>`), matching `FilterPopoverProps.fields`
// exactly. A narrower per-field literal union (e.g. "all" | "active" | "archived") looks more
// precise but fails to widen back into `FilterField[]`: `onChange` is a function-typed property, so
// it is checked contravariantly, and `(value: "all" | "active" | "archived") => void` is not
// assignable to `(value: string) => void` under strict mode.
function statusField(overrides: Partial<FilterField> = {}): FilterField {
  return {
    id: "status",
    label: "Status",
    options: [
      { value: "all", label: "All statuses" },
      { value: "active", label: "Active" },
      { value: "archived", label: "Archived" },
    ],
    value: "all",
    defaultValue: "all",
    onChange: vi.fn(),
    ...overrides,
  };
}

function sourceField(overrides: Partial<FilterField> = {}): FilterField {
  return {
    id: "source",
    label: "Source",
    options: [
      { value: "all", label: "All sources" },
      { value: "git", label: "Git" },
      { value: "manual", label: "Manual" },
    ],
    value: "all",
    defaultValue: "all",
    onChange: vi.fn(),
    ...overrides,
  };
}

describe("FilterPopover", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("always shows the trigger with its label and no count badge when every field is at its default", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField(), sourceField()]} triggerLabel="Filters" />);
    expect(screen.getByRole("button", { name: /Filters/ })).toBeTruthy();
    expect(screen.queryByTestId("filter-popover-count")).toBeNull();
  });

  it("badges the trigger with the count of fields away from their default, and only those", () => {
    renderWithAppProviders(
      <FilterPopover fields={[statusField({ value: "active" }), sourceField()]} triggerLabel="Filters" />,
    );
    expect(screen.getByTestId("filter-popover-count").textContent).toBe("1");
  });

  it("opens the popover on trigger click and renders one select per field using ucd-input styling", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField(), sourceField()]} triggerLabel="Filters" />);
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);

    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    const selects = screen.getAllByRole("combobox");
    expect(selects).toHaveLength(2);
    for (const select of selects) expect(select.className).toContain("ucd-input");
  });

  it("toggles the popover closed on a second trigger click", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField()]} triggerLabel="Filters" />);
    const trigger = screen.getByRole("button", { name: /Filters/ });
    fireEvent.click(trigger);
    expect(screen.getAllByRole("combobox")).toHaveLength(1);
    fireEvent.click(trigger);
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
  });

  it("closes the popover when a pointer event lands outside it", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField()]} triggerLabel="Filters" />);
    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    expect(screen.getAllByRole("combobox")).toHaveLength(1);

    fireEvent.pointerDown(document.body);
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
  });

  it("closes on Escape from within the popover and returns focus to the trigger", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField()]} triggerLabel="Filters" />);
    const trigger = screen.getByRole("button", { name: /Filters/ });
    fireEvent.click(trigger);
    const [select] = screen.getAllByRole("combobox");
    fireEvent.keyDown(select, { key: "Escape" });

    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
    expect(document.activeElement).toBe(trigger);
  });

  it("does not close on Escape dispatched outside the popover, unlike a document-level modal trap", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField()]} triggerLabel="Filters" />);
    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    expect(screen.getAllByRole("combobox")).toHaveLength(1);

    fireEvent.keyDown(document.body, { key: "Escape" });
    expect(screen.getAllByRole("combobox")).toHaveLength(1);
  });

  it("calls the field's onChange with the newly selected option's value", () => {
    const onChange = vi.fn();
    renderWithAppProviders(<FilterPopover fields={[statusField({ onChange })]} triggerLabel="Filters" />);
    fireEvent.click(screen.getByRole("button", { name: /Filters/ }));
    fireEvent.change(screen.getByRole("combobox", { name: "Status" }), { target: { value: "active" } });
    expect(onChange).toHaveBeenCalledWith("active");
  });

  it("renders a chip only for fields away from their default, formatted as 'label: option label'", () => {
    renderWithAppProviders(
      <FilterPopover fields={[statusField({ value: "active" }), sourceField()]} triggerLabel="Filters" />,
    );
    expect(screen.getByText("Status: Active")).toBeTruthy();
    expect(screen.queryByText(/^Source:/)).toBeNull();
  });

  it("keeps a field at its default value chip-free, and keeps chips visible while the popover is closed", () => {
    renderWithAppProviders(<FilterPopover fields={[statusField({ value: "archived" })]} triggerLabel="Filters" />);
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
    expect(screen.getByText("Status: Archived")).toBeTruthy();
  });

  it("dismisses a single field via its chip's close button, without opening the popover", () => {
    const onChange = vi.fn();
    renderWithAppProviders(<FilterPopover fields={[statusField({ value: "active", onChange })]} triggerLabel="Filters" />);
    fireEvent.click(screen.getByRole("button", { name: "Clear Status" }));

    expect(onChange).toHaveBeenCalledWith("all");
    expect(screen.queryAllByRole("combobox")).toHaveLength(0);
  });

  it("dismisses only the targeted field's chip, leaving other active fields untouched", () => {
    const statusOnChange = vi.fn();
    const sourceOnChange = vi.fn();
    renderWithAppProviders(
      <FilterPopover
        fields={[
          statusField({ value: "active", onChange: statusOnChange }),
          sourceField({ value: "git", onChange: sourceOnChange }),
        ]}
        triggerLabel="Filters"
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: "Clear Status" }));

    expect(statusOnChange).toHaveBeenCalledWith("all");
    expect(sourceOnChange).not.toHaveBeenCalled();
    expect(screen.getByText("Source: Git")).toBeTruthy();
  });
});
