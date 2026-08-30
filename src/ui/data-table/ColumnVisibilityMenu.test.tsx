// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { ColumnVisibilityMenu } from "./ColumnVisibilityMenu";

const COLUMNS = [{ id: "name", header: "Name" }, { id: "status", header: "Status" }];

describe("ColumnVisibilityMenu", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("stays closed until the trigger is activated", () => {
    render(<ColumnVisibilityMenu columns={COLUMNS} onVisibleColumnIdsChange={vi.fn()} visibleColumnIds={["name", "status"]} />);
    expect(screen.queryByRole("checkbox")).toBeNull();
    fireEvent.click(screen.getByRole("button", { name: "Columns" }));
    expect(screen.getAllByRole("checkbox")).toHaveLength(2);
  });

  it("reflects the current visibility per column and toggles it", () => {
    const onVisibleColumnIdsChange = vi.fn();
    render(<ColumnVisibilityMenu columns={COLUMNS} onVisibleColumnIdsChange={onVisibleColumnIdsChange} visibleColumnIds={["name"]} />);
    fireEvent.click(screen.getByRole("button", { name: "Columns" }));
    const nameCheckbox = screen.getByRole("checkbox", { name: "Name" });
    const statusCheckbox = screen.getByRole("checkbox", { name: "Status" });
    expect((nameCheckbox as HTMLInputElement).checked).toBe(true);
    expect((statusCheckbox as HTMLInputElement).checked).toBe(false);

    fireEvent.click(statusCheckbox);
    expect(onVisibleColumnIdsChange).toHaveBeenCalledWith(["name", "status"]);
  });

  it("disables unchecking the last remaining visible column, and visibly explains why", () => {
    render(<ColumnVisibilityMenu columns={COLUMNS} onVisibleColumnIdsChange={vi.fn()} visibleColumnIds={["name"]} />);
    fireEvent.click(screen.getByRole("button", { name: "Columns" }));
    expect((screen.getByRole("checkbox", { name: "Name" }) as HTMLInputElement).disabled).toBe(true);
    expect(screen.getByText("At least one column must stay visible")).toBeTruthy();
  });

  it("closes when a pointer event lands outside the menu", () => {
    render(<ColumnVisibilityMenu columns={COLUMNS} onVisibleColumnIdsChange={vi.fn()} visibleColumnIds={["name", "status"]} />);
    fireEvent.click(screen.getByRole("button", { name: "Columns" }));
    expect(screen.getAllByRole("checkbox")).toHaveLength(2);
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole("checkbox")).toBeNull();
  });

  it("closes on Escape and returns focus to the trigger", () => {
    render(<ColumnVisibilityMenu columns={COLUMNS} onVisibleColumnIdsChange={vi.fn()} visibleColumnIds={["name", "status"]} />);
    const trigger = screen.getByRole("button", { name: "Columns" });
    fireEvent.click(trigger);
    const [firstCheckbox] = screen.getAllByRole("checkbox");
    fireEvent.keyDown(firstCheckbox, { key: "Escape" });
    expect(screen.queryByRole("checkbox")).toBeNull();
    expect(document.activeElement).toBe(trigger);
  });
});
