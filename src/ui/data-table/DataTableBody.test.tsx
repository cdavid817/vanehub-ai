// @vitest-environment jsdom

import { fireEvent, render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { DataTableBody } from "./DataTableBody";
import type { DataTableColumn } from "./types";

interface Run {
  id: string;
  name: string;
  status: string;
}

const RUNS: Run[] = [
  { id: "r1", name: "Nightly build", status: "Succeeded" },
  { id: "r2", name: "Release candidate", status: "Failed" },
];

const COLUMNS: DataTableColumn<Run>[] = [
  { id: "name", header: "Name", cell: (row) => row.name, sortable: true },
  { id: "status", header: "Status", cell: (row) => row.status },
];

describe("DataTableBody", () => {
  beforeAll(async () => activateAppLanguage("en"));

  it("renders a real table with a header per column when not compact", () => {
    render(<DataTableBody ariaLabel="Runs" columns={COLUMNS} compact={false} rowKey={(row) => row.id} rows={RUNS} />);
    expect(screen.getByRole("table", { name: "Runs" })).toBeTruthy();
    expect(screen.getByRole("columnheader", { name: "Name" })).toBeTruthy();
    expect(screen.getByText("Nightly build")).toBeTruthy();
  });

  it("falls back to a stacked card list when compact", () => {
    render(<DataTableBody ariaLabel="Runs" columns={COLUMNS} compact rowKey={(row) => row.id} rows={RUNS} />);
    expect(screen.queryByRole("table")).toBeNull();
    expect(screen.getByRole("list", { name: "Runs" })).toBeTruthy();
    expect(screen.getByText("Nightly build")).toBeTruthy();
    expect(screen.getAllByText("Name")).toHaveLength(RUNS.length);
  });

  it("cycles sort direction on a sortable header and ignores clicks on a non-sortable one", () => {
    const onSortChange = vi.fn();
    render(<DataTableBody ariaLabel="Runs" columns={COLUMNS} compact={false} onSortChange={onSortChange} rowKey={(row) => row.id} rows={RUNS} />);
    fireEvent.click(screen.getByRole("button", { name: /Name/ }));
    expect(onSortChange).toHaveBeenCalledWith({ columnId: "name", direction: "asc" });
    expect(screen.queryByRole("button", { name: /Status/ })).toBeNull();
  });

  it("toggles from ascending to descending on the already-sorted column", () => {
    const onSortChange = vi.fn();
    render(
      <DataTableBody
        ariaLabel="Runs"
        columns={COLUMNS}
        compact={false}
        onSortChange={onSortChange}
        rowKey={(row) => row.id}
        rows={RUNS}
        sort={{ columnId: "name", direction: "asc" }}
      />,
    );
    fireEvent.click(screen.getByRole("button", { name: /Name/ }));
    expect(onSortChange).toHaveBeenCalledWith({ columnId: "name", direction: "desc" });
  });

  it("selects individual rows and select-all independently of table density", () => {
    const onSelectedRowKeysChange = vi.fn();
    render(
      <DataTableBody
        ariaLabel="Runs"
        columns={COLUMNS}
        compact={false}
        onSelectedRowKeysChange={onSelectedRowKeysChange}
        rowKey={(row) => row.id}
        rows={RUNS}
        selectedRowKeys={new Set()}
      />,
    );
    fireEvent.click(screen.getAllByRole("checkbox", { name: "Select row" })[0]);
    expect(onSelectedRowKeysChange).toHaveBeenCalledWith(new Set(["r1"]));

    fireEvent.click(screen.getByRole("checkbox", { name: "Select all rows" }));
    expect(onSelectedRowKeysChange).toHaveBeenLastCalledWith(new Set(["r1", "r2"]));
  });

  it("hides columns not present in visibleColumnIds", () => {
    render(
      <DataTableBody
        ariaLabel="Runs"
        columns={COLUMNS}
        compact={false}
        onVisibleColumnIdsChange={vi.fn()}
        rowKey={(row) => row.id}
        rows={RUNS}
        visibleColumnIds={["name"]}
      />,
    );
    expect(screen.queryByRole("columnheader", { name: "Status" })).toBeNull();
    expect(screen.getByRole("columnheader", { name: "Name" })).toBeTruthy();
  });

  it("renders pagination and disables Previous/Next at the respective ends", () => {
    const onPageChange = vi.fn();
    render(
      <DataTableBody
        ariaLabel="Runs"
        columns={COLUMNS}
        compact={false}
        onPageChange={onPageChange}
        page={{ index: 0, size: 2, totalCount: 4 }}
        rowKey={(row) => row.id}
        rows={RUNS}
      />,
    );
    expect(screen.getByText("Page 1 of 2")).toBeTruthy();
    expect((screen.getByRole("button", { name: "Previous page" }) as HTMLButtonElement).disabled).toBe(true);
    fireEvent.click(screen.getByRole("button", { name: "Next page" }));
    expect(onPageChange).toHaveBeenCalledWith(1);
  });

  it("renders the supplied empty state instead of an empty table when there are no rows", () => {
    render(<DataTableBody ariaLabel="Runs" columns={COLUMNS} compact={false} emptyState={<p>No runs yet</p>} rowKey={(row) => row.id} rows={[]} />);
    expect(screen.getByText("No runs yet")).toBeTruthy();
    expect(screen.queryByRole("table")).toBeNull();
  });
});
