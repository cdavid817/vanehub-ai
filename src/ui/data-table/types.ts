import type { ReactNode } from "react";

export interface DataTableColumn<T> {
  id: string;
  /** Already-localized column header. */
  header: string;
  cell: (row: T) => ReactNode;
  sortable?: boolean;
  align?: "start" | "end";
}

export interface DataTableSort {
  columnId: string;
  direction: "asc" | "desc";
}

export interface DataTablePage {
  index: number;
  size: number;
  totalCount: number;
}

export interface DataTableRowMeta {
  /**
   * Spread directly onto the row element (`<tr>`/compact `<li>`) — e.g. `data-testid`,
   * `data-attempt-id`. The primitive has no opinion on what a caller's rows need to expose
   * externally; it only owns rendering the values through untouched.
   */
  attributes?: Record<string, string>;
  /** Row-level activation (e.g. open a detail pane) — omit for tables with no such affordance. */
  onClick?: () => void;
}
