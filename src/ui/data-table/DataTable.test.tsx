// @vitest-environment jsdom

import { render, screen } from "@testing-library/react";
import { beforeAll, describe, expect, it, vi } from "vitest";
import { activateAppLanguage } from "../../i18n";
import { DataTable } from "./DataTable";
import type { DataTableColumn } from "./types";

interface Run {
  id: string;
  name: string;
}

describe("DataTable", () => {
  beforeAll(async () => {
    await activateAppLanguage("en");
    // jsdom does not implement ResizeObserver; this repo's convention (shell-tab.test.tsx) is a
    // no-op stub — compact-mode composition is covered directly against DataTableBody instead.
    globalThis.ResizeObserver = class {
      observe() {}
      disconnect() {}
    } as unknown as typeof ResizeObserver;
  });

  it("renders its rows through the non-compact table by default", () => {
    const columns: DataTableColumn<Run>[] = [{ id: "name", header: "Name", cell: (row) => row.name }];
    render(<DataTable ariaLabel="Runs" columns={columns} onSortChange={vi.fn()} rowKey={(row) => row.id} rows={[{ id: "r1", name: "Nightly build" }]} />);
    expect(screen.getByRole("table", { name: "Runs" })).toBeTruthy();
    expect(screen.getByText("Nightly build")).toBeTruthy();
  });
});
